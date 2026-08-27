use std::{
    collections::HashMap,
    fmt::Display,
    io::{self, BufRead, Read, Result, Write},
};

use crate::ResponseType::{BulkString, Error, Integer, NullString, SimpleString};

pub struct Context {
    kv_store: HashMap<String, String>,
}

impl Context {
    fn new() -> Self {
        Context {
            kv_store: HashMap::new(),
        }
    }

    fn set_val(&mut self, key: String, value: String) {
        self.kv_store.insert(key, value);
    }

    fn get_val(&self, key: &String) -> Option<&String> {
        self.kv_store.get(key)
    }
}

enum ResponseType<'a> {
    SimpleString(&'a str),
    BulkString(&'a str),
    NullString,
    Integer(i64),
    Error(&'a str),
}

impl<'a> Display for ResponseType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleString(s) => write!(f, "+{}\r\n", s),
            BulkString(s) => write!(f, "${}\r\n{}\r\n", s.len(), s),
            NullString => write!(f, "$-1\r\n"),
            Integer(n) => write!(f, ":{}\r\n", n),
            Error(e) => write!(f, "-{}\r\n", e),
        }
    }
}

fn dispatcher(args: &[String], ctx: &mut Context) -> String {
    let cmd = args[0].to_uppercase();
    let rest_args = &args[1..];

    let cmd = match &cmd[..] {
        "PING" => Cmd::PING,
        "ECHO" => Cmd::ECHO,
        "COMMAND" => Cmd::COMMAND,
        "SET" => Cmd::SET,
        "GET" => Cmd::GET,
        "DBSIZE" => Cmd::DBSIZE,
        _ => return format!("-ERR unknown command '{}'\r\n", cmd),
    };

    if let Some(err) = cmd.check_arity(rest_args) {
        return err;
    }

    cmd.run(rest_args, ctx)
}

pub enum Cmd {
    PING,
    ECHO,
    COMMAND,
    SET,
    GET,
    DBSIZE,
}

impl Display for Cmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Cmd::PING => write!(f, "PING"),
            Cmd::ECHO => write!(f, "ECHO"),
            Cmd::COMMAND => write!(f, "COMMAND"),
            Cmd::SET => write!(f, "SET"),
            Cmd::GET => write!(f, "GET"),
            Cmd::DBSIZE => write!(f, "DBSIZE"),
        }
    }
}
impl Cmd {
    pub fn run(&self, args: &[String], ctx: &mut Context) -> String {
        match self {
            Cmd::PING => Self::cmd_ping(args),
            Cmd::ECHO => Self::cmd_echo(args),
            Cmd::COMMAND => Self::cmd_command_docs(args, ctx),
            Cmd::SET => Self::cmd_set(args, ctx),
            Cmd::GET => Self::cmd_get(args, ctx),
            Cmd::DBSIZE => Self::cmd_dbsize(args, ctx),
        }
    }

    pub fn check_arity(&self, args: &[String]) -> Option<String> {
        let cmd = self.to_string();
        let (low, high) = self.arity();

        if !(low <= args.len() && args.len() <= high) {
            return Some(
                ResponseType::Error(&format!(
                    "ERR wrong number of arguments for '{}' command",
                    cmd
                ))
                .to_string(),
            );
        }

        None
    }

    fn arity(&self) -> (usize, usize) {
        match self {
            Cmd::PING => (0, 1),
            Cmd::ECHO => (1, 1),
            Cmd::COMMAND => (1, 1),
            Cmd::SET => (2, 4),
            Cmd::GET => (1, 1),
            Cmd::DBSIZE => (0, 0),
        }
    }

    fn cmd_ping(args: &[String]) -> String {
        if !args.is_empty() {
            ResponseType::BulkString(&args[0]).to_string()
        } else {
            ResponseType::SimpleString("PONG").to_string()
        }
    }

    fn cmd_echo(args: &[String]) -> String {
        ResponseType::BulkString(&args[0]).to_string()
    }

    fn cmd_command_docs(args: &[String], _ctx: &mut Context) -> String {
        if !args.is_empty() && args[0].to_uppercase() == "DOCS" {
            ResponseType::SimpleString("OK").to_string()
        } else {
            ResponseType::Error("ERR subcommand not impl").to_string()
        }
    }

    fn cmd_set(args: &[String], ctx: &mut Context) -> String {
        let len = args.len();
        let key = args[0].clone();
        let value = args[1].clone();

        if len == 2 {
            ctx.set_val(key, value);
            return ResponseType::SimpleString("OK").to_string();
        }

        // NOTE:conditional writes
        // the doc said in real Redis `SET key val NX XX` returna synatx error
        // and real Redis also allows 'EX/PX' alongside 'NX/XX', but i do not handle this so far
        let flag = args[2].clone().to_uppercase();
        if len == 4 {
            let flag2 = args[3].clone().to_uppercase();
            if (flag == "NX" && flag2 == "XX") || (flag == "XX" && flag2 == "NX") {
                return ResponseType::Error("ERR NX and XX are mutually exclusive").to_string();
            }
        }
        let key_exits = ctx.get_val(&key).is_some();

        let response = |flag: bool| -> String {
            if flag {
                return ResponseType::NullString.to_string();
            }

            ctx.set_val(key, value);
            ResponseType::SimpleString("OK").to_string()
        };

        response((flag == "NX" && key_exits) || (flag == "XX" && !key_exits))
    }

    fn cmd_get(args: &[String], ctx: &mut Context) -> String {
        if let Some(val) = ctx.get_val(&args[0]) {
            ResponseType::BulkString(val).to_string()
        } else {
            ResponseType::NullString.to_string()
        }
    }
    fn cmd_dbsize(args: &[String], ctx: &mut Context) -> String {
        ResponseType::Integer(ctx.kv_store.len() as i64).to_string()
    }
}

fn main() {
    let mut stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut ctx = Context::new();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let args = parse_args(&line);
        let response = dispatcher(&args, &mut ctx);
        write!(out, "{}", response).unwrap();
        out.flush().unwrap();
    }

    #[cfg(foobar_non_exist)]
    {
        loop {
            match parse_command(&mut stdin) {
                Ok(args) => {
                    if args.is_empty() {
                        continue;
                    };
                    let response = dispatcher(&args, &mut ctx);
                    write!(out, "{}", response).unwrap();
                    out.flush().unwrap();
                }
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                    break;
                }
                Err(_e) => return,
            }
        }
    }
}
fn parse_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in line.chars() {
        match ch {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => in_quotes = false,
            ' ' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        args.push(current);
    }

    args
}

fn read_line<S: Read>(stream: &mut S) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        stream.read_exact(&mut byte)?;
        if byte[0] == b'\n' && buf.ends_with(b"\r") {
            buf.pop();

            return Ok(buf);
        }

        buf.push(byte[0]);
    }
}

fn parse_command<S: Read>(stream: &mut S) -> Result<Vec<String>> {
    let mut args = Vec::new();
    let header = read_line(stream)?;

    assert_eq!(header[0], b'*');

    let text = std::str::from_utf8(&header[1..]).unwrap();
    let n: usize = text.parse().unwrap();

    let mut tail = [0_u8; 2];
    for _ in 0..n {
        let bulk_header = read_line(stream)?;
        assert_eq!(bulk_header[0], b'$');
        let bulk_text = std::str::from_utf8(&bulk_header[1..]).unwrap();
        let len: usize = bulk_text.parse().unwrap();

        // NOTE: if len == 0, vec![0_u8;]
        let mut data = vec![0_u8; len];

        stream.read_exact(&mut data)?;
        // NOTE: disacrd trailing \r\n
        stream.read_exact(&mut tail)?;

        args.push(String::from_utf8(data).unwrap());
    }

    Ok(args)
}

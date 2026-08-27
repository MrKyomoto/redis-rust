use std::{
    collections::HashMap,
    io::{self, Read, Result, Write},
};

type CmdHandler = fn(&[String], &mut Context) -> String;
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

fn build_dispatch_table() -> HashMap<&'static str, CmdHandler> {
    let mut handlers: HashMap<&'static str, CmdHandler> = HashMap::new();
    handlers.insert("PING", cmd_ping);
    handlers.insert("ECHO", cmd_echo);
    handlers.insert("COMMAND", cmd_command_docs);
    handlers.insert("SET", cmd_set);
    handlers.insert("GET", cmd_get);

    handlers
}

fn build_cmd_arity() -> HashMap<&'static str, Vec<usize>> {
    let mut arity = HashMap::new();
    arity.insert("PING", vec![0_usize, 1]);
    arity.insert("ECHO", vec![1_usize, 1]);
    arity.insert("COMMAND", vec![1_usize, 1]);
    arity.insert("SET", vec![2_usize, 2]);
    arity.insert("GET", vec![1_usize, 1]);

    arity
}

fn encode_simple_string(s: &str) -> String {
    format!("+{}\r\n", s)
}

fn encode_integer(num: i32) -> String {
    format!(":{}\r\n", num)
}

fn encode_error(msg: &str) -> String {
    format!("-{}\r\n", msg)
}

fn encode_bulk_string(s: &str) -> String {
    format!("${}\r\n{}\r\n", s.len(), s)
}

fn encode_null_string() -> String {
    format!("$-1\r\n")
}

fn dispatcher(
    args: &[String],
    table: &HashMap<&str, CmdHandler>,
    arity: &HashMap<&str, Vec<usize>>,
    ctx: &mut Context,
) -> String {
    let cmd = args[0].to_uppercase();
    let rest_args = &args[1..];

    match table.get(cmd.as_str()) {
        Some(handler) => {
            if let Some(err) = check_arity(&cmd, rest_args, &arity) {
                return err;
            }

            handler(rest_args, ctx)
        }

        None => format!("-ERR unknown command '{}'\r\n", cmd),
    }
}

fn check_arity(cmd: &str, args: &[String], arity: &HashMap<&str, Vec<usize>>) -> Option<String> {
    let low = arity[cmd][0];
    let high = arity[cmd][1];

    if !(low <= args.len() && args.len() <= high) {
        return Some(encode_error(&format!(
            "ERR wrong number of arguments for '{}' command",
            cmd
        )));
    }

    None
}

fn cmd_ping(args: &[String], _ctx: &mut Context) -> String {
    if !args.is_empty() {
        let first_arg = args.first().unwrap();
        encode_bulk_string(first_arg)
    } else {
        encode_simple_string("PONG")
    }
}

fn cmd_echo(args: &[String], _ctx: &mut Context) -> String {
    encode_bulk_string(&args[0])
}

fn cmd_command_docs(args: &[String], _ctx: &mut Context) -> String {
    if !args.is_empty() && args[0].to_uppercase() == "DOCS" {
        encode_simple_string("OK")
    } else {
        encode_error("ERR subcommand not impl")
    }
}

fn cmd_set(args: &[String], ctx: &mut Context) -> String {
    let key = args[0].clone();
    let value = args[1].clone();
    ctx.set_val(key, value);
    encode_simple_string("OK")
}

fn cmd_get(args: &[String], ctx: &mut Context) -> String {
    if let Some(val) = ctx.get_val(&args[0]) {
        encode_bulk_string(val)
    } else {
        encode_null_string()
    }
}

fn main() {
    let mut stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let table = build_dispatch_table();
    let arity = build_cmd_arity();
    let mut ctx = Context::new();

    loop {
        match parse_command(&mut stdin) {
            Ok(args) => {
                if args.is_empty() {
                    continue;
                };
                let response = dispatcher(&args, &table, &arity, &mut ctx);
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

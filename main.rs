use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
};

type CmdHandler = fn(&[String]) -> String;

fn build_dispatch_table() -> HashMap<&'static str, CmdHandler> {
    let mut handlers: HashMap<&'static str, CmdHandler> = HashMap::new();
    handlers.insert("PING", cmd_ping);
    handlers.insert("ECHO", cmd_echo);
    handlers.insert("COMMAND", cmd_command_docs);

    handlers
}

fn build_cmd_arity() -> HashMap<&'static str, Vec<usize>> {
    let mut arity = HashMap::new();
    arity.insert("PING", vec![0_usize, 1]);
    arity.insert("ECHO", vec![1_usize, 1]);

    arity
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

fn handle_command(args: &[String]) -> String {
    let table = build_dispatch_table();
    let arity = build_cmd_arity();

    let cmd = args[0].to_uppercase();
    let rest_args = &args[1..];

    match table.get(cmd.as_str()) {
        Some(handler) => {
            if let Some(err) = check_arity(&cmd, rest_args, &arity) {
                return err;
            }

            handler(rest_args)
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

fn cmd_ping(args: &[String]) -> String {
    if !args.is_empty() {
        let first_arg = args.first().unwrap();
        encode_bulk_string(first_arg)
    } else {
        encode_simple_string("PONG")
    }
}

fn cmd_echo(args: &[String]) -> String {
    if !args.is_empty() {
        // NOTE: 目前这里只选取了第一个arg而非整个
        let first_arg = args.first().unwrap();
        encode_bulk_string(first_arg)
    } else {
        encode_simple_string("ERR echo empty")
    }
}

fn cmd_command_docs(args: &[String]) -> String {
    if !args.is_empty() && args[0].to_uppercase() == "DOCS" {
        encode_simple_string("OK")
    } else {
        encode_error("ERR subcommand not impl")
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let args = parse_args(&line);
        let response = handle_command(&args);
        write!(out, "{}", response).unwrap();
        out.flush().unwrap();
    }
}

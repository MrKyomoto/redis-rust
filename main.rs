use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
};

type CmdHandler = fn(&[String]) -> String;

fn build_dispatch_table() -> HashMap<&'static str, CmdHandler> {
    let mut handlers: HashMap<&'static str, CmdHandler> = HashMap::new();
    handlers.insert("PING", cmd_ping);
    handlers.insert("ECHO", cmd_echo);

    handlers
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

fn encode_bulk_string(s: &str) -> String {
    format!("${}\r\n{}\r\n", s.len(), s)
}

fn handle_command(args: &[String]) -> String {
    let table = build_dispatch_table();
    let cmd = args[0].to_uppercase();
    let rest_args = &args[1..];

    match table.get(cmd.as_str()) {
        Some(handler) => handler(rest_args),
        None => format!("-ERR unknown command\r\n"),
    }
}

fn cmd_ping(args: &[String]) -> String {
    if !args.is_empty() {
        let first_arg = args.first().unwrap();
        encode_bulk_string(first_arg)
    } else {
        format!("+PONG\r\n")
    }
}

fn cmd_echo(args: &[String]) -> String {
    if !args.is_empty() {
        let first_arg = args.first().unwrap();
        encode_bulk_string(first_arg)
    } else {
        format!("+ECHO EMPTY ERROR\r\n")
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

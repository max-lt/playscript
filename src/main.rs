mod ast;
mod error;
mod interp;
mod lexer;
mod parser;
mod value;

use std::io::{self, Write};

use crate::error::Result;
use crate::interp::{Environment, exec};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::Value;

/// Run a whole program against `env`; return the value of the last expression.
fn run(src: &str, env: &mut Environment) -> Result<Option<Value>> {
    let tokens = tokenize(src)?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;

    let mut last = None;

    for stmt in &program {
        last = exec(stmt, env)?;
    }

    Ok(last)
}

fn repl() {
    let stdin = io::stdin();
    let mut line = String::new();
    let mut env = Environment::default(); // persists across lines

    println!("playscript v{} — Ctrl+D to quit.", env!("CARGO_PKG_VERSION"));

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        line.clear();

        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();

                if trimmed.is_empty() {
                    continue;
                }

                match run(trimmed, &mut env) {
                    Ok(Some(value)) => println!("{value}"),
                    Ok(None) => {} // e.g. `var x = 5` — nothing to print
                    Err(e) => eprintln!("error: {e}"),
                }
            }
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }
    }
}

fn main() {
    // With arguments: evaluate one program and exit.
    // Without arguments: start the REPL.
    let args: Vec<String> = std::env::args().skip(1).collect();

    if !args.is_empty() {
        let src = args.join(" ");
        let mut env = Environment::default();

        match run(&src, &mut env) {
            Ok(Some(value)) => println!("{value}"),
            Ok(None) => {}
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }

        return;
    }

    repl();
}

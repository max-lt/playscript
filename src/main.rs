mod ast;
mod error;
mod interp;
mod lexer;
mod parser;
mod value;

use std::io::{self, Write};

use crate::interp::{DEFAULT_FUEL_LIMIT, Interpreter};

fn repl() {
    let stdin = io::stdin();
    let mut line = String::new();
    let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT); // env persists across lines

    println!(
        "playscript v{} — fuel: {DEFAULT_FUEL_LIMIT} ops per line. Ctrl+D to quit.",
        env!("CARGO_PKG_VERSION")
    );

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

                match interp.run(trimmed) {
                    Ok(Some(value)) => println!("{value}  [{} ops]", interp.fuel_used()),
                    Ok(None) => println!("[{} ops]", interp.fuel_used()),
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
        let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);

        match interp.run(&src) {
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

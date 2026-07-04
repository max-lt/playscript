use std::io::{self, Write};

use playscript::{DEFAULT_FUEL_LIMIT, Interpreter, TraceEvent};

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

/// Print a recorded trace: one line per event, the op-clock in the left
/// column, indented by call depth.
fn print_trace(events: &[TraceEvent]) {
    println!("── trace: {} events ──", events.len());

    for event in events {
        let indent = "  ".repeat(event.depth);
        println!("op {:>7} │ L{:<3} {indent}{}", event.op, event.line, event.kind);
    }

    println!("──");
}

fn main() {
    // `--trace <program>` records and prints an execution trace.
    // Other args: evaluate one program and exit. No args: start the REPL.
    let args: Vec<String> = std::env::args().skip(1).collect();

    let (tracing, program) = match args.split_first() {
        Some((flag, rest)) if flag == "--trace" => (true, rest.join(" ")),
        _ => (false, args.join(" ")),
    };

    if program.is_empty() {
        repl();
        return;
    }

    let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);

    if tracing {
        interp.enable_tracing();
    }

    match interp.run(&program) {
        Ok(value) => {

            if let Some(events) = interp.trace() {
                print_trace(events);
            }

            if let Some(value) = value {
                println!("{value}");
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

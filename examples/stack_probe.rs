// Probe: how much native stack does MAX_CALL_DEPTH (256) actually need?
//
//   cargo run --example stack_probe -- <stack_kib>   spawn a thread with that stack
//   cargo run --example stack_probe -- default       plain spawn (RUST_MIN_STACK applies)
//
// If the guard fires before the stack runs out, the probe prints "guard fired";
// otherwise the process dies with SIGSEGV/SIGABRT and the caller sees the crash.

use playscript::{DEFAULT_FUEL_LIMIT, Interpreter};

fn probe() {
    let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);
    let err = interp.run("function f(n) { return f(n + 1) } f(0)").unwrap_err();
    println!("guard fired cleanly: {err}");
}

fn main() {
    let arg = std::env::args().nth(1).expect("usage: stack_probe <kib>|default");

    let handle = if arg == "default" {
        std::thread::spawn(probe)
    } else {
        let kib: usize = arg.parse().expect("stack size in KiB");
        std::thread::Builder::new()
            .stack_size(kib * 1024)
            .spawn(probe)
            .unwrap()
    };

    handle.join().unwrap();
}

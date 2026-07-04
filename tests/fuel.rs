mod common;

use common::*;

#[test]
fn ops_are_a_stable_cost_tripwire() {
    // These exact counts are part of the language's observable behavior.
    // If the cost table changes, this test is meant to notice — the ops
    // counter is the logical clock a future replay reader seeks on.
    assert_eq!(ops("1 + 2 * 3"), 6);
    assert_eq!(ops("var s = 0; var i = 1; while (i <= 100) { s = s + i; i = i + 1 }; s"), 1210);
}

#[test]
fn execution_is_deterministic() {
    let prog = "var n = 0; while (getOperations() < 1000) { n = n + 1 }; n";

    let first = {
        let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);
        let value = interp.run(prog).unwrap();
        (value, interp.fuel_used())
    };

    let second = {
        let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);
        let value = interp.run(prog).unwrap();
        (value, interp.fuel_used())
    };

    assert_eq!(first, second);
    // 52 is pinned on purpose: a change here means the cost table (the
    // logical clock) moved, which any replay format must version.
    assert_eq!(first.0, Some(num(52.0)));
}

#[test]
fn infinite_loop_runs_out_of_fuel() {
    let mut interp = Interpreter::new(50_000);
    let err = interp.run("while (true) { }").unwrap_err();
    assert!(matches!(err, LangError::OutOfFuel { limit: 50_000 }));
}

#[test]
fn recursion_is_bounded_by_fuel_too() {
    // A tiny budget stops even finite-but-heavy recursion before it finishes.
    let mut interp = Interpreter::new(500);
    let fib = "function fib(n) { if (n < 2) { return n } return fib(n - 1) + fib(n - 2) } fib(30)";
    assert!(matches!(interp.run(fib), Err(LangError::OutOfFuel { .. })));
}

#[test]
fn print_returns_its_argument_and_costs_extra() {
    // print passes its value through (the Rust dbg! idiom)...
    assert_eq!(eval("print(5) * 2"), num(10.0));
    // ...and I/O is deliberately expensive.
    assert!(ops("print(0)") >= ops("0") + 100);
}

#[test]
fn get_operations_limit_reflects_the_budget() {
    assert_eq!(eval("getOperationsLimit()"), num(DEFAULT_FUEL_LIMIT as f64));
}

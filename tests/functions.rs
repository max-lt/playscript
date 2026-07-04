mod common;

use common::*;

#[test]
fn call_and_arguments() {
    assert_eq!(eval("function add(a, b) { return a + b } add(2, 3)"), num(5.0));
}

#[test]
fn recursion() {
    let fib = "function fib(n) { if (n < 2) { return n } return fib(n - 1) + fib(n - 2) }";
    assert_eq!(eval(&format!("{fib} fib(15)")), num(610.0));
}

#[test]
fn functions_can_mutate_globals() {
    assert_eq!(
        eval("var c = 0; function bump() { c = c + 1; return c } bump(); bump(); c"),
        num(2.0),
    );
}

#[test]
fn return_escapes_nested_loop_and_if() {
    let prog = "function first(limit) { var i = 0; while (true) { if (i * i >= limit) { return i } i = i + 1 } } first(1000)";
    assert_eq!(eval(prog), num(32.0));
}

#[test]
fn scoping_is_lexical_not_dynamic() {
    // f() must NOT see the caller's local z. Dynamic scoping would return 5.
    assert!(matches!(
        eval_err("function f() { return z } { var z = 5; f() }"),
        LangError::UndefinedVariable(_),
    ));
}

#[test]
fn arity_is_checked() {
    assert!(matches!(
        eval_err("function g(a) { return a } g(1, 2)"),
        LangError::WrongArity { expected: 1, got: 2, .. },
    ));
}

#[test]
fn calling_unknown_function() {
    // Functions are variables now, so an unknown callee is an unknown name.
    assert!(matches!(eval_err("boom(1)"), LangError::UndefinedVariable(_)));
}

#[test]
fn function_must_return_a_value() {
    assert!(matches!(
        eval_err("function h() { var x = 1 } h()"),
        LangError::NoReturnValue { .. },
    ));
}

#[test]
fn return_outside_function_is_rejected() {
    assert!(matches!(eval_err("return 5"), LangError::ReturnOutsideFunction));
}

#[test]
fn runaway_recursion_hits_the_depth_limit() {
    // The depth guard must fire before infinite recursion exhausts fuel.
    // Each playscript call nests several native frames, so the guard assumes
    // a normal stack; the test harness gives each test thread only ~2 MiB,
    // so we run on a large-stack thread like the real binary (main thread).
    let handle = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(|| eval_err("function f(n) { return f(n + 1) } f(0)"))
        .unwrap();

    assert!(matches!(handle.join().unwrap(), LangError::CallDepthExceeded { .. }));
}

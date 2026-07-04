mod common;

use common::*;

#[test]
fn functions_are_values() {
    assert_eq!(
        eval("function double(x) { return x * 2 } function apply(f, x) { return f(x) } apply(double, 3)"),
        num(6.0),
    );
}

#[test]
fn lambdas() {
    assert_eq!(eval("var inc = x => x + 1; inc(41)"), num(42.0));
    assert_eq!(eval("var add = (a, b) => a + b; add(1, 2)"), num(3.0));
    assert_eq!(eval("var f = () => 7; f()"), num(7.0));
    // Block body with explicit return.
    assert_eq!(eval("var sign = n => { if (n < 0) { return -1 } return 1 }; sign(-5)"), num(-1.0));
    // Immediately invoked.
    assert_eq!(eval("(x => x * 2)(21)"), num(42.0));
}

#[test]
fn globals_stay_live_locals_are_captured_by_value() {
    // Top-level variables are globals: closures see them LIVE (this is also
    // what makes recursion and global counters work).
    assert_eq!(eval("var y = 10; var f = x => x + y; y = 999; f(1)"), num(1000.0));

    // Locals are captured BY VALUE at creation: the closure snapshots y,
    // later writes are invisible to it. No aliasing, ever.
    let prog = r#"
        function make() {
            var y = 10;
            var f = x => x + y;
            y = 999;
            return f
        }

        make()(1)
    "#;
    assert_eq!(eval(prog), num(11.0));
}

#[test]
fn calls_chain_as_postfix() {
    assert_eq!(eval("var fs = [x => x + 1, x => x * 10]; fs[1](5)"), num(50.0));
    assert_eq!(eval("var make = n => (x => x + n); make(100)(1)"), num(101.0));
}

#[test]
fn map_written_in_playscript() {
    let prog = r#"
        function map(arr, f) {
            var out = array(len(arr), 0);
            var i = 0;

            while (i < len(arr)) {
                out[i] = f(arr[i]);
                i = i + 1
            }

            return out
        }

        map([1, 2, 3], x => x * 2)
    "#;
    assert_eq!(eval(prog), eval("[2, 4, 6]"));
}

#[test]
fn maybe_monad() {
    // The user's goal: monads. Maybe as arrays — [] is None, [x] is Just x.
    // The inner lambda capturing `a` is exactly what closures are for.
    let maybe = r#"
        function unit(x) { return [x] }
        function bind(m, f) { if (len(m) == 0) { return [] } return f(m[0]) }
        function safediv(a, b) { if (b == 0) { return [] } return [a / b] }
    "#;

    // Happy path: 20 / 2 / 5 = 2.
    assert_eq!(
        eval(&format!("{maybe} bind(bind(unit(20), x => safediv(x, 2)), y => safediv(y, 5))")),
        eval("[2]"),
    );
    // Division by zero mid-chain: the whole chain collapses to None.
    assert_eq!(
        eval(&format!("{maybe} bind(bind(unit(20), x => safediv(x, 0)), y => safediv(y, 5))")),
        eval("[]"),
    );
    // Nested binds where the inner lambda captures `a`.
    assert_eq!(
        eval(&format!("{maybe} bind(unit(6), a => bind(unit(7), b => unit(a * b)))")),
        eval("[42]"),
    );
}

#[test]
fn named_functions_still_recurse() {
    // Local named functions can call themselves (self-binding at call time).
    let prog = r#"
        function outer(n) {
            function fact(k) { if (k <= 1) { return 1 } return fact(k - 1) * k }
            return fact(n)
        }

        outer(5)
    "#;
    assert_eq!(eval(prog), num(120.0));
}

#[test]
fn builtins_are_values_too() {
    assert_eq!(eval("var l = len; l([1, 2])"), num(2.0));
    assert_eq!(eval("str(print)"), string("<builtin print>"));
}

#[test]
fn function_equality_is_identity() {
    assert_eq!(eval("var f = x => x; var g = f; f == g"), boolean(true));
    assert_eq!(eval("(x => x) == (x => x)"), boolean(false));
    assert_eq!(eval("var f = x => x; f == 1"), boolean(false));
}

#[test]
fn display_of_functions() {
    assert_eq!(eval("function f() { return 1 } str(f)"), string("<function f>"));
    assert_eq!(eval("str(x => x)"), string("<lambda>"));
}

#[test]
fn calling_a_non_function_is_an_error() {
    assert!(matches!(eval_err("5(1)"), LangError::InvalidUnaryOp { op: "()", .. }));
    assert!(matches!(eval_err(r#""f"(1)"#), LangError::InvalidUnaryOp { op: "()", .. }));
}

#[test]
fn lambda_arity_is_checked() {
    assert!(matches!(
        eval_err("(x => x)(1, 2)"),
        LangError::WrongArity { expected: 1, got: 2, .. },
    ));
}

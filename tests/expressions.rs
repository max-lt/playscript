mod common;

use common::*;

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(eval("1 + 2 * 3"), num(7.0));
    assert_eq!(eval("(1 + 2) * 3"), num(9.0));
    assert_eq!(eval("10 - 2 - 3"), num(5.0));
    assert_eq!(eval("2 * (3 + 4) - 1"), num(13.0));
}

#[test]
fn unary_minus_and_not() {
    assert_eq!(eval("-5 + 10"), num(5.0));
    assert_eq!(eval("- -5"), num(5.0));
    assert_eq!(eval("!true"), boolean(false));
    assert_eq!(eval("!(3 > 4)"), boolean(true));
}

#[test]
fn division_and_by_zero() {
    assert_eq!(eval("7 / 2"), num(3.5));
    assert!(matches!(eval_err("10 / 0"), LangError::DivisionByZero));
}

#[test]
fn comparisons_yield_bools() {
    assert_eq!(eval("1 < 2"), boolean(true));
    assert_eq!(eval("2 <= 2"), boolean(true));
    assert_eq!(eval("3 > 4"), boolean(false));
    assert_eq!(eval("5 >= 5"), boolean(true));
}

#[test]
fn equality_is_strict() {
    // No coercion: a number is never equal to a bool.
    assert_eq!(eval("1 == true"), boolean(false));
    assert_eq!(eval("1 == 1"), boolean(true));
    assert_eq!(eval("true == true"), boolean(true));
    assert_eq!(eval("1 != 2"), boolean(true));
    assert_eq!(eval("false != true"), boolean(true));
}

#[test]
fn modulo() {
    assert_eq!(eval("10 % 3"), num(1.0));
    assert_eq!(eval("7.5 % 2"), num(1.5));
    assert_eq!(eval("-7 % 3"), num(-1.0)); // sign of the dividend, like JS
    assert_eq!(eval("10 % 3 * 2"), num(2.0)); // same precedence as *
    assert!(matches!(eval_err("5 % 0"), LangError::DivisionByZero));
    assert!(matches!(eval_err(r#""a" % 2"#), LangError::InvalidBinaryOp { op: "%", .. }));
}

#[test]
fn logical_operators() {
    assert_eq!(eval("true && false"), boolean(false));
    assert_eq!(eval("true && true"), boolean(true));
    assert_eq!(eval("false || true"), boolean(true));
    assert_eq!(eval("false || false"), boolean(false));
    // Comparisons bind tighter than &&, && tighter than ||.
    assert_eq!(eval("1 < 2 && 2 < 3"), boolean(true));
    assert_eq!(eval("true || false && false"), boolean(true));
}

#[test]
fn logical_operators_short_circuit() {
    // The right side must never run: 1 / 0 would be an error.
    assert_eq!(eval("false && 1 / 0 == 0"), boolean(false));
    assert_eq!(eval("true || 1 / 0 == 0"), boolean(true));
}

#[test]
fn logical_operators_are_strict() {
    assert!(matches!(eval_err("1 && true"), LangError::InvalidUnaryOp { op: "&&", .. }));
    assert!(matches!(eval_err("true && 1"), LangError::InvalidUnaryOp { op: "&&", .. }));
    assert!(matches!(eval_err("0 || true"), LangError::InvalidUnaryOp { op: "||", .. }));
}

#[test]
fn line_comments() {
    assert_eq!(eval("1 + 1 // plus a comment"), num(2.0));
    assert_eq!(eval("// leading comment\n40 + 2"), num(42.0));
    assert_eq!(eval("var x = 1 // set x\nx + 1"), num(2.0));
    // // inside a string literal is not a comment.
    assert_eq!(eval(r#""a // b""#), string("a // b"));
    // A comment-only program runs and produces no value.
    assert!(matches!(run("// nothing"), Ok(None)));
}

#[test]
fn type_errors_on_bad_operands() {
    assert!(matches!(eval_err("true + 1"), LangError::InvalidBinaryOp { op: "+", .. }));
    assert!(matches!(eval_err("-true"), LangError::InvalidUnaryOp { op: "-", .. }));
    assert!(matches!(eval_err("!1"), LangError::InvalidUnaryOp { op: "!", .. }));
    assert!(matches!(eval_err("true < false"), LangError::InvalidBinaryOp { op: "<", .. }));
    assert!(matches!(eval_err("1 < true"), LangError::InvalidBinaryOp { op: "<", .. }));
}

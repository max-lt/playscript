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
fn type_errors_on_bad_operands() {
    assert!(matches!(eval_err("true + 1"), LangError::InvalidBinaryOp { op: "+", .. }));
    assert!(matches!(eval_err("-true"), LangError::InvalidUnaryOp { op: "-", .. }));
    assert!(matches!(eval_err("!1"), LangError::InvalidUnaryOp { op: "!", .. }));
    assert!(matches!(eval_err("true < false"), LangError::InvalidBinaryOp { op: "<", .. }));
    assert!(matches!(eval_err("1 < true"), LangError::InvalidBinaryOp { op: "<", .. }));
}

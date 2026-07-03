mod common;

use common::*;

#[test]
fn variables_and_reassignment() {
    assert_eq!(eval("var x = 5; x * 2"), num(10.0));
    assert_eq!(eval("var x = 1; x = x + 4; x"), num(5.0));
}

#[test]
fn assigning_undeclared_is_an_error() {
    assert!(matches!(eval_err("y = 3"), LangError::UndefinedVariable(_)));
}

#[test]
fn reading_undefined_is_an_error() {
    assert!(matches!(eval_err("z + 1"), LangError::UndefinedVariable(_)));
}

#[test]
fn block_scope_shadowing_dies_with_the_block() {
    // The inner `var x` shadows, then vanishes when the block ends.
    assert_eq!(eval("var x = 1; { var x = 2 }; x"), num(1.0));
}

#[test]
fn assignment_reaches_outer_scope() {
    // `=` walks outward and finds the existing binding.
    assert_eq!(eval("var x = 1; { x = 2 }; x"), num(2.0));
}

#[test]
fn if_else_if_else_chain() {
    let prog = |n: i32| {
        format!("var n = {n}; if (n > 0) {{ n = 1 }} else if (n == 0) {{ n = 42 }} else {{ n = 3 }}; n")
    };

    assert_eq!(eval(&prog(5)), num(1.0));
    assert_eq!(eval(&prog(0)), num(42.0));
    assert_eq!(eval(&prog(-5)), num(3.0));
}

#[test]
fn condition_must_be_bool() {
    assert!(matches!(eval_err("if (1) { 2 }"), LangError::InvalidCondition { got: "number" }));
    assert!(matches!(eval_err("while (1) { }"), LangError::InvalidCondition { got: "number" }));
}

#[test]
fn while_loop_sums() {
    assert_eq!(
        eval("var s = 0; var i = 1; while (i <= 100) { s = s + i; i = i + 1 }; s"),
        num(5050.0),
    );
}

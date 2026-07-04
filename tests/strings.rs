mod common;

use common::*;

#[test]
fn literals_and_concatenation() {
    assert_eq!(eval(r#""foo" + "bar""#), string("foobar"));
    assert_eq!(eval(r#"var name = "leek"; "hello " + name"#), string("hello leek"));
}

#[test]
fn escapes() {
    assert_eq!(eval(r#""a\nb""#), string("a\nb"));
    assert_eq!(eval(r#""quote: \" backslash: \\""#), string("quote: \" backslash: \\"));
    assert!(matches!(eval_err(r#""bad \q""#), LangError::InvalidEscape('q')));
    assert!(matches!(eval_err(r#""no end"#), LangError::UnterminatedString));
}

#[test]
fn equality_is_structural_and_strict() {
    assert_eq!(eval(r#""a" == "a""#), boolean(true));
    assert_eq!(eval(r#""a" == "b""#), boolean(false));
    // Still no coercion: a string is never equal to a number.
    assert_eq!(eval(r#""1" == 1"#), boolean(false));
}

#[test]
fn ordering_is_lexicographic() {
    assert_eq!(eval(r#""abc" < "abd""#), boolean(true));
    assert_eq!(eval(r#""b" >= "a""#), boolean(true));
    assert!(matches!(eval_err(r#""a" < 1"#), LangError::InvalidBinaryOp { op: "<", .. }));
}

#[test]
fn no_implicit_conversion_in_concat() {
    assert!(matches!(eval_err(r#""n = " + 1"#), LangError::InvalidBinaryOp { op: "+", .. }));
    // The explicit path works.
    assert_eq!(eval(r#""n = " + str(1)"#), string("n = 1"));
    assert_eq!(eval(r#"str(true) + "!""#), string("true!"));
    assert_eq!(eval(r#"str("already")"#), string("already"));
}

#[test]
fn len_counts_unicode_chars() {
    assert_eq!(eval(r#"len("héllo")"#), num(5.0));
    assert_eq!(eval(r#"len("")"#), num(0.0));
    assert_eq!(eval(r#"len("a\nb")"#), num(3.0));
    assert!(matches!(eval_err("len(1)"), LangError::InvalidUnaryOp { op: "len", .. }));
}

#[test]
fn strings_flow_through_functions() {
    assert_eq!(
        eval(r#"function greet(name) { return "hello " + name } greet("world")"#),
        string("hello world"),
    );
}

#[test]
fn strings_are_not_conditions() {
    assert!(matches!(
        eval_err(r#"if ("truthy?") { 1 }"#),
        LangError::InvalidCondition { got: "string" },
    ));
}

#[test]
fn concat_is_charged_by_length_so_fuel_bounds_memory() {
    // A doubling loop must die of fuel exhaustion, not OOM the host:
    // each concatenation costs one op per byte of its result.
    let mut interp = Interpreter::new(100_000);
    let bomb = r#"var s = "xxxxxxxxxxxxxxxx"; while (true) { s = s + s }"#;
    assert!(matches!(interp.run(bomb), Err(LangError::OutOfFuel { .. })));
}

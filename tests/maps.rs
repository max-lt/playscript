mod common;

use common::*;

#[test]
fn literals_get_and_update() {
    assert_eq!(eval(r#"var m = {"a": 1, "b": 2}; m["a"] + m["b"]"#), num(3.0));
    assert_eq!(eval(r#"var m = {"x": 1}; m["x"] = 9; m["x"]"#), num(9.0));
    assert_eq!(eval(r#"len({"a": 1, "b": 2, "c": 3})"#), num(3.0));
    assert_eq!(eval("len({})"), num(0.0));
}

#[test]
fn keys_preserve_insertion_order() {
    // Not hash order: z, a, m stays z, a, m.
    assert_eq!(eval(r#"str(keys({"z": 1, "a": 2, "m": 3}))"#), string("[z, a, m]"));
    // Updating a key keeps its original position.
    assert_eq!(eval(r#"var m = {"a": 1, "b": 2}; m["a"] = 9; str(keys(m))"#), string("[a, b]"));
}

#[test]
fn value_semantics_copy_on_write() {
    assert_eq!(eval(r#"var a = {"x": 1}; var b = a; b["x"] = 9; a["x"]"#), num(1.0));
    // A function argument never aliases the caller's map.
    let prog = r#"var m = {"x": 1}; function f(g) { g["x"] = 9; return g["x"] } f(m); m["x"]"#;
    assert_eq!(eval(prog), num(1.0));
}

#[test]
fn equality_is_order_insensitive_but_strict() {
    assert_eq!(eval(r#"{"a": 1, "b": 2} == {"b": 2, "a": 1}"#), boolean(true));
    assert_eq!(eval(r#"{"a": 1} == {"a": 2}"#), boolean(false));
    assert_eq!(eval(r#"{"a": 1} == {"a": 1, "b": 2}"#), boolean(false));
    assert_eq!(eval(r#"{"a": 1} == [1]"#), boolean(false));
}

#[test]
fn mixed_primitive_keys() {
    assert_eq!(eval(r#"var m = {1: "one", true: "yes", "k": 3}; m[1]"#), string("one"));
    assert_eq!(eval(r#"{1: "one", true: "yes"}[true]"#), string("yes"));
}

#[test]
fn has_and_remove() {
    assert_eq!(eval(r#"has({"a": 1}, "a")"#), boolean(true));
    assert_eq!(eval(r#"has({"a": 1}, "z")"#), boolean(false));
    assert_eq!(eval(r#"len(remove({"a": 1, "b": 2}, "a"))"#), num(1.0));
    assert_eq!(eval(r#"has(remove({"a": 1}, "a"), "a")"#), boolean(false));
    // remove returns a new map; the original is untouched.
    assert_eq!(eval(r#"var m = {"a": 1}; remove(m, "a"); len(m)"#), num(1.0));
}

#[test]
fn missing_key_is_an_error() {
    assert!(matches!(eval_err(r#"{"a": 1}["z"]"#), LangError::MissingKey(_)));
}

#[test]
fn non_primitive_keys_are_rejected() {
    assert!(matches!(eval_err(r#"var m = {}; m[[1]] = 2"#), LangError::InvalidMapKey { .. }));
    assert!(matches!(eval_err(r#"{[1]: 2}"#), LangError::InvalidMapKey { .. }));
}

#[test]
fn brace_is_map_or_block_by_content() {
    // A top-level ':' makes it a map expression; otherwise a block.
    assert_eq!(eval(r#"{"a": 1}["a"]"#), num(1.0));
    assert_eq!(eval("var x = 1; { var y = 2 }; x"), num(1.0));
    // The block after `if` is always a block, never a map.
    assert!(matches!(eval_err("if (true) { var z = 3 }; z"), LangError::UndefinedVariable(_)));
}

#[test]
fn word_count() {
    let prog = r#"
        var counts = {};
        var words = ["a", "b", "a", "a", "b"];
        var i = 0;

        while (i < len(words)) {
            var w = words[i];

            if (has(counts, w)) {
                counts[w] = counts[w] + 1
            } else {
                counts[w] = 1
            }

            i = i + 1
        }

        counts["a"]
    "#;
    assert_eq!(eval(prog), num(3.0));
}

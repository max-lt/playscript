mod common;

use common::*;

#[test]
fn literals_and_indexing() {
    assert_eq!(eval("[1, 2, 3][0]"), num(1.0));
    assert_eq!(eval("var a = [1, 2, 3]; a[1] + a[2]"), num(5.0));
    assert_eq!(eval("[[1, 2], [3, 4]][1][0]"), num(3.0));
    assert_eq!(eval(r#"["a", "b"][1]"#), string("b"));
}

#[test]
fn index_writes() {
    assert_eq!(eval("var a = [1, 2, 3]; a[1] = 9; a[1]"), num(9.0));
    assert_eq!(eval("var a = [0]; a[0] = a[0] + 1; a[0] = a[0] + 1; a[0]"), num(2.0));
}

#[test]
fn value_semantics_assignment_copies() {
    // THE design decision: `var b = a` copies (copy-on-write), so writing
    // through b never touches a. No observable aliasing, ever.
    assert_eq!(eval("var a = [1, 2, 3]; var b = a; b[0] = 99; a[0]"), num(1.0));
    assert_eq!(eval("var a = [1, 2, 3]; var b = a; b[0] = 99; b[0]"), num(99.0));
}

#[test]
fn value_semantics_function_arguments_do_not_alias() {
    let prog = "var a = [1]; function f(x) { x[0] = 99; return x[0] } f(a); a[0]";
    assert_eq!(eval(prog), num(1.0));
}

#[test]
fn equality_is_structural() {
    assert_eq!(eval("[1, 2] == [1, 2]"), boolean(true));
    assert_eq!(eval("[1, 2] == [1, 3]"), boolean(false));
    assert_eq!(eval("[[1], [2]] == [[1], [2]]"), boolean(true));
    assert_eq!(eval("[1] == 1"), boolean(false));
}

#[test]
fn arrays_are_not_ordered() {
    assert!(matches!(eval_err("[1] < [2]"), LangError::InvalidBinaryOp { op: "<", .. }));
}

#[test]
fn display_via_str() {
    assert_eq!(eval("str([1, [2, 3], true])"), string("[1, [2, 3], true]"));
}

#[test]
fn builtins_len_array_push() {
    assert_eq!(eval("len([1, 2, 3])"), num(3.0));
    assert_eq!(eval("len(array(5, 0))"), num(5.0));
    assert_eq!(eval("var a = array(2, 7); a[0] + a[1]"), num(14.0));
    assert_eq!(eval("var a = [1]; a = push(a, 2); a[1]"), num(2.0));
    assert_eq!(eval("len(push([], 1))"), num(1.0));
}

#[test]
fn index_errors() {
    assert!(matches!(eval_err("[1, 2][5]"), LangError::IndexOutOfBounds { index: 5, len: 2 }));
    assert!(matches!(eval_err("var a = [1]; a[-1]"), LangError::InvalidIndex(_)));
    assert!(matches!(eval_err("[1][0.5]"), LangError::InvalidIndex(_)));
    assert!(matches!(eval_err("[1][true]"), LangError::InvalidIndex(_)));
    assert!(matches!(eval_err("5[0]"), LangError::InvalidUnaryOp { op: "[]", .. }));
    assert!(matches!(eval_err("var x = 1; x[0] = 2"), LangError::InvalidUnaryOp { op: "[]=", .. }));
}

#[test]
fn invalid_assignment_targets() {
    assert!(matches!(eval_err("1 = 2"), LangError::InvalidAssignTarget));
    assert!(matches!(eval_err("[1][0] = 2"), LangError::InvalidAssignTarget));
}

#[test]
fn nested_write_via_read_modify_write() {
    // Single-level index assignment only (for now): nested writes go
    // through an explicit read-modify-write, in pure value-semantics style.
    let prog = "var m = [[0, 0], [0, 0]]; var row = m[1]; row[0] = 5; m[1] = row; m[1][0]";
    assert_eq!(eval(prog), num(5.0));
}

#[test]
fn iterative_fib_with_array() {
    let prog = r#"
        var f = array(79, 0);
        f[1] = 1;
        var i = 2;

        while (i < 79) {
            f[i] = f[i - 1] + f[i - 2];
            i = i + 1
        }

        f[78]
    "#;
    assert_eq!(eval(prog), num(8944394323791464.0));
}

#[test]
fn bubble_sort() {
    let prog = r#"
        var a = [5, 3, 8, 1, 9, 2];
        var n = len(a);
        var i = 0;

        while (i < n) {
            var j = 0;

            while (j < n - 1) {

                if (a[j] > a[j + 1]) {
                    var t = a[j];
                    a[j] = a[j + 1];
                    a[j + 1] = t
                }

                j = j + 1
            }

            i = i + 1
        }

        a
    "#;
    assert_eq!(eval(prog), eval("[1, 2, 3, 5, 8, 9]"));
}

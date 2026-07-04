mod common;

use common::*;

#[test]
fn tracing_is_a_pure_observer() {
    // THE invariant: a traced run and a plain run agree on result AND on the
    // op-clock. Recording must never consume fuel, or the trace's timeline
    // would not match a normal run — and replay would drift.
    let prog = "function fib(n) { if (n < 2) { return n } return fib(n - 1) + fib(n - 2) } fib(8)";

    let mut plain = Interpreter::new(DEFAULT_FUEL_LIMIT);
    let plain_result = plain.run(prog).unwrap();
    let plain_fuel = plain.fuel_used();

    let mut traced = Interpreter::new(DEFAULT_FUEL_LIMIT);
    traced.enable_tracing();
    let traced_result = traced.run(prog).unwrap();
    let traced_fuel = traced.fuel_used();

    assert_eq!(plain_result, traced_result);
    assert_eq!(plain_fuel, traced_fuel);
    assert!(!traced.trace().unwrap().is_empty());
}

#[test]
fn the_op_clock_is_monotonic() {
    // Events are recorded in execution order and fuel only ever increases,
    // so the op stamps must be non-decreasing — the trace is a timeline.
    let trace = trace_of("var s = 0; var i = 0; while (i < 5) { s = s + i; i = i + 1 }");
    let ops: Vec<u64> = trace.iter().map(|e| e.op).collect();

    assert!(!ops.is_empty());
    assert!(ops.windows(2).all(|w| w[0] <= w[1]), "not monotonic: {ops:?}");
}

#[test]
fn assignments_capture_target_and_value() {
    let trace = trace_of("var x = 5; x = x + 1");

    let assigns: Vec<(String, Value)> = trace
        .iter()
        .filter_map(|e| match &e.kind {
            EventKind::Assign { target, value } => Some((target.clone(), value.clone())),
            _ => None,
        })
        .collect();

    assert_eq!(assigns, vec![("x".to_string(), num(5.0)), ("x".to_string(), num(6.0))]);
}

#[test]
fn index_assignments_render_the_slot() {
    let trace = trace_of("var a = [0, 0]; a[1] = 9");

    let last = trace
        .iter()
        .rev()
        .find_map(|e| match &e.kind {
            EventKind::Assign { target, value } => Some((target.clone(), value.clone())),
            _ => None,
        })
        .unwrap();

    assert_eq!(last, ("a[1]".to_string(), num(9.0)));
}

#[test]
fn calls_and_returns_nest_by_depth() {
    // f(2) -> f(1) -> f(0): three calls, deepening; three returns, matching.
    let trace = trace_of("function f(n) { if (n <= 0) { return 0 } return f(n - 1) } f(2)");

    let call_depths: Vec<usize> = trace
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Call { .. } => Some(e.depth),
            _ => None,
        })
        .collect();

    assert_eq!(call_depths, vec![0, 1, 2]);

    let returns = trace.iter().filter(|e| matches!(e.kind, EventKind::Return { .. })).count();
    assert_eq!(returns, 3);
}

#[test]
fn call_events_carry_the_arguments() {
    let trace = trace_of("function add(a, b) { return a + b } add(3, 4)");

    let call = trace
        .iter()
        .find_map(|e| match &e.kind {
            EventKind::Call { name, args } => Some((name.clone(), args.clone())),
            _ => None,
        })
        .unwrap();

    assert_eq!(call, ("add".to_string(), vec![num(3.0), num(4.0)]));
}

#[test]
fn branch_decisions_are_recorded() {
    // while runs the body twice, then the condition fails: true, true, false.
    let trace = trace_of("var i = 0; while (i < 2) { i = i + 1 }");

    let branches: Vec<bool> = trace
        .iter()
        .filter_map(|e| match e.kind {
            EventKind::Branch { value, .. } => Some(value),
            _ => None,
        })
        .collect();

    assert_eq!(branches, vec![true, true, false]);
}

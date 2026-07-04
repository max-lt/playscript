//! Wasm bridge: the same tree-walking engine, exposed to JavaScript.
//!
//! `run(source)` executes a program with tracing on and returns the whole
//! outcome — the result value, the operations consumed, and the execution
//! trace — as JSON, the pivot format the browser visualizer consumes.

use playscript::{DEFAULT_FUEL_LIMIT, EventKind, Interpreter, TraceEvent, Value};
use serde_json::{Value as Json, json};
use wasm_bindgen::prelude::*;

/// Run a program with tracing enabled and return the outcome as a JSON string.
///
/// Shape: `{ ok, value | error, ops, trace: [...] }`. It never throws — a
/// program error becomes `{ ok: false, error }`, still carrying the partial
/// trace and op count up to the failure.
#[wasm_bindgen]
pub fn run(source: &str) -> String {
    run_to_json(source)
}

/// The pure core, callable and testable natively (no wasm runtime needed).
pub fn run_to_json(source: &str) -> String {
    let mut interp = Interpreter::new(DEFAULT_FUEL_LIMIT);
    interp.enable_tracing();

    let outcome = interp.run(source);
    let ops = interp.fuel_used();
    let trace = trace_to_json(interp.trace().unwrap_or(&[]));

    let result = match outcome {
        Ok(value) => json!({
            "ok": true,
            "value": value.as_ref().map(value_to_json),
            "ops": ops,
            "trace": trace,
        }),
        Err(error) => json!({
            "ok": false,
            "error": error.to_string(),
            "ops": ops,
            "trace": trace,
        }),
    };

    result.to_string()
}

fn trace_to_json(events: &[TraceEvent]) -> Json {
    Json::Array(events.iter().map(event_to_json).collect())
}

fn event_to_json(event: &TraceEvent) -> Json {
    let mut obj = json!({ "op": event.op, "depth": event.depth });
    let map = obj.as_object_mut().expect("json! built an object");

    // A flat object with a `kind` discriminant — easy for the UI to switch on.
    match &event.kind {
        EventKind::Assign { target, value } => {
            map.insert("kind".into(), json!("assign"));
            map.insert("target".into(), json!(target));
            map.insert("value".into(), value_to_json(value));
        }
        EventKind::Call { name, args } => {
            map.insert("kind".into(), json!("call"));
            map.insert("name".into(), json!(name));
            map.insert("args".into(), Json::Array(args.iter().map(value_to_json).collect()));
        }
        EventKind::Return { name, value } => {
            map.insert("kind".into(), json!("return"));
            map.insert("name".into(), json!(name));
            map.insert("value".into(), value_to_json(value));
        }
        EventKind::Branch { construct, value } => {
            map.insert("kind".into(), json!("branch"));
            map.insert("construct".into(), json!(construct));
            map.insert("value".into(), json!(value));
        }
    }

    obj
}

/// Convert a runtime value to JSON. Numbers, bools, strings and arrays map
/// naturally; functions have no data form, so they render as their label.
fn value_to_json(value: &Value) -> Json {

    match value {
        Value::Number(n) => serde_json::Number::from_f64(*n)
            .map(Json::Number)
            // JSON has no NaN/Infinity — fall back to the textual form.
            .unwrap_or_else(|| Json::String(n.to_string())),
        Value::Bool(b) => Json::Bool(*b),
        Value::Str(s) => Json::String(s.to_string()),
        Value::Array(items) => Json::Array(items.iter().map(value_to_json).collect()),
        _ => Json::String(value.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::run_to_json;
    use serde_json::Value as Json;

    fn parse(src: &str) -> Json {
        serde_json::from_str(&run_to_json(src)).expect("bridge emits valid JSON")
    }

    #[test]
    fn ok_result_carries_value_ops_and_trace() {
        let out = parse("var x = 21; x * 2");

        assert_eq!(out["ok"], true);
        assert_eq!(out["value"], 42.0);
        assert!(out["ops"].as_u64().unwrap() > 0);
        assert!(out["trace"].as_array().unwrap().iter().any(|e| e["kind"] == "assign"));
    }

    #[test]
    fn error_result_carries_message_and_partial_trace() {
        let out = parse("var x = 1; y");

        assert_eq!(out["ok"], false);
        assert!(out["error"].as_str().unwrap().contains("undefined variable"));
        // The assignment that ran before the error is still in the trace.
        assert!(out["trace"].as_array().unwrap().iter().any(|e| e["target"] == "x"));
    }

    #[test]
    fn trace_events_are_shaped_for_the_visualizer() {
        let out = parse("function f(n) { if (n < 1) { return 0 } return f(n - 1) } f(1)");
        let trace = out["trace"].as_array().unwrap();

        let call = trace.iter().find(|e| e["kind"] == "call").unwrap();
        assert_eq!(call["name"], "f");
        assert_eq!(call["args"][0], 1.0);
        assert!(call["op"].is_number());
        assert!(call["depth"].is_number());

        assert!(trace.iter().any(|e| e["kind"] == "branch"));
        assert!(trace.iter().any(|e| e["kind"] == "return"));
    }

    #[test]
    fn values_serialize_structurally() {
        let out = parse(r#"[1, [2, true], "hi"]"#);
        assert_eq!(out["value"], serde_json::json!([1.0, [2.0, true], "hi"]));
    }
}

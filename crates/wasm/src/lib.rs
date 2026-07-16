//! Browser API: pass C++ source, get return value + event stream.
//!
//! No stdin/stdout in v1 — visualizers consume `events`.
//! Event/value JSON shapes: repository `docs/events.md`.

use rscpp_ast::Span;
use rscpp_runtime::{args_from_json, Engine, Event, RuntimeError, Value, DEFAULT_FUEL};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Structured run error for UI highlighting.
#[derive(Debug, Clone, Serialize)]
pub struct RunError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl From<&RuntimeError> for RunError {
    fn from(e: &RuntimeError) -> Self {
        Self {
            message: e.message.clone(),
            span: e.span,
        }
    }
}

impl From<RuntimeError> for RunError {
    fn from(e: RuntimeError) -> Self {
        Self::from(&e)
    }
}

/// Result of `run(source)` / `run_method(...)`.
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    pub events: Vec<Event>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RunError>,
}

fn finish(mut eng: Engine, result: Result<Value, RuntimeError>) -> RunResult {
    match result {
        Ok(value) => RunResult {
            ok: true,
            value: Some(value),
            events: eng.take_events(),
            error: None,
        },
        Err(e) => RunResult {
            ok: false,
            value: None,
            events: eng.take_events(),
            error: Some(RunError::from(e)),
        },
    }
}

fn load_error(e: RuntimeError) -> RunResult {
    RunResult {
        ok: false,
        value: None,
        events: vec![],
        error: Some(RunError::from(e)),
    }
}

/// Execute `main` with the tree-walker. Host-testable without a browser.
pub fn run_result(source: &str) -> RunResult {
    run_result_with_fuel(source, DEFAULT_FUEL)
}

pub fn run_result_with_fuel(source: &str, fuel: u64) -> RunResult {
    match Engine::from_source_with_fuel(source, fuel) {
        Ok(mut eng) => {
            let r = eng.run_main();
            finish(eng, r)
        }
        Err(e) => load_error(e),
    }
}

/// Call `Class::method` (or free function) with JSON args array.
/// Example: `run_method(src, "Solution::twoSum", [[2,7,11,15], 9])`.
pub fn run_method_result(source: &str, method: &str, args_json: &serde_json::Value) -> RunResult {
    match Engine::from_source(source) {
        Ok(mut eng) => match args_from_json(&mut eng, args_json) {
            Ok(args) => {
                let r = eng.call(method, &args);
                finish(eng, r)
            }
            Err(e) => {
                let mut out = load_error(e);
                out.events = eng.take_events();
                out
            }
        },
        Err(e) => load_error(e),
    }
}

/// WASM entry: `run(source: string) → RunResult`.
#[wasm_bindgen]
pub fn run(source: &str) -> Result<JsValue, JsValue> {
    let result = run_result(source);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// WASM entry: `run_method(source, method, argsJsonArray) → RunResult`.
#[wasm_bindgen]
pub fn run_method(source: &str, method: &str, args: JsValue) -> Result<JsValue, JsValue> {
    let args_json: serde_json::Value = serde_wasm_bindgen::from_value(args)
        .map_err(|e| JsValue::from_str(&format!("bad args: {e}")))?;
    let result = run_method_result(source, method, &args_json);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rscpp_runtime::Event;

    #[test]
    fn run_main_returns_int_and_events() {
        let r = run_result(
            r#"
int main() {
  int x = 1;
  return x + 2;
}
"#,
        );
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.value, Some(Value::Int(3)));
        assert!(r
            .events
            .iter()
            .any(|e| matches!(e, Event::FnEnter { name, .. } if name == "main")));
        assert!(r.error.is_none());
    }

    #[test]
    fn include_is_stripped() {
        let r = run_result(
            r#"
#include <vector>
#include <iostream>
int main() { return 7; }
"#,
        );
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.value, Some(Value::Int(7)));
    }

    #[test]
    fn define_still_errors() {
        let r = run_result("#define X 1\nint main() { return 0; }");
        assert!(!r.ok);
        assert!(r.error.is_some());
    }

    #[test]
    fn structured_error_has_span() {
        let r = run_result("int main() { return foo; }");
        assert!(!r.ok);
        let err = r.error.unwrap();
        assert!(err.span.is_some(), "{err:?}");
    }

    #[test]
    fn fuel_stops_infinite_loop() {
        let r = run_result_with_fuel("int main() { while (1) {} return 0; }", 50);
        assert!(!r.ok);
        assert!(
            r.error
                .as_ref()
                .unwrap()
                .message
                .contains("step limit"),
            "{:?}",
            r.error
        );
        assert!(!r.events.is_empty());
    }

    #[test]
    fn run_method_two_sum() {
        let src = r#"
class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        map<int, int> seen;
        for (int i = 0; i < nums.size(); ++i) {
            int need = target - nums[i];
            if (seen.count(need)) {
                return {seen[need], i};
            }
            seen[nums[i]] = i;
        }
        return {};
    }
};
"#;
        let args = serde_json::json!([[2, 7, 11, 15], 9]);
        let r = run_method_result(src, "Solution::twoSum", &args);
        assert!(r.ok, "{:?}", r.error);
        let Value::Object(_) = r.value.unwrap() else {
            panic!("expected vector object");
        };
    }

    #[test]
    fn run_method_is_valid() {
        let src = include_str!("../../../examples/valid_parentheses.cpp");
        let r = run_method_result(src, "Solution::isValid", &serde_json::json!(["()[]{}"]));
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.value, Some(Value::Bool(true)));
        assert!(r.events.iter().any(|e| matches!(
            e,
            Event::FnEnter { name, .. } if name == "Solution::isValid"
        )));
        assert!(r.events.iter().any(|e| matches!(
            e,
            Event::ContainerMod { kind, .. } if kind.starts_with("stack::")
        )));

        let r = run_method_result(src, "Solution::isValid", &serde_json::json!(["(]"]));
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.value, Some(Value::Bool(false)));
    }

    #[test]
    fn run_method_count_components() {
        let src = include_str!("../../../examples/dfs.cpp");
        let args = serde_json::json!([
            6,
            [[1], [0, 2], [1], [4], [3], []]
        ]);
        let r = run_method_result(src, "Solution::countComponents", &args);
        assert!(r.ok, "{:?}", r.error);
        assert_eq!(r.value, Some(Value::Int(3)));
    }

    #[test]
    fn events_serialize_with_kind_tag() {
        let e = Event::Step {
            call_id: None,
            span: Span::new(0, 1),
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains(r#""kind":"Step""#), "{s}");
    }

    #[test]
    fn run_result_json_shape() {
        let r = run_result("int main() { return 42; }");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["value"]["kind"], "Int");
        assert_eq!(v["value"]["value"], 42);
        assert!(v["events"].as_array().unwrap().len() > 0);
    }

    #[test]
    fn error_json_is_object() {
        let r = run_result("int main() { return foo; }");
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["ok"], false);
        assert!(v["error"]["message"].is_string());
        assert!(v["error"]["span"]["start"].is_number());
    }
}

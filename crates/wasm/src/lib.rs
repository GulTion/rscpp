//! Browser API: pass C++ source, get return value + event stream.
//!
//! No stdin/stdout in v1 — visualizers consume `events`.

use rscpp_runtime::{Engine, Event, Value};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Result of `run(source)` (JSON / JS object).
#[derive(Debug, Clone, Serialize)]
pub struct RunResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    pub events: Vec<Event>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Execute `main` with the tree-walker. Host-testable without a browser.
pub fn run_result(source: &str) -> RunResult {
    match Engine::from_source(source) {
        Ok(mut eng) => match eng.run_main() {
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
                error: Some(e.to_string()),
            },
        },
        Err(e) => RunResult {
            ok: false,
            value: None,
            events: vec![],
            error: Some(e.to_string()),
        },
    }
}

/// WASM entry: `run(source: string) → RunResult`.
#[wasm_bindgen]
pub fn run(source: &str) -> Result<JsValue, JsValue> {
    let result = run_result(source);
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rscpp_ast::Span;
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
        assert!(r.events.iter().any(|e| matches!(e, Event::FnEnter { name, .. } if name == "main")));
        assert!(r.error.is_none());
    }

    #[test]
    fn parse_error_is_ok_false() {
        let r = run_result("#include <iostream>\nint main() { return 0; }");
        assert!(!r.ok);
        assert!(r.error.is_some());
        assert!(r.events.is_empty());
    }

    #[test]
    fn events_serialize_with_kind_tag() {
        let e = Event::Step {
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
}

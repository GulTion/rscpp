//! Build runtime `Value`s from JSON (for `run_method` args).

use crate::error::RuntimeError;
use crate::value::Value;
use crate::Engine;

type Result<T> = std::result::Result<T, RuntimeError>;

/// Convert a JSON value into a runtime value.
/// Arrays become `vector`s; numbers → `Int`/`Float`; strings → heap `string`.
pub fn value_from_json(eng: &mut Engine, v: &serde_json::Value) -> Result<Value> {
    match v {
        serde_json::Value::Null => Ok(Value::Nullptr),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Int(i))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::Int(u as i64))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::Float(f))
            } else {
                Err(RuntimeError::new("unsupported number"))
            }
        }
        serde_json::Value::String(s) => Ok(eng.make_string(s.clone())),
        serde_json::Value::Array(arr) => {
            let mut elems = Vec::with_capacity(arr.len());
            for e in arr {
                elems.push(value_from_json(eng, e)?);
            }
            Ok(eng.make_vector(elems))
        }
        serde_json::Value::Object(_) => Err(RuntimeError::new(
            "object JSON args not supported (use arrays / primitives)",
        )),
    }
}

/// Parse a JSON array of args into runtime values.
pub fn args_from_json(eng: &mut Engine, v: &serde_json::Value) -> Result<Vec<Value>> {
    let serde_json::Value::Array(arr) = v else {
        return Err(RuntimeError::new("args must be a JSON array"));
    };
    arr.iter().map(|e| value_from_json(eng, e)).collect()
}

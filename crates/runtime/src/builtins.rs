//! LeetCode-common `<algorithm>` / `<climits>` names (not a full libc).

use crate::error::RuntimeError;
use crate::value::Value;
use rscpp_ast::Span;

type Result<T> = std::result::Result<T, RuntimeError>;

/// `INT_MAX`, `INT_MIN`, … — unqualified identifiers from pasted LeetCode.
pub fn const_value(name: &str) -> Option<Value> {
    Some(match name {
        "INT_MAX" => Value::Int(2_147_483_647),
        "INT_MIN" => Value::Int(-2_147_483_648),
        "LONG_MAX" | "LLONG_MAX" => Value::Int(i64::MAX),
        "LONG_MIN" | "LLONG_MIN" => Value::Int(i64::MIN),
        "UINT_MAX" => Value::Int(u32::MAX as i64),
        _ => return None,
    })
}

/// Names handled like `swap` / `sort` in `Call` (with or without `std::`).
pub fn is_builtin_call(name: &str) -> bool {
    matches!(
        name,
        "min"
            | "max"
            | "abs"
            | "pow"
            | "sqrt"
            | "ceil"
            | "floor"
            | "__builtin_popcount"
            | "__builtin_popcountll"
            | "std::min"
            | "std::max"
            | "std::abs"
            | "std::pow"
            | "std::sqrt"
            | "std::ceil"
            | "std::floor"
            | "labs"
            | "llabs"
    )
}

pub struct BuiltinOutcome {
    pub value: Value,
    pub chosen: Option<usize>,
}

pub fn call_builtin(name: &str, args: &[Value], span: Span) -> Result<BuiltinOutcome> {
    match name {
        "min" | "std::min" | "max" | "std::max" => {
            if args.len() != 2 {
                return Err(RuntimeError::at(
                    span,
                    format!("`{name}` expects 2 arguments"),
                ));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)?;
            let b = args[1].as_int().map_err(RuntimeError::new)?;
            let is_max = matches!(name, "max" | "std::max");
            let (out, chosen) = if is_max {
                if a >= b { (a, 0usize) } else { (b, 1usize) }
            } else if a <= b {
                (a, 0usize)
            } else {
                (b, 1usize)
            };
            Ok(BuiltinOutcome {
                value: Value::Int(out),
                chosen: Some(chosen),
            })
        }
        "abs" | "std::abs" | "labs" | "llabs" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(
                    span,
                    format!("`{name}` expects 1 argument"),
                ));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)?;
            Ok(BuiltinOutcome {
                value: Value::Int(a.abs()),
                chosen: None,
            })
        }
        "pow" | "std::pow" => {
            if args.len() != 2 {
                return Err(RuntimeError::at(span, format!("`{name}` expects 2 arguments")));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)? as f64;
            let b = args[1].as_int().map_err(RuntimeError::new)? as f64;
            Ok(BuiltinOutcome {
                value: Value::Float(a.powf(b)),
                chosen: None,
            })
        }
        "sqrt" | "std::sqrt" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(span, format!("`{name}` expects 1 argument")));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)? as f64;
            Ok(BuiltinOutcome {
                value: Value::Float(a.sqrt()),
                chosen: None,
            })
        }
        "ceil" | "std::ceil" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(span, format!("`{name}` expects 1 argument")));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)? as f64;
            Ok(BuiltinOutcome {
                value: Value::Float(a.ceil()),
                chosen: None,
            })
        }
        "floor" | "std::floor" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(span, format!("`{name}` expects 1 argument")));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)? as f64;
            Ok(BuiltinOutcome {
                value: Value::Float(a.floor()),
                chosen: None,
            })
        }
        "__builtin_popcount" | "__builtin_popcountll" => {
            if args.len() != 1 {
                return Err(RuntimeError::at(span, format!("`{name}` expects 1 argument")));
            }
            let a = args[0].as_int().map_err(RuntimeError::new)? as u64;
            Ok(BuiltinOutcome {
                value: Value::Int(a.count_ones() as i64),
                chosen: None,
            })
        }
        _ => Err(RuntimeError::at(span, format!("unknown builtin `{name}`"))),
    }
}

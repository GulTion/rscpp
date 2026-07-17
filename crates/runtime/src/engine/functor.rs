//! `<functional>` function objects: construct + apply.

use super::Result;
use crate::error::RuntimeError;
use crate::value::{FunctorKind, Value};
use rscpp_ast::Span;

pub fn functor_kind(name: &str) -> Option<FunctorKind> {
    let name = name.strip_prefix("std::").unwrap_or(name);
    Some(match name {
        "plus" => FunctorKind::Plus,
        "minus" => FunctorKind::Minus,
        "multiplies" => FunctorKind::Multiplies,
        "divides" => FunctorKind::Divides,
        "modulus" => FunctorKind::Modulus,
        "negate" => FunctorKind::Negate,
        "equal_to" => FunctorKind::EqualTo,
        "not_equal_to" => FunctorKind::NotEqualTo,
        "greater" => FunctorKind::Greater,
        "less" => FunctorKind::Less,
        "greater_equal" => FunctorKind::GreaterEqual,
        "less_equal" => FunctorKind::LessEqual,
        "logical_and" => FunctorKind::LogicalAnd,
        "logical_or" => FunctorKind::LogicalOr,
        "logical_not" => FunctorKind::LogicalNot,
        "bit_and" => FunctorKind::BitAnd,
        "bit_or" => FunctorKind::BitOr,
        "bit_xor" => FunctorKind::BitXor,
        "bit_not" => FunctorKind::BitNot,
        _ => return None,
    })
}

pub fn functor_apply(kind: FunctorKind, args: &[Value], span: Span) -> Result<Value> {
    use FunctorKind::*;
    match kind {
        Negate | LogicalNot | BitNot => {
            if args.len() != 1 {
                return Err(RuntimeError::at(
                    span,
                    format!("functor expects 1 argument, got {}", args.len()),
                ));
            }
        }
        _ => {
            if args.len() != 2 {
                return Err(RuntimeError::at(
                    span,
                    format!("functor expects 2 arguments, got {}", args.len()),
                ));
            }
        }
    }

    Ok(match kind {
        Plus => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            + args[1].as_int().map_err(RuntimeError::new)?),
        Minus => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            - args[1].as_int().map_err(RuntimeError::new)?),
        Multiplies => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            * args[1].as_int().map_err(RuntimeError::new)?),
        Divides => {
            let b = args[1].as_int().map_err(RuntimeError::new)?;
            if b == 0 {
                return Err(RuntimeError::at(span, "division by zero"));
            }
            Value::Int(args[0].as_int().map_err(RuntimeError::new)? / b)
        }
        Modulus => {
            let b = args[1].as_int().map_err(RuntimeError::new)?;
            if b == 0 {
                return Err(RuntimeError::at(span, "division by zero"));
            }
            Value::Int(args[0].as_int().map_err(RuntimeError::new)? % b)
        }
        Negate => Value::Int(-args[0].as_int().map_err(RuntimeError::new)?),
        EqualTo => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                == args[1].as_int().map_err(RuntimeError::new)?,
        ),
        NotEqualTo => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                != args[1].as_int().map_err(RuntimeError::new)?,
        ),
        Greater => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                > args[1].as_int().map_err(RuntimeError::new)?,
        ),
        Less => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                < args[1].as_int().map_err(RuntimeError::new)?,
        ),
        GreaterEqual => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                >= args[1].as_int().map_err(RuntimeError::new)?,
        ),
        LessEqual => Value::Bool(
            args[0].as_int().map_err(RuntimeError::new)?
                <= args[1].as_int().map_err(RuntimeError::new)?,
        ),
        LogicalAnd => Value::Bool(
            args[0].as_bool().map_err(RuntimeError::new)?
                && args[1].as_bool().map_err(RuntimeError::new)?,
        ),
        LogicalOr => Value::Bool(
            args[0].as_bool().map_err(RuntimeError::new)?
                || args[1].as_bool().map_err(RuntimeError::new)?,
        ),
        LogicalNot => Value::Bool(!args[0].as_bool().map_err(RuntimeError::new)?),
        BitAnd => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            & args[1].as_int().map_err(RuntimeError::new)?),
        BitOr => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            | args[1].as_int().map_err(RuntimeError::new)?),
        BitXor => Value::Int(args[0].as_int().map_err(RuntimeError::new)?
            ^ args[1].as_int().map_err(RuntimeError::new)?),
        BitNot => Value::Int(!args[0].as_int().map_err(RuntimeError::new)?),
    })
}

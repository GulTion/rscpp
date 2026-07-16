use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {

    pub(super) fn eval_binary(&mut self, op: BinaryOp, l: &Value, r: &Value, span: Span) -> Result<Value> {
        use BinaryOp::*;
        match op {
            Add | Sub | Mul | Div | Rem | BitAnd | BitXor | BitOr | Shl | Shr => {
                let a = l.as_int().map_err(RuntimeError::new)?;
                let b = r.as_int().map_err(RuntimeError::new)?;
                let n = match op {
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a / b
                    }
                    Rem => {
                        if b == 0 {
                            return Err(RuntimeError::at(span, "division by zero"));
                        }
                        a % b
                    }
                    BitAnd => a & b,
                    BitXor => a ^ b,
                    BitOr => a | b,
                    Shl => a << b,
                    Shr => a >> b,
                    _ => unreachable!(),
                };
                Ok(Value::Int(n))
            }
            Lt | Gt | Le | Ge | Eq | Ne => {
                let result = match (l, r, op) {
                    (Value::Ptr(a), Value::Ptr(b), Eq) => a == b,
                    (Value::Ptr(a), Value::Ptr(b), Ne) => a != b,
                    (Value::Ptr(a), Value::Nullptr, Eq) | (Value::Nullptr, Value::Ptr(a), Eq) => {
                        *a == Address::Null
                    }
                    (Value::Ptr(a), Value::Nullptr, Ne) | (Value::Nullptr, Value::Ptr(a), Ne) => {
                        *a != Address::Null
                    }
                    (Value::Nullptr, Value::Nullptr, Eq) => true,
                    (Value::Nullptr, Value::Nullptr, Ne) => false,
                    (Value::Int(a), Value::Int(b), _) => cmp_ord(op, a.cmp(b)),
                    (Value::Float(a), Value::Float(b), _) => {
                        cmp_ord(op, a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    }
                    _ => {
                        let a = l.as_int().map_err(RuntimeError::new)?;
                        let b = r.as_int().map_err(RuntimeError::new)?;
                        cmp_ord(op, a.cmp(&b))
                    }
                };
                self.emit(Event::Compare {
                    call_id: self.current_call_id(),
                    op: format!("{op:?}"),
                    left: l.clone(),
                    right: r.clone(),
                    result,
                    span,
                });
                Ok(Value::Bool(result))
            }
            And => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    && r.as_bool().map_err(RuntimeError::new)?,
            )),
            Or => Ok(Value::Bool(
                l.as_bool().map_err(RuntimeError::new)?
                    || r.as_bool().map_err(RuntimeError::new)?,
            )),
        }
    }

    pub(super) fn default_value_for_type(&mut self, ty: &Type) -> Result<Value> {
        match ty {
            Type::Builtin { kind, .. } => Ok(match kind {
                BuiltinType::Bool => Value::Bool(false),
                BuiltinType::Float | BuiltinType::Double => Value::Float(0.0),
                BuiltinType::Void => Value::Void,
                BuiltinType::Char | BuiltinType::UnsignedChar => Value::Char('\0'),
                _ => Value::Int(0),
            }),
            Type::Named { path, args, .. } => {
                let name = path
                    .segments
                    .last()
                    .map(|s| s.name.as_str())
                    .unwrap_or("");
                if let Some(obj) = Object::empty_named(name) {
                    let kind = obj.kind_name().to_string();
                    let id = self.heap.alloc(obj);
                    // Remember V for map<K,V> / unordered_map<K,V> so operator[] can default-insert.
                    if (kind == "map" || kind == "unordered_map") && args.len() >= 2 {
                        self.map_value_tys.insert(id, args[1].clone());
                    }
                    self.emit_alloc(id, kind, path.span);
                    return Ok(Value::Object(id));
                }
                if self.classes.contains_key(name) {
                    let id = self.heap.alloc(Object::Class {
                        name: name.into(),
                        fields: std::collections::HashMap::new(),
                    });
                    return Ok(Value::Object(id));
                }
                Ok(Value::Int(0))
            }
            Type::Pointer { .. } => Ok(Value::Nullptr),
            Type::Reference { inner, .. } => self.default_value_for_type(inner),
            Type::Const { inner, .. } => self.default_value_for_type(inner),
        }
    }

    /// C++ `map::operator[]`: return existing value, or default-construct mapped type and insert.
    pub(super) fn map_index_get_or_insert(
        &mut self,
        map_id: ObjId,
        key: &MapKey,
        span: Span,
    ) -> Result<Value> {
        let existing = match self.heap.get(map_id) {
            Some(Object::Map(m)) => m.get(key).cloned(),
            Some(Object::UnorderedMap(m)) => m.get(key).cloned(),
            _ => return Err(RuntimeError::at(span, "not a map")),
        };
        if let Some(v) = existing {
            return Ok(v);
        }

        let mapped_ty = self.map_value_tys.get(&map_id).cloned().unwrap_or(Type::Builtin {
            kind: BuiltinType::Int,
            span,
        });
        let def = self.default_value_for_type(&mapped_ty)?;
        match self.heap.get_mut(map_id) {
            Some(Object::Map(m)) => {
                m.insert(key.clone(), def.clone());
            }
            Some(Object::UnorderedMap(m)) => {
                m.insert(key.clone(), def.clone());
            }
            _ => return Err(RuntimeError::at(span, "not a map")),
        }
        self.emit(Event::Write {
            call_id: self.current_call_id(),
            slot: Slot::MapEntry {
                obj: map_id,
                key: key.to_string(),
            },
            old: None,
            value: def.clone(),
            span,
        });
        self.emit(Event::ContainerMod {
            call_id: self.current_call_id(),
            container: Value::Object(map_id),
            kind: "map_default_insert".into(),
            index: None,
            key: Some(key.to_value()),
            old: None,
            value: Some(def.clone()),
            span,
        });
        Ok(def)
    }
}

fn cmp_ord(op: BinaryOp, o: std::cmp::Ordering) -> bool {
    use BinaryOp::*;
    match op {
        Lt => o.is_lt(),
        Gt => o.is_gt(),
        Le => o.is_le(),
        Ge => o.is_ge(),
        Eq => o.is_eq(),
        Ne => o.is_ne(),
        _ => false,
    }
}

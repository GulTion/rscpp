use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn slot_of(lv: &LValue) -> Slot {
        match lv {
            LValue::Name(n) => Slot::Local { name: n.clone() },
            LValue::Index { obj, index } => Slot::Index {
                obj: *obj,
                index: *index,
            },
            LValue::MapEntry { obj, key } => Slot::MapEntry {
                obj: *obj,
                key: key.clone(),
            },
            LValue::Field { obj, field } => Slot::Field {
                obj: *obj,
                field: field.clone(),
            },
        }
    }

    pub(super) fn address_to_slot(addr: &Address) -> Option<Slot> {
        Some(match addr {
            Address::Null => return None,
            Address::Stack { name, .. } => Slot::Local { name: name.clone() },
            Address::Heap(id) => Slot::Object { obj: *id },
            Address::Index { obj, index } => Slot::Index {
                obj: *obj,
                index: *index,
            },
            Address::Field { obj, field } => Slot::Field {
                obj: *obj,
                field: field.clone(),
            },
            Address::MapEntry { obj, key } => Slot::MapEntry {
                obj: *obj,
                key: key.clone(),
            },
        })
    }

    pub(super) fn lvalue_to_address(&self, lv: &LValue) -> Result<Address> {
        Ok(match lv {
            LValue::Name(n) => {
                // Find which frame owns n
                for (i, frame) in self.stack.iter().enumerate().rev() {
                    if frame.locals.contains_key(n) {
                        // If local is already a Ref, address is the target
                        if let Some(Value::Ref(a)) = frame.locals.get(n) {
                            return Ok(a.clone());
                        }
                        return Ok(Address::Stack {
                            frame: i,
                            name: n.clone(),
                        });
                    }
                }
                if self.globals.contains_key(n) {
                    // treat as frame 0 global name via Stack depth usize::MAX? use name-only
                    return Ok(Address::Stack {
                        frame: usize::MAX,
                        name: n.clone(),
                    });
                }
                return Err(RuntimeError::new(format!("cannot take address of `{n}`")));
            }
            LValue::Index { obj, index } => Address::Index {
                obj: *obj,
                index: *index,
            },
            LValue::MapEntry { obj, key } => Address::MapEntry {
                obj: *obj,
                key: key.clone(),
            },
            LValue::Field { obj, field } => Address::Field {
                obj: *obj,
                field: field.clone(),
            },
        })
    }

    pub(super) fn address_to_lvalue(&self, addr: &Address) -> Option<LValue> {
        match addr {
            Address::Stack { name, frame }
                if *frame == usize::MAX || self.stack.get(*frame).is_some() =>
            {
                Some(LValue::Name(name.clone()))
            }
            Address::Index { obj, index } => Some(LValue::Index {
                obj: *obj,
                index: *index,
            }),
            Address::MapEntry { obj, key } => Some(LValue::MapEntry {
                obj: *obj,
                key: key.clone(),
            }),
            Address::Field { obj, field } => Some(LValue::Field {
                obj: *obj,
                field: field.clone(),
            }),
            Address::Heap(_) | Address::Null | Address::Stack { .. } => None,
        }
    }

    pub(super) fn load_address(&self, addr: &Address) -> Result<Value> {
        match addr {
            Address::Null => Err(RuntimeError::new("null pointer dereference")),
            Address::Stack { frame, name } if *frame == usize::MAX => self
                .globals
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::new(format!("dangling ref `{name}`"))),
            Address::Stack { frame, name } => {
                let frame = self
                    .stack
                    .get(*frame)
                    .ok_or_else(|| RuntimeError::new("dangling stack address"))?;
                frame
                    .locals
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new(format!("dangling ref `{name}`")))
            }
            Address::Heap(id) => Ok(Value::Object(*id)),
            Address::Index { obj, index } => match self.heap.get(*obj) {
                Some(Object::Vector(e)) | Some(Object::List(e)) => e
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("index out of bounds")),
                Some(Object::Array { elems, n }) if *index < *n => elems
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("index out of bounds")),
                Some(Object::Deque(e)) => e
                    .get(*index)
                    .cloned()
                    .ok_or_else(|| RuntimeError::new("index out of bounds")),
                Some(Object::Bitset { bits }) => bits
                    .get(*index)
                    .map(|b| Value::Bool(*b))
                    .ok_or_else(|| RuntimeError::new("bitset index out of bounds")),
                _ => Err(RuntimeError::new("bad index address")),
            },
            Address::Field { obj, field } => match self.heap.get(*obj) {
                Some(Object::Class { fields, .. }) => {
                    Ok(fields.get(field).cloned().unwrap_or(Value::Int(0)))
                }
                Some(Object::Pair { first, second }) => match field.as_str() {
                    "first" => Ok(first.clone()),
                    "second" => Ok(second.clone()),
                    _ => Err(RuntimeError::new("bad field")),
                },
                _ => Err(RuntimeError::new("bad field address")),
            },
            Address::MapEntry { obj, key } => match self.heap.get(*obj) {
                Some(Object::Map(m)) => Ok(m.get(key).cloned().unwrap_or(Value::Int(0))),
                Some(Object::UnorderedMap(m)) => {
                    Ok(m.get(key).cloned().unwrap_or(Value::Int(0)))
                }
                _ => Err(RuntimeError::new("bad map address")),
            },
        }
    }

    pub(super) fn store_address(&mut self, addr: &Address, val: Value, span: Span) -> Result<()> {
        match addr {
            Address::Null => Err(RuntimeError::at(span, "null pointer write")),
            Address::Stack { frame, name } if *frame == usize::MAX => {
                self.globals.insert(name.clone(), val.clone());
                self.emit_mutation(Event::Write {
                    slot: Slot::Global { name: name.clone() },
                    old: None,
                    value: val,
                    span,
                });
                Ok(())
            }
            Address::Stack { frame, name } => {
                let frame = self
                    .stack
                    .get_mut(*frame)
                    .ok_or_else(|| RuntimeError::at(span, "dangling stack address"))?;
                let old = frame.locals.insert(name.clone(), val.clone());
                self.emit_mutation(Event::Write {
                    slot: Slot::Local { name: name.clone() },
                    old,
                    value: val,
                    span,
                });
                Ok(())
            }
            Address::Heap(_) => Err(RuntimeError::at(
                span,
                "cannot store through object address",
            )),
            other => {
                let Some(lv) = self.address_to_lvalue(other) else {
                    return Err(RuntimeError::at(span, "cannot store to address"));
                };
                self.write_lvalue(&lv, val, span)
            }
        }
    }
}

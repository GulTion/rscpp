use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn define_local(&mut self, name: &str, val: Value, span: Span) -> Result<()> {
        if let Value::Ref(addr) = &val {
            if let Some(slot) = Self::address_to_slot(addr) {
                self.emit(Event::RefBind {
                    name: name.to_string(),
                    target: slot,
                    span,
                });
            }
        }
        if let Value::Ptr(addr) = &val {
            self.emit(Event::PtrMove {
                name: name.to_string(),
                to: Value::Ptr(addr.clone()),
                span,
            });
        }
        if let Some(frame) = self.stack.last_mut() {
            frame.locals.insert(name.to_string(), val.clone());
        } else {
            self.globals.insert(name.to_string(), val.clone());
        }
        self.emit(Event::VarCreate {
            name: name.to_string(),
            value: val,
            span,
        });
        Ok(())
    }

    pub(super) fn lookup_raw(&self, name: &str) -> Result<Value> {
        for frame in self.stack.iter().rev() {
            if let Some(v) = frame.locals.get(name) {
                return Ok(v.clone());
            }
        }
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        Err(RuntimeError::new(format!("undefined variable `{name}`")))
    }

    #[allow(dead_code)]
    pub(super) fn lookup(&self, name: &str) -> Result<Value> {
        let v = self.lookup_raw(name)?;
        match v {
            Value::Ref(addr) => self.load_address(&addr),
            other => Ok(other),
        }
    }

    pub(super) fn assign_name(&mut self, name: &str, val: Value, span: Span) -> Result<()> {
        // Write-through references.
        if let Ok(Value::Ref(addr)) = self.lookup_raw(name) {
            return self.store_address(&addr, val, span);
        }

        for i in (0..self.stack.len()).rev() {
            if self.stack[i].locals.contains_key(name) {
                let old = self.stack[i].locals.get(name).cloned();
                if matches!(val, Value::Ptr(_)) {
                    self.emit(Event::PtrMove {
                        name: name.to_string(),
                        to: val.clone(),
                        span,
                    });
                }
                self.stack[i].locals.insert(name.to_string(), val.clone());
                self.emit(Event::VarAssign {
                    name: name.to_string(),
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::Write {
                    slot: Slot::Local {
                        name: name.to_string(),
                    },
                    old,
                    value: val,
                    span,
                });
                return Ok(());
            }
        }
        if self.globals.contains_key(name) {
            let old = self.globals.get(name).cloned();
            if matches!(val, Value::Ptr(_)) {
                self.emit(Event::PtrMove {
                    name: name.to_string(),
                    to: val.clone(),
                    span,
                });
            }
            self.globals.insert(name.to_string(), val.clone());
            self.emit(Event::VarAssign {
                name: name.to_string(),
                old: old.clone(),
                value: val.clone(),
                span,
            });
            self.emit(Event::Write {
                slot: Slot::Global {
                    name: name.to_string(),
                },
                old,
                value: val,
                span,
            });
            return Ok(());
        }
        Err(RuntimeError::at(
            span,
            format!("assignment to undeclared `{name}`"),
        ))
    }

    pub(super) fn write_lvalue(&mut self, lv: &LValue, val: Value, span: Span) -> Result<()> {
        match lv {
            LValue::Name(n) => self.assign_name(n, val, span),
            LValue::Index { obj, index } => {
                match self.heap.get_mut(*obj) {
                    Some(Object::Vector(elems)) | Some(Object::List(elems)) => {
                        if *index >= elems.len() {
                            return Err(RuntimeError::at(span, "index out of bounds"));
                        }
                        let old = elems[*index].clone();
                        elems[*index] = val.clone();
                        self.emit(Event::Write {
                            slot: Slot::Index {
                                obj: *obj,
                                index: *index,
                            },
                            old: Some(old.clone()),
                            value: val.clone(),
                            span,
                        });
                        self.emit(Event::ContainerMod {
                            container: Value::Object(*obj),
                            kind: "index_assign".into(),
                            index: Some(*index),
                            key: Some(Value::Int(*index as i64)),
                            old: Some(old),
                            value: Some(val),
                            elems: vec![],
                            span,
                        });
                        Ok(())
                    }
                    Some(Object::Array { elems, n }) => {
                        if *index >= *n {
                            return Err(RuntimeError::at(span, "index out of bounds"));
                        }
                        let old = elems[*index].clone();
                        elems[*index] = val.clone();
                        self.emit(Event::Write {
                            slot: Slot::Index {
                                obj: *obj,
                                index: *index,
                            },
                            old: Some(old.clone()),
                            value: val.clone(),
                            span,
                        });
                        self.emit(Event::ContainerMod {
                            container: Value::Object(*obj),
                            kind: "index_assign".into(),
                            index: Some(*index),
                            key: Some(Value::Int(*index as i64)),
                            old: Some(old),
                            value: Some(val),
                            elems: vec![],
                            span,
                        });
                        Ok(())
                    }
                    Some(Object::Deque(elems)) => {
                        if *index >= elems.len() {
                            return Err(RuntimeError::at(span, "index out of bounds"));
                        }
                        let old = elems[*index].clone();
                        elems[*index] = val.clone();
                        self.emit(Event::Write {
                            slot: Slot::Index {
                                obj: *obj,
                                index: *index,
                            },
                            old: Some(old.clone()),
                            value: val.clone(),
                            span,
                        });
                        self.emit(Event::ContainerMod {
                            container: Value::Object(*obj),
                            kind: "index_assign".into(),
                            index: Some(*index),
                            key: Some(Value::Int(*index as i64)),
                            old: Some(old),
                            value: Some(val),
                            elems: vec![],
                            span,
                        });
                        Ok(())
                    }
                    _ => Err(RuntimeError::at(span, "index assignment on non-sequence")),
                }
            }
            LValue::MapEntry { obj, key } => {
                let key_s = key.to_string();
                let old = match self.heap.get(*obj) {
                    Some(Object::Map(m)) => m.get(key).cloned(),
                    Some(Object::UnorderedMap(m)) => m.get(key).cloned(),
                    _ => None,
                };
                match self.heap.get_mut(*obj) {
                    Some(Object::Map(m)) => {
                        m.insert(key.clone(), val.clone());
                    }
                    Some(Object::UnorderedMap(m)) => {
                        m.insert(key.clone(), val.clone());
                    }
                    _ => return Err(RuntimeError::at(span, "map entry assign on non-map")),
                }
                self.emit(Event::Write {
                    slot: Slot::MapEntry {
                        obj: *obj,
                        key: key_s,
                    },
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::ContainerMod {
                    container: Value::Object(*obj),
                    kind: "map_assign".into(),
                    index: None,
                    key: Some(key.to_value()),
                    old,
                    value: Some(val),
                    elems: vec![],
                    span,
                });
                Ok(())
            }
            LValue::Field { obj, field } => {
                let old = match self.heap.get(*obj) {
                    Some(Object::Class { fields, .. }) => fields.get(field).cloned(),
                    Some(Object::Pair { first, second }) => match field.as_str() {
                        "first" => Some(first.clone()),
                        "second" => Some(second.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                match self.heap.get_mut(*obj) {
                    Some(Object::Class { fields, .. }) => {
                        fields.insert(field.clone(), val.clone());
                    }
                    Some(Object::Pair { first, second }) => match field.as_str() {
                        "first" => *first = val.clone(),
                        "second" => *second = val.clone(),
                        _ => {
                            return Err(RuntimeError::at(
                                span,
                                format!("no field `{field}` on pair"),
                            ));
                        }
                    },
                    _ => return Err(RuntimeError::at(span, "field assign on bad object")),
                }
                self.emit(Event::Write {
                    slot: Slot::Field {
                        obj: *obj,
                        field: field.clone(),
                    },
                    old: old.clone(),
                    value: val.clone(),
                    span,
                });
                self.emit(Event::VarAssign {
                    name: field.clone(),
                    old,
                    value: val,
                    span,
                });
                Ok(())
            }
        }
    }
}

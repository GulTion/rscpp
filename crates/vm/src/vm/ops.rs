use super::{Frame, Result, Vm};
use crate::chunk::{Op, Program};
use crate::error::VmError;
use rscpp_ast::Span;
use rscpp_runtime::{Event, Heap, MapKey, Object, Slot, Value};

impl Vm {
    pub(super) fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| VmError::new("stack underflow"))
    }

    pub(super) fn index_get(&mut self, base: Value, idx: Value, span: Span) -> Result<Value> {
        let Value::Object(id) = base.clone() else {
            return Err(VmError::at(span, "index on non-object"));
        };
        let v = match self.heap.get(id) {
            Some(Object::Vector(e)) => {
                let i = idx.as_int().map_err(VmError::new)? as usize;
                e.get(i).cloned().ok_or_else(|| VmError::at(span, "oob"))?
            }
            Some(Object::Map(m)) => {
                let k = super::map_key(&idx)?;
                m.get(&k).cloned().unwrap_or(Value::Int(0))
            }
            Some(Object::UnorderedMap(m)) => {
                let k = super::map_key(&idx)?;
                m.get(&k).cloned().unwrap_or(Value::Int(0))
            }
            _ => return Err(VmError::at(span, "not indexable")),
        };
        Ok(rscpp_runtime::stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
        }
        .query(base, "index", Some(idx), v, span))
    }

    pub(super) fn index_set(
        &mut self,
        base: Value,
        idx: Value,
        val: Value,
        span: Span,
    ) -> Result<()> {
        let Value::Object(id) = base else {
            return Err(VmError::at(span, "index set on non-object"));
        };
        match self.heap.get_mut(id) {
            Some(Object::Vector(e)) => {
                let i = idx.as_int().map_err(VmError::new)? as usize;
                if i >= e.len() {
                    return Err(VmError::at(span, "oob"));
                }
                let old = e[i].clone();
                e[i] = val.clone();
                self.emit(Event::Write {
                    slot: Slot::Index { obj: id, index: i },
                    old: Some(old),
                    value: val,
                    span,
                });
            }
            Some(Object::Map(m)) => {
                let k = super::map_key(&idx)?;
                let old = m.insert(k.clone(), val.clone());
                self.emit(Event::Write {
                    slot: Slot::MapEntry {
                        obj: id,
                        key: k.clone(),
                    },
                    old,
                    value: val,
                    span,
                });
            }
            Some(Object::UnorderedMap(m)) => {
                let k = super::map_key(&idx)?;
                let old = m.insert(k.clone(), val.clone());
                self.emit(Event::Write {
                    slot: Slot::MapEntry {
                        obj: id,
                        key: k.clone(),
                    },
                    old,
                    value: val,
                    span,
                });
            }
            _ => return Err(VmError::at(span, "not indexable")),
        }
        Ok(())
    }

    pub(super) fn call_method(
        &mut self,
        base: Value,
        method: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value> {
        let Value::Object(id) = base.clone() else {
            return Err(VmError::at(span, "method on non-object"));
        };
        let kind = self
            .heap
            .get(id)
            .map(|o| o.kind_name())
            .ok_or_else(|| VmError::at(span, "dangling object"))?;

        let mut ctx = rscpp_runtime::stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
        };
        rscpp_runtime::stl::call_method(&mut ctx, id, base, kind, method, args, span)
            .map_err(VmError::from)
    }

    pub fn make_vector(&mut self, elems: Vec<Value>) -> Value {
        let id = self.heap.alloc(Object::Vector(elems));
        Value::Object(id)
    }

    pub fn vector_as_ints(&self, v: &Value) -> Result<Vec<i64>> {
        let Value::Object(id) = v else {
            return Err(VmError::new("expected vector"));
        };
        match self.heap.get(*id) {
            Some(Object::Vector(e)) => e.iter().map(|x| x.as_int().map_err(VmError::new)).collect(),
            _ => Err(VmError::new("expected vector")),
        }
    }
}

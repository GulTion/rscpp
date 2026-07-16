use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {

    pub(super) fn builtin_swap(&mut self, a: &Expr, b: &Expr, span: Span) -> Result<Value> {
        let (va, la) = self.eval_expr_lv(a)?;
        let (vb, lb) = self.eval_expr_lv(b)?;
        let Some(la) = la else {
            return Err(RuntimeError::at(span, "swap arg is not an lvalue"));
        };
        let Some(lb) = lb else {
            return Err(RuntimeError::at(span, "swap arg is not an lvalue"));
        };
        self.write_lvalue(&la, vb.clone(), span)?;
        self.write_lvalue(&lb, va.clone(), span)?;
        self.emit(Event::Swap {
            call_id: self.current_call_id(),
            a: Self::slot_of(&la),
            b: Self::slot_of(&lb),
            value_a: va,
            value_b: vb,
            span,
        });
        Ok(Value::Void)
    }

    /// `sort(v.begin(), v.end())` — only this pattern; sorts the vector in place.
    pub(super) fn builtin_sort(&mut self, begin: &Expr, end: &Expr, span: Span) -> Result<Value> {
        let id = match (begin, end) {
            (
                Expr::Call {
                    callee: bcal, args: ba, ..
                },
                Expr::Call {
                    callee: ecal, args: ea, ..
                },
            ) if ba.is_empty() && ea.is_empty() => {
                let (bname, bbase) = match bcal.as_ref() {
                    Expr::Member {
                        base,
                        field,
                        arrow: false,
                        ..
                    } if field.name == "begin" => ("begin", base.as_ref()),
                    _ => return Err(RuntimeError::at(span, "sort: expected v.begin()")),
                };
                let (ename, ebase) = match ecal.as_ref() {
                    Expr::Member {
                        base,
                        field,
                        arrow: false,
                        ..
                    } if field.name == "end" => ("end", base.as_ref()),
                    _ => return Err(RuntimeError::at(span, "sort: expected v.end()")),
                };
                let _ = (bname, ename);
                let bbase_v = self.eval_expr(bbase)?;
                let ebase_v = self.eval_expr(ebase)?;
                if bbase_v != ebase_v {
                    return Err(RuntimeError::at(span, "sort: begin/end from different objects"));
                }
                let Value::Object(id) = bbase_v else {
                    return Err(RuntimeError::at(span, "sort: not a vector"));
                };
                id
            }
            _ => {
                return Err(RuntimeError::at(
                    span,
                    "sort only supports sort(v.begin(), v.end())",
                ))
            }
        };
        if let Some(Object::Vector(elems)) = self.heap.get_mut(id) {
            elems.sort_by(|a, b| {
                let ai = a.as_int().unwrap_or(0);
                let bi = b.as_int().unwrap_or(0);
                ai.cmp(&bi)
            });
        } else {
            return Err(RuntimeError::at(span, "sort: not a vector"));
        }
        self.emit(Event::ContainerMod {
            call_id: self.current_call_id(),
            container: Value::Object(id),
            kind: "sort".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            span,
        });
        Ok(Value::Void)
    }

    pub(super) fn resolve_vector_range(&mut self, begin: &Expr, end: &Expr, span: Span) -> Result<ObjId> {
        match (begin, end) {
            (
                Expr::Call {
                    callee: bcal, args: ba, ..
                },
                Expr::Call {
                    callee: ecal, args: ea, ..
                },
            ) if ba.is_empty() && ea.is_empty() => {
                let bbase = match bcal.as_ref() {
                    Expr::Member {
                        base,
                        field,
                        arrow: false,
                        ..
                    } if field.name == "begin" => base.as_ref(),
                    _ => return Err(RuntimeError::at(span, "expected v.begin()")),
                };
                let ebase = match ecal.as_ref() {
                    Expr::Member {
                        base,
                        field,
                        arrow: false,
                        ..
                    } if field.name == "end" => base.as_ref(),
                    _ => return Err(RuntimeError::at(span, "expected v.end()")),
                };
                let b = self.eval_expr(bbase)?;
                let e = self.eval_expr(ebase)?;
                if b != e {
                    return Err(RuntimeError::at(span, "begin/end from different objects"));
                }
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(span, "range must be vector"));
                };
                match self.heap.get(id) {
                    Some(Object::Vector(_)) => Ok(id),
                    _ => Err(RuntimeError::at(span, "range must be vector")),
                }
            }
            _ => Err(RuntimeError::at(
                span,
                "algorithm expects (v.begin(), v.end())",
            )),
        }
    }

    pub(super) fn builtin_reverse(&mut self, begin: &Expr, end: &Expr, span: Span) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        if let Some(Object::Vector(elems)) = self.heap.get_mut(id) {
            elems.reverse();
        }
        self.emit(Event::ContainerMod {
            call_id: self.current_call_id(),
            container: Value::Object(id),
            kind: "reverse".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            span,
        });
        Ok(Value::Void)
    }

    pub(super) fn builtin_binary_search(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let t = target.as_int().map_err(RuntimeError::new)?;
        let found = match self.heap.get(id) {
            Some(Object::Vector(elems)) => elems
                .iter()
                .any(|v| v.as_int().map(|n| n == t).unwrap_or(false)),
            _ => false,
        };
        Ok(Value::Bool(found))
    }

    pub(super) fn builtin_bound(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        upper: bool,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let t = target.as_int().map_err(RuntimeError::new)?;
        let idx = match self.heap.get(id) {
            Some(Object::Vector(elems)) => {
                let mut i = 0usize;
                while i < elems.len() {
                    let n = elems[i].as_int().map_err(RuntimeError::new)?;
                    if (upper && n > t) || (!upper && n >= t) {
                        break;
                    }
                    i += 1;
                }
                i as i64
            }
            _ => 0,
        };
        Ok(Value::Int(idx))
    }

    pub(super) fn builtin_accumulate(
        &mut self,
        begin: &Expr,
        end: &Expr,
        init: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let mut sum = init
            .map(|v| v.as_int().map_err(RuntimeError::new))
            .transpose()?
            .unwrap_or(0);
        if let Some(Object::Vector(elems)) = self.heap.get(id) {
            for v in elems {
                sum += v.as_int().map_err(RuntimeError::new)?;
            }
        }
        Ok(Value::Int(sum))
    }

    pub(super) fn builtin_element_ptr(
        &mut self,
        begin: &Expr,
        end: &Expr,
        want_max: bool,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let (idx, _val) = match self.heap.get(id) {
            Some(Object::Vector(elems)) if !elems.is_empty() => {
                let mut best_i = 0usize;
                let mut best_v = elems[0].as_int().map_err(RuntimeError::new)?;
                for (i, v) in elems.iter().enumerate().skip(1) {
                    let n = v.as_int().map_err(RuntimeError::new)?;
                    if (want_max && n > best_v) || (!want_max && n < best_v) {
                        best_v = n;
                        best_i = i;
                    }
                }
                (best_i, best_v)
            }
            _ => return Err(RuntimeError::at(span, "min/max_element on empty/non-vector")),
        };
        Ok(Value::Ptr(Address::Index { obj: id, index: idx }))
}
}

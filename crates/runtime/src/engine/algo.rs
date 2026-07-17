use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
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
            a: Self::slot_of(&la),
            b: Self::slot_of(&lb),
            value_a: va,
            value_b: vb,
            span,
        });
        Ok(Value::Void)
    }

    /// `sort(v.begin(), v.end())` / optional comparator lambda.
    pub(super) fn builtin_sort(
        &mut self,
        begin: &Expr,
        end: &Expr,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = match (begin, end) {
            (
                Expr::Call {
                    callee: bcal,
                    args: ba,
                    ..
                },
                Expr::Call {
                    callee: ecal,
                    args: ea,
                    ..
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
                    return Err(RuntimeError::at(
                        span,
                        "sort: begin/end from different objects",
                    ));
                }
                let Value::Object(id) = bbase_v else {
                    return Err(RuntimeError::at(span, "sort: not a vector"));
                };
                id
            }
            _ => {
                // Also `sort(begin(v), end(v))`
                match self.resolve_vector_range(begin, end, span) {
                    Ok(id) => id,
                    Err(_) => {
                        return Err(RuntimeError::at(
                            span,
                            "sort only supports sort(v.begin(), v.end())",
                        ))
                    }
                }
            }
        };
        if let Some(Value::Object(opid)) = cmp {
            match self.heap.get(*opid).cloned() {
                Some(Object::Closure { .. }) => {
                    let mut elems = match self.heap.get(id) {
                        Some(Object::Vector(e)) => e.clone(),
                        _ => return Err(RuntimeError::at(span, "sort: not a vector")),
                    };
                    for i in 1..elems.len() {
                        let mut j = i;
                        while j > 0 {
                            let less = self.call_closure(
                                *opid,
                                &[elems[j].clone(), elems[j - 1].clone()],
                                span,
                            )?;
                            if less.as_bool().map_err(RuntimeError::new)? {
                                elems.swap(j, j - 1);
                                j -= 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if let Some(Object::Vector(dst)) = self.heap.get_mut(id) {
                        *dst = elems;
                    }
                }
                Some(Object::Functor { kind }) => {
                    let mut elems = match self.heap.get(id) {
                        Some(Object::Vector(e)) => e.clone(),
                        _ => return Err(RuntimeError::at(span, "sort: not a vector")),
                    };
                    for i in 1..elems.len() {
                        let mut j = i;
                        while j > 0 {
                            let less = super::functor::functor_apply(
                                kind,
                                &[elems[j].clone(), elems[j - 1].clone()],
                                span,
                            )?;
                            if less.as_bool().map_err(RuntimeError::new)? {
                                elems.swap(j, j - 1);
                                j -= 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if let Some(Object::Vector(dst)) = self.heap.get_mut(id) {
                        *dst = elems;
                    }
                }
                _ => {
                    return Err(RuntimeError::at(
                        span,
                        "sort comparator must be a lambda or functional object",
                    ))
                }
            }
        } else if let Some(Object::Vector(elems)) = self.heap.get_mut(id) {
            elems.sort_by(|a, b| {
                let ai = a.as_int().unwrap_or(0);
                let bi = b.as_int().unwrap_or(0);
                ai.cmp(&bi)
            });
        } else {
            return Err(RuntimeError::at(span, "sort: not a vector"));
        }
        let elems = self.sequence_elems(id);
        self.emit(Event::ContainerMod {
            container: Value::Object(id),
            kind: "sort".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            elems,
            span,
        });
        Ok(Value::Void)
    }

    pub(super) fn resolve_vector_range(
        &mut self,
        begin: &Expr,
        end: &Expr,
        span: Span,
    ) -> Result<ObjId> {
        /// `v.begin()` / `begin(v)` / `cbegin(v)` → container expr
        fn range_base<'a>(call: &'a Expr) -> Option<&'a Expr> {
            let Expr::Call { callee, args, .. } = call else {
                return None;
            };
            match callee.as_ref() {
                Expr::Member {
                    base,
                    field,
                    arrow: false,
                    ..
                } if matches!(field.name.as_str(), "begin" | "end" | "cbegin" | "cend")
                    && args.is_empty() =>
                {
                    Some(base.as_ref())
                }
                Expr::Name(path)
                    if args.len() == 1
                        && matches!(
                            path.segments.last().map(|s| s.name.as_str()),
                            Some("begin" | "end" | "cbegin" | "cend")
                        ) =>
                {
                    Some(&args[0])
                }
                _ => None,
            }
        }

        let (Some(bbase), Some(ebase)) = (range_base(begin), range_base(end)) else {
            return Err(RuntimeError::at(
                span,
                "algorithm expects (v.begin(), v.end()) or (begin(v), end(v))",
            ));
        };
        let b = self.eval_expr(bbase)?;
        let e = self.eval_expr(ebase)?;
        if b != e {
            return Err(RuntimeError::at(span, "begin/end from different objects"));
        }
        let Value::Object(id) = b else {
            return Err(RuntimeError::at(span, "range must be a container"));
        };
        match self.heap.get(id) {
            Some(Object::Vector(_)) | Some(Object::String(_)) => Ok(id),
            _ => Err(RuntimeError::at(span, "range must be vector or string")),
        }
    }

    pub(super) fn builtin_reverse(
        &mut self,
        begin: &Expr,
        end: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        match self.heap.get_mut(id) {
            Some(Object::Vector(elems)) => elems.reverse(),
            Some(Object::String(s)) => {
                let rev: String = s.chars().rev().collect();
                *s = rev;
            }
            _ => {}
        }
        let elems = self.sequence_elems(id);
        self.emit(Event::ContainerMod {
            container: Value::Object(id),
            kind: "reverse".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            elems,
            span,
        });
        Ok(Value::Void)
    }

    pub(super) fn builtin_binary_search(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems: Vec<Value> = match self.heap.get(id) {
            Some(Object::Vector(e)) => e.clone(),
            _ => return Ok(Value::Bool(false)),
        };
        for v in &elems {
            let less_vx = self.cmp_less(cmp, v, target, span)?;
            let less_xv = self.cmp_less(cmp, target, v, span)?;
            if !less_vx && !less_xv {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    pub(super) fn builtin_bound(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        upper: bool,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems: Vec<Value> = match self.heap.get(id) {
            Some(Object::Vector(e)) => e.clone(),
            _ => return Ok(Value::Int(0)),
        };
        let mut i = 0usize;
        while i < elems.len() {
            let hit = if upper {
                self.cmp_less(cmp, target, &elems[i], span)?
            } else {
                !self.cmp_less(cmp, &elems[i], target, span)?
            };
            if hit {
                break;
            }
            i += 1;
        }
        Ok(Value::Int(i as i64))
    }

    /// `cmp(a,b)` if closure/functor; else int `a < b`.
    fn cmp_less(&mut self, cmp: Option<&Value>, a: &Value, b: &Value, span: Span) -> Result<bool> {
        if let Some(Value::Object(opid)) = cmp {
            match self.heap.get(*opid).cloned() {
                Some(Object::Closure { .. }) => {
                    let v = self.call_closure(*opid, &[a.clone(), b.clone()], span)?;
                    return v.as_bool().map_err(RuntimeError::new);
                }
                Some(Object::Functor { kind }) => {
                    let v = super::functor::functor_apply(kind, &[a.clone(), b.clone()], span)?;
                    return v.as_bool().map_err(RuntimeError::new);
                }
                _ => {
                    return Err(RuntimeError::at(
                        span,
                        "comparator must be a lambda or functional object",
                    ))
                }
            }
        }
        let ai = a.as_int().map_err(RuntimeError::new)?;
        let bi = b.as_int().map_err(RuntimeError::new)?;
        Ok(ai < bi)
    }

    pub(super) fn builtin_accumulate(
        &mut self,
        begin: &Expr,
        end: &Expr,
        init: Option<&Value>,
        op: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let mut acc = init.cloned().unwrap_or(Value::Int(0));
        let elems: Vec<Value> = match self.heap.get(id) {
            Some(Object::Vector(e)) => e.clone(),
            Some(Object::String(s)) => s.chars().map(Value::Char).collect(),
            _ => vec![],
        };
        if let Some(Value::Object(opid)) = op {
            match self.heap.get(*opid).cloned() {
                Some(Object::Closure { .. }) => {
                    for v in elems {
                        acc = self.call_closure(*opid, &[acc, v], span)?;
                    }
                    return Ok(acc);
                }
                Some(Object::Functor { kind }) => {
                    for v in elems {
                        acc = super::functor::functor_apply(kind, &[acc, v], span)?;
                    }
                    return Ok(acc);
                }
                _ => {}
            }
        }
        // Default: sum as ints
        let mut sum = acc.as_int().map_err(RuntimeError::new)?;
        for v in elems {
            sum += v.as_int().map_err(RuntimeError::new)?;
        }
        Ok(Value::Int(sum))
    }

    pub(super) fn builtin_iota(
        &mut self,
        begin: &Expr,
        end: &Expr,
        start: &Value,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let mut n = start.as_int().map_err(RuntimeError::new)?;
        if let Some(Object::Vector(elems)) = self.heap.get_mut(id) {
            for e in elems.iter_mut() {
                *e = Value::Int(n);
                n += 1;
            }
        }
        let elems = self.sequence_elems(id);
        self.emit(Event::ContainerMod {
            container: Value::Object(id),
            kind: "iota".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            elems,
            span,
        });
        Ok(Value::Void)
    }

    /// `partial_sum(begin, end, begin)` — in-place prefix sums (LeetCode runningSum style).
    pub(super) fn builtin_partial_sum(
        &mut self,
        begin: &Expr,
        end: &Expr,
        _out: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        if let Some(Object::Vector(elems)) = self.heap.get_mut(id) {
            let mut acc = 0i64;
            for e in elems.iter_mut() {
                acc += e.as_int().unwrap_or(0);
                *e = Value::Int(acc);
            }
        }
        let elems = self.sequence_elems(id);
        self.emit(Event::ContainerMod {
            container: Value::Object(id),
            kind: "partial_sum".into(),
            index: None,
            key: None,
            old: None,
            value: None,
            elems,
            span,
        });
        Ok(Value::Void)
    }

    pub(super) fn builtin_element_ptr(
        &mut self,
        begin: &Expr,
        end: &Expr,
        want_max: bool,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems: Vec<Value> = match self.heap.get(id) {
            Some(Object::Vector(e)) if !e.is_empty() => e.clone(),
            _ => {
                return Err(RuntimeError::at(
                    span,
                    "min/max_element on empty/non-vector",
                ))
            }
        };
        let mut best_i = 0usize;
        for i in 1..elems.len() {
            let take = if want_max {
                self.cmp_less(cmp, &elems[best_i], &elems[i], span)?
            } else {
                self.cmp_less(cmp, &elems[i], &elems[best_i], span)?
            };
            if take {
                best_i = i;
            }
        }
        Ok(Value::Ptr(Address::Index {
            obj: id,
            index: best_i,
        }))
    }
}

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
        self.suppress_write_events = true;
        let w = (|| {
            self.write_lvalue(&la, vb.clone(), span)?;
            self.write_lvalue(&lb, va.clone(), span)
        })();
        self.suppress_write_events = false;
        w?;
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
        Ok(self.resolve_sequence_range(begin, end, span)?.0)
    }

    /// Resolve `(v.begin(), v.end())` / reverse pair. Returns `(container, reversed)`.
    pub(super) fn resolve_sequence_range(
        &mut self,
        begin: &Expr,
        end: &Expr,
        span: Span,
    ) -> Result<(ObjId, bool)> {
        fn iter_info<'a>(call: &'a Expr) -> Option<(&'a Expr, bool)> {
            let Expr::Call { callee, args, .. } = call else {
                return None;
            };
            match callee.as_ref() {
                Expr::Member {
                    base,
                    field,
                    arrow: false,
                    ..
                } if args.is_empty() => {
                    let rev = matches!(
                        field.name.as_str(),
                        "rbegin" | "rend" | "crbegin" | "crend"
                    );
                    let fwd = matches!(
                        field.name.as_str(),
                        "begin" | "end" | "cbegin" | "cend"
                    );
                    if rev || fwd {
                        Some((base.as_ref(), rev))
                    } else {
                        None
                    }
                }
                Expr::Name(path)
                    if args.len() == 1
                        && matches!(
                            path.segments.last().map(|s| s.name.as_str()),
                            Some(
                                "begin" | "end" | "cbegin" | "cend" | "rbegin" | "rend"
                                    | "crbegin" | "crend"
                            )
                        ) =>
                {
                    let name = path.segments.last().unwrap().name.as_str();
                    let rev = matches!(name, "rbegin" | "rend" | "crbegin" | "crend");
                    Some((&args[0], rev))
                }
                _ => None,
            }
        }

        let (Some((bbase, brev)), Some((ebase, erev))) = (iter_info(begin), iter_info(end)) else {
            return Err(RuntimeError::at(
                span,
                "algorithm expects (v.begin(), v.end()) or (begin(v), end(v))",
            ));
        };
        if brev != erev {
            return Err(RuntimeError::at(span, "mixed forward/reverse iterators"));
        }
        let b = self.eval_expr(bbase)?;
        let e = self.eval_expr(ebase)?;
        if b != e {
            return Err(RuntimeError::at(span, "begin/end from different objects"));
        }
        let Value::Object(id) = b else {
            return Err(RuntimeError::at(span, "range must be a container"));
        };
        match self.heap.get(id) {
            Some(Object::Vector(_))
            | Some(Object::Deque(_))
            | Some(Object::List(_))
            | Some(Object::Array { .. })
            | Some(Object::String(_)) => Ok((id, brev)),
            _ => Err(RuntimeError::at(span, "range must be a sequence container")),
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
        self.element_ptr_on(id, want_max, cmp, span)
    }

    /// `ranges::max_element(v)` / `ranges::max_element(v, cmp)`.
    pub(super) fn builtin_element_on_range(
        &mut self,
        range: &Expr,
        want_max: bool,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let v = self.eval_expr(range)?;
        let Value::Object(id) = v else {
            return Err(RuntimeError::at(span, "min/max_element expects a container"));
        };
        self.element_ptr_on(id, want_max, cmp, span)
    }

    fn element_ptr_on(
        &mut self,
        id: ObjId,
        want_max: bool,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
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

    fn sequence_values(&self, id: ObjId, span: Span) -> Result<Vec<Value>> {
        match self.heap.get(id) {
            Some(Object::Vector(e)) | Some(Object::List(e)) => Ok(e.clone()),
            Some(Object::Array { elems, n }) => Ok(elems.iter().take(*n).cloned().collect()),
            Some(Object::Deque(d)) => Ok(d.iter().cloned().collect()),
            Some(Object::String(s)) => Ok(s.chars().map(Value::Char).collect()),
            _ => Err(RuntimeError::at(span, "not a sequence")),
        }
    }

    fn write_sequence_values(&mut self, id: ObjId, elems: Vec<Value>, span: Span) -> Result<()> {
        match self.heap.get_mut(id) {
            Some(Object::Vector(e)) | Some(Object::List(e)) => *e = elems,
            Some(Object::Array {
                elems: dst,
                n,
            }) => {
                if elems.len() != *n {
                    return Err(RuntimeError::at(span, "array size mismatch"));
                }
                *dst = elems;
            }
            Some(Object::Deque(d)) => {
                d.clear();
                d.extend(elems);
            }
            _ => return Err(RuntimeError::at(span, "cannot write sequence")),
        }
        Ok(())
    }

    fn call_pred(&mut self, pred: &Value, args: &[Value], span: Span) -> Result<bool> {
        let Value::Object(opid) = pred else {
            return Err(RuntimeError::at(span, "predicate must be callable"));
        };
        let v = match self.heap.get(*opid).cloned() {
            Some(Object::Closure { .. }) => self.call_closure(*opid, args, span)?,
            Some(Object::Functor { kind }) => {
                super::functor::functor_apply(kind, args, span)?
            }
            _ => {
                return Err(RuntimeError::at(
                    span,
                    "predicate must be a lambda or functional object",
                ))
            }
        };
        v.as_bool().map_err(RuntimeError::new)
    }

    fn values_eq(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Char(x), Value::Char(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            _ => a == b,
        }
    }

    pub(super) fn builtin_all_any_none(
        &mut self,
        begin: &Expr,
        end: &Expr,
        pred: &Value,
        mode: u8, // 0 all, 1 any, 2 none
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        let mut any = false;
        let mut all = true;
        for e in &elems {
            let ok = self.call_pred(pred, &[e.clone()], span)?;
            any |= ok;
            all &= ok;
        }
        let out = match mode {
            0 => all,
            1 => any,
            _ => !any,
        };
        Ok(Value::Bool(out || (mode == 0 && elems.is_empty())))
    }

    pub(super) fn builtin_find(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        for (i, e) in elems.iter().enumerate() {
            if Self::values_eq(e, target) {
                return Ok(Value::Int(i as i64));
            }
        }
        Ok(Value::Int(elems.len() as i64))
    }

    pub(super) fn builtin_find_if(
        &mut self,
        begin: &Expr,
        end: &Expr,
        pred: &Value,
        negate: bool,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        for (i, e) in elems.iter().enumerate() {
            let ok = self.call_pred(pred, &[e.clone()], span)?;
            if ok != negate {
                return Ok(Value::Int(i as i64));
            }
        }
        Ok(Value::Int(elems.len() as i64))
    }

    pub(super) fn builtin_count(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: Option<&Value>,
        pred: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        let mut n = 0i64;
        for e in &elems {
            let hit = if let Some(p) = pred {
                self.call_pred(p, &[e.clone()], span)?
            } else if let Some(t) = target {
                Self::values_eq(e, t)
            } else {
                false
            };
            if hit {
                n += 1;
            }
        }
        Ok(Value::Int(n))
    }

    pub(super) fn builtin_equal_ranges(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        pred: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.sequence_id_from_begin(b2, span)?;
        let a = self.sequence_values(id1, span)?;
        let b = self.sequence_values(id2, span)?;
        if a.len() > b.len() {
            return Ok(Value::Bool(false));
        }
        for i in 0..a.len() {
            let ok = if let Some(p) = pred {
                self.call_pred(p, &[a[i].clone(), b[i].clone()], span)?
            } else {
                Self::values_eq(&a[i], &b[i])
            };
            if !ok {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn sequence_id_from_begin(&mut self, begin: &Expr, span: Span) -> Result<ObjId> {
        // Reuse resolve with begin,begin and temporarily allow — instead eval member base
        let Expr::Call { callee, args, .. } = begin else {
            return Err(RuntimeError::at(span, "expected begin iterator"));
        };
        let base = match callee.as_ref() {
            Expr::Member { base, .. } if args.is_empty() => base.as_ref(),
            Expr::Name(_) if args.len() == 1 => &args[0],
            _ => return Err(RuntimeError::at(span, "expected begin iterator")),
        };
        match self.eval_expr(base)? {
            Value::Object(id) => Ok(id),
            _ => Err(RuntimeError::at(span, "begin base not a container")),
        }
    }

    pub(super) fn builtin_search(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        e2: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.resolve_vector_range(b2, e2, span)?;
        let hay = self.sequence_values(id1, span)?;
        let needle = self.sequence_values(id2, span)?;
        if needle.is_empty() {
            return Ok(Value::Int(0));
        }
        'outer: for i in 0..=hay.len().saturating_sub(needle.len()) {
            for j in 0..needle.len() {
                if !Self::values_eq(&hay[i + j], &needle[j]) {
                    continue 'outer;
                }
            }
            return Ok(Value::Int(i as i64));
        }
        Ok(Value::Int(hay.len() as i64))
    }

    pub(super) fn builtin_search_n(
        &mut self,
        begin: &Expr,
        end: &Expr,
        count: &Value,
        value: &Value,
        span: Span,
    ) -> Result<Value> {
        let n = count.as_int().map_err(RuntimeError::new)? as usize;
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        if n == 0 {
            return Ok(Value::Int(0));
        }
        let mut run = 0usize;
        for (i, e) in elems.iter().enumerate() {
            if Self::values_eq(e, value) {
                run += 1;
                if run >= n {
                    return Ok(Value::Int((i + 1 - n) as i64));
                }
            } else {
                run = 0;
            }
        }
        Ok(Value::Int(elems.len() as i64))
    }

    pub(super) fn builtin_adjacent_find(
        &mut self,
        begin: &Expr,
        end: &Expr,
        pred: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        for i in 0..elems.len().saturating_sub(1) {
            let hit = if let Some(p) = pred {
                self.call_pred(p, &[elems[i].clone(), elems[i + 1].clone()], span)?
            } else {
                Self::values_eq(&elems[i], &elems[i + 1])
            };
            if hit {
                return Ok(Value::Int(i as i64));
            }
        }
        Ok(Value::Int(elems.len() as i64))
    }

    pub(super) fn builtin_fill(
        &mut self,
        begin: &Expr,
        end: &Expr,
        value: &Value,
        span: Span,
    ) -> Result<Value> {
        // fill(begin, end, v) — fill whole resolved container (A: full range only)
        let id = self.resolve_vector_range(begin, end, span)?;
        let mut elems = self.sequence_values(id, span)?;
        for e in &mut elems {
            *e = value.clone();
        }
        self.write_sequence_values(id, elems, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_fill_n(
        &mut self,
        begin: &Expr,
        count: &Value,
        value: &Value,
        span: Span,
    ) -> Result<Value> {
        let id = self.sequence_id_from_begin(begin, span)?;
        let n = count.as_int().map_err(RuntimeError::new)? as usize;
        let mut elems = self.sequence_values(id, span)?;
        let limit = n.min(elems.len());
        for e in elems.iter_mut().take(limit) {
            *e = value.clone();
        }
        self.write_sequence_values(id, elems, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_copy(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        pred: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.sequence_id_from_begin(b2, span)?;
        let src = self.sequence_values(id1, span)?;
        let mut dst = self.sequence_values(id2, span)?;
        let mut j = 0usize;
        for e in src {
            let take = if let Some(p) = pred {
                self.call_pred(p, &[e.clone()], span)?
            } else {
                true
            };
            if take {
                if j < dst.len() {
                    dst[j] = e;
                } else {
                    dst.push(e);
                }
                j += 1;
            }
        }
        self.write_sequence_values(id2, dst, span)?;
        Ok(Value::Int(j as i64))
    }

    pub(super) fn builtin_transform(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        op: &Value,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.sequence_id_from_begin(b2, span)?;
        let src = self.sequence_values(id1, span)?;
        let mut dst = self.sequence_values(id2, span)?;
        for (i, e) in src.iter().enumerate() {
            let Value::Object(opid) = op else {
                return Err(RuntimeError::at(span, "transform needs callable"));
            };
            let out = match self.heap.get(*opid).cloned() {
                Some(Object::Closure { .. }) => {
                    self.call_closure(*opid, &[e.clone()], span)?
                }
                Some(Object::Functor { kind }) => {
                    super::functor::functor_apply(kind, &[e.clone()], span)?
                }
                _ => return Err(RuntimeError::at(span, "transform needs callable")),
            };
            if i < dst.len() {
                dst[i] = out;
            } else {
                dst.push(out);
            }
        }
        self.write_sequence_values(id2, dst, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_replace(
        &mut self,
        begin: &Expr,
        end: &Expr,
        old: Option<&Value>,
        pred: Option<&Value>,
        newv: &Value,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let mut elems = self.sequence_values(id, span)?;
        for e in &mut elems {
            let hit = if let Some(p) = pred {
                self.call_pred(p, &[e.clone()], span)?
            } else if let Some(o) = old {
                Self::values_eq(e, o)
            } else {
                false
            };
            if hit {
                *e = newv.clone();
            }
        }
        self.write_sequence_values(id, elems, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_remove(
        &mut self,
        begin: &Expr,
        end: &Expr,
        value: Option<&Value>,
        pred: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        let mut kept = Vec::new();
        for e in elems {
            let drop = if let Some(p) = pred {
                self.call_pred(p, &[e.clone()], span)?
            } else if let Some(v) = value {
                Self::values_eq(&e, v)
            } else {
                false
            };
            if !drop {
                kept.push(e);
            }
        }
        let new_end = kept.len() as i64;
        self.write_sequence_values(id, kept, span)?;
        Ok(Value::Int(new_end))
    }

    pub(super) fn builtin_unique(
        &mut self,
        begin: &Expr,
        end: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let elems = self.sequence_values(id, span)?;
        let mut out = Vec::new();
        for e in elems {
            if out.last().is_none_or(|p| !Self::values_eq(p, &e)) {
                out.push(e);
            }
        }
        let n = out.len() as i64;
        self.write_sequence_values(id, out, span)?;
        Ok(Value::Int(n))
    }

    pub(super) fn builtin_rotate(
        &mut self,
        begin: &Expr,
        middle: &Expr,
        end: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id = self.resolve_vector_range(begin, end, span)?;
        let mid = match middle {
            Expr::Binary { .. } | Expr::Call { .. } => self.eval_expr(middle)?,
            _ => self.eval_expr(middle)?,
        };
        let m = mid.as_int().map_err(RuntimeError::new)? as usize;
        let mut elems = self.sequence_values(id, span)?;
        if m < elems.len() {
            elems.rotate_left(m);
        }
        self.write_sequence_values(id, elems, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_generate(
        &mut self,
        begin: &Expr,
        end: &Expr,
        gen: &Value,
        n: Option<usize>,
        span: Span,
    ) -> Result<Value> {
        let id = if n.is_some() {
            self.sequence_id_from_begin(begin, span)?
        } else {
            self.resolve_vector_range(begin, end, span)?
        };
        let mut elems = self.sequence_values(id, span)?;
        let limit = n.unwrap_or(elems.len()).min(elems.len());
        let Value::Object(opid) = gen else {
            return Err(RuntimeError::at(span, "generate needs callable"));
        };
        for e in elems.iter_mut().take(limit) {
            *e = match self.heap.get(*opid).cloned() {
                Some(Object::Closure { .. }) => self.call_closure(*opid, &[], span)?,
                Some(Object::Functor { kind }) => {
                    super::functor::functor_apply(kind, &[], span)?
                }
                _ => return Err(RuntimeError::at(span, "generate needs callable")),
            };
        }
        self.write_sequence_values(id, elems, span)?;
        Ok(Value::Void)
    }

    pub(super) fn builtin_equal_range_algo(
        &mut self,
        begin: &Expr,
        end: &Expr,
        target: &Value,
        cmp: Option<&Value>,
        span: Span,
    ) -> Result<Value> {
        let lo = self.builtin_bound(begin, end, target, false, cmp, span)?;
        let hi = self.builtin_bound(begin, end, target, true, cmp, span)?;
        let pid = self.heap.alloc(Object::Pair {
            first: lo,
            second: hi,
        });
        Ok(Value::Object(pid))
    }

    pub(super) fn builtin_set_op(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        e2: &Expr,
        out_begin: &Expr,
        kind: &str,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.resolve_vector_range(b2, e2, span)?;
        let out_id = self.sequence_id_from_begin(out_begin, span)?;
        let a: Vec<i64> = self
            .sequence_values(id1, span)?
            .into_iter()
            .map(|v| v.as_int().unwrap_or(0))
            .collect();
        let b: Vec<i64> = self
            .sequence_values(id2, span)?
            .into_iter()
            .map(|v| v.as_int().unwrap_or(0))
            .collect();
        let mut out = Vec::new();
        match kind {
            "merge" => {
                let mut i = 0;
                let mut j = 0;
                while i < a.len() && j < b.len() {
                    if a[i] <= b[j] {
                        out.push(a[i]);
                        i += 1;
                    } else {
                        out.push(b[j]);
                        j += 1;
                    }
                }
                out.extend_from_slice(&a[i..]);
                out.extend_from_slice(&b[j..]);
            }
            "set_union" => {
                let mut i = 0;
                let mut j = 0;
                while i < a.len() && j < b.len() {
                    if a[i] < b[j] {
                        out.push(a[i]);
                        i += 1;
                    } else if b[j] < a[i] {
                        out.push(b[j]);
                        j += 1;
                    } else {
                        out.push(a[i]);
                        i += 1;
                        j += 1;
                    }
                }
                out.extend_from_slice(&a[i..]);
                out.extend_from_slice(&b[j..]);
            }
            "set_intersection" => {
                let mut i = 0;
                let mut j = 0;
                while i < a.len() && j < b.len() {
                    if a[i] < b[j] {
                        i += 1;
                    } else if b[j] < a[i] {
                        j += 1;
                    } else {
                        out.push(a[i]);
                        i += 1;
                        j += 1;
                    }
                }
            }
            "set_difference" => {
                let mut i = 0;
                let mut j = 0;
                while i < a.len() && j < b.len() {
                    if a[i] < b[j] {
                        out.push(a[i]);
                        i += 1;
                    } else if b[j] < a[i] {
                        j += 1;
                    } else {
                        i += 1;
                        j += 1;
                    }
                }
                out.extend_from_slice(&a[i..]);
            }
            "set_symmetric_difference" => {
                let mut i = 0;
                let mut j = 0;
                while i < a.len() && j < b.len() {
                    if a[i] < b[j] {
                        out.push(a[i]);
                        i += 1;
                    } else if b[j] < a[i] {
                        out.push(b[j]);
                        j += 1;
                    } else {
                        i += 1;
                        j += 1;
                    }
                }
                out.extend_from_slice(&a[i..]);
                out.extend_from_slice(&b[j..]);
            }
            _ => return Err(RuntimeError::at(span, "unknown set op")),
        }
        let vals: Vec<_> = out.into_iter().map(Value::Int).collect();
        let n = vals.len() as i64;
        self.write_sequence_values(out_id, vals, span)?;
        Ok(Value::Int(n))
    }

    pub(super) fn builtin_includes(
        &mut self,
        b1: &Expr,
        e1: &Expr,
        b2: &Expr,
        e2: &Expr,
        span: Span,
    ) -> Result<Value> {
        let id1 = self.resolve_vector_range(b1, e1, span)?;
        let id2 = self.resolve_vector_range(b2, e2, span)?;
        let a: Vec<i64> = self
            .sequence_values(id1, span)?
            .into_iter()
            .map(|v| v.as_int().unwrap_or(0))
            .collect();
        let b: Vec<i64> = self
            .sequence_values(id2, span)?
            .into_iter()
            .map(|v| v.as_int().unwrap_or(0))
            .collect();
        let mut i = 0;
        let mut j = 0;
        while i < a.len() && j < b.len() {
            if a[i] < b[j] {
                i += 1;
            } else if a[i] == b[j] {
                i += 1;
                j += 1;
            } else {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(j == b.len()))
    }
}

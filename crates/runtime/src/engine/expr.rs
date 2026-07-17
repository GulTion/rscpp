use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn eval_expr(&mut self, expr: &Expr) -> Result<Value> {
        let (v, _) = self.eval_expr_lv(expr)?;
        Ok(v)
    }

    pub(super) fn eval_expr_lv(&mut self, expr: &Expr) -> Result<(Value, Option<LValue>)> {
        self.eval_expr_lv_opts(expr, true)
    }

    pub(super) fn eval_expr_lv_opts(
        &mut self,
        expr: &Expr,
        emit_index_lookup: bool,
    ) -> Result<(Value, Option<LValue>)> {
        match expr {
            Expr::IntLit { value, .. } => Ok((Value::Int(*value as i64), None)),
            Expr::FloatLit { value, .. } => Ok((Value::Float(*value), None)),
            Expr::CharLit { value, .. } => Ok((Value::Char(*value), None)),
            Expr::StringLit { value, span } => {
                let id = self.heap.alloc(Object::String(value.clone()));
                self.emit_alloc(id, "string", *span);
                Ok((Value::Object(id), None))
            }
            Expr::BoolLit { value, .. } => Ok((Value::Bool(*value), None)),
            Expr::Nullptr { .. } => Ok((Value::Nullptr, None)),
            Expr::Name(path) => self.eval_name_expr(path),
            Expr::Unary { op, expr, span } => {
                let (v, lv) = self.eval_expr_lv(expr)?;
                let out = match op {
                    UnaryOp::Plus => Value::Int(v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::Minus => Value::Int(-v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::Not => Value::Bool(!v.as_bool().map_err(RuntimeError::new)?),
                    UnaryOp::BitNot => Value::Int(!v.as_int().map_err(RuntimeError::new)?),
                    UnaryOp::PreInc => {
                        let n = v.as_int().map_err(RuntimeError::new)? + 1;
                        let nv = Value::Int(n);
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, nv.clone(), *span)?;
                        }
                        nv
                    }
                    UnaryOp::PreDec => {
                        let n = v.as_int().map_err(RuntimeError::new)? - 1;
                        let nv = Value::Int(n);
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, nv.clone(), *span)?;
                        }
                        nv
                    }
                    UnaryOp::PostInc => {
                        let old = v.clone();
                        let n = v.as_int().map_err(RuntimeError::new)? + 1;
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, Value::Int(n), *span)?;
                        }
                        old
                    }
                    UnaryOp::PostDec => {
                        let old = v.clone();
                        let n = v.as_int().map_err(RuntimeError::new)? - 1;
                        if let Some(lv) = &lv {
                            self.write_lvalue(lv, Value::Int(n), *span)?;
                        }
                        old
                    }
                    UnaryOp::AddrOf => {
                        let Some(lv) = lv else {
                            return Err(RuntimeError::at(*span, "cannot take address of rvalue"));
                        };
                        let addr = self.lvalue_to_address(&lv)?;
                        Value::Ptr(addr)
                    }
                    UnaryOp::Deref => match &v {
                        Value::Ptr(addr) | Value::Ref(addr) => {
                            let loaded = self.load_address(addr)?;
                            let out_lv = self.address_to_lvalue(addr);
                            return Ok((loaded, out_lv));
                        }
                        Value::Nullptr => {
                            return Err(RuntimeError::at(*span, "null pointer dereference"));
                        }
                        other => {
                            return Err(RuntimeError::at(
                                *span,
                                format!("cannot dereference `{other}`"),
                            ));
                        }
                    },
                };
                Ok((out, None))
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                // C++ short-circuit for `&&` / `||` (common in LeetCode guards like `i > 0 && a[i]`).
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let l = self.eval_expr(left)?;
                    let lb = l.as_bool().map_err(RuntimeError::new)?;
                    let out = match op {
                        BinaryOp::And if !lb => Value::Bool(false),
                        BinaryOp::Or if lb => Value::Bool(true),
                        _ => {
                            let r = self.eval_expr(right)?;
                            self.eval_binary(*op, &l, &r, *span)?
                        }
                    };
                    return Ok((out, None));
                }
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                let out = self.eval_binary(*op, &l, &r, *span)?;
                Ok((out, None))
            }
            Expr::Assign {
                op,
                left,
                right,
                span,
            } => {
                // `tie(a, b) = pair(...)` / `tie(a,b,c) = make_tuple(...)`
                if *op == AssignOp::Assign {
                    if let Expr::Call {
                        callee,
                        args: targs,
                        ..
                    } = left.as_ref()
                    {
                        if let Expr::Name(path) = callee.as_ref() {
                            let tname = path
                                .segments
                                .iter()
                                .map(|s| s.name.as_str())
                                .collect::<Vec<_>>()
                                .join("::");
                            if tname == "tie" || tname == "std::tie" {
                                let rv = self.eval_expr(right)?;
                                let vals = self.unpack_tie_source(&rv, targs.len(), *span)?;
                                for (dst, val) in targs.iter().zip(vals.into_iter()) {
                                    let (_, lv) = self.eval_expr_lv(dst)?;
                                    let lv = lv.ok_or_else(|| {
                                        RuntimeError::at(*span, "tie target is not an lvalue")
                                    })?;
                                    self.write_lvalue(&lv, val, *span)?;
                                }
                                return Ok((rv, None));
                            }
                        }
                    }
                }
                let (rv, _) = self.eval_expr_lv(right)?;
                // Pure `=` : resolve LHS without ContainerLookup (Write covers the store).
                // Compound assigns still lookup so the UI sees the old value read.
                let lookup = *op != AssignOp::Assign;
                let (lv_val, lv) = self.eval_expr_lv_opts(left, lookup)?;
                let Some(lv) = lv else {
                    return Err(RuntimeError::at(*span, "invalid assignment target"));
                };
                let new_val = if *op == AssignOp::Assign {
                    rv
                } else {
                    let bin = match op {
                        AssignOp::AddAssign => BinaryOp::Add,
                        AssignOp::SubAssign => BinaryOp::Sub,
                        AssignOp::MulAssign => BinaryOp::Mul,
                        AssignOp::DivAssign => BinaryOp::Div,
                        AssignOp::RemAssign => BinaryOp::Rem,
                        AssignOp::AndAssign => BinaryOp::BitAnd,
                        AssignOp::OrAssign => BinaryOp::BitOr,
                        AssignOp::XorAssign => BinaryOp::BitXor,
                        AssignOp::ShlAssign => BinaryOp::Shl,
                        AssignOp::ShrAssign => BinaryOp::Shr,
                        AssignOp::Assign => unreachable!(),
                    };
                    self.eval_binary(bin, &lv_val, &rv, *span)?
                };
                self.write_lvalue(&lv, new_val.clone(), *span)?;
                Ok((new_val, Some(lv)))
            }
            Expr::Call { callee, args, span } => {
                self.eval_call_expr(callee, args, *span, emit_index_lookup)
            }
            Expr::Index { base, index, span } => {
                let b = self.eval_expr(base)?;
                let idx_val = self.eval_expr(index)?;
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(*span, "cannot index non-object"));
                };
                match self.heap.get(id).cloned() {
                    Some(Object::Vector(elems)) => {
                        let i = idx_val.as_int().map_err(RuntimeError::new)? as usize;
                        let v = elems
                            .get(i)
                            .cloned()
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        let v = if emit_index_lookup {
                            self.query(Value::Object(id), "index", Some(idx_val), v, *span)
                        } else {
                            v
                        };
                        Ok((v, Some(LValue::Index { obj: id, index: i })))
                    }
                    Some(Object::String(s)) => {
                        let i = idx_val.as_int().map_err(RuntimeError::new)? as usize;
                        let ch = s
                            .chars()
                            .nth(i)
                            .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?;
                        let v = if emit_index_lookup {
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                Value::Char(ch),
                                *span,
                            )
                        } else {
                            Value::Char(ch)
                        };
                        Ok((v, None))
                    }
                    Some(Object::Map(_)) | Some(Object::UnorderedMap(_)) => {
                        let key = self.value_to_key(&idx_val)?;
                        let v = self.map_index_get_or_insert(id, &key, *span)?;
                        let v = if emit_index_lookup {
                            self.query(Value::Object(id), "index", Some(idx_val), v, *span)
                        } else {
                            v
                        };
                        Ok((v, Some(LValue::MapEntry { obj: id, key })))
                    }
                    _ => Err(RuntimeError::at(*span, "type not subscriptable")),
                }
            }
            Expr::Member {
                base,
                field,
                arrow,
                span,
            } => {
                let b = self.eval_expr(base)?;
                let id = if *arrow {
                    match b {
                        Value::Ptr(Address::Heap(id)) => id,
                        Value::Object(id) => id,
                        Value::Nullptr => {
                            return Err(RuntimeError::at(*span, "null pointer dereference"));
                        }
                        other => {
                            return Err(RuntimeError::at(
                                *span,
                                format!("`->` on non-pointer `{other}`"),
                            ));
                        }
                    }
                } else {
                    let Value::Object(id) = b else {
                        return Err(RuntimeError::at(*span, "member access on non-object"));
                    };
                    id
                };
                match self.heap.get(id) {
                    Some(Object::Pair { first, second }) => match field.name.as_str() {
                        "first" => Ok((
                            first.clone(),
                            Some(LValue::Field {
                                obj: id,
                                field: "first".into(),
                            }),
                        )),
                        "second" => Ok((
                            second.clone(),
                            Some(LValue::Field {
                                obj: id,
                                field: "second".into(),
                            }),
                        )),
                        _ => Err(RuntimeError::at(*span, "pair has first/second only")),
                    },
                    Some(Object::Class { fields, .. }) => {
                        let v = fields.get(&field.name).cloned().unwrap_or(Value::Nullptr);
                        Ok((
                            v,
                            Some(LValue::Field {
                                obj: id,
                                field: field.name.clone(),
                            }),
                        ))
                    }
                    // member methods without call are not values
                    _ => Err(RuntimeError::at(
                        *span,
                        format!("cannot read field `{}`", field.name),
                    )),
                }
            }
            Expr::Cast { expr, .. } => {
                let v = self.eval_expr(expr)?;
                Ok((v, None))
            }
            Expr::InitList { elems, span } => {
                let mut vs = Vec::new();
                for e in elems {
                    vs.push(self.eval_expr(e)?);
                }
                let id = self.heap.alloc(Object::Vector(vs));
                self.emit_alloc(id, "vector", *span);
                Ok((Value::Object(id), None))
            }
            Expr::Conditional {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let c = self.eval_expr(cond)?;
                if c.as_bool().map_err(RuntimeError::new)? {
                    let v = self.eval_expr(then_branch)?;
                    Ok((v, None))
                } else {
                    let v = self.eval_expr(else_branch)?;
                    Ok((v, None))
                }
            }
            Expr::Lambda { params, body, span } => {
                let param_names: Vec<String> = params
                    .iter()
                    .filter_map(|p| p.name.as_ref().map(|n| n.name.clone()))
                    .collect();
                let captures = self.capture_locals();
                let id = self.heap.alloc(Object::Closure {
                    params: param_names,
                    body: body.clone(),
                    captures,
                });
                self.emit_alloc(id, "closure", *span);
                Ok((Value::Object(id), None))
            }
            Expr::New { ty, args, span } => self.eval_new_expr(ty, args, *span),
            Expr::Sizeof { .. } => Ok((Value::Int(8), None)),
            Expr::Delete { expr, .. } => {
                let _ = self.eval_expr(expr)?;
                Ok((Value::Void, None))
            }
        }
    }

    pub(super) fn eval_call_expr(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
        _emit_index_lookup: bool,
    ) -> Result<(Value, Option<LValue>)> {
        // Algorithm / swap builtins re-walk arg exprs — must not pre-evaluate (side effects).
        if let Expr::Name(path) = callee {
            let name = path
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            // Local / captured lambda call: `fn(args)` where `fn` is a closure object.
            if path.segments.len() == 1 {
                if let Ok(raw) = self.lookup_raw(&name) {
                    let v = match raw {
                        Value::Ref(addr) => self.load_address(&addr)?,
                        other => other,
                    };
                    if let Value::Object(id) = v {
                        if matches!(self.heap.get(id), Some(Object::Closure { .. })) {
                            let arg_vals: Result<Vec<_>> =
                                args.iter().map(|a| self.eval_expr(a)).collect();
                            let arg_vals = arg_vals?;
                            let ret = self.call_closure(id, &arg_vals, span)?;
                            return Ok((ret, None));
                        }
                    }
                }
            }
            if (name == "swap" || name == "std::swap") && args.len() == 2 {
                let ret = self.builtin_swap(&args[0], &args[1], span)?;
                return Ok((ret, None));
            }
            if (name == "sort" || name == "std::sort") && (args.len() == 2 || args.len() == 3) {
                let cmp = if args.len() == 3 {
                    Some(self.eval_expr(&args[2])?)
                } else {
                    None
                };
                let ret = self.builtin_sort(&args[0], &args[1], cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "reverse" || name == "std::reverse") && args.len() == 2 {
                let ret = self.builtin_reverse(&args[0], &args[1], span)?;
                return Ok((ret, None));
            }
            if (name == "binary_search" || name == "std::binary_search")
                && (args.len() == 3 || args.len() == 4)
            {
                let target = self.eval_expr(&args[2])?;
                let cmp = if args.len() == 4 {
                    Some(self.eval_expr(&args[3])?)
                } else {
                    None
                };
                let ret =
                    self.builtin_binary_search(&args[0], &args[1], &target, cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "lower_bound" || name == "std::lower_bound")
                && (args.len() == 3 || args.len() == 4)
            {
                let target = self.eval_expr(&args[2])?;
                let cmp = if args.len() == 4 {
                    Some(self.eval_expr(&args[3])?)
                } else {
                    None
                };
                let ret =
                    self.builtin_bound(&args[0], &args[1], &target, false, cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "upper_bound" || name == "std::upper_bound")
                && (args.len() == 3 || args.len() == 4)
            {
                let target = self.eval_expr(&args[2])?;
                let cmp = if args.len() == 4 {
                    Some(self.eval_expr(&args[3])?)
                } else {
                    None
                };
                let ret =
                    self.builtin_bound(&args[0], &args[1], &target, true, cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "accumulate" || name == "std::accumulate")
                && (args.len() == 2 || args.len() == 3 || args.len() == 4)
            {
                let init = if args.len() >= 3 {
                    Some(self.eval_expr(&args[2])?)
                } else {
                    None
                };
                let op = if args.len() == 4 {
                    Some(self.eval_expr(&args[3])?)
                } else {
                    None
                };
                let ret =
                    self.builtin_accumulate(&args[0], &args[1], init.as_ref(), op.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "iota" || name == "std::iota") && args.len() == 3 {
                let start = self.eval_expr(&args[2])?;
                let ret = self.builtin_iota(&args[0], &args[1], &start, span)?;
                return Ok((ret, None));
            }
            if (name == "move" || name == "std::move") && args.len() == 1 {
                let v = self.eval_expr(&args[0])?;
                return Ok((v, None));
            }
            if (name == "partial_sum" || name == "std::partial_sum") && args.len() == 3 {
                let ret = self.builtin_partial_sum(&args[0], &args[1], &args[2], span)?;
                return Ok((ret, None));
            }
            if (name == "min_element" || name == "std::min_element")
                && (args.len() == 2 || args.len() == 3)
            {
                let cmp = if args.len() == 3 {
                    Some(self.eval_expr(&args[2])?)
                } else {
                    None
                };
                let ret =
                    self.builtin_element_ptr(&args[0], &args[1], false, cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
            if (name == "max_element" || name == "std::max_element")
                && (args.len() == 2 || args.len() == 3)
            {
                let cmp = if args.len() == 3 {
                    Some(self.eval_expr(&args[2])?)
                } else {
                    None
                };
                let ret = self.builtin_element_ptr(&args[0], &args[1], true, cmp.as_ref(), span)?;
                return Ok((ret, None));
            }
        }

        let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
        let arg_vals = arg_vals?;

        // member call: a.b(...) already parsed as Call { Member }
        if let Expr::Member {
            base,
            field,
            arrow,
            span: mspan,
        } = callee
        {
            let base_v = self.eval_expr(base)?;
            let base_v = if *arrow {
                match base_v {
                    Value::Ptr(Address::Heap(id)) => Value::Object(id),
                    Value::Object(id) => Value::Object(id),
                    Value::Nullptr => {
                        return Err(RuntimeError::at(*mspan, "null pointer dereference"));
                    }
                    other => {
                        return Err(RuntimeError::at(
                            *mspan,
                            format!("`->` on non-pointer `{other}`"),
                        ));
                    }
                }
            } else {
                base_v
            };
            let ret = self.call_member(base_v, &field.name, &arg_vals, span)?;
            return Ok((ret, None));
        }

        if let Expr::Name(path) = callee {
            let name = path
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::");
            if builtins::is_builtin_call(&name) {
                let out = builtins::call_builtin(&name, &arg_vals, span)?;
                if let Some(chosen) = out.chosen {
                    let chosen_span = args.get(chosen).map(|a| a.span()).unwrap_or(span);
                    self.emit(Event::BuiltinSelect {
                        name: name.clone(),
                        args: arg_vals.clone(),
                        chosen,
                        value: out.value.clone(),
                        span: chosen_span,
                    });
                }
                return Ok((out.value, None));
            }
            // `numeric_limits<T>::min()` / `max()` / `lowest()` (template args erased in parse)
            if matches!(
                name.as_str(),
                "numeric_limits::min"
                    | "numeric_limits::max"
                    | "numeric_limits::lowest"
                    | "std::numeric_limits::min"
                    | "std::numeric_limits::max"
                    | "std::numeric_limits::lowest"
            ) {
                let v = match name.rsplit("::").next() {
                    Some("max") => Value::Int(i32::MAX as i64),
                    Some("min") | Some("lowest") => Value::Int(i32::MIN as i64),
                    _ => Value::Int(0),
                };
                return Ok((v, None));
            }
            // C++17 free `size(c)` / `empty(c)`
            if (name == "size" || name == "std::size") && arg_vals.len() == 1 {
                let v = self.call_member(arg_vals[0].clone(), "size", &[], span)?;
                return Ok((v, None));
            }
            if (name == "empty" || name == "std::empty") && arg_vals.len() == 1 {
                let v = self.call_member(arg_vals[0].clone(), "empty", &[], span)?;
                return Ok((v, None));
            }
            // Free `begin(c)` / `end(c)` / `cbegin` / `cend` — iterator stubs (algos match patterns)
            if matches!(
                name.as_str(),
                "begin"
                    | "end"
                    | "cbegin"
                    | "cend"
                    | "std::begin"
                    | "std::end"
                    | "std::cbegin"
                    | "std::cend"
            ) && arg_vals.len() == 1
            {
                return Ok((Value::Int(0), None));
            }
            if (name == "to_string" || name == "std::to_string") && arg_vals.len() == 1 {
                let s = match &arg_vals[0] {
                    Value::Int(n) => n.to_string(),
                    Value::Float(f) => f.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Char(c) => c.to_string(),
                    other => {
                        return Err(RuntimeError::at(
                            span,
                            format!("to_string unsupported for `{other}`"),
                        ))
                    }
                };
                let id = self.heap.alloc(Object::String(s));
                self.emit_alloc(id, "string", span);
                return Ok((Value::Object(id), None));
            }
            // `string("hi")` / `string(s)`
            if name == "string" {
                let s = match arg_vals.first() {
                    None => String::new(),
                    Some(Value::Object(id)) => match self.heap.get(*id) {
                        Some(Object::String(s)) => s.clone(),
                        _ => return Err(RuntimeError::at(span, "string() expected string")),
                    },
                    Some(Value::Str(s)) => s.clone(),
                    Some(Value::Char(c)) => c.to_string(),
                    Some(Value::Int(n)) => n.to_string(),
                    Some(other) => {
                        return Err(RuntimeError::at(
                            span,
                            format!("string() unsupported for `{other}`"),
                        ))
                    }
                };
                let id = self.heap.alloc(Object::String(s));
                self.emit_alloc(id, "string", span);
                return Ok((Value::Object(id), None));
            }
            // `vector<T>(n)` / `vector<T>(n, fill)` / `vector<T>(other)` (template args erased)
            if name == "vector" {
                let elems = match arg_vals.as_slice() {
                    [] => vec![],
                    [Value::Object(id)] => match self.heap.get(*id) {
                        Some(Object::Vector(e)) => e.clone(),
                        _ => return Err(RuntimeError::at(span, "vector() copy needs vector")),
                    },
                    [n] => {
                        let n = n.as_int().map_err(RuntimeError::new)? as usize;
                        vec![Value::Int(0); n]
                    }
                    [n, fill] => {
                        let n = n.as_int().map_err(RuntimeError::new)? as usize;
                        vec![fill.clone(); n]
                    }
                    _ => return Err(RuntimeError::at(span, "vector() expects 0..=2 args")),
                };
                let id = self.heap.alloc(Object::Vector(elems));
                self.emit_alloc(id, "vector", span);
                return Ok((Value::Object(id), None));
            }
            // Type-construction: pair / tuple as function name
            if (name == "pair"
                || name == "make_pair"
                || name == "std::pair"
                || name == "std::make_pair")
                && arg_vals.len() == 2
            {
                let id = self.heap.alloc(Object::Pair {
                    first: arg_vals[0].clone(),
                    second: arg_vals[1].clone(),
                });
                self.emit_alloc(id, "pair", span);
                return Ok((Value::Object(id), None));
            }
            if (name == "tuple"
                || name == "make_tuple"
                || name == "std::make_tuple"
                || name == "std::tuple")
                && !arg_vals.is_empty()
            {
                let id = self.heap.alloc(Object::Vector(arg_vals.clone()));
                self.emit_alloc(id, "tuple", span);
                return Ok((Value::Object(id), None));
            }
            // User class construction: `UnionFind(n)` / `Trie()`
            if self.classes.contains_key(&name) {
                let id = self.alloc_class_instance(&name, span)?;
                let ctor = format!("{name}::{name}");
                if self.functions.contains_key(&ctor) {
                    self.call_fn(&ctor, &arg_vals, Some(Value::Object(id)))?;
                }
                return Ok((Value::Object(id), None));
            }
            // Functional casts: `int64_t(x)` / `uint64_t(x)` (LeetCode-common)
            if matches!(
                name.as_str(),
                "int64_t" | "uint64_t" | "int32_t" | "uint32_t" | "size_t" | "int" | "long"
            ) && arg_vals.len() == 1
            {
                let n = arg_vals[0].as_int().map_err(RuntimeError::new)?;
                return Ok((Value::Int(n), None));
            }
            if (name == "stoi" || name == "std::stoi" || name == "stol" || name == "std::stol")
                && arg_vals.len() == 1
            {
                let s = match &arg_vals[0] {
                    Value::Object(id) => match self.heap.get(*id) {
                        Some(Object::String(s)) => s.clone(),
                        _ => return Err(RuntimeError::at(span, "stoi expects string")),
                    },
                    Value::Str(s) => s.clone(),
                    other => {
                        return Err(RuntimeError::at(
                            span,
                            format!("stoi expects string, got `{other}`"),
                        ))
                    }
                };
                let n: i64 = s
                    .parse()
                    .map_err(|_| RuntimeError::at(span, format!("stoi failed on `{s}`")))?;
                return Ok((Value::Int(n), None));
            }
            let (resolved, this) = self.resolve_fn_call(&name)?;
            let ret = self.call_fn(&resolved, &arg_vals, this)?;
            return Ok((ret, None));
        }

        Err(RuntimeError::at(span, "unsupported call"))
    }

    pub(super) fn eval_name_expr(&mut self, path: &Path) -> Result<(Value, Option<LValue>)> {
        if path.segments.len() == 1 {
            let n = &path.segments[0].name;
            if let Some(v) = builtins::const_value(n) {
                return Ok((v, None));
            }
            match self.lookup_raw(n) {
                Ok(raw) => match raw {
                    Value::Ref(addr) => {
                        let v = self.load_address(&addr)?;
                        Ok((v, self.address_to_lvalue(&addr)))
                    }
                    other => Ok((other, Some(LValue::Name(n.clone())))),
                },
                Err(_) => {
                    // Unqualified class member: `set_` → `this->set_`
                    if let Ok(this_v) = self.lookup_raw("this") {
                        if let Value::Object(id) = this_v {
                            if let Some(Object::Class { fields, .. }) = self.heap.get(id) {
                                if fields.contains_key(n) {
                                    let v = fields.get(n).cloned().unwrap_or(Value::Void);
                                    return Ok((
                                        v,
                                        Some(LValue::Field {
                                            obj: id,
                                            field: n.clone(),
                                        }),
                                    ));
                                }
                            }
                        }
                    }
                    Err(RuntimeError::new(format!("undefined variable `{n}`")))
                }
            }
        } else {
            Err(RuntimeError::at(
                path.span,
                "qualified names in expressions not supported yet",
            ))
        }
    }

    pub(super) fn eval_new_expr(
        &mut self,
        ty: &Type,
        args: &[Expr],
        span: Span,
    ) -> Result<(Value, Option<LValue>)> {
        let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
        let arg_vals = arg_vals?;
        let name = match ty {
            Type::Named { path, .. } => path
                .segments
                .last()
                .map(|s| s.name.as_str())
                .unwrap_or("object"),
            _ => "object",
        };
        let mut fields = HashMap::new();
        match name {
            "TreeNode" => {
                fields.insert(
                    "val".into(),
                    arg_vals.first().cloned().unwrap_or(Value::Int(0)),
                );
                fields.insert("left".into(), Value::Nullptr);
                fields.insert("right".into(), Value::Nullptr);
            }
            "ListNode" => {
                fields.insert(
                    "val".into(),
                    arg_vals.first().cloned().unwrap_or(Value::Int(0)),
                );
                fields.insert("next".into(), Value::Nullptr);
            }
            _ => {
                if let Some(v) = arg_vals.first() {
                    fields.insert("val".into(), v.clone());
                }
            }
        }
        let id = self.heap.alloc(Object::Class {
            name: name.into(),
            fields,
        });
        self.emit_alloc(id, name, span);
        Ok((Value::Ptr(Address::Heap(id)), None))
    }

    pub(super) fn value_to_key(&self, v: &Value) -> Result<MapKey> {
        MapKey::from_value(v, |id| self.heap.string_value(id)).map_err(RuntimeError::new)
    }

    pub(super) fn call_member(
        &mut self,
        base: Value,
        method: &str,
        args: &[Value],
        span: Span,
    ) -> Result<Value> {
        let Value::Object(id) = base.clone() else {
            return Err(RuntimeError::at(span, "method call on non-object"));
        };
        let kind = self.heap.get(id).map(|o| o.kind_name()).unwrap_or("?");

        if kind == "class" {
            let name = match self.heap.get(id) {
                Some(Object::Class { name, .. }) => name.clone(),
                _ => return Err(RuntimeError::at(span, "dangling object")),
            };
            let q = format!("{name}::{method}");
            return self.call_fn(&q, args, Some(base));
        }

        if self.heap.get(id).is_none() {
            return Err(RuntimeError::at(span, "dangling object"));
        }

        let mut ctx = stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
        };
        stl::call_method(&mut ctx, id, base, kind, method, args, span)
    }

    /// Unpack `pair` / tuple-vector for `tie(...) = ...`.
    pub(super) fn unpack_tie_source(
        &self,
        src: &Value,
        n: usize,
        span: Span,
    ) -> Result<Vec<Value>> {
        let Value::Object(id) = src else {
            return Err(RuntimeError::at(span, "tie rhs must be pair/tuple"));
        };
        match self.heap.get(*id) {
            Some(Object::Pair { first, second }) if n == 2 => {
                Ok(vec![first.clone(), second.clone()])
            }
            Some(Object::Vector(elems)) if elems.len() >= n => {
                Ok(elems.iter().take(n).cloned().collect())
            }
            _ => Err(RuntimeError::at(
                span,
                format!("tie expects {n} values from pair/tuple"),
            )),
        }
    }
}

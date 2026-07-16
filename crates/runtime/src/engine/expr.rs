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
            Expr::Name(path) => {
                if path.segments.len() == 1 {
                    let n = &path.segments[0].name;
                    if let Some(v) = builtins::const_value(n) {
                        return Ok((v, None));
                    }
                    let raw = self.lookup_raw(n)?;
                    match raw {
                        Value::Ref(addr) => {
                            let v = self.load_address(&addr)?;
                            Ok((v, self.address_to_lvalue(&addr)))
                        }
                        other => Ok((other, Some(LValue::Name(n.clone())))),
                    }
                } else {
                    Err(RuntimeError::at(
                        path.span,
                        "qualified names in expressions not supported yet",
                    ))
                }
            }
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
                let arg_vals: Result<Vec<_>> = args.iter().map(|a| self.eval_expr(a)).collect();
                let arg_vals = arg_vals?;

                // member call: a.b(...) already parsed as Call { Member }
                if let Expr::Member {
                    base,
                    field,
                    arrow,
                    span: mspan,
                } = callee.as_ref()
                {
                    let base_v = self.eval_expr(base)?;
                    if *arrow {
                        // no real pointers yet
                        return Err(RuntimeError::at(*mspan, "`->` not supported yet"));
                    }
                    let ret = self.call_member(base_v, &field.name, &arg_vals, *span)?;
                    return Ok((ret, None));
                }

                if let Expr::Name(path) = callee.as_ref() {
                    let name = path
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join("::");
                    if (name == "swap" || name == "std::swap") && args.len() == 2 {
                        let ret = self.builtin_swap(&args[0], &args[1], *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "sort" || name == "std::sort") && args.len() == 2 {
                        let ret = self.builtin_sort(&args[0], &args[1], *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "reverse" || name == "std::reverse") && args.len() == 2 {
                        let ret = self.builtin_reverse(&args[0], &args[1], *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "binary_search" || name == "std::binary_search") && args.len() == 3 {
                        let ret = self.builtin_binary_search(&args[0], &args[1], &arg_vals[2], *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "lower_bound" || name == "std::lower_bound") && args.len() == 3 {
                        let ret = self.builtin_bound(&args[0], &args[1], &arg_vals[2], false, *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "upper_bound" || name == "std::upper_bound") && args.len() == 3 {
                        let ret = self.builtin_bound(&args[0], &args[1], &arg_vals[2], true, *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "accumulate" || name == "std::accumulate") && (args.len() == 2 || args.len() == 3) {
                        let init = if args.len() == 3 { Some(&arg_vals[2]) } else { None };
                        let ret = self.builtin_accumulate(&args[0], &args[1], init, *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "min_element" || name == "std::min_element") && args.len() == 2 {
                        let ret = self.builtin_element_ptr(&args[0], &args[1], false, *span)?;
                        return Ok((ret, None));
                    }
                    if (name == "max_element" || name == "std::max_element") && args.len() == 2 {
                        let ret = self.builtin_element_ptr(&args[0], &args[1], true, *span)?;
                        return Ok((ret, None));
                    }
                    if builtins::is_builtin_call(&name) {
                        let out = builtins::call_builtin(&name, &arg_vals, *span)?;
                        if let Some(chosen) = out.chosen {
                            let chosen_span = args
                                .get(chosen)
                                .map(|a| a.span())
                                .unwrap_or(*span);
                            self.emit(Event::BuiltinSelect {
                                call_id: self.current_call_id(),
                                name: name.clone(),
                                args: arg_vals.clone(),
                                chosen,
                                value: out.value.clone(),
                                span: chosen_span,
                            });
                        }
                        return Ok((out.value, None));
                    }
                    // C++17 free `size(c)` / `empty(c)`
                    if (name == "size" || name == "std::size") && arg_vals.len() == 1 {
                        let v = self.call_member(arg_vals[0].clone(), "size", &[], *span)?;
                        return Ok((v, None));
                    }
                    if (name == "empty" || name == "std::empty") && arg_vals.len() == 1 {
                        let v = self.call_member(arg_vals[0].clone(), "empty", &[], *span)?;
                        return Ok((v, None));
                    }
                    if (name == "to_string" || name == "std::to_string") && arg_vals.len() == 1 {
                        let s = match &arg_vals[0] {
                            Value::Int(n) => n.to_string(),
                            Value::Float(f) => f.to_string(),
                            Value::Bool(b) => b.to_string(),
                            Value::Char(c) => c.to_string(),
                            other => {
                                return Err(RuntimeError::at(
                                    *span,
                                    format!("to_string unsupported for `{other}`"),
                                ))
                            }
                        };
                        let id = self.heap.alloc(Object::String(s));
                        self.emit_alloc(id, "string", *span);
                        return Ok((Value::Object(id), None));
                    }
                    // Type-construction: vector / pair as function name
                    if name == "pair" && arg_vals.len() == 2 {
                        let id = self.heap.alloc(Object::Pair {
                            first: arg_vals[0].clone(),
                            second: arg_vals[1].clone(),
                        });
                        self.emit_alloc(id, "pair", *span);
                        return Ok((Value::Object(id), None));
                    }
                    let (resolved, this) = self.resolve_fn_call(&name)?;
                    let ret = self.call_fn(&resolved, &arg_vals, this)?;
                    return Ok((ret, None));
                }

                Err(RuntimeError::at(*span, "unsupported call"))
            }
            Expr::Index {
                base,
                index,
                span,
            } => {
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
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                v,
                                *span,
                            )
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
                            self.query(
                                Value::Object(id),
                                "index",
                                Some(idx_val),
                                v,
                                *span,
                            )
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
                if *arrow {
                    return Err(RuntimeError::at(*span, "`->` not supported yet"));
                }
                let b = self.eval_expr(base)?;
                let Value::Object(id) = b else {
                    return Err(RuntimeError::at(*span, "member access on non-object"));
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
                        let v = fields.get(&field.name).cloned().unwrap_or(Value::Int(0));
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
        }
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
        let kind = self
            .heap
            .get(id)
            .map(|o| o.kind_name())
            .unwrap_or("?");

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

        let call_id = self.current_call_id();
        let mut ctx = stl::Ctx {
            heap: &mut self.heap,
            events: &mut self.events,
            call_id,
        };
        stl::call_method(&mut ctx, id, base, kind, method, args, span)
}
}

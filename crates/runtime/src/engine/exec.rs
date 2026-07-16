use super::{Engine, Flow, Frame, LValue, Result};
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::builtins;
use crate::value::{Address, Heap, MapKey, Object, ObjId, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {

    pub(super) fn exec_block(&mut self, block: &Block) -> Result<Flow> {
        let before: HashSet<String> = self
            .stack
            .last()
            .map(|f| f.locals.keys().cloned().collect())
            .unwrap_or_default();
        self.emit(Event::ScopeEnter {
            call_id: self.current_call_id(),
            span: block.span,
        });
        for stmt in &block.stmts {
            match self.exec_stmt(stmt)? {
                Flow::Next => {}
                other => {
                    self.destroy_scope_locals(&before, block.span);
                    self.emit(Event::ScopeExit {
                        call_id: self.current_call_id(),
                        span: block.span,
                    });
                    return Ok(other);
                }
            }
        }
        self.destroy_scope_locals(&before, block.span);
        self.emit(Event::ScopeExit {
            call_id: self.current_call_id(),
            span: block.span,
        });
        Ok(Flow::Next)
    }

    pub(super) fn destroy_scope_locals(&mut self, before: &HashSet<String>, span: Span) {
        let call_id = self.current_call_id();
        let mut destroyed: Vec<(String, Value)> = vec![];
        if let Some(frame) = self.stack.last_mut() {
            let names: Vec<String> = frame
                .locals
                .keys()
                .filter(|n| !before.contains(*n))
                .cloned()
                .collect();
            for name in names {
                if let Some(v) = frame.locals.remove(&name) {
                    destroyed.push((name, v));
                }
            }
        }
        for (name, value) in destroyed {
            if let Value::Object(id) = value.clone() {
                if !self.value_mentions_obj(id) && self.heap.free(id).is_some() {
                    self.emit(Event::Dealloc {
                        call_id,
                        id,
                        span,
                    });
                }
            }
            self.emit(Event::VarDestroy {
                call_id,
                name,
                value,
                span,
            });
        }
    }

    pub(super) fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow> {
        self.burn()?;
        // Nested blocks emit ScopeEnter instead of a single Step.
        if !matches!(stmt, Stmt::Block(_)) {
            self.emit(Event::Step {
                call_id: self.current_call_id(),
                span: stmt.span(),
            });
        }
        match stmt {
            Stmt::Block(b) => self.exec_block(b),
            Stmt::Decl(d) => {
                for decl in &d.declarators {
                    let val = self.eval_decl_init(&d.ty, decl)?;
                    self.define_local(&decl.name.name, val, decl.span)?;
                }
                Ok(Flow::Next)
            }
            Stmt::Expr { expr, .. } => {
                let _ = self.eval_expr(expr)?;
                Ok(Flow::Next)
            }
            Stmt::Return { value, .. } => {
                let v = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Void,
                };
                Ok(Flow::Return(v))
            }
            Stmt::Break { .. } => Ok(Flow::Break),
            Stmt::Continue { .. } => Ok(Flow::Continue),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.eval_expr(cond)?;
                let then_taken = c.as_bool().map_err(RuntimeError::new)?;
                self.emit(Event::Branch {
                    call_id: self.current_call_id(),
                    then_taken,
                    span: *span,
                });
                if then_taken {
                    self.exec_stmt(then_branch)
                } else if let Some(e) = else_branch {
                    self.exec_stmt(e)
                } else {
                    Ok(Flow::Next)
                }
            }
            Stmt::While {
                cond, body, span, ..
            } => {
                loop {
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                    self.burn()?;
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::DoWhile {
                body, cond, span, ..
            } => {
                loop {
                    self.burn()?;
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                span,
                ..
            } => {
                let before: HashSet<String> = self
                    .stack
                    .last()
                    .map(|f| f.locals.keys().cloned().collect())
                    .unwrap_or_default();
                match init {
                    Some(ForInit::Decl(d)) => {
                        let _ = self.exec_stmt(&Stmt::Decl(d.clone()))?;
                    }
                    Some(ForInit::Expr(e)) => {
                        let _ = self.eval_expr(e)?;
                    }
                    None => {}
                }
                loop {
                    if let Some(c) = cond {
                        let cv = self.eval_expr(c)?;
                        if !cv.as_bool().map_err(RuntimeError::new)? {
                            break;
                        }
                    }
                    self.burn()?;
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    if let Some(s) = step {
                        let _ = self.eval_expr(s)?;
                    }
                }
                self.destroy_scope_locals(&before, *span);
                Ok(Flow::Next)
            }
            Stmt::ForRange {
                ty,
                names,
                iter,
                body,
                span,
                ..
            } => {
                let before: HashSet<String> = self
                    .stack
                    .last()
                    .map(|f| f.locals.keys().cloned().collect())
                    .unwrap_or_default();
                let container = self.eval_expr(iter)?;
                let Value::Object(id) = container else {
                    return Err(RuntimeError::at(*span, "range-for needs a container"));
                };
                let by_ref = Self::type_is_ref(ty);
                let is_string = matches!(self.heap.get(id), Some(Object::String(_)));
                // String element refs not supported yet — copy chars even for `auto&`.
                let by_ref = by_ref && !is_string;
                let len = match self.heap.get(id) {
                    Some(Object::Vector(e)) => e.len(),
                    Some(Object::String(s)) => s.chars().count(),
                    _ => {
                        return Err(RuntimeError::at(
                            *span,
                            "range-for only supports vector/string for now",
                        ))
                    }
                };
                for i in 0..len {
                    self.burn()?;
                    self.emit(Event::LoopIter {
                        call_id: self.current_call_id(),
                        span: *span,
                    });
                    if let Some(first) = names.first() {
                        let item = if by_ref {
                            Value::Ref(Address::Index {
                                obj: id,
                                index: i,
                            })
                        } else {
                            match self.heap.get(id) {
                                Some(Object::Vector(e)) => e
                                    .get(i)
                                    .cloned()
                                    .ok_or_else(|| RuntimeError::at(*span, "index out of bounds"))?,
                                Some(Object::String(s)) => Value::Char(
                                    s.chars().nth(i).ok_or_else(|| {
                                        RuntimeError::at(*span, "index out of bounds")
                                    })?,
                                ),
                                _ => unreachable!(),
                            }
                        };
                        self.define_local(&first.name, item, first.span)?;
                    }
                    for n in names.iter().skip(1) {
                        self.define_local(&n.name, Value::Int(0), n.span)?;
                    }
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                self.destroy_scope_locals(&before, *span);
                Ok(Flow::Next)
            }
            Stmt::Destructure {
                names, init, span, ..
            } => {
                let v = self.eval_expr(init)?;
                if let Some(first) = names.first() {
                    self.define_local(&first.name, v, first.span)?;
                }
                for n in names.iter().skip(1) {
                    self.define_local(&n.name, Value::Int(0), n.span)?;
                }
                let _ = span;
                Ok(Flow::Next)
            }
            Stmt::TypeAlias { .. } => Ok(Flow::Next),
        }
    }
}

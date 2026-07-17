use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    /// Mint or reuse this loop instance's id and emit `LoopIter`.
    pub(super) fn emit_loop_iter(&mut self, loop_id: &mut Option<u64>, span: Span) -> u64 {
        let id = match *loop_id {
            Some(id) => id,
            None => {
                let id = self.next_loop_id;
                self.next_loop_id += 1;
                *loop_id = Some(id);
                if let Some(frame) = self.stack.last_mut() {
                    frame.loop_stack.push(id);
                }
                id
            }
        };
        self.emit(Event::LoopIter { loop_id: id, span });
        id
    }

    pub(super) fn current_loop_id(&self) -> Option<u64> {
        self.stack.last().and_then(|f| f.loop_stack.last().copied())
    }

    pub(super) fn emit_loop_end(&mut self, loop_id: u64, reason: &str, span: Span) {
        self.emit(Event::LoopEnd {
            loop_id,
            reason: reason.into(),
            span,
        });
        if let Some(frame) = self.stack.last_mut() {
            if frame.loop_stack.last() == Some(&loop_id) {
                frame.loop_stack.pop();
            } else {
                frame.loop_stack.retain(|&x| x != loop_id);
            }
        }
    }

    /// Close every open loop on the current activation (`return` unwind).
    pub(super) fn close_open_loops_on_return(&mut self, span: Span) {
        let ids: Vec<u64> = self
            .stack
            .last()
            .map(|f| f.loop_stack.clone())
            .unwrap_or_default();
        for id in ids.into_iter().rev() {
            self.emit(Event::LoopEnd {
                loop_id: id,
                reason: "return".into(),
                span,
            });
        }
        if let Some(frame) = self.stack.last_mut() {
            frame.loop_stack.clear();
        }
    }

    pub(super) fn exec_block(&mut self, block: &Block) -> Result<Flow> {
        let before: HashSet<String> = self
            .stack
            .last()
            .map(|f| f.locals.keys().cloned().collect())
            .unwrap_or_default();
        self.emit(Event::ScopeEnter { span: block.span });
        for stmt in &block.stmts {
            match self.exec_stmt(stmt)? {
                Flow::Next => {}
                other => {
                    self.destroy_scope_locals(&before, block.span);
                    self.emit(Event::ScopeExit { span: block.span });
                    return Ok(other);
                }
            }
        }
        self.destroy_scope_locals(&before, block.span);
        self.emit(Event::ScopeExit { span: block.span });
        Ok(Flow::Next)
    }

    pub(super) fn destroy_scope_locals(&mut self, before: &HashSet<String>, span: Span) {
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
                    self.emit(Event::Dealloc { id, span });
                }
            }
            self.emit(Event::VarDestroy { name, value, span });
        }
    }

    pub(super) fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Flow> {
        self.burn()?;
        // Nested blocks emit ScopeEnter instead of a single Step.
        if !matches!(stmt, Stmt::Block(_)) {
            self.emit(Event::Step { span: stmt.span() });
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
            Stmt::Return { value, span } => {
                let v = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Void,
                };
                let _ = span;
                Ok(Flow::Return(v))
            }
            Stmt::Break { span } => {
                let loop_id = self
                    .current_loop_id()
                    .ok_or_else(|| RuntimeError::at(*span, "break outside loop"))?;
                self.emit(Event::Break {
                    loop_id,
                    span: *span,
                });
                Ok(Flow::Break)
            }
            Stmt::Continue { span } => {
                let loop_id = self
                    .current_loop_id()
                    .ok_or_else(|| RuntimeError::at(*span, "continue outside loop"))?;
                self.emit(Event::Continue {
                    loop_id,
                    span: *span,
                });
                Ok(Flow::Continue)
            }
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                span,
            } => {
                let c = self.eval_expr(cond)?;
                let then_taken = c.as_bool().map_err(RuntimeError::new)?;
                self.emit(Event::Branch {
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
                let mut loop_id: Option<u64> = None;
                let mut ended_by_break = false;
                loop {
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                    self.burn()?;
                    self.emit_loop_iter(&mut loop_id, *span);
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => {
                            if let Some(id) = loop_id {
                                self.emit_loop_end(id, "break", *span);
                            }
                            ended_by_break = true;
                            break;
                        }
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
                if !ended_by_break {
                    if let Some(id) = loop_id {
                        self.emit_loop_end(id, "exhausted", *span);
                    }
                }
                Ok(Flow::Next)
            }
            Stmt::DoWhile {
                body, cond, span, ..
            } => {
                let mut loop_id: Option<u64> = None;
                let mut ended_by_break = false;
                loop {
                    self.burn()?;
                    self.emit_loop_iter(&mut loop_id, *span);
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {}
                        Flow::Break => {
                            if let Some(id) = loop_id {
                                self.emit_loop_end(id, "break", *span);
                            }
                            ended_by_break = true;
                            break;
                        }
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    let c = self.eval_expr(cond)?;
                    if !c.as_bool().map_err(RuntimeError::new)? {
                        break;
                    }
                }
                if !ended_by_break {
                    if let Some(id) = loop_id {
                        self.emit_loop_end(id, "exhausted", *span);
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
                let mut loop_id: Option<u64> = None;
                let mut ended_by_break = false;
                loop {
                    if let Some(c) = cond {
                        let cv = self.eval_expr(c)?;
                        if !cv.as_bool().map_err(RuntimeError::new)? {
                            break;
                        }
                    }
                    self.burn()?;
                    self.emit_loop_iter(&mut loop_id, *span);
                    match self.exec_stmt(body)? {
                        Flow::Next | Flow::Continue => {
                            if let Some(s) = step {
                                let _ = self.eval_expr(s)?;
                            }
                        }
                        Flow::Break => {
                            if let Some(id) = loop_id {
                                self.emit_loop_end(id, "break", *span);
                            }
                            ended_by_break = true;
                            break;
                        }
                        Flow::Return(v) => {
                            self.destroy_scope_locals(&before, *span);
                            return Ok(Flow::Return(v));
                        }
                    }
                }
                if !ended_by_break {
                    if let Some(id) = loop_id {
                        self.emit_loop_end(id, "exhausted", *span);
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

                // Map / set range-for: yield (key,value) pairs or keys.
                let map_pairs: Option<Vec<(Value, Value)>> = match self.heap.get(id) {
                    Some(Object::Map(m)) => {
                        Some(m.iter().map(|(k, v)| (k.to_value(), v.clone())).collect())
                    }
                    Some(Object::UnorderedMap(m)) => {
                        Some(m.iter().map(|(k, v)| (k.to_value(), v.clone())).collect())
                    }
                    _ => None,
                };
                let set_keys: Option<Vec<Value>> = match self.heap.get(id) {
                    Some(Object::Set(s)) => Some(s.iter().map(|k| k.to_value()).collect()),
                    Some(Object::UnorderedSet(s)) => Some(s.iter().map(|k| k.to_value()).collect()),
                    _ => None,
                };
                if let Some(pairs) = map_pairs {
                    let mut loop_id: Option<u64> = None;
                    let mut ended_by_break = false;
                    for (k, v) in pairs {
                        self.burn()?;
                        self.emit_loop_iter(&mut loop_id, *span);
                        if names.len() >= 2 {
                            self.define_local(&names[0].name, k, names[0].span)?;
                            self.define_local(&names[1].name, v, names[1].span)?;
                        } else if let Some(first) = names.first() {
                            // `for (const auto& kvp : m)` — materialize a pair object.
                            let pid = self.heap.alloc(Object::Pair {
                                first: k,
                                second: v,
                            });
                            self.emit_alloc(pid, "pair", first.span);
                            self.define_local(&first.name, Value::Object(pid), first.span)?;
                        }
                        match self.exec_stmt(body)? {
                            Flow::Next | Flow::Continue => {}
                            Flow::Break => {
                                if let Some(id) = loop_id {
                                    self.emit_loop_end(id, "break", *span);
                                }
                                ended_by_break = true;
                                break;
                            }
                            Flow::Return(ret) => {
                                self.destroy_scope_locals(&before, *span);
                                return Ok(Flow::Return(ret));
                            }
                        }
                    }
                    if !ended_by_break {
                        if let Some(id) = loop_id {
                            self.emit_loop_end(id, "exhausted", *span);
                        }
                    }
                    self.destroy_scope_locals(&before, *span);
                    return Ok(Flow::Next);
                }
                if let Some(keys) = set_keys {
                    let mut loop_id: Option<u64> = None;
                    let mut ended_by_break = false;
                    for key in keys {
                        self.burn()?;
                        self.emit_loop_iter(&mut loop_id, *span);
                        if let Some(first) = names.first() {
                            self.define_local(&first.name, key, first.span)?;
                        }
                        match self.exec_stmt(body)? {
                            Flow::Next | Flow::Continue => {}
                            Flow::Break => {
                                if let Some(id) = loop_id {
                                    self.emit_loop_end(id, "break", *span);
                                }
                                ended_by_break = true;
                                break;
                            }
                            Flow::Return(ret) => {
                                self.destroy_scope_locals(&before, *span);
                                return Ok(Flow::Return(ret));
                            }
                        }
                    }
                    if !ended_by_break {
                        if let Some(id) = loop_id {
                            self.emit_loop_end(id, "exhausted", *span);
                        }
                    }
                    self.destroy_scope_locals(&before, *span);
                    return Ok(Flow::Next);
                }

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
                let mut loop_id: Option<u64> = None;
                let mut ended_by_break = false;
                for i in 0..len {
                    self.burn()?;
                    self.emit_loop_iter(&mut loop_id, *span);
                    if let Some(first) = names.first() {
                        let item = if by_ref {
                            Value::Ref(Address::Index { obj: id, index: i })
                        } else {
                            match self.heap.get(id) {
                                Some(Object::Vector(e)) => e.get(i).cloned().ok_or_else(|| {
                                    RuntimeError::at(*span, "index out of bounds")
                                })?,
                                Some(Object::String(s)) => {
                                    Value::Char(s.chars().nth(i).ok_or_else(|| {
                                        RuntimeError::at(*span, "index out of bounds")
                                    })?)
                                }
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
                        Flow::Break => {
                            if let Some(lid) = loop_id {
                                self.emit_loop_end(lid, "break", *span);
                            }
                            ended_by_break = true;
                            break;
                        }
                        Flow::Return(v) => {
                            self.destroy_scope_locals(&before, *span);
                            return Ok(Flow::Return(v));
                        }
                    }
                }
                if !ended_by_break {
                    if let Some(lid) = loop_id {
                        self.emit_loop_end(lid, "exhausted", *span);
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

use super::{Engine, Flow, Frame, LValue, Result};
use crate::builtins;
use crate::error::RuntimeError;
use crate::event::{Event, Slot};
use crate::stl;
use crate::value::{Address, Heap, MapKey, ObjId, Object, Value};
use rscpp_ast::*;
use std::collections::{HashMap, HashSet};

impl Engine {
    pub(super) fn exec_global_decl(&mut self, d: &Decl) -> Result<()> {
        for decl in &d.declarators {
            let val = if let Some(init) = &decl.init {
                self.eval_expr(init)?
            } else {
                self.default_value_for_type(&d.ty)?
            };
            self.globals.insert(decl.name.name.clone(), val.clone());
            self.emit(Event::VarCreate {
                name: decl.name.name.clone(),
                value: val,
                span: decl.span,
            });
        }
        Ok(())
    }
}

use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

impl Context {

    pub(super) fn check_item(&mut self, item: &Item) {
        match item {
            Item::UsingNamespace { .. } | Item::Decl(_) => {}
            Item::Function(f) => self.check_function(f, None),
            Item::Class(c) => {
                self.current_class = Some(c.name.name.clone());
                for m in &c.members {
                    if let Member::Function(f) = m {
                        self.check_function(f, Some(&c.name.name));
                    }
                }
                self.current_class = None;
            }
        }
    }

    pub(super) fn check_function(&mut self, f: &FunctionDef, class: Option<&str>) {
        self.symbols.push();
        // `this` in methods
        if let Some(c) = class {
            let _ = self.symbols.define(Symbol {
                name: "this".into(),
                ty: Ty::Pointer(Box::new(Ty::named(c, vec![]))),
                kind: SymbolKind::Var,
            });
        }
        for p in &f.params {
            let ty = self.resolve_ast_type(&p.ty);
            if let Some(name) = &p.name {
                if let Err(_) = self.symbols.define(Symbol {
                    name: name.name.clone(),
                    ty,
                    kind: SymbolKind::Var,
                }) {
                    self.err(name.span, format!("redefinition of parameter `{}`", name.name));
                }
            }
        }
        self.check_block(&f.body);
        self.symbols.pop();
    }

    pub(super) fn check_block(&mut self, block: &Block) {
        self.symbols.push();
        for s in &block.stmts {
            self.check_stmt(s);
        }
        self.symbols.pop();
    }

    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(b) => self.check_block(b),
            Stmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let _ = self.check_expr(cond);
                self.check_stmt(then_branch);
                if let Some(e) = else_branch {
                    self.check_stmt(e);
                }
            }
            Stmt::While { cond, body, .. } | Stmt::DoWhile { cond, body, .. } => {
                let _ = self.check_expr(cond);
                self.check_stmt(body);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.symbols.push();
                match init {
                    Some(ForInit::Decl(d)) => self.check_decl(d),
                    Some(ForInit::Expr(e)) => {
                        let _ = self.check_expr(e);
                    }
                    None => {}
                }
                if let Some(c) = cond {
                    let _ = self.check_expr(c);
                }
                if let Some(s) = step {
                    let _ = self.check_expr(s);
                }
                self.check_stmt(body);
                self.symbols.pop();
            }
            Stmt::ForRange {
                ty,
                name,
                iter,
                body,
                ..
            } => {
                self.symbols.push();
                let _ = self.check_expr(iter);
                let t = self.resolve_ast_type(ty);
                let _ = self.symbols.define(Symbol {
                    name: name.name.clone(),
                    ty: t,
                    kind: SymbolKind::Var,
                });
                self.check_stmt(body);
                self.symbols.pop();
            }
            Stmt::Return { value, span } => {
                if let Some(v) = value {
                    let _ = self.check_expr(v);
                }
                let _ = span;
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Expr { expr, .. } => {
                let _ = self.check_expr(expr);
            }
            Stmt::Decl(d) => self.check_decl(d),
        }
    }

    pub(super) fn check_decl(&mut self, d: &Decl) {
        let base = self.resolve_ast_type(&d.ty);
        for decl in &d.declarators {
            let mut ty = base.clone();
            for p in &decl.ptrs {
                ty = match p {
                    PtrKind::Pointer => Ty::Pointer(Box::new(ty)),
                    PtrKind::Reference => Ty::Reference(Box::new(ty)),
                };
            }
            if let Some(init) = &decl.init {
                let it = self.check_expr(init);
                if !self.assignable(&ty, &it) && it != Ty::Error && it != Ty::Unknown {
                    self.err(
                        decl.span,
                        format!("cannot initialize `{ty}` with `{it}`"),
                    );
                }
            }
            if let Err(_) = self.symbols.define(Symbol {
                name: decl.name.name.clone(),
                ty,
                kind: SymbolKind::Var,
            }) {
                self.err(
                    decl.name.span,
                    format!("redefinition of `{}`", decl.name.name),
                );
            }
        }
    }
}

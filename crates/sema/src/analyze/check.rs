use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

impl Context {
    pub(super) fn check_item(&mut self, item: &Item) {
        match item {
            Item::UsingNamespace { .. } | Item::Decl(_) | Item::TypeAlias { .. } => {}
            Item::Function(f) => self.check_function(f, None),
            Item::Class(c) => {
                self.check_class(c);
            }
        }
    }

    pub(super) fn check_class(&mut self, c: &ClassDef) {
        let prev = self.current_class.replace(c.name.name.clone());
        for m in &c.members {
            match m {
                Member::Function(f) => self.check_function(f, Some(&c.name.name)),
                Member::Class(nested) => self.check_class(nested),
                Member::Access(_) | Member::Field(_) | Member::TypeAlias { .. } => {}
            }
        }
        self.current_class = prev;
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
                    self.err(
                        name.span,
                        format!("redefinition of parameter `{}`", name.name),
                    );
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
                names,
                iter,
                body,
                ..
            } => {
                self.symbols.push();
                let it = self.check_expr(iter);
                let mut t = self.resolve_ast_type(ty);
                if matches!(t.strip_cv_ref(), Ty::Auto) {
                    let elem = self.range_elem_type(&it, iter.span());
                    // `for (auto [a,b] : m)` — names.len()>1 means structured binding
                    if names.len() > 1 {
                        let tys = destructure_elem_tys(&elem, names.len());
                        for (name, ty) in names.iter().zip(tys) {
                            let _ = self.symbols.define(Symbol {
                                name: name.name.clone(),
                                ty,
                                kind: SymbolKind::Var,
                            });
                        }
                    } else {
                        t = deduce_auto(&t, &elem);
                        for name in names {
                            let _ = self.symbols.define(Symbol {
                                name: name.name.clone(),
                                ty: t.clone(),
                                kind: SymbolKind::Var,
                            });
                        }
                    }
                } else if names.len() > 1 {
                    let tys = destructure_elem_tys(&t, names.len());
                    for (name, ty) in names.iter().zip(tys) {
                        let _ = self.symbols.define(Symbol {
                            name: name.name.clone(),
                            ty,
                            kind: SymbolKind::Var,
                        });
                    }
                } else {
                    for name in names {
                        let _ = self.symbols.define(Symbol {
                            name: name.name.clone(),
                            ty: t.clone(),
                            kind: SymbolKind::Var,
                        });
                    }
                }
                self.check_stmt(body);
                self.symbols.pop();
            }
            Stmt::Destructure { names, init, .. } => {
                let it = self.check_expr(init);
                let tys = destructure_elem_tys(&it, names.len());
                for (name, ty) in names.iter().zip(tys) {
                    let _ = self.symbols.define(Symbol {
                        name: name.name.clone(),
                        ty,
                        kind: SymbolKind::Var,
                    });
                }
            }
            Stmt::TypeAlias { name, ty, .. } => {
                let t = self.resolve_ast_type(ty);
                let _ = self.symbols.define(Symbol {
                    name: name.name.clone(),
                    ty: t,
                    kind: SymbolKind::Class,
                });
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
            // C++: name is in scope in its own initializer (recursive lambda /
            // `function<>` that calls itself). Predeclare before checking init.
            let provisional = if needs_auto_deduce(&ty) {
                Ty::Unknown
            } else {
                ty.clone()
            };
            self.symbols.redefine(Symbol {
                name: decl.name.name.clone(),
                ty: provisional,
                kind: SymbolKind::Var,
            });
            if let Some(init) = &decl.init {
                let it = self.check_expr(init);
                if !self.assignable(&ty, &it) && it != Ty::Error && it != Ty::Unknown {
                    self.err(decl.span, format!("cannot initialize `{ty}` with `{it}`"));
                }
                if needs_auto_deduce(&ty) {
                    ty = deduce_auto(&ty, &it);
                }
            }
            self.symbols.redefine(Symbol {
                name: decl.name.name.clone(),
                ty,
                kind: SymbolKind::Var,
            });
        }
    }
}

fn needs_auto_deduce(ty: &Ty) -> bool {
    match ty {
        Ty::Auto => true,
        Ty::Pointer(inner) | Ty::Reference(inner) | Ty::Const(inner) => needs_auto_deduce(inner),
        _ => false,
    }
}

/// Element types for `auto [a,b,c] = …` / `for (auto& [k,v] : m)`.
fn destructure_elem_tys(init: &Ty, n: usize) -> Vec<Ty> {
    let t = init.strip_cv_ref();
    let mut out = match t {
        Ty::Named { name, args } if name == "tuple" || name == "pair" => args.clone(),
        Ty::Named { name, args } if name == "map" || name == "unordered_map" => {
            // map iteration yields pair<const K, V>
            let k = args.first().cloned().unwrap_or(Ty::Unknown);
            let v = args.get(1).cloned().unwrap_or(Ty::Unknown);
            vec![k, v]
        }
        _ => Vec::new(),
    };
    while out.len() < n {
        out.push(Ty::Unknown);
    }
    out.truncate(n);
    out
}

fn deduce_auto(declared: &Ty, init: &Ty) -> Ty {
    match declared {
        Ty::Auto => init.clone(),
        Ty::Const(inner) => Ty::Const(Box::new(deduce_auto(inner, init))),
        Ty::Reference(inner) => Ty::Reference(Box::new(deduce_auto(inner, init))),
        Ty::Pointer(inner) if matches!(inner.as_ref(), Ty::Auto) || needs_auto_deduce(inner) => {
            match init.strip_cv_ref() {
                Ty::Pointer(p) => Ty::Pointer(Box::new(deduce_auto(inner, p))),
                other => Ty::Pointer(Box::new(if matches!(inner.as_ref(), Ty::Auto) {
                    other.clone()
                } else {
                    deduce_auto(inner, other)
                })),
            }
        }
        other => other.clone(),
    }
}

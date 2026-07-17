use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

impl Context {
    pub(super) fn analyze_tu(&mut self, tu: &TranslationUnit) {
        // Pass 1: declare classes and functions (signatures).
        for item in &tu.items {
            self.declare_item(item);
        }
        // Pass 2: check bodies.
        for item in &tu.items {
            self.check_item(item);
        }
    }

    pub(super) fn declare_item(&mut self, item: &Item) {
        match item {
            Item::UsingNamespace { .. } => {}
            Item::TypeAlias { name, ty, .. } => {
                self.type_aliases.insert(name.name.clone(), ty.clone());
            }
            Item::Class(c) => {
                let name = c.name.name.clone();
                if let Err(prev) = self.symbols.define(Symbol {
                    name: name.clone(),
                    ty: Ty::named(&name, vec![]),
                    kind: SymbolKind::Class,
                }) {
                    if prev.kind != SymbolKind::Class {
                        self.err(c.name.span, format!("redefinition of `{}`", c.name.name));
                    }
                }
                // Register methods as ClassName::method in a flat map via qualified later;
                // store fields/methods on a side table keyed by class name.
                self.register_class_members(c);
            }
            Item::Function(f) => {
                let ret = self.resolve_ast_type(&f.return_type);
                let params: Vec<Ty> = f
                    .params
                    .iter()
                    .map(|p| self.resolve_ast_type(&p.ty))
                    .collect();
                let ty = Ty::Function {
                    ret: Box::new(ret),
                    params,
                };
                if let Err(_) = self.symbols.define(Symbol {
                    name: f.name.name.clone(),
                    ty,
                    kind: SymbolKind::Func,
                }) {
                    self.err(f.name.span, format!("redefinition of `{}`", f.name.name));
                }
            }
            Item::Decl(d) => {
                let ty = self.resolve_ast_type(&d.ty);
                for decl in &d.declarators {
                    let mut t = ty.clone();
                    for p in &decl.ptrs {
                        t = match p {
                            PtrKind::Pointer => Ty::Pointer(Box::new(t)),
                            PtrKind::Reference => Ty::Reference(Box::new(t)),
                        };
                    }
                    if let Err(_) = self.symbols.define(Symbol {
                        name: decl.name.name.clone(),
                        ty: t,
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
    }

    /// class_name -> (field/method name -> Ty)
    // stored in symbols as "Class::name" for ponytail simplicity
    pub(super) fn register_class_members(&mut self, c: &ClassDef) {
        let class = &c.name.name;
        // Pass 1: type aliases so fields can resolve `Parent` / `TrieNode`.
        for m in &c.members {
            if let Member::TypeAlias { name, ty, .. } = m {
                self.type_aliases.insert(name.name.clone(), ty.clone());
            }
        }
        for m in &c.members {
            match m {
                Member::Access(_) | Member::TypeAlias { .. } => {}
                Member::Field(d) => {
                    let ty = self.resolve_ast_type(&d.ty);
                    for decl in &d.declarators {
                        let qname = format!("{class}::{}", decl.name.name);
                        self.symbols.redefine(Symbol {
                            name: qname,
                            ty: ty.clone(),
                            kind: SymbolKind::Field,
                        });
                    }
                }
                Member::Function(f) => {
                    let ret = self.resolve_ast_type(&f.return_type);
                    let params: Vec<Ty> = f
                        .params
                        .iter()
                        .map(|p| self.resolve_ast_type(&p.ty))
                        .collect();
                    let qname = format!("{class}::{}", f.name.name);
                    self.symbols.redefine(Symbol {
                        name: qname,
                        ty: Ty::Function {
                            ret: Box::new(ret),
                            params,
                        },
                        kind: SymbolKind::Func,
                    });
                }
                Member::Class(nested) => {
                    let _ = self.symbols.define(Symbol {
                        name: nested.name.name.clone(),
                        ty: Ty::named(&nested.name.name, vec![]),
                        kind: SymbolKind::Class,
                    });
                    self.register_class_members(nested);
                }
            }
        }
    }
}

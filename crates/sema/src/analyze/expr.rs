use super::{Context, SemaResult};
use crate::error::SemaError;
use crate::symbols::{Symbol, SymbolKind, SymbolTable};
use crate::ty::Ty;
use rscpp_ast::*;

impl Context {

    pub(super) fn check_expr(&mut self, expr: &Expr) -> Ty {
        match expr {
            Expr::IntLit { .. } => Ty::Int,
            Expr::FloatLit { .. } => Ty::Double,
            Expr::CharLit { .. } => Ty::Char,
            Expr::StringLit { .. } => Ty::named("string", vec![]),
            Expr::BoolLit { .. } => Ty::Bool,
            Expr::Nullptr { .. } => Ty::Pointer(Box::new(Ty::Void)),
            Expr::Name(path) => self.lookup_path(path),
            Expr::Unary { op, expr, span } => {
                let t = self.check_expr(expr);
                self.check_unary(*op, &t, *span)
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                self.check_binary(*op, &lt, &rt, *span)
            }
            Expr::Assign {
                op,
                left,
                right,
                span,
            } => {
                let lt = self.check_expr(left);
                let rt = self.check_expr(right);
                if *op == AssignOp::Assign {
                    if !self.assignable(&lt, &rt) && rt != Ty::Error && lt != Ty::Error {
                        self.err(*span, format!("cannot assign `{rt}` to `{lt}`"));
                    }
                    lt
                } else {
                    // compound assign: require numerics mostly
                    if !lt.is_numeric() || !rt.is_numeric() {
                        if lt != Ty::Error && rt != Ty::Error {
                            self.err(*span, format!("invalid compound assignment on `{lt}` and `{rt}`"));
                        }
                    }
                    lt
                }
            }
            Expr::Call { callee, args, span } => {
                if let Expr::Member {
                    base,
                    field,
                    arrow,
                    span: mspan,
                } = callee.as_ref()
                {
                    let mut bt = self.check_expr(base);
                    if *arrow {
                        match bt.strip_cv_ref() {
                            Ty::Pointer(inner) => bt = *inner.clone(),
                            Ty::Error | Ty::Unknown | Ty::Auto => {}
                            other => {
                                self.err(*mspan, format!("base of `->` has type `{other}`, not a pointer"));
                            }
                        }
                    }
                    let ft = self.lookup_member(&bt, &field.name, field.span);
                    return self.check_fn_call(&ft, args, *span);
                }
                let ct = self.check_expr(callee);
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.check_expr(a)).collect();
                self.check_fn_call_types(&ct, &arg_tys, *span)
            }
            Expr::Index { base, index, span } => {
                let bt = self.check_expr(base);
                let it = self.check_expr(index);
                let it = it.strip_cv_ref();
                if matches!(it, Ty::Auto | Ty::Unknown) {
                    // structured bindings / auto params — allow for now
                } else if !it.is_integral() && *it != Ty::Error {
                    self.err(*span, format!("array index must be integral, got `{it}`"));
                }
                self.elem_type(&bt, *span)
            }
            Expr::Member {
                base,
                field,
                arrow,
                span,
            } => {
                let mut bt = self.check_expr(base);
                if *arrow {
                    match bt.strip_cv_ref() {
                        Ty::Pointer(inner) => bt = *inner.clone(),
                        Ty::Error | Ty::Unknown | Ty::Auto => {}
                        other => {
                            self.err(*span, format!("base of `->` has type `{other}`, not a pointer"));
                        }
                    }
                }
                self.lookup_member(&bt, &field.name, field.span)
            }
            Expr::Cast { ty, expr, .. } => {
                let _ = self.check_expr(expr);
                self.resolve_ast_type(ty)
            }
            Expr::InitList { elems, .. } => {
                for e in elems {
                    let _ = self.check_expr(e);
                }
                // brace init — unknown concrete type without context
                Ty::Unknown
            }
            Expr::Conditional {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let _ = self.check_expr(cond);
                let t = self.check_expr(then_branch);
                let e = self.check_expr(else_branch);
                if t == Ty::Error || e == Ty::Error {
                    Ty::Error
                } else if self.assignable(&t, &e) || self.assignable(&e, &t) {
                    t
                } else {
                    Ty::Unknown
                }
            }
            Expr::Lambda { params, body, .. } => {
                self.symbols.push();
                for p in params {
                    let ty = self.resolve_ast_type(&p.ty);
                    if let Some(n) = &p.name {
                        let _ = self.symbols.define(Symbol {
                            name: n.name.clone(),
                            ty,
                            kind: SymbolKind::Var,
                        });
                    }
                }
                self.check_block(body);
                self.symbols.pop();
                Ty::Unknown
            }
            Expr::New { ty, args, .. } => {
                for a in args {
                    let _ = self.check_expr(a);
                }
                let t = self.resolve_ast_type(ty);
                Ty::Pointer(Box::new(t))
            }
        }
    }
}

use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {

    pub(super) fn parse_function_rest(
        &mut self,
        start: usize,
        return_type: Type,
        name: Ident,
    ) -> Result<FunctionDef, ParseError> {
        self.expect_punct(Punct::LParen)?;
        let mut params = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                params.push(self.parse_param()?);
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect_punct(Punct::RParen)?;
        // optional trailing `const` / `override` / `final` / `noexcept`
        loop {
            if self.at_keyword(Keyword::Const) {
                self.bump();
                continue;
            }
            if let TokenKind::Ident(name) = self.peek_kind() {
                if name == "override" || name == "final" {
                    self.bump();
                    continue;
                }
                if name == "noexcept" {
                    self.bump();
                    if self.at_punct(Punct::LParen) {
                        self.bump();
                        let mut depth = 1i32;
                        while depth > 0 && !self.at_eof() {
                            if self.at_punct(Punct::LParen) {
                                depth += 1;
                            } else if self.at_punct(Punct::RParen) {
                                depth -= 1;
                            }
                            self.bump();
                        }
                    }
                    continue;
                }
            }
            break;
        }
        // Optional ctor-initializer: `: a(x), b{y} { body }`
        if self.at_punct(Punct::Colon) {
            self.bump();
            loop {
                if self.at_eof() || self.at_punct(Punct::Semi) || self.at_punct(Punct::LBrace) {
                    break;
                }
                // member name
                while matches!(self.peek_kind(), TokenKind::Ident(_))
                    || self.at_punct(Punct::Scope)
                    || self.at_punct(Punct::Tilde)
                {
                    self.bump();
                }
                if self.at_punct(Punct::LParen) {
                    self.skip_balanced(Punct::LParen, Punct::RParen);
                } else if self.at_punct(Punct::LBrace) {
                    self.skip_balanced(Punct::LBrace, Punct::RBrace);
                }
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let body = if self.at_punct(Punct::LBrace) {
            self.parse_block()?
        } else if self.at_punct(Punct::Eq) {
            // `= default` / `= delete`
            self.bump();
            let _ = self.bump();
            let end = self.expect_punct(Punct::Semi)?.span.end;
            Block {
                stmts: Vec::new(),
                span: Span::new(start, end),
            }
        } else if self.at_punct(Punct::Semi) {
            let t = self.bump();
            Block {
                stmts: Vec::new(),
                span: t.span,
            }
        } else {
            return Err(self.err("expected function body"));
        };
        let end = body.span.end;
        Ok(FunctionDef {
            return_type,
            name,
            params,
            body,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_param(&mut self) -> Result<Param, ParseError> {
        let start = self.peek_span().start;
        let mut ty = self.parse_type()?;
        // Optional ptr/ref after type for params like `int& nums`
        ty = self.apply_ptr_suffixes(ty)?;
        let name = if matches!(self.peek_kind(), TokenKind::Ident(_)) {
            Some(self.parse_ident()?)
        } else {
            None
        };
        // Soft-skip default arguments: `double rel_tol = 1e-09`
        if self.at_punct(Punct::Eq) {
            self.bump();
            let _ = self.parse_expr_bp(2)?;
        }
        let end = name
            .as_ref()
            .map(|n| n.span.end)
            .unwrap_or_else(|| ty.span().end);
        Ok(Param {
            ty,
            name,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_decl_rest(
        &mut self,
        start: usize,
        ty: Type,
        first_name: Ident,
    ) -> Result<Decl, ParseError> {
        let mut declarators = Vec::new();
        declarators.push(self.parse_init_declarator_after_name(first_name, &ty)?);
        while self.at_punct(Punct::Comma) {
            self.bump();
            // optional ptrs then name
            let mut ptrs = Vec::new();
            while self.at_punct(Punct::Star) || self.at_punct(Punct::Amp) {
                if self.at_punct(Punct::Star) {
                    self.bump();
                    ptrs.push(PtrKind::Pointer);
                } else {
                    self.bump();
                    ptrs.push(PtrKind::Reference);
                }
            }
            let name = self.parse_ident()?;
            let mut d = self.parse_init_declarator_after_name(name, &ty)?;
            d.ptrs = ptrs;
            declarators.push(d);
        }
        let end = self.expect_punct(Punct::Semi)?.span.end;
        Ok(Decl {
            ty,
            declarators,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_init_declarator_after_name(
        &mut self,
        name: Ident,
        ty: &Type,
    ) -> Result<InitDeclarator, ParseError> {
        let start = name.span.start;
        let init = if self.at_punct(Punct::Eq) {
            self.bump();
            // assignment-expr: `int a = 0, b = 1` must not eat the declarator comma
            Some(self.parse_expr_bp(2)?)
        } else if self.at_punct(Punct::LBrace) {
            Some(self.parse_init_list()?)
        } else if self.at_punct(Punct::LParen) {
            // `T x(args);` — ctor / direct init
            if let Some(tname) = named_type_ctor(ty) {
                let callee = Expr::Name(Path {
                    segments: vec![Ident {
                        name: tname,
                        span: name.span,
                    }],
                    span: name.span,
                });
                Some(self.parse_call(callee)?)
            } else {
                self.bump();
                let e = self.parse_expr_bp(2)?;
                self.expect_punct(Punct::RParen)?;
                Some(e)
            }
        } else {
            None
        };
        let end = init
            .as_ref()
            .map(|e| e.span().end)
            .unwrap_or(name.span.end);
        Ok(InitDeclarator {
            name,
            ptrs: Vec::new(),
            init,
            span: Span::new(start, end),
        })
    }

    pub(super) fn apply_ptr_suffixes(&mut self, mut ty: Type) -> Result<Type, ParseError> {
        loop {
            if self.at_punct(Punct::Star) {
                let star = self.bump();
                let span = Span::new(ty.span().start, star.span.end);
                ty = Type::Pointer {
                    inner: Box::new(ty),
                    span,
                };
            } else if self.at_punct(Punct::Amp) {
                let amp = self.bump();
                let span = Span::new(ty.span().start, amp.span.end);
                ty = Type::Reference {
                    inner: Box::new(ty),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(ty)
    }
}

fn named_type_ctor(ty: &Type) -> Option<String> {
    match ty {
        Type::Named { path, .. } => path.segments.last().map(|s| s.name.clone()),
        Type::Const { inner, .. } | Type::Reference { inner, .. } | Type::Pointer { inner, .. } => {
            named_type_ctor(inner)
        }
        _ => None,
    }
}

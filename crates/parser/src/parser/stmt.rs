use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {
    pub(super) fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect_punct(Punct::LBrace)?.span.start;
        let mut stmts = Vec::new();
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            stmts.push(self.parse_stmt()?);
        }
        let end = self.expect_punct(Punct::RBrace)?.span.end;
        Ok(Block {
            stmts,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        if self.at_punct(Punct::Semi) {
            let t = self.bump();
            return Ok(Stmt::Block(Block {
                stmts: Vec::new(),
                span: t.span,
            }));
        }
        if self.at_punct(Punct::LBrace) {
            return Ok(Stmt::Block(self.parse_block()?));
        }
        if self.at_keyword(Keyword::If) {
            return self.parse_if();
        }
        if self.at_keyword(Keyword::While) {
            return self.parse_while();
        }
        if self.at_keyword(Keyword::Do) {
            return self.parse_do_while();
        }
        if self.at_keyword(Keyword::For) {
            return self.parse_for();
        }
        if self.at_keyword(Keyword::Return) {
            let start = self.bump().span.start;
            let value = if self.at_punct(Punct::Semi) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            let end = self.expect_punct(Punct::Semi)?.span.end;
            return Ok(Stmt::Return {
                value,
                span: Span::new(start, end),
            });
        }
        if self.at_keyword(Keyword::Break) {
            let start = self.bump().span.start;
            let end = self.expect_punct(Punct::Semi)?.span.end;
            return Ok(Stmt::Break {
                span: Span::new(start, end),
            });
        }
        if self.at_keyword(Keyword::Continue) {
            let start = self.bump().span.start;
            let end = self.expect_punct(Punct::Semi)?.span.end;
            return Ok(Stmt::Continue {
                span: Span::new(start, end),
            });
        }

        // Anonymous / local enum: `enum { A, B };` → introduce int constants.
        if self.at_keyword(Keyword::Enum) {
            return self.parse_local_enum_as_ints();
        }

        if self.at_keyword(Keyword::Using) {
            let start = self.bump().span.start;
            let name = self.parse_ident()?;
            self.expect_punct(Punct::Eq)?;
            let ty = self.parse_type()?;
            let end = self.expect_punct(Punct::Semi)?.span.end;
            return Ok(Stmt::TypeAlias {
                name,
                ty,
                span: Span::new(start, end),
            });
        }

        if self.at_declaration_start() {
            let start = self.peek_span().start;
            self.skip_decl_specs();
            let ty = self.parse_type()?;
            if self.at_punct(Punct::LBracket) {
                let names = self.parse_binding_names()?;
                self.expect_punct(Punct::Eq)?;
                let init = self.parse_expr()?;
                let end = self.expect_punct(Punct::Semi)?.span.end;
                return Ok(Stmt::Destructure {
                    names,
                    init,
                    span: Span::new(start, end),
                });
            }
            let name = self.parse_ident()?;
            return Ok(Stmt::Decl(self.parse_decl_rest(start, ty, name)?));
        }

        let expr = self.parse_expr()?;
        let start = expr.span().start;
        let end = self.expect_punct(Punct::Semi)?.span.end;
        Ok(Stmt::Expr {
            expr,
            span: Span::new(start, end),
        })
    }

    pub(super) fn at_declaration_start(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Keyword(
                Keyword::Static
                | Keyword::Constexpr
                | Keyword::Inline
                | Keyword::Mutable
                | Keyword::Const
                | Keyword::Void
                | Keyword::Bool
                | Keyword::Char
                | Keyword::Short
                | Keyword::Int
                | Keyword::Long
                | Keyword::Float
                | Keyword::Double
                | Keyword::Unsigned
                | Keyword::Signed
                | Keyword::Auto
                | Keyword::WcharT,
            ) => true,
            TokenKind::Ident(_) => self.looks_like_decl_from(self.pos),
            _ => false,
        }
    }

    /// `std::vector<int> x` yes; `std::sort(...)` / `a < b` no.
    pub(super) fn looks_like_decl_from(&self, mut i: usize) -> bool {
        if !matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
        ) {
            return false;
        }
        i += 1;
        while matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Punct(Punct::Scope))
        ) {
            i += 1;
            if !matches!(
                self.tokens.get(i).map(|t| &t.kind),
                Some(TokenKind::Ident(_))
            ) {
                return false;
            }
            i += 1;
        }
        if matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Punct(Punct::Lt))
        ) {
            if !self.looks_like_template_args_at(i) {
                return false;
            }
            i = match self.skip_template_args(i) {
                Some(n) => n,
                None => return false,
            };
        }
        matches!(
            self.tokens.get(i).map(|t| &t.kind),
            Some(TokenKind::Ident(_))
                | Some(TokenKind::Punct(Punct::Star | Punct::Amp | Punct::AmpAmp))
        )
    }

    pub(super) fn looks_like_template_args_at(&self, lt_pos: usize) -> bool {
        let Some(t) = self.tokens.get(lt_pos + 1) else {
            return false;
        };
        match &t.kind {
            TokenKind::Keyword(
                Keyword::Void
                | Keyword::Bool
                | Keyword::Char
                | Keyword::Int
                | Keyword::Long
                | Keyword::Short
                | Keyword::Float
                | Keyword::Double
                | Keyword::Unsigned
                | Keyword::Signed
                | Keyword::Const
                | Keyword::Auto,
            ) => true,
            // Non-type template args: `bitset<32>`, `array<int, 4>`
            TokenKind::IntLit { .. } => matches!(
                self.tokens.get(lt_pos + 2).map(|t| &t.kind),
                Some(TokenKind::Punct(Punct::Gt | Punct::GtGt | Punct::Comma))
            ),
            TokenKind::Ident(_) => matches!(
                self.tokens.get(lt_pos + 2).map(|t| &t.kind),
                Some(
                    TokenKind::Punct(
                        Punct::Gt
                            | Punct::GtGt
                            | Punct::Comma
                            | Punct::Scope
                            | Punct::Lt
                            | Punct::Star
                            | Punct::Amp
                    ) | TokenKind::Keyword(_)
                )
            ),
            _ => false,
        }
    }

    /// Index after the matching `>` / split `>>` (not a token index into pending_gt).
    pub(super) fn skip_template_args(&self, lt_pos: usize) -> Option<usize> {
        let mut i = lt_pos + 1;
        let mut depth = 1i32;
        while let Some(t) = self.tokens.get(i) {
            match &t.kind {
                TokenKind::Punct(Punct::Lt) => depth += 1,
                TokenKind::Punct(Punct::Gt) => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                TokenKind::Punct(Punct::GtGt) => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1); // one `>` consumed conceptually; rest is shift-ish
                    }
                    depth -= 1;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                TokenKind::Eof => return None,
                _ => {}
            }
            i += 1;
        }
        None
    }

    pub(super) fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::If)?.span.start;
        self.expect_punct(Punct::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_punct(Punct::RParen)?;
        let then_branch = Box::new(self.parse_stmt()?);
        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            Some(Box::new(self.parse_stmt()?))
        } else {
            None
        };
        let end = else_branch
            .as_ref()
            .map(|s| s.span().end)
            .unwrap_or_else(|| then_branch.span().end);
        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::While)?.span.start;
        self.expect_punct(Punct::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_punct(Punct::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = body.span().end;
        Ok(Stmt::While {
            cond,
            body,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_do_while(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::Do)?.span.start;
        let body = Box::new(self.parse_stmt()?);
        self.expect_keyword(Keyword::While)?;
        self.expect_punct(Punct::LParen)?;
        let cond = self.parse_expr()?;
        self.expect_punct(Punct::RParen)?;
        let end = self.expect_punct(Punct::Semi)?.span.end;
        Ok(Stmt::DoWhile {
            body,
            cond,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::For)?.span.start;
        self.expect_punct(Punct::LParen)?;

        // Range-for: `for (T name : expr)` or `for (T [a, b] : expr)`
        if self.at_declaration_start() {
            let dstart = self.peek_span().start;
            self.skip_decl_specs();
            let ty = self.parse_type()?;
            if self.at_punct(Punct::LBracket) {
                let names = self.parse_binding_names()?;
                self.expect_punct(Punct::Colon)?;
                let iter = self.parse_expr()?;
                self.expect_punct(Punct::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = body.span().end;
                return Ok(Stmt::ForRange {
                    ty,
                    names,
                    iter,
                    body,
                    span: Span::new(start, end),
                });
            }
            let name = self.parse_ident()?;
            if self.at_punct(Punct::Colon) {
                self.bump();
                let iter = self.parse_expr()?;
                self.expect_punct(Punct::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = body.span().end;
                return Ok(Stmt::ForRange {
                    ty,
                    names: vec![name],
                    iter,
                    body,
                    span: Span::new(start, end),
                });
            }
            // Classic for with decl init
            let init = Some(ForInit::Decl(self.parse_decl_rest(dstart, ty, name)?));
            let cond = if self.at_punct(Punct::Semi) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect_punct(Punct::Semi)?;
            let step = if self.at_punct(Punct::RParen) {
                None
            } else {
                Some(self.parse_expr()?)
            };
            self.expect_punct(Punct::RParen)?;
            let body = Box::new(self.parse_stmt()?);
            let end = body.span().end;
            return Ok(Stmt::For {
                init,
                cond,
                step,
                body,
                span: Span::new(start, end),
            });
        }

        let init = if self.at_punct(Punct::Semi) {
            None
        } else {
            let e = self.parse_expr()?;
            self.expect_punct(Punct::Semi)?;
            Some(ForInit::Expr(e))
        };
        if init.is_none() {
            self.expect_punct(Punct::Semi)?;
        }

        let cond = if self.at_punct(Punct::Semi) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect_punct(Punct::Semi)?;

        let step = if self.at_punct(Punct::RParen) {
            None
        } else {
            Some(self.parse_expr()?)
        };
        self.expect_punct(Punct::RParen)?;
        let body = Box::new(self.parse_stmt()?);
        let end = body.span().end;
        Ok(Stmt::For {
            init,
            cond,
            step,
            body,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_binding_names(&mut self) -> Result<Vec<Ident>, ParseError> {
        self.expect_punct(Punct::LBracket)?;
        let mut names = Vec::new();
        if !self.at_punct(Punct::RBracket) {
            loop {
                names.push(self.parse_ident()?);
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        self.expect_punct(Punct::RBracket)?;
        if names.is_empty() {
            return Err(self.err("structured binding needs at least one name"));
        }
        Ok(names)
    }

    /// `enum { A, B = 1 };` inside a function — expose enumerators as `int` locals.
    fn parse_local_enum_as_ints(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::Enum)?.span.start;
        if matches!(self.peek_kind(), TokenKind::Ident(_)) {
            let _ = self.bump();
        }
        let mut names = Vec::new();
        if self.at_punct(Punct::LBrace) {
            self.bump();
            while !self.at_punct(Punct::RBrace) && !self.at_eof() {
                if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                    let name = self.parse_ident()?;
                    if self.at_punct(Punct::Eq) {
                        self.bump();
                        let _ = self.parse_expr_bp(2)?;
                    }
                    names.push(name);
                }
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
            self.expect_punct(Punct::RBrace)?;
        }
        let end = self.expect_punct(Punct::Semi)?.span.end;
        let span = Span::new(start, end);
        if names.is_empty() {
            return Ok(Stmt::Block(Block {
                stmts: Vec::new(),
                span,
            }));
        }
        let ty = Type::Builtin {
            kind: BuiltinType::Int,
            span,
        };
        let declarators = names
            .into_iter()
            .map(|name| {
                let nspan = name.span;
                InitDeclarator {
                    name,
                    ptrs: Vec::new(),
                    init: None,
                    span: nspan,
                }
            })
            .collect();
        Ok(Stmt::Decl(Decl {
            ty,
            declarators,
            span,
        }))
    }
}

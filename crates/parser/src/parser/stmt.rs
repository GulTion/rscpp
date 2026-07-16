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

        if self.at_declaration_start() {
            let start = self.peek_span().start;
            self.skip_decl_specs();
            let ty = self.parse_type()?;
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
            TokenKind::Ident(_) => {
                // Heuristic: Ident followed by Ident / * / & / < → type
                let next = self.tokens.get(self.pos + 1).map(|t| &t.kind);
                matches!(
                    next,
                    Some(TokenKind::Ident(_))
                        | Some(TokenKind::Punct(Punct::Star | Punct::Amp | Punct::Lt | Punct::Scope))
                )
            }
            _ => false,
        }
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

        // Range-for: `for (T name : expr)`
        if self.at_declaration_start() {
            let dstart = self.peek_span().start;
            self.skip_decl_specs();
            let ty = self.parse_type()?;
            let name = self.parse_ident()?;
            if self.at_punct(Punct::Colon) {
                self.bump();
                let iter = self.parse_expr()?;
                self.expect_punct(Punct::RParen)?;
                let body = Box::new(self.parse_stmt()?);
                let end = body.span().end;
                return Ok(Stmt::ForRange {
                    ty,
                    name,
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
}

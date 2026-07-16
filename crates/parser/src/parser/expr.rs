use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {

    pub(super) fn parse_ident(&mut self) -> Result<Ident, ParseError> {
        match self.peek_kind() {
            TokenKind::Ident(_) => {
                let tok = self.bump();
                match tok.kind {
                    TokenKind::Ident(name) => Ok(Ident {
                        name,
                        span: tok.span,
                    }),
                    _ => unreachable!(),
                }
            }
            other => Err(ParseError::new(
                self.peek_span(),
                format!("expected identifier, found {other:?}"),
            )),
        }
    }

    pub(super) fn parse_path(&mut self) -> Result<Path, ParseError> {
        let first = self.parse_ident()?;
        let start = first.span.start;
        let mut segments = vec![first];
        while self.at_punct(Punct::Scope) {
            self.bump();
            segments.push(self.parse_ident()?);
        }
        let end = segments.last().map(|s| s.span.end).unwrap_or(start);
        Ok(Path {
            segments,
            span: Span::new(start, end),
        })
    }

    // --- Expressions (Pratt) -------------------------------------------------

    pub(super) fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    pub(super) fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
            // `numeric_limits<int>::max` — treat `<...>` after a name as template-args (erase).
            if matches!(lhs, Expr::Name(_))
                && self.at_punct(Punct::Lt)
                && self.looks_like_template_args()
            {
                self.bump();
                if !self.at_punct(Punct::Gt) && !self.at_punct(Punct::GtGt) {
                    loop {
                        // NTTP: `bitset<32>(i)` — int lit is not a type
                        if matches!(self.peek_kind(), TokenKind::IntLit { .. }) {
                            self.bump();
                        } else {
                            let _ = self.parse_type()?;
                        }
                        if self.at_punct(Punct::Comma) {
                            self.bump();
                            continue;
                        }
                        break;
                    }
                }
                self.bump_template_gt()?;
                continue;
            }
            // Continue `name::member` after template args erased above.
            if let Expr::Name(path) = &lhs {
                if self.at_punct(Punct::Scope) {
                    self.bump();
                    let field = self.parse_ident()?;
                    let mut path = path.clone();
                    let end = field.span.end;
                    path.segments.push(field);
                    path.span = Span::new(path.span.start, end);
                    lhs = Expr::Name(path);
                    continue;
                }
            }
            // `vector<int>{1,2}` / `T{args}` braced temporary
            if matches!(lhs, Expr::Name(_)) && self.at_punct(Punct::LBrace) {
                let init = self.parse_init_list()?;
                let span = Span::new(lhs.span().start, init.span().end);
                let args = match init {
                    Expr::InitList { elems, .. } => elems,
                    other => vec![other],
                };
                lhs = Expr::Call {
                    callee: Box::new(lhs),
                    args,
                    span,
                };
                continue;
            }
            // Postfix
            if self.at_punct(Punct::LParen) {
                lhs = self.parse_call(lhs)?;
                continue;
            }
            if self.at_punct(Punct::LBracket) {
                lhs = self.parse_index(lhs)?;
                continue;
            }
            if self.at_punct(Punct::Dot) || self.at_punct(Punct::Arrow) {
                lhs = self.parse_member_expr(lhs)?;
                continue;
            }
            if self.at_punct(Punct::PlusPlus) {
                let tok = self.bump();
                let span = Span::new(lhs.span().start, tok.span.end);
                lhs = Expr::Unary {
                    op: UnaryOp::PostInc,
                    expr: Box::new(lhs),
                    span,
                };
                continue;
            }
            if self.at_punct(Punct::MinusMinus) {
                let tok = self.bump();
                let span = Span::new(lhs.span().start, tok.span.end);
                lhs = Expr::Unary {
                    op: UnaryOp::PostDec,
                    expr: Box::new(lhs),
                    span,
                };
                continue;
            }

            let Some((op_kind, l_bp, r_bp)) = self.infix_bp() else {
                break;
            };
            if l_bp < min_bp {
                break;
            }
            self.bump(); // operator

            if let Some(assign) = as_assign_op(&op_kind) {
                let rhs = self.parse_expr_bp(r_bp)?;
                let span = Span::new(lhs.span().start, rhs.span().end);
                lhs = Expr::Assign {
                    op: assign,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                    span,
                };
                continue;
            }

            let rhs = self.parse_expr_bp(r_bp)?;
            let span = Span::new(lhs.span().start, rhs.span().end);
            lhs = Expr::Binary {
                op: as_binary_op(&op_kind).unwrap(),
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }

        // Ternary: cond ? then : else  (bp between || and assignment)
        if min_bp <= 2 && self.at_punct(Punct::Question) {
            self.bump();
            let then_branch = self.parse_expr()?;
            self.expect_punct(Punct::Colon)?;
            let else_branch = self.parse_expr_bp(2)?;
            let span = Span::new(lhs.span().start, else_branch.span().end);
            lhs = Expr::Conditional {
                cond: Box::new(lhs),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span,
            };
        }

        Ok(lhs)
    }

    pub(super) fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        if self.at_punct(Punct::LBrace) {
            return self.parse_init_list();
        }
        if self.at_punct(Punct::LBracket) {
            return self.parse_lambda();
        }
        if self.at_keyword(Keyword::New) {
            return self.parse_new();
        }
        if self.at_keyword(Keyword::Delete) {
            return self.parse_delete();
        }
        match self.peek_kind() {
            TokenKind::Punct(Punct::Plus)
            | TokenKind::Punct(Punct::Minus)
            | TokenKind::Punct(Punct::Not)
            | TokenKind::Punct(Punct::Tilde)
            | TokenKind::Punct(Punct::Star)
            | TokenKind::Punct(Punct::Amp)
            | TokenKind::Punct(Punct::PlusPlus)
            | TokenKind::Punct(Punct::MinusMinus) => {
                let tok = self.bump();
                let op = match tok.kind {
                    TokenKind::Punct(Punct::Plus) => UnaryOp::Plus,
                    TokenKind::Punct(Punct::Minus) => UnaryOp::Minus,
                    TokenKind::Punct(Punct::Not) => UnaryOp::Not,
                    TokenKind::Punct(Punct::Tilde) => UnaryOp::BitNot,
                    TokenKind::Punct(Punct::Star) => UnaryOp::Deref,
                    TokenKind::Punct(Punct::Amp) => UnaryOp::AddrOf,
                    TokenKind::Punct(Punct::PlusPlus) => UnaryOp::PreInc,
                    TokenKind::Punct(Punct::MinusMinus) => UnaryOp::PreDec,
                    _ => unreachable!(),
                };
                let expr = self.parse_expr_bp(prefix_bp())?;
                let span = Span::new(tok.span.start, expr.span().end);
                Ok(Expr::Unary {
                    op,
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::Punct(Punct::LParen) => {
                // Cast `(type)expr` vs grouping `(expr)`.
                // Lookahead: if after `(` we see a type then `)`, it's a cast.
                self.bump();
                if self.looks_like_cast() {
                    let ty = self.parse_type()?;
                    self.expect_punct(Punct::RParen)?;
                    let expr = self.parse_expr_bp(prefix_bp())?;
                    let span = Span::new(ty.span().start, expr.span().end);
                    Ok(Expr::Cast {
                        ty,
                        expr: Box::new(expr),
                        span,
                    })
                } else {
                    let e = self.parse_expr()?;
                    self.expect_punct(Punct::RParen)?;
                    Ok(e)
                }
            }
            TokenKind::IntLit { .. } => {
                let t = self.bump();
                let value = match t.kind {
                    TokenKind::IntLit { value, .. } => value,
                    _ => unreachable!(),
                };
                Ok(Expr::IntLit {
                    value,
                    span: t.span,
                })
            }
            TokenKind::FloatLit { .. } => {
                let t = self.bump();
                let value = match t.kind {
                    TokenKind::FloatLit { value, .. } => value,
                    _ => unreachable!(),
                };
                Ok(Expr::FloatLit {
                    value,
                    span: t.span,
                })
            }
            TokenKind::CharLit(_) => {
                let t = self.bump();
                let value = match t.kind {
                    TokenKind::CharLit(c) => c,
                    _ => unreachable!(),
                };
                Ok(Expr::CharLit {
                    value,
                    span: t.span,
                })
            }
            TokenKind::StringLit(_) => {
                let t = self.bump();
                let value = match t.kind {
                    TokenKind::StringLit(s) => s,
                    _ => unreachable!(),
                };
                Ok(Expr::StringLit {
                    value,
                    span: t.span,
                })
            }
            TokenKind::Keyword(Keyword::True) => {
                let t = self.bump();
                Ok(Expr::BoolLit {
                    value: true,
                    span: t.span,
                })
            }
            TokenKind::Keyword(Keyword::False) => {
                let t = self.bump();
                Ok(Expr::BoolLit {
                    value: false,
                    span: t.span,
                })
            }
            TokenKind::Keyword(Keyword::Nullptr) => {
                let t = self.bump();
                Ok(Expr::Nullptr { span: t.span })
            }
            TokenKind::Keyword(Keyword::This) => {
                let t = self.bump();
                Ok(Expr::Name(Path::single(Ident {
                    name: "this".into(),
                    span: t.span,
                })))
            }
            TokenKind::Keyword(Keyword::Sizeof) => self.parse_sizeof(),
            TokenKind::Keyword(
                Keyword::Void
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
            ) if self.looks_like_functional_cast() => self.parse_functional_cast(),
            TokenKind::Ident(_) => Ok(Expr::Name(self.parse_path()?)),
            TokenKind::Keyword(Keyword::StaticCast | Keyword::ReinterpretCast) => {
                let start = self.bump().span.start;
                self.expect_punct(Punct::Lt)?;
                let ty = self.parse_type()?;
                self.bump_template_gt()?;
                self.expect_punct(Punct::LParen)?;
                let expr = self.parse_expr()?;
                let end = self.expect_punct(Punct::RParen)?.span.end;
                Ok(Expr::Cast {
                    ty,
                    expr: Box::new(expr),
                    span: Span::new(start, end),
                })
            }
            _ => Err(self.err(format!(
                "expected expression, found {:?}",
                self.peek_kind()
            ))),
        }
    }

    pub(super) fn looks_like_cast(&self) -> bool {
        // Very simple: type keyword, or Ident then `)`.
        match self.peek_kind() {
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
            TokenKind::Ident(_) => {
                // `(T*)expr` cast — do not treat `>>`/`>` as type syntax (that's `(N >> i)`).
                let mut i = self.pos + 1;
                let mut depth_lt = 0i32;
                while let Some(t) = self.tokens.get(i) {
                    match &t.kind {
                        TokenKind::Punct(Punct::Lt) => {
                            depth_lt += 1;
                            i += 1;
                        }
                        TokenKind::Punct(Punct::Gt | Punct::GtGt) if depth_lt > 0 => {
                            depth_lt -= if matches!(t.kind, TokenKind::Punct(Punct::GtGt)) {
                                2
                            } else {
                                1
                            };
                            if depth_lt < 0 {
                                depth_lt = 0;
                            }
                            i += 1;
                        }
                        TokenKind::Punct(Punct::Star | Punct::Amp | Punct::Scope | Punct::Comma)
                        | TokenKind::Ident(_)
                        | TokenKind::Keyword(_) => i += 1,
                        TokenKind::Punct(Punct::RParen) => {
                            return matches!(
                                self.tokens.get(i + 1).map(|t| &t.kind),
                                Some(
                                    TokenKind::Ident(_)
                                        | TokenKind::IntLit { .. }
                                        | TokenKind::FloatLit { .. }
                                        | TokenKind::CharLit(_)
                                        | TokenKind::StringLit(_)
                                        | TokenKind::Punct(
                                            Punct::LParen
                                                | Punct::Plus
                                                | Punct::Minus
                                                | Punct::Star
                                                | Punct::Amp
                                                | Punct::Not
                                        )
                                        | TokenKind::Keyword(
                                            Keyword::True | Keyword::False | Keyword::Nullptr
                                        )
                                )
                            );
                        }
                        _ => return false,
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub(super) fn parse_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        self.expect_punct(Punct::LParen)?;
        let mut args = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                // assignment-expr: stop before comma so `f(a, b)` stays two args
                args.push(self.parse_expr_bp(2)?);
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        let end = self.expect_punct(Punct::RParen)?.span.end;
        let span = Span::new(callee.span().start, end);
        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
            span,
        })
    }

    pub(super) fn parse_index(&mut self, base: Expr) -> Result<Expr, ParseError> {
        self.expect_punct(Punct::LBracket)?;
        let index = self.parse_expr_bp(2)?;
        let end = self.expect_punct(Punct::RBracket)?.span.end;
        let span = Span::new(base.span().start, end);
        Ok(Expr::Index {
            base: Box::new(base),
            index: Box::new(index),
            span,
        })
    }

    pub(super) fn parse_member_expr(&mut self, base: Expr) -> Result<Expr, ParseError> {
        let arrow = self.at_punct(Punct::Arrow);
        self.bump();
        let field = self.parse_ident()?;
        let span = Span::new(base.span().start, field.span.end);
        Ok(Expr::Member {
            base: Box::new(base),
            field,
            arrow,
            span,
        })
    }

    pub(super) fn parse_init_list(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_punct(Punct::LBrace)?.span.start;
        let mut elems = Vec::new();
        if !self.at_punct(Punct::RBrace) {
            loop {
                elems.push(self.parse_expr_bp(2)?);
                if self.at_punct(Punct::Comma) {
                    self.bump();
                    if self.at_punct(Punct::RBrace) {
                        break;
                    }
                    continue;
                }
                break;
            }
        }
        let end = self.expect_punct(Punct::RBrace)?.span.end;
        Ok(Expr::InitList {
            elems,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_new(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_keyword(Keyword::New)?.span.start;
        let ty = self.parse_type()?;
        let mut args = Vec::new();
        let mut end = ty.span().end;
        if self.at_punct(Punct::LParen) {
            self.bump();
            if !self.at_punct(Punct::RParen) {
                loop {
                    args.push(self.parse_expr_bp(2)?);
                    if self.at_punct(Punct::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            end = self.expect_punct(Punct::RParen)?.span.end;
        }
        Ok(Expr::New {
            ty,
            args,
            span: Span::new(start, end),
        })
    }

    pub(super) fn looks_like_functional_cast(&self) -> bool {
        let mut i = self.pos;
        let start = i;
        loop {
            match self.tokens.get(i).map(|t| &t.kind) {
                Some(TokenKind::Keyword(
                    Keyword::Void
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
                )) => i += 1,
                Some(TokenKind::Punct(Punct::LParen)) if i > start => return true,
                _ => return false,
            }
        }
    }

    pub(super) fn parse_functional_cast(&mut self) -> Result<Expr, ParseError> {
        let ty = self.parse_builtin_type()?;
        self.expect_punct(Punct::LParen)?;
        let expr = self.parse_expr_bp(2)?;
        let end = self.expect_punct(Punct::RParen)?.span.end;
        Ok(Expr::Cast {
            span: Span::new(ty.span().start, end),
            ty,
            expr: Box::new(expr),
        })
    }

    pub(super) fn parse_sizeof(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_keyword(Keyword::Sizeof)?.span.start;
        if self.at_punct(Punct::LParen) {
            self.bump();
            // `sizeof(int)` / `sizeof(int*)` vs `sizeof(x)`
            let is_type = match self.peek_kind() {
                TokenKind::Keyword(
                    Keyword::Void
                        | Keyword::Bool
                        | Keyword::Char
                        | Keyword::Short
                        | Keyword::Int
                        | Keyword::Long
                        | Keyword::Float
                        | Keyword::Double
                        | Keyword::Unsigned
                        | Keyword::Signed
                        | Keyword::Const
                        | Keyword::Auto
                        | Keyword::WcharT,
                ) => true,
                TokenKind::Ident(_) => matches!(
                    self.tokens.get(self.pos + 1).map(|t| &t.kind),
                    Some(TokenKind::Punct(
                        Punct::Star | Punct::Amp | Punct::Lt | Punct::Scope
                    ))
                ),
                _ => false,
            };
            if is_type {
                let ty = self.parse_type()?;
                let end = self.expect_punct(Punct::RParen)?.span.end;
                return Ok(Expr::Sizeof {
                    ty: Some(ty),
                    expr: None,
                    span: Span::new(start, end),
                });
            }
            let expr = self.parse_expr()?;
            let end = self.expect_punct(Punct::RParen)?.span.end;
            return Ok(Expr::Sizeof {
                ty: None,
                expr: Some(Box::new(expr)),
                span: Span::new(start, end),
            });
        }
        let expr = self.parse_expr_bp(prefix_bp())?;
        let end = expr.span().end;
        Ok(Expr::Sizeof {
            ty: None,
            expr: Some(Box::new(expr)),
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_delete(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_keyword(Keyword::Delete)?.span.start;
        let is_array = if self.at_punct(Punct::LBracket) {
            self.bump();
            self.expect_punct(Punct::RBracket)?;
            true
        } else {
            false
        };
        let expr = self.parse_expr_bp(prefix_bp())?;
        let end = expr.span().end;
        Ok(Expr::Delete {
            expr: Box::new(expr),
            is_array,
            span: Span::new(start, end),
        })
    }

    pub(super) fn infix_bp(&self) -> Option<(TokenKind, u8, u8)> {
        let kind = self.peek_kind().clone();
        let (l, r) = match &kind {
            TokenKind::Punct(Punct::Eq)
            | TokenKind::Punct(Punct::PlusEq)
            | TokenKind::Punct(Punct::MinusEq)
            | TokenKind::Punct(Punct::StarEq)
            | TokenKind::Punct(Punct::SlashEq)
            | TokenKind::Punct(Punct::PercentEq)
            | TokenKind::Punct(Punct::AmpEq)
            | TokenKind::Punct(Punct::PipeEq)
            | TokenKind::Punct(Punct::CaretEq)
            | TokenKind::Punct(Punct::LtLtEq)
            | TokenKind::Punct(Punct::GtGtEq) => (2u8, 1u8), // right-assoc
            TokenKind::Punct(Punct::PipePipe) => (3, 4),
            TokenKind::Punct(Punct::AmpAmp) => (5, 6),
            TokenKind::Punct(Punct::Pipe) => (7, 8),
            TokenKind::Punct(Punct::Caret) => (9, 10),
            TokenKind::Punct(Punct::Amp) => (11, 12),
            TokenKind::Punct(Punct::EqEq) | TokenKind::Punct(Punct::NotEq) => (13, 14),
            TokenKind::Punct(Punct::Lt)
            | TokenKind::Punct(Punct::Gt)
            | TokenKind::Punct(Punct::LtEq)
            | TokenKind::Punct(Punct::GtEq) => (15, 16),
            TokenKind::Punct(Punct::LtLt) | TokenKind::Punct(Punct::GtGt) => (17, 18),
            TokenKind::Punct(Punct::Plus) | TokenKind::Punct(Punct::Minus) => (19, 20),
            TokenKind::Punct(Punct::Star)
            | TokenKind::Punct(Punct::Slash)
            | TokenKind::Punct(Punct::Percent) => (21, 22),
            // Lowest; l_bp=0 so assign RHS (min_bp=1) stops before comma: `a=b, c` → `(a=b), c`
            TokenKind::Punct(Punct::Comma) => (0, 1),
            _ => return None,
        };
        Some((kind, l, r))
    }

    /// `name<Type, ...>` vs `a < b` comparison.
    pub(super) fn looks_like_template_args(&self) -> bool {
        self.looks_like_template_args_at(self.pos)
    }

    pub(super) fn parse_lambda(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_punct(Punct::LBracket)?.span.start;
        // Skip capture list (ignored for now).
        let mut depth = 1i32;
        while depth > 0 {
            if self.at_eof() {
                return Err(self.err("unterminated lambda capture list"));
            }
            match self.peek_kind() {
                TokenKind::Punct(Punct::LBracket) => {
                    depth += 1;
                    self.bump();
                }
                TokenKind::Punct(Punct::RBracket) => {
                    depth -= 1;
                    self.bump();
                }
                _ => {
                    self.bump();
                }
            }
        }
        let mut params = Vec::new();
        if self.at_punct(Punct::LParen) {
            self.bump();
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
        }
        // optional `mutable` / trailing return — skip `-> type`
        if self.at_keyword(Keyword::Mutable) {
            self.bump();
        }
        if self.at_punct(Punct::Arrow) {
            self.bump();
            let _ = self.parse_type()?;
        }
        let body = self.parse_block()?;
        let end = body.span.end;
        Ok(Expr::Lambda {
            params,
            body,
            span: Span::new(start, end),
        })
    }
}

pub(super) fn prefix_bp() -> u8 {
    23
}

pub(super) fn as_binary_op(kind: &TokenKind) -> Option<BinaryOp> {
    Some(match kind {
        TokenKind::Punct(Punct::Plus) => BinaryOp::Add,
        TokenKind::Punct(Punct::Minus) => BinaryOp::Sub,
        TokenKind::Punct(Punct::Star) => BinaryOp::Mul,
        TokenKind::Punct(Punct::Slash) => BinaryOp::Div,
        TokenKind::Punct(Punct::Percent) => BinaryOp::Rem,
        TokenKind::Punct(Punct::LtLt) => BinaryOp::Shl,
        TokenKind::Punct(Punct::GtGt) => BinaryOp::Shr,
        TokenKind::Punct(Punct::Lt) => BinaryOp::Lt,
        TokenKind::Punct(Punct::Gt) => BinaryOp::Gt,
        TokenKind::Punct(Punct::LtEq) => BinaryOp::Le,
        TokenKind::Punct(Punct::GtEq) => BinaryOp::Ge,
        TokenKind::Punct(Punct::EqEq) => BinaryOp::Eq,
        TokenKind::Punct(Punct::NotEq) => BinaryOp::Ne,
        TokenKind::Punct(Punct::Amp) => BinaryOp::BitAnd,
        TokenKind::Punct(Punct::Caret) => BinaryOp::BitXor,
        TokenKind::Punct(Punct::Pipe) => BinaryOp::BitOr,
        TokenKind::Punct(Punct::AmpAmp) => BinaryOp::And,
        TokenKind::Punct(Punct::PipePipe) => BinaryOp::Or,
        TokenKind::Punct(Punct::Comma) => BinaryOp::Comma,
        _ => return None,
    })
}

pub(super) fn as_assign_op(kind: &TokenKind) -> Option<AssignOp> {
    Some(match kind {
        TokenKind::Punct(Punct::Eq) => AssignOp::Assign,
        TokenKind::Punct(Punct::PlusEq) => AssignOp::AddAssign,
        TokenKind::Punct(Punct::MinusEq) => AssignOp::SubAssign,
        TokenKind::Punct(Punct::StarEq) => AssignOp::MulAssign,
        TokenKind::Punct(Punct::SlashEq) => AssignOp::DivAssign,
        TokenKind::Punct(Punct::PercentEq) => AssignOp::RemAssign,
        TokenKind::Punct(Punct::AmpEq) => AssignOp::AndAssign,
        TokenKind::Punct(Punct::PipeEq) => AssignOp::OrAssign,
        TokenKind::Punct(Punct::CaretEq) => AssignOp::XorAssign,
        TokenKind::Punct(Punct::LtLtEq) => AssignOp::ShlAssign,
        TokenKind::Punct(Punct::GtGtEq) => AssignOp::ShrAssign,
        _ => return None,
    })
}

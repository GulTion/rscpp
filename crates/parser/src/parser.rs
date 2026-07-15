//! Recursive-descent + Pratt parser.

use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{tokenize, Keyword, Punct, Token, TokenKind};

pub fn parse(source: &str) -> Result<TranslationUnit, ParseError> {
    let tokens = tokenize(source)?;
    Parser::new(tokens).parse_translation_unit()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Extra `>` closes introduced by splitting `>>` in template args.
    pending_gt: u32,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            pending_gt: 0,
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        if self.pending_gt > 0 {
            return &TokenKind::Punct(Punct::Gt);
        }
        self.tokens
            .get(self.pos)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn peek_span(&self) -> Span {
        if self.pending_gt > 0 {
            // Approximate: use current token span if available
            return self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span)
                .unwrap_or(Span::new(0, 0));
        }
        self.tokens
            .get(self.pos)
            .map(|t| t.span)
            .unwrap_or_else(|| {
                self.tokens
                    .last()
                    .map(|t| t.span)
                    .unwrap_or(Span::new(0, 0))
            })
    }

    fn bump(&mut self) -> Token {
        if self.pending_gt > 0 {
            self.pending_gt -= 1;
            let span = self.peek_span();
            return Token::new(TokenKind::Punct(Punct::Gt), span);
        }
        let tok = self
            .tokens
            .get(self.pos)
            .cloned()
            .unwrap_or_else(|| Token::new(TokenKind::Eof, Span::new(0, 0)));
        if !matches!(tok.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        tok
    }

    fn bump_template_gt(&mut self) -> Result<(), ParseError> {
        match self.peek_kind() {
            TokenKind::Punct(Punct::Gt) => {
                self.bump();
                Ok(())
            }
            TokenKind::Punct(Punct::GtGt) => {
                // Consume `>>` as one `>`; leave one pending.
                self.bump();
                self.pending_gt += 1;
                Ok(())
            }
            _ => Err(self.err("expected `>`")),
        }
    }

    fn err(&self, msg: impl Into<String>) -> ParseError {
        ParseError::new(self.peek_span(), msg)
    }

    fn expect_punct(&mut self, p: Punct) -> Result<Token, ParseError> {
        match self.peek_kind() {
            TokenKind::Punct(q) if *q == p => Ok(self.bump()),
            _ => Err(self.err(format!("expected `{p:?}`"))),
        }
    }

    fn expect_keyword(&mut self, kw: Keyword) -> Result<Token, ParseError> {
        match self.peek_kind() {
            TokenKind::Keyword(k) if *k == kw => Ok(self.bump()),
            _ => Err(self.err(format!("expected keyword `{kw:?}`"))),
        }
    }

    fn at_punct(&self, p: Punct) -> bool {
        matches!(self.peek_kind(), TokenKind::Punct(q) if *q == p)
    }

    fn at_keyword(&self, kw: Keyword) -> bool {
        matches!(self.peek_kind(), TokenKind::Keyword(k) if *k == kw)
    }

    fn at_eof(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof) && self.pending_gt == 0
    }

    fn parse_translation_unit(&mut self) -> Result<TranslationUnit, ParseError> {
        let start = self.peek_span().start;
        let mut items = Vec::new();
        while !self.at_eof() {
            // Skip stray `#` lines lightly — not a full preprocessor.
            if self.at_punct(Punct::Hash) {
                return Err(self.err(
                    "preprocessor directives are not supported yet (no `#include` / macros)",
                ));
            }
            items.push(self.parse_item()?);
        }
        let end = self.peek_span().end;
        Ok(TranslationUnit {
            items,
            span: Span::new(start, end),
        })
    }

    fn parse_item(&mut self) -> Result<Item, ParseError> {
        if self.at_keyword(Keyword::Class) || self.at_keyword(Keyword::Struct) {
            return Ok(Item::Class(self.parse_class()?));
        }
        if self.at_keyword(Keyword::Using) {
            return self.parse_using_namespace();
        }
        // Function or declaration
        let start = self.peek_span().start;
        let ty = self.parse_type()?;
        let name = self.parse_ident()?;
        if self.at_punct(Punct::LParen) {
            let func = self.parse_function_rest(start, ty, name)?;
            return Ok(Item::Function(func));
        }
        let decl = self.parse_decl_rest(start, ty, name)?;
        Ok(Item::Decl(decl))
    }

    fn parse_using_namespace(&mut self) -> Result<Item, ParseError> {
        let start = self.expect_keyword(Keyword::Using)?.span.start;
        self.expect_keyword(Keyword::Namespace)?;
        let path = self.parse_path()?;
        let end = self.expect_punct(Punct::Semi)?.span.end;
        Ok(Item::UsingNamespace {
            path,
            span: Span::new(start, end),
        })
    }

    fn parse_class(&mut self) -> Result<ClassDef, ParseError> {
        let start = self.peek_span().start;
        let kind = if self.at_keyword(Keyword::Class) {
            self.bump();
            ClassKind::Class
        } else {
            self.expect_keyword(Keyword::Struct)?;
            ClassKind::Struct
        };
        let name = self.parse_ident()?;
        self.expect_punct(Punct::LBrace)?;
        let mut members = Vec::new();
        while !self.at_punct(Punct::RBrace) && !self.at_eof() {
            members.push(self.parse_member()?);
        }
        let end = self.expect_punct(Punct::RBrace)?.span.end;
        if self.at_punct(Punct::Semi) {
            self.bump();
        }
        Ok(ClassDef {
            kind,
            name,
            members,
            span: Span::new(start, end),
        })
    }

    fn parse_member(&mut self) -> Result<Member, ParseError> {
        if self.at_keyword(Keyword::Public)
            || self.at_keyword(Keyword::Private)
            || self.at_keyword(Keyword::Protected)
        {
            let kw = match self.bump().kind {
                TokenKind::Keyword(Keyword::Public) => AccessSpec::Public,
                TokenKind::Keyword(Keyword::Private) => AccessSpec::Private,
                TokenKind::Keyword(Keyword::Protected) => AccessSpec::Protected,
                _ => unreachable!(),
            };
            self.expect_punct(Punct::Colon)?;
            return Ok(Member::Access(kw));
        }
        let start = self.peek_span().start;
        let ty = self.parse_type()?;
        let name = self.parse_ident()?;
        if self.at_punct(Punct::LParen) {
            Ok(Member::Function(self.parse_function_rest(start, ty, name)?))
        } else {
            Ok(Member::Field(self.parse_decl_rest(start, ty, name)?))
        }
    }

    fn parse_function_rest(
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
        let body = self.parse_block()?;
        let end = body.span.end;
        Ok(FunctionDef {
            return_type,
            name,
            params,
            body,
            span: Span::new(start, end),
        })
    }

    fn parse_param(&mut self) -> Result<Param, ParseError> {
        let start = self.peek_span().start;
        let mut ty = self.parse_type()?;
        // Optional ptr/ref after type for params like `int& nums`
        ty = self.apply_ptr_suffixes(ty)?;
        let name = if matches!(self.peek_kind(), TokenKind::Ident(_)) {
            Some(self.parse_ident()?)
        } else {
            None
        };
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

    fn parse_decl_rest(
        &mut self,
        start: usize,
        ty: Type,
        first_name: Ident,
    ) -> Result<Decl, ParseError> {
        let mut declarators = Vec::new();
        declarators.push(self.parse_init_declarator_after_name(first_name)?);
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
            let mut d = self.parse_init_declarator_after_name(name)?;
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

    fn parse_init_declarator_after_name(
        &mut self,
        name: Ident,
    ) -> Result<InitDeclarator, ParseError> {
        let start = name.span.start;
        let init = if self.at_punct(Punct::Eq) {
            self.bump();
            Some(self.parse_expr()?)
        } else if self.at_punct(Punct::LBrace) {
            Some(self.parse_init_list()?)
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

    fn apply_ptr_suffixes(&mut self, mut ty: Type) -> Result<Type, ParseError> {
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

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let start = self.peek_span().start;
        let mut is_const = false;
        if self.at_keyword(Keyword::Const) {
            self.bump();
            is_const = true;
        }

        let mut ty = self.parse_type_primary()?;

        // trailing const: `int const`
        if self.at_keyword(Keyword::Const) {
            self.bump();
            is_const = true;
        }

        ty = self.apply_ptr_suffixes(ty)?;

        if is_const {
            let span = Span::new(start, ty.span().end);
            ty = Type::Const {
                inner: Box::new(ty),
                span,
            };
        }
        Ok(ty)
    }

    fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
        // unsigned / signed / long combinations — LeetCode-simple
        if self.at_keyword(Keyword::Unsigned)
            || self.at_keyword(Keyword::Signed)
            || self.at_keyword(Keyword::Long)
            || self.at_keyword(Keyword::Short)
            || self.at_keyword(Keyword::Int)
            || self.at_keyword(Keyword::Char)
            || self.at_keyword(Keyword::Void)
            || self.at_keyword(Keyword::Bool)
            || self.at_keyword(Keyword::Float)
            || self.at_keyword(Keyword::Double)
            || self.at_keyword(Keyword::Auto)
            || self.at_keyword(Keyword::WcharT)
        {
            return self.parse_builtin_type();
        }

        let path = self.parse_path()?;
        let start = path.span.start;
        let mut args = Vec::new();
        if self.at_punct(Punct::Lt) {
            self.bump();
            if !self.at_punct(Punct::Gt) && !self.at_punct(Punct::GtGt) {
                loop {
                    args.push(self.parse_type()?);
                    if self.at_punct(Punct::Comma) {
                        self.bump();
                        continue;
                    }
                    break;
                }
            }
            self.bump_template_gt()?;
        }
        let end = if args.is_empty() {
            path.span.end
        } else {
            self.tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.end)
                .unwrap_or(path.span.end)
        };
        Ok(Type::Named {
            path,
            args,
            span: Span::new(start, end),
        })
    }

    fn parse_builtin_type(&mut self) -> Result<Type, ParseError> {
        let start = self.peek_span().start;
        let mut unsigned = false;
        let mut signed = false;
        let mut long_count = 0u8;
        let mut short = false;
        let mut core: Option<BuiltinType> = None;

        loop {
            match self.peek_kind() {
                TokenKind::Keyword(Keyword::Unsigned) => {
                    self.bump();
                    unsigned = true;
                }
                TokenKind::Keyword(Keyword::Signed) => {
                    self.bump();
                    signed = true;
                }
                TokenKind::Keyword(Keyword::Long) => {
                    self.bump();
                    long_count += 1;
                }
                TokenKind::Keyword(Keyword::Short) => {
                    self.bump();
                    short = true;
                }
                TokenKind::Keyword(Keyword::Int) => {
                    self.bump();
                    core = Some(BuiltinType::Int);
                    break;
                }
                TokenKind::Keyword(Keyword::Char) => {
                    self.bump();
                    core = Some(BuiltinType::Char);
                    break;
                }
                TokenKind::Keyword(Keyword::Void) => {
                    self.bump();
                    core = Some(BuiltinType::Void);
                    break;
                }
                TokenKind::Keyword(Keyword::Bool) => {
                    self.bump();
                    core = Some(BuiltinType::Bool);
                    break;
                }
                TokenKind::Keyword(Keyword::Float) => {
                    self.bump();
                    core = Some(BuiltinType::Float);
                    break;
                }
                TokenKind::Keyword(Keyword::Double) => {
                    self.bump();
                    core = Some(BuiltinType::Double);
                    break;
                }
                TokenKind::Keyword(Keyword::Auto) => {
                    self.bump();
                    core = Some(BuiltinType::Auto);
                    break;
                }
                TokenKind::Keyword(Keyword::WcharT) => {
                    self.bump();
                    core = Some(BuiltinType::WcharT);
                    break;
                }
                _ => break,
            }
        }

        let _ = signed; // accepted but ignored for kind selection except char
        let kind = match (unsigned, short, long_count, core) {
            (_, _, _, Some(BuiltinType::Void)) => BuiltinType::Void,
            (_, _, _, Some(BuiltinType::Bool)) => BuiltinType::Bool,
            (_, _, _, Some(BuiltinType::Float)) => BuiltinType::Float,
            (_, _, _, Some(BuiltinType::Double)) => BuiltinType::Double,
            (_, _, _, Some(BuiltinType::Auto)) => BuiltinType::Auto,
            (_, _, _, Some(BuiltinType::WcharT)) => BuiltinType::WcharT,
            (true, false, 0, Some(BuiltinType::Char)) => BuiltinType::UnsignedChar,
            (false, false, 0, Some(BuiltinType::Char)) => BuiltinType::Char,
            (true, true, _, _) => BuiltinType::UnsignedShort,
            (false, true, _, _) => BuiltinType::Short,
            (true, false, 0, _) => BuiltinType::UnsignedInt,
            (true, false, 1, _) => BuiltinType::UnsignedLong,
            (true, false, 2, _) => BuiltinType::UnsignedLongLong,
            (false, false, 1, _) => BuiltinType::Long,
            (false, false, 2, _) => BuiltinType::LongLong,
            (false, false, 0, Some(BuiltinType::Int) | None) => BuiltinType::Int,
            _ => BuiltinType::Int,
        };

        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(start);
        Ok(Type::Builtin {
            kind,
            span: Span::new(start, end),
        })
    }

    fn parse_block(&mut self) -> Result<Block, ParseError> {
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

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
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

    fn at_declaration_start(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Keyword(
                Keyword::Const
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

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
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

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
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

    fn parse_do_while(&mut self) -> Result<Stmt, ParseError> {
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

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect_keyword(Keyword::For)?.span.start;
        self.expect_punct(Punct::LParen)?;
        let init = if self.at_punct(Punct::Semi) {
            None
        } else if self.at_declaration_start() {
            let dstart = self.peek_span().start;
            let ty = self.parse_type()?;
            let name = self.parse_ident()?;
            Some(ForInit::Decl(self.parse_decl_rest(dstart, ty, name)?))
        } else {
            let e = self.parse_expr()?;
            self.expect_punct(Punct::Semi)?;
            Some(ForInit::Expr(e))
        };
        // decl_rest already consumed `;`; expr path also did.
        if matches!(init, Some(ForInit::Decl(_))) {
            // semi already eaten
        } else if init.is_none() {
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

    fn parse_ident(&mut self) -> Result<Ident, ParseError> {
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

    fn parse_path(&mut self) -> Result<Path, ParseError> {
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

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_prefix()?;

        loop {
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

        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        if self.at_punct(Punct::LBrace) {
            return self.parse_init_list();
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
            TokenKind::Ident(_) => Ok(Expr::Name(self.parse_path()?)),
            _ => Err(self.err(format!(
                "expected expression, found {:?}",
                self.peek_kind()
            ))),
        }
    }

    fn looks_like_cast(&self) -> bool {
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
                // `(foo)` could be grouping — only treat as cast if followed by type-ish then `)` then primary
                // Heuristic: Ident / * / & then `)` then another primary token
                let mut i = self.pos + 1;
                while let Some(t) = self.tokens.get(i) {
                    match &t.kind {
                        TokenKind::Punct(Punct::Star | Punct::Amp | Punct::Lt | Punct::Gt | Punct::GtGt | Punct::Scope | Punct::Comma)
                        | TokenKind::Ident(_)
                        | TokenKind::Keyword(_) => {
                            i += 1;
                            continue;
                        }
                        TokenKind::Punct(Punct::RParen) => {
                            // next after ) should look like expression start
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

    fn parse_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        self.expect_punct(Punct::LParen)?;
        let mut args = Vec::new();
        if !self.at_punct(Punct::RParen) {
            loop {
                args.push(self.parse_expr()?);
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

    fn parse_index(&mut self, base: Expr) -> Result<Expr, ParseError> {
        self.expect_punct(Punct::LBracket)?;
        let index = self.parse_expr()?;
        let end = self.expect_punct(Punct::RBracket)?.span.end;
        let span = Span::new(base.span().start, end);
        Ok(Expr::Index {
            base: Box::new(base),
            index: Box::new(index),
            span,
        })
    }

    fn parse_member_expr(&mut self, base: Expr) -> Result<Expr, ParseError> {
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

    fn parse_init_list(&mut self) -> Result<Expr, ParseError> {
        let start = self.expect_punct(Punct::LBrace)?.span.start;
        let mut elems = Vec::new();
        if !self.at_punct(Punct::RBrace) {
            loop {
                elems.push(self.parse_expr()?);
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

    fn infix_bp(&self) -> Option<(TokenKind, u8, u8)> {
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
            _ => return None,
        };
        Some((kind, l, r))
    }
}

fn prefix_bp() -> u8 {
    23
}

fn as_binary_op(kind: &TokenKind) -> Option<BinaryOp> {
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
        _ => return None,
    })
}

fn as_assign_op(kind: &TokenKind) -> Option<AssignOp> {
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

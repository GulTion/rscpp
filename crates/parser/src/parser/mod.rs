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

    pub(super) fn skip_decl_specs(&mut self) {
        while matches!(
            self.peek_kind(),
            TokenKind::Keyword(
                Keyword::Static
                    | Keyword::Constexpr
                    | Keyword::Inline
                    | Keyword::Mutable
                    | Keyword::Explicit
                    | Keyword::Virtual
                    | Keyword::Friend
            )
        ) {
            self.bump();
        }
    }

    pub(super) fn skip_balanced(&mut self, open: Punct, close: Punct) {
        if !self.at_punct(open) {
            return;
        }
        self.bump();
        let mut depth = 1i32;
        while depth > 0 && !self.at_eof() {
            if self.at_punct(open) {
                depth += 1;
            } else if self.at_punct(close) {
                depth -= 1;
            }
            self.bump();
        }
    }

    fn parse_translation_unit(&mut self) -> Result<TranslationUnit, ParseError> {
        let start = self.peek_span().start;
        let mut items = Vec::new();
        while !self.at_eof() {
            if self.at_punct(Punct::Hash) {
                self.skip_preprocessor()?;
                continue;
            }
            items.push(self.parse_item()?);
        }
        let end = self.peek_span().end;
        Ok(TranslationUnit {
            items,
            span: Span::new(start, end),
        })
    }

    /// Strip LeetCode-style `#include` / `#pragma once`. Real macros still error.
    fn skip_preprocessor(&mut self) -> Result<(), ParseError> {
        self.expect_punct(Punct::Hash)?;
        let dir = match self.peek_kind() {
            TokenKind::Ident(name) => name.clone(),
            _ => return Err(self.err("expected preprocessor directive name")),
        };
        self.bump();
        match dir.as_str() {
            "include" => {
                if self.at_punct(Punct::Lt) {
                    self.bump();
                    while !self.at_eof() && !self.at_punct(Punct::Gt) {
                        self.bump();
                    }
                    self.expect_punct(Punct::Gt)?;
                } else if matches!(self.peek_kind(), TokenKind::StringLit(_)) {
                    self.bump();
                } else {
                    return Err(self.err("malformed #include"));
                }
                Ok(())
            }
            "pragma" => {
                // `#pragma once` (and ignore other pragmas' tokens until next item — just `once`)
                if matches!(self.peek_kind(), TokenKind::Ident(n) if n == "once") {
                    self.bump();
                }
                Ok(())
            }
            _ => Err(self.err(format!(
                "unsupported preprocessor directive `#{dir}` (only #include / #pragma once)"
            ))),
        }
    }
}

mod item;
mod func;
mod ty;
mod stmt;
mod expr;

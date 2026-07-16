//! Hand-written C++ scanner for the LeetCode subset.

use crate::error::LexError;
use crate::span::Span;
use crate::token::{
    FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind,
};

/// Tokenize `source` into a vector ending with [`TokenKind::Eof`].
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(source);
    let mut tokens = Vec::new();
    loop {
        let tok = scanner.next_token()?;
        let is_eof = matches!(tok.kind, TokenKind::Eof);
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    Ok(tokens)
}

struct Scanner<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Scanner<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            src: source.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn span_from(&self, start: usize) -> Span {
        Span::new(start, self.pos)
    }

    fn err_at(&self, start: usize, message: impl Into<String>) -> LexError {
        LexError::new(self.span_from(start), message)
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_trivia()?;

        let start = self.pos;
        let Some(c) = self.peek() else {
            return Ok(Token::new(TokenKind::Eof, Span::new(start, start)));
        };

        // Reject wide / UTF / raw string prefixes early when followed by quote.
        if matches!(c, b'L' | b'u' | b'U' | b'R') {
            if let Some(err) = self.check_unsupported_string_prefix(start)? {
                return Err(err);
            }
        }

        match c {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident_or_keyword(start),
            b'0'..=b'9' => self.lex_number(start),
            b'.' if self.peek_at(1).is_some_and(|d| d.is_ascii_digit()) => {
                self.lex_number(start)
            }
            b'\'' => self.lex_char(start),
            b'"' => self.lex_string(start),
            _ => self.lex_punct(start),
        }
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b') => {
                    self.bump();
                }
                Some(b'/') if self.peek_at(1) == Some(b'/') => {
                    self.bump();
                    self.bump();
                    while let Some(c) = self.peek() {
                        self.bump();
                        if c == b'\n' {
                            break;
                        }
                    }
                }
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    let start = self.pos;
                    self.bump();
                    self.bump();
                    loop {
                        match self.bump() {
                            Some(b'*') if self.peek() == Some(b'/') => {
                                self.bump();
                                break;
                            }
                            Some(_) => {}
                            None => {
                                return Err(self.err_at(start, "unterminated block comment"));
                            }
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Returns `Ok(Some(err))` if an unsupported literal prefix is present.
    fn check_unsupported_string_prefix(
        &self,
        start: usize,
    ) -> Result<Option<LexError>, LexError> {
        let rest = &self.src[self.pos..];
        let unsupported = |msg: &str| -> Option<LexError> {
            Some(LexError::new(Span::new(start, start + 1), msg))
        };

        if rest.starts_with(b"u8\"") || rest.starts_with(b"u8\'") {
            return Ok(unsupported("UTF-8 character/string literals (u8) are not supported yet"));
        }
        if rest.starts_with(b"u\"") || rest.starts_with(b"u\'") {
            return Ok(unsupported("UTF-16 character/string literals (u) are not supported yet"));
        }
        if rest.starts_with(b"U\"") || rest.starts_with(b"U\'") {
            return Ok(unsupported("UTF-32 character/string literals (U) are not supported yet"));
        }
        if rest.starts_with(b"L\"") || rest.starts_with(b"L\'") {
            return Ok(unsupported("wide character/string literals (L) are not supported yet"));
        }
        // Raw string: R", LR", u8R", uR", UR"
        if rest.starts_with(b"R\"")
            || rest.starts_with(b"LR\"")
            || rest.starts_with(b"u8R\"")
            || rest.starts_with(b"uR\"")
            || rest.starts_with(b"UR\"")
        {
            return Ok(unsupported("raw string literals are not supported yet"));
        }
        Ok(None)
    }
}

mod ident;
mod number;
mod literal;
mod punct;
mod digits;

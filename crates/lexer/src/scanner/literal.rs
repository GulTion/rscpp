use super::digits::hex_val;
use super::Scanner;
use crate::error::LexError;
use crate::span::Span;
use crate::token::{FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind};

impl<'a> Scanner<'a> {

    pub(super) fn lex_char(&mut self, start: usize) -> Result<Token, LexError> {
        self.bump(); // '
        let ch = match self.peek() {
            Some(b'\'') => {
                return Err(self.err_at(start, "empty character literal"));
            }
            Some(b'\\') => {
                self.bump();
                self.lex_escape(start)?
            }
            Some(b'\n') | None => {
                return Err(self.err_at(start, "unterminated character literal"));
            }
            Some(_) => {
                let b = self.bump().unwrap();
                b as char
            }
        };
        if self.bump() != Some(b'\'') {
            return Err(self.err_at(start, "unterminated character literal"));
        }
        Ok(Token::new(TokenKind::CharLit(ch), self.span_from(start)))
    }

    pub(super) fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
        self.bump(); // "
        let mut out = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    return Err(self.err_at(start, "unterminated string literal"));
                }
                Some(b'"') => {
                    self.bump();
                    break;
                }
                Some(b'\\') => {
                    self.bump();
                    out.push(self.lex_escape(start)?);
                }
                Some(c) => {
                    self.bump();
                    out.push(c as char);
                }
            }
        }
        Ok(Token::new(TokenKind::StringLit(out), self.span_from(start)))
    }

    fn lex_escape(&mut self, start: usize) -> Result<char, LexError> {
        match self.bump() {
            Some(b'n') => Ok('\n'),
            Some(b't') => Ok('\t'),
            Some(b'r') => Ok('\r'),
            Some(b'\\') => Ok('\\'),
            Some(b'\'') => Ok('\''),
            Some(b'"') => Ok('"'),
            Some(b'0') => Ok('\0'),
            Some(b'a') => Ok('\x07'),
            Some(b'b') => Ok('\x08'),
            Some(b'f') => Ok('\x0c'),
            Some(b'v') => Ok('\x0b'),
            Some(b'x') => {
                let mut val: u32 = 0;
                let mut count = 0;
                while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                    let d = self.bump().unwrap();
                    val = val * 16 + hex_val(d) as u32;
                    count += 1;
                    if count >= 2 {
                        // consume remaining hex digits as C++ allows variable length,
                        // but truncate to char-ish for simplicity: take more into value
                        // ponytail: limit to byte for now
                    }
                    if count > 8 {
                        return Err(self.err_at(start, "hex escape too long"));
                    }
                }
                if count == 0 {
                    return Err(self.err_at(start, "hex escape has no digits"));
                }
                char::from_u32(val)
                    .ok_or_else(|| self.err_at(start, "invalid hex escape value"))
            }
            Some(d @ b'1'..=b'7') => {
                let mut val = (d - b'0') as u32;
                for _ in 0..2 {
                    if let Some(n @ b'0'..=b'7') = self.peek() {
                        self.bump();
                        val = val * 8 + (n - b'0') as u32;
                    } else {
                        break;
                    }
                }
                char::from_u32(val)
                    .ok_or_else(|| self.err_at(start, "invalid octal escape"))
            }
            Some(_) => Err(self.err_at(start, "unknown escape sequence")),
            None => Err(self.err_at(start, "unterminated escape sequence")),
        }
    }
}

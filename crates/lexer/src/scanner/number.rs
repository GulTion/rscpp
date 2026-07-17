use super::digits::{
    parse_binary_digits, parse_decimal_digits, parse_hex_digits, parse_octal_digits,
};
use super::Scanner;
use crate::error::LexError;
use crate::span::Span;
use crate::token::{FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind};

impl<'a> Scanner<'a> {
    pub(super) fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
        // Float starting with '.'
        if self.peek() == Some(b'.') {
            return self.lex_float_after_dot(start, false);
        }

        let first = self.bump().unwrap();

        if first == b'0' {
            match self.peek() {
                Some(b'x' | b'X') => {
                    self.bump();
                    return self.lex_hex(start);
                }
                Some(b'b' | b'B') => {
                    self.bump();
                    return self.lex_binary(start);
                }
                Some(b'0'..=b'7') => return self.lex_octal(start),
                Some(b'.') | Some(b'e' | b'E') => {
                    // fall through to decimal/float from "0"
                }
                Some(b'8' | b'9') => {
                    // Invalid octal → treat as decimal for friendlier errors later;
                    // C++ makes 08 ill-formed. Reject.
                    return Err(self.err_at(start, "invalid octal integer literal"));
                }
                _ => {
                    let suffix = self.lex_int_suffix();
                    return Ok(Token::new(
                        TokenKind::IntLit {
                            value: 0,
                            suffix,
                            base: IntBase::Octal,
                        },
                        self.span_from(start),
                    ));
                }
            }
        }

        // Decimal integer or float
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return Err(self.err_at(start, "digit separator must be followed by a digit"));
                }
            } else {
                self.bump();
            }
        }

        match self.peek() {
            Some(b'.') => self.lex_float_continue(start),
            Some(b'e' | b'E') => self.lex_float_exponent(start),
            _ => {
                let text = &self.src[start..self.pos];
                let value = parse_decimal_digits(text).map_err(|m| self.err_at(start, m))?;
                let suffix = self.lex_int_suffix();
                Ok(Token::new(
                    TokenKind::IntLit {
                        value,
                        suffix,
                        base: IntBase::Decimal,
                    },
                    self.span_from(start),
                ))
            }
        }
    }

    fn lex_hex(&mut self, start: usize) -> Result<Token, LexError> {
        let digits_start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_hexdigit() || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                    return Err(
                        self.err_at(start, "digit separator must be followed by a hex digit")
                    );
                }
            } else {
                self.bump();
            }
        }
        if self.pos == digits_start {
            return Err(self.err_at(start, "hex literal has no digits"));
        }
        let text = &self.src[digits_start..self.pos];
        let value = parse_hex_digits(text).map_err(|m| self.err_at(start, m))?;
        let suffix = self.lex_int_suffix();
        Ok(Token::new(
            TokenKind::IntLit {
                value,
                suffix,
                base: IntBase::Hex,
            },
            self.span_from(start),
        ))
    }

    fn lex_binary(&mut self, start: usize) -> Result<Token, LexError> {
        let digits_start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c == b'0' || c == b'1' || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !matches!(self.peek(), Some(b'0' | b'1')) {
                    return Err(
                        self.err_at(start, "digit separator must be followed by a binary digit")
                    );
                }
            } else {
                self.bump();
            }
        }
        if self.pos == digits_start {
            return Err(self.err_at(start, "binary literal has no digits"));
        }
        let text = &self.src[digits_start..self.pos];
        let value = parse_binary_digits(text).map_err(|m| self.err_at(start, m))?;
        let suffix = self.lex_int_suffix();
        Ok(Token::new(
            TokenKind::IntLit {
                value,
                suffix,
                base: IntBase::Binary,
            },
            self.span_from(start),
        ))
    }

    fn lex_octal(&mut self, start: usize) -> Result<Token, LexError> {
        // Leading 0 already consumed; continue octal digits
        while self
            .peek()
            .is_some_and(|c| (b'0'..=b'7').contains(&c) || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| (b'0'..=b'7').contains(&c)) {
                    return Err(
                        self.err_at(start, "digit separator must be followed by an octal digit")
                    );
                }
            } else {
                self.bump();
            }
        }
        if self.peek().is_some_and(|c| c == b'8' || c == b'9') {
            return Err(self.err_at(start, "invalid octal integer literal"));
        }
        // Could still be float: 0123. or 0123e
        if matches!(self.peek(), Some(b'.') | Some(b'e' | b'E')) {
            // Re-parse from start as decimal/float is wrong for C++;
            // octal floats don't exist. `.` after octal digits is float if we
            // treat whole thing as decimal — C++ allows 01.5 as float.
            return self.lex_float_continue(start);
        }
        let text = &self.src[start..self.pos];
        let value = parse_octal_digits(text).map_err(|m| self.err_at(start, m))?;
        let suffix = self.lex_int_suffix();
        Ok(Token::new(
            TokenKind::IntLit {
                value,
                suffix,
                base: IntBase::Octal,
            },
            self.span_from(start),
        ))
    }

    fn lex_float_after_dot(
        &mut self,
        start: usize,
        _had_digits_before: bool,
    ) -> Result<Token, LexError> {
        self.bump(); // '.'
        if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Err(self.err_at(start, "expected digit after '.' in floating literal"));
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return Err(self.err_at(start, "digit separator must be followed by a digit"));
                }
            } else {
                self.bump();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            return self.lex_float_exponent(start);
        }
        let value = self.parse_float_slice(start)?;
        let suffix = self.lex_float_suffix();
        Ok(Token::new(
            TokenKind::FloatLit { value, suffix },
            self.span_from(start),
        ))
    }

    fn lex_float_continue(&mut self, start: usize) -> Result<Token, LexError> {
        if self.peek() == Some(b'.') {
            self.bump();
            while self
                .peek()
                .is_some_and(|c| c.is_ascii_digit() || c == b'\'')
            {
                if self.peek() == Some(b'\'') {
                    self.bump();
                    if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        return Err(
                            self.err_at(start, "digit separator must be followed by a digit")
                        );
                    }
                } else {
                    self.bump();
                }
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            return self.lex_float_exponent(start);
        }
        let value = self.parse_float_slice(start)?;
        let suffix = self.lex_float_suffix();
        Ok(Token::new(
            TokenKind::FloatLit { value, suffix },
            self.span_from(start),
        ))
    }

    fn lex_float_exponent(&mut self, start: usize) -> Result<Token, LexError> {
        self.bump(); // e/E
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.bump();
        }
        if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Err(self.err_at(start, "exponent has no digits"));
        }
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_digit() || c == b'\'')
        {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return Err(self.err_at(start, "digit separator must be followed by a digit"));
                }
            } else {
                self.bump();
            }
        }
        let value = self.parse_float_slice(start)?;
        let suffix = self.lex_float_suffix();
        Ok(Token::new(
            TokenKind::FloatLit { value, suffix },
            self.span_from(start),
        ))
    }

    fn parse_float_slice(&self, start: usize) -> Result<f64, LexError> {
        let raw = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err_at(start, "float literal is not valid UTF-8"))?;
        let cleaned: String = raw.chars().filter(|&c| c != '\'').collect();
        cleaned
            .parse::<f64>()
            .map_err(|_| self.err_at(start, format!("invalid floating literal `{raw}`")))
    }

    fn lex_int_suffix(&mut self) -> IntSuffix {
        let s = self.pos;
        let mut has_u = false;
        let mut l_count = 0u8;

        // Order-independent enough for common forms: u, l, ul, lu, ll, ull, llu
        loop {
            match self.peek() {
                Some(b'u' | b'U') if !has_u => {
                    self.bump();
                    has_u = true;
                }
                Some(b'l' | b'L') if l_count < 2 => {
                    let c = self.bump().unwrap();
                    if l_count == 1 {
                        // must match case of previous L
                        let prev = self.src[self.pos - 2];
                        if (prev == b'l' && c == b'L') || (prev == b'L' && c == b'l') {
                            // mixed ll is ill-formed in C++; treat as end
                            self.pos -= 1;
                            break;
                        }
                    }
                    l_count += 1;
                }
                _ => break,
            }
        }

        // If we consumed something that looks like a float suffix alone, leave it —
        // int suffixes don't include f. Restore if we only saw junk? Already handled.
        let _ = s;
        match (has_u, l_count) {
            (false, 0) => IntSuffix::None,
            (true, 0) => IntSuffix::U,
            (false, 1) => IntSuffix::L,
            (true, 1) => IntSuffix::Ul,
            (false, 2) => IntSuffix::Ll,
            (true, 2) => IntSuffix::Ull,
            _ => IntSuffix::None,
        }
    }

    fn lex_float_suffix(&mut self) -> FloatSuffix {
        match self.peek() {
            Some(b'f' | b'F') => {
                self.bump();
                FloatSuffix::F
            }
            Some(b'l' | b'L') => {
                self.bump();
                FloatSuffix::L
            }
            _ => FloatSuffix::None,
        }
    }
}

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

    fn lex_ident_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            self.bump();
        }
        let text = std::str::from_utf8(&self.src[start..self.pos])
            .map_err(|_| self.err_at(start, "identifier is not valid UTF-8"))?;
        let kind = match Keyword::from_str(text) {
            Some(kw) => TokenKind::Keyword(kw),
            None => TokenKind::Ident(text.to_string()),
        };
        Ok(Token::new(kind, self.span_from(start)))
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, LexError> {
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
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'\'') {
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
                    return Err(self.err_at(start, "digit separator must be followed by a hex digit"));
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
        while self.peek().is_some_and(|c| c == b'0' || c == b'1' || c == b'\'') {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !matches!(self.peek(), Some(b'0' | b'1')) {
                    return Err(self.err_at(start, "digit separator must be followed by a binary digit"));
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
        while self.peek().is_some_and(|c| (b'0'..=b'7').contains(&c) || c == b'\'') {
            if self.peek() == Some(b'\'') {
                self.bump();
                if !self.peek().is_some_and(|c| (b'0'..=b'7').contains(&c)) {
                    return Err(self.err_at(start, "digit separator must be followed by an octal digit"));
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

    fn lex_float_after_dot(&mut self, start: usize, _had_digits_before: bool) -> Result<Token, LexError> {
        self.bump(); // '.'
        if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
            return Err(self.err_at(start, "expected digit after '.' in floating literal"));
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'\'') {
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
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'\'') {
                if self.peek() == Some(b'\'') {
                    self.bump();
                    if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        return Err(self.err_at(start, "digit separator must be followed by a digit"));
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
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'\'') {
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

    fn lex_char(&mut self, start: usize) -> Result<Token, LexError> {
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

    fn lex_string(&mut self, start: usize) -> Result<Token, LexError> {
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

    fn lex_punct(&mut self, start: usize) -> Result<Token, LexError> {
        let c = self.bump().unwrap();
        let kind = match c {
            b'{' => Punct::LBrace,
            b'}' => Punct::RBrace,
            b'[' => Punct::LBracket,
            b']' => Punct::RBracket,
            b'(' => Punct::LParen,
            b')' => Punct::RParen,
            b';' => Punct::Semi,
            b',' => Punct::Comma,
            b'?' => Punct::Question,
            b'~' => Punct::Tilde,
            b'#' => Punct::Hash,
            b':' => {
                if self.peek() == Some(b':') {
                    self.bump();
                    Punct::Scope
                } else {
                    Punct::Colon
                }
            }
            b'.' => {
                if self.peek() == Some(b'*') {
                    self.bump();
                    Punct::DotStar
                } else if self.peek() == Some(b'.') && self.peek_at(1) == Some(b'.') {
                    self.bump();
                    self.bump();
                    Punct::Ellipsis
                } else {
                    Punct::Dot
                }
            }
            b'+' => {
                if self.peek() == Some(b'+') {
                    self.bump();
                    Punct::PlusPlus
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::PlusEq
                } else {
                    Punct::Plus
                }
            }
            b'-' => {
                if self.peek() == Some(b'-') {
                    self.bump();
                    Punct::MinusMinus
                } else if self.peek() == Some(b'>') {
                    self.bump();
                    if self.peek() == Some(b'*') {
                        self.bump();
                        Punct::ArrowStar
                    } else {
                        Punct::Arrow
                    }
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::MinusEq
                } else {
                    Punct::Minus
                }
            }
            b'*' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::StarEq
                } else {
                    Punct::Star
                }
            }
            b'/' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::SlashEq
                } else {
                    Punct::Slash
                }
            }
            b'%' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::PercentEq
                } else {
                    Punct::Percent
                }
            }
            b'^' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::CaretEq
                } else {
                    Punct::Caret
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.bump();
                    Punct::AmpAmp
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::AmpEq
                } else {
                    Punct::Amp
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.bump();
                    Punct::PipePipe
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::PipeEq
                } else {
                    Punct::Pipe
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::NotEq
                } else {
                    Punct::Not
                }
            }
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::EqEq
                } else {
                    Punct::Eq
                }
            }
            b'<' => {
                if self.peek() == Some(b'<') {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        Punct::LtLtEq
                    } else {
                        Punct::LtLt
                    }
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::LtEq
                } else {
                    Punct::Lt
                }
            }
            b'>' => {
                if self.peek() == Some(b'>') {
                    self.bump();
                    if self.peek() == Some(b'=') {
                        self.bump();
                        Punct::GtGtEq
                    } else {
                        Punct::GtGt
                    }
                } else if self.peek() == Some(b'=') {
                    self.bump();
                    Punct::GtEq
                } else {
                    Punct::Gt
                }
            }
            _ => {
                return Err(self.err_at(
                    start,
                    format!("unexpected character `{}`", c as char),
                ));
            }
        };
        Ok(Token::new(TokenKind::Punct(kind), self.span_from(start)))
    }
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

fn parse_decimal_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        if !c.is_ascii_digit() {
            return Err("invalid decimal digit".into());
        }
        v = v
            .checked_mul(10)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

fn parse_hex_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        v = v
            .checked_mul(16)
            .and_then(|x| x.checked_add(hex_val(c) as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

fn parse_binary_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        v = v
            .checked_mul(2)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

fn parse_octal_digits(text: &[u8]) -> Result<u128, String> {
    let mut v: u128 = 0;
    for &c in text {
        if c == b'\'' {
            continue;
        }
        // skip leading 0 styling; all are octal digits
        if !(b'0'..=b'7').contains(&c) {
            return Err("invalid octal digit".into());
        }
        v = v
            .checked_mul(8)
            .and_then(|x| x.checked_add((c - b'0') as u128))
            .ok_or_else(|| "integer literal overflow".to_string())?;
    }
    Ok(v)
}

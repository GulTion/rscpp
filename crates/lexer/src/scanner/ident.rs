use super::Scanner;
use crate::error::LexError;
use crate::span::Span;
use crate::token::{FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind};

impl<'a> Scanner<'a> {
    pub(super) fn lex_ident_or_keyword(&mut self, start: usize) -> Result<Token, LexError> {
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
}

use super::Scanner;
use crate::error::LexError;
use crate::span::Span;
use crate::token::{FloatSuffix, IntBase, IntSuffix, Keyword, Punct, Token, TokenKind};

impl<'a> Scanner<'a> {

    pub(super) fn lex_punct(&mut self, start: usize) -> Result<Token, LexError> {
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

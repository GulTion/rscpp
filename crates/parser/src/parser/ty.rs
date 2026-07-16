use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {

    pub(super) fn parse_type(&mut self) -> Result<Type, ParseError> {
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

    pub(super) fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
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

    pub(super) fn parse_builtin_type(&mut self) -> Result<Type, ParseError> {
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
}

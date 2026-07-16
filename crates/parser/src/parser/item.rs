use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {

    pub(super) fn parse_item(&mut self) -> Result<Item, ParseError> {
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

    pub(super) fn parse_using_namespace(&mut self) -> Result<Item, ParseError> {
        let start = self.expect_keyword(Keyword::Using)?.span.start;
        self.expect_keyword(Keyword::Namespace)?;
        let path = self.parse_path()?;
        let end = self.expect_punct(Punct::Semi)?.span.end;
        Ok(Item::UsingNamespace {
            path,
            span: Span::new(start, end),
        })
    }

    pub(super) fn parse_class(&mut self) -> Result<ClassDef, ParseError> {
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

    pub(super) fn parse_member(&mut self) -> Result<Member, ParseError> {
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
}

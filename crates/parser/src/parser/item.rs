use super::Parser;
use crate::error::ParseError;
use rscpp_ast::*;
use rscpp_lexer::{Keyword, Punct, Token, TokenKind};

impl Parser {

    pub(super) fn parse_item(&mut self) -> Result<Item, ParseError> {
        if self.at_keyword(Keyword::Template) {
            self.skip_template_decl()?;
            if self.at_eof() {
                // Template was the last top-level decl.
                return Ok(Item::UsingNamespace {
                    path: Path {
                        segments: vec![],
                        span: Span::new(0, 0),
                    },
                    span: Span::new(0, 0),
                });
            }
            return self.parse_item();
        }
        if self.at_keyword(Keyword::Class) || self.at_keyword(Keyword::Struct) {
            return Ok(Item::Class(self.parse_class()?));
        }
        if self.at_keyword(Keyword::Using) {
            return self.parse_using_namespace();
        }
        // Function or declaration
        let start = self.peek_span().start;
        self.skip_decl_specs();
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
        if self.at_keyword(Keyword::Class) || self.at_keyword(Keyword::Struct) {
            return Ok(Member::Class(self.parse_class()?));
        }
        if self.at_keyword(Keyword::Template) {
            self.skip_template_decl()?;
            return Ok(Member::Access(AccessSpec::Public));
        }
        if self.at_keyword(Keyword::Enum) {
            // Soft: skip `enum Name { A, B };` — treat as no-op member.
            self.bump();
            if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                let _ = self.bump();
            }
            if self.at_punct(Punct::LBrace) {
                self.bump();
                let mut depth = 1i32;
                while depth > 0 && !self.at_eof() {
                    if self.at_punct(Punct::LBrace) {
                        depth += 1;
                    } else if self.at_punct(Punct::RBrace) {
                        depth -= 1;
                    }
                    self.bump();
                }
            }
            if self.at_punct(Punct::Semi) {
                self.bump();
            }
            return Ok(Member::Access(AccessSpec::Public));
        }
        // Destructor: `~Name()`
        if self.at_punct(Punct::Tilde) {
            let start = self.bump().span.start;
            let name = self.parse_ident()?;
            let mut dname = name.clone();
            dname.name = format!("~{}", name.name);
            return Ok(Member::Function(self.parse_function_rest(
                start,
                Type::Builtin {
                    kind: BuiltinType::Void,
                    span: name.span,
                },
                dname,
            )?));
        }
        let start = self.peek_span().start;
        self.skip_decl_specs();
        let ty = self.parse_type()?;
        // `operator==` / `operator()` / `operator[]` etc.
        if self.at_keyword(Keyword::Operator) {
            let op_start = self.bump().span;
            // Call operator: `operator()` — the `()` is the spelling, not params.
            if self.at_punct(Punct::LParen) {
                self.bump();
                self.expect_punct(Punct::RParen)?;
            } else {
                while !self.at_eof() && !self.at_punct(Punct::LParen) {
                    self.bump();
                }
            }
            let name = Ident {
                name: "operator".into(),
                span: op_start,
            };
            return Ok(Member::Function(
                self.parse_function_rest(start, ty, name)?,
            ));
        }
        // Constructor: `AllOne()` / `AllOne() { ... }` — type name is the ctor name.
        if self.at_punct(Punct::LParen) {
            if let Type::Named { path, args, .. } = &ty {
                if args.is_empty() && path.segments.len() == 1 {
                    let name = path.segments[0].clone();
                    return Ok(Member::Function(self.parse_function_rest(
                        start,
                        Type::Builtin {
                            kind: BuiltinType::Void,
                            span: name.span,
                        },
                        name,
                    )?));
                }
            }
        }
        let name = self.parse_ident()?;
        if self.at_punct(Punct::LParen) {
            Ok(Member::Function(self.parse_function_rest(start, ty, name)?))
        } else {
            Ok(Member::Field(self.parse_decl_rest(start, ty, name)?))
        }
    }

    /// Drop `template<…> class/struct/function …` (LeetCode helpers we don't need).
    pub(super) fn skip_template_decl(&mut self) -> Result<(), ParseError> {
        self.expect_keyword(Keyword::Template)?;
        if self.at_punct(Punct::Lt) {
            self.bump();
            let mut depth = 1i32;
            while depth > 0 && !self.at_eof() {
                match self.peek_kind() {
                    TokenKind::Punct(Punct::Lt) => depth += 1,
                    TokenKind::Punct(Punct::Gt) => depth -= 1,
                    TokenKind::Punct(Punct::GtGt) => depth -= 2,
                    _ => {}
                }
                self.bump();
            }
        }
        if self.at_keyword(Keyword::Class) || self.at_keyword(Keyword::Struct) {
            let _ = self.parse_class()?;
            return Ok(());
        }
        // function / using / alias — skip to `;` or `{…}`
        while !self.at_eof()
            && !self.at_punct(Punct::Semi)
            && !self.at_punct(Punct::LBrace)
        {
            self.bump();
        }
        if self.at_punct(Punct::LBrace) {
            self.bump();
            let mut depth = 1i32;
            while depth > 0 && !self.at_eof() {
                if self.at_punct(Punct::LBrace) {
                    depth += 1;
                } else if self.at_punct(Punct::RBrace) {
                    depth -= 1;
                }
                self.bump();
            }
            if self.at_punct(Punct::Semi) {
                self.bump();
            }
        } else if self.at_punct(Punct::Semi) {
            self.bump();
        }
        Ok(())
    }
}

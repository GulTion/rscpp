use rscpp_lexer::{
    tokenize, FloatSuffix, IntBase, IntSuffix, Keyword, LexError, Punct, TokenKind,
};

fn kinds(src: &str) -> Result<Vec<TokenKind>, LexError> {
    Ok(tokenize(src)?.into_iter().map(|t| t.kind).collect())
}

fn without_eof(src: &str) -> Vec<TokenKind> {
    let mut ks = kinds(src).expect("tokenize");
    assert!(matches!(ks.last(), Some(TokenKind::Eof)));
    ks.pop();
    ks
}

#[test]
fn empty_source() {
    let ks = kinds("").unwrap();
    assert_eq!(ks, vec![TokenKind::Eof]);
}

#[test]
fn keywords_vs_identifiers() {
    assert_eq!(
        without_eof("int x"),
        vec![
            TokenKind::Keyword(Keyword::Int),
            TokenKind::Ident("x".into()),
        ]
    );
    assert_eq!(
        without_eof("integer"),
        vec![TokenKind::Ident("integer".into())]
    );
    assert_eq!(
        without_eof("nullptr true false"),
        vec![
            TokenKind::Keyword(Keyword::Nullptr),
            TokenKind::Keyword(Keyword::True),
            TokenKind::Keyword(Keyword::False),
        ]
    );
}

#[test]
fn comments_and_whitespace_skipped() {
    assert_eq!(
        without_eof("a // comment\nb /* block */ c"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Ident("b".into()),
            TokenKind::Ident("c".into()),
        ]
    );
}

#[test]
fn unterminated_block_comment() {
    let err = kinds("/* oops").unwrap_err();
    assert!(err.message.contains("unterminated"));
}

#[test]
fn integer_literals() {
    assert_eq!(
        without_eof("42"),
        vec![TokenKind::IntLit {
            value: 42,
            suffix: IntSuffix::None,
            base: IntBase::Decimal,
        }]
    );
    assert_eq!(
        without_eof("0xFF"),
        vec![TokenKind::IntLit {
            value: 255,
            suffix: IntSuffix::None,
            base: IntBase::Hex,
        }]
    );
    assert_eq!(
        without_eof("0b1010"),
        vec![TokenKind::IntLit {
            value: 10,
            suffix: IntSuffix::None,
            base: IntBase::Binary,
        }]
    );
    assert_eq!(
        without_eof("07"),
        vec![TokenKind::IntLit {
            value: 7,
            suffix: IntSuffix::None,
            base: IntBase::Octal,
        }]
    );
    assert_eq!(
        without_eof("1'000ull"),
        vec![TokenKind::IntLit {
            value: 1000,
            suffix: IntSuffix::Ull,
            base: IntBase::Decimal,
        }]
    );
}

#[test]
fn float_literals() {
    assert_eq!(
        without_eof("3.14f"),
        vec![TokenKind::FloatLit {
            value: 3.14,
            suffix: FloatSuffix::F,
        }]
    );
    assert_eq!(
        without_eof("1e3"),
        vec![TokenKind::FloatLit {
            value: 1000.0,
            suffix: FloatSuffix::None,
        }]
    );
    assert_eq!(
        without_eof(".5"),
        vec![TokenKind::FloatLit {
            value: 0.5,
            suffix: FloatSuffix::None,
        }]
    );
}

#[test]
fn char_and_string_literals() {
    assert_eq!(without_eof("'a'"), vec![TokenKind::CharLit('a')]);
    assert_eq!(without_eof("'\\n'"), vec![TokenKind::CharLit('\n')]);
    assert_eq!(
        without_eof("\"hi\\t\""),
        vec![TokenKind::StringLit("hi\t".into())]
    );
}

#[test]
fn unterminated_string() {
    let err = kinds("\"abc").unwrap_err();
    assert!(err.message.contains("unterminated"));
}

#[test]
fn maximal_munch_operators() {
    assert_eq!(
        without_eof("a++ + ++b"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Punct(Punct::PlusPlus),
            TokenKind::Punct(Punct::Plus),
            TokenKind::Punct(Punct::PlusPlus),
            TokenKind::Ident("b".into()),
        ]
    );
    assert_eq!(
        without_eof("a->b::c"),
        vec![
            TokenKind::Ident("a".into()),
            TokenKind::Punct(Punct::Arrow),
            TokenKind::Ident("b".into()),
            TokenKind::Punct(Punct::Scope),
            TokenKind::Ident("c".into()),
        ]
    );
    assert_eq!(
        without_eof("1<<2>>3"),
        vec![
            TokenKind::IntLit {
                value: 1,
                suffix: IntSuffix::None,
                base: IntBase::Decimal,
            },
            TokenKind::Punct(Punct::LtLt),
            TokenKind::IntLit {
                value: 2,
                suffix: IntSuffix::None,
                base: IntBase::Decimal,
            },
            TokenKind::Punct(Punct::GtGt),
            TokenKind::IntLit {
                value: 3,
                suffix: IntSuffix::None,
                base: IntBase::Decimal,
            },
        ]
    );
}

#[test]
fn hash_token_for_future_preprocessor() {
    assert_eq!(
        without_eof("# include"),
        vec![
            TokenKind::Punct(Punct::Hash),
            TokenKind::Ident("include".into()),
        ]
    );
}

#[test]
fn rejects_wide_and_raw_strings() {
    assert!(kinds("L\"wide\"").is_err());
    assert!(kinds("u8\"utf8\"").is_err());
    assert!(kinds("R\"(raw)\"").is_err());
}

#[test]
fn spans_cover_lexeme() {
    let toks = tokenize("  int").unwrap();
    assert_eq!(toks[0].span.start, 2);
    assert_eq!(toks[0].span.end, 5);
    assert!(matches!(toks[0].kind, TokenKind::Keyword(Keyword::Int)));
}

#[test]
fn leetcode_like_snippet() {
    let src = r#"
class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        for (int i = 0; i < nums.size(); ++i) {
            if (nums[i] == target) return {i};
        }
        return {};
    }
};
"#;
    let toks = tokenize(src).expect("snippet should lex");
    assert!(toks.len() > 20);
    assert!(matches!(toks.last().map(|t| &t.kind), Some(TokenKind::Eof)));
    // Spot-check a few kinds present
    let kinds: Vec<_> = toks.iter().map(|t| &t.kind).collect();
    assert!(kinds
        .iter()
        .any(|k| matches!(k, TokenKind::Keyword(Keyword::Class))));
    assert!(kinds
        .iter()
        .any(|k| matches!(k, TokenKind::Keyword(Keyword::Public))));
    assert!(kinds
        .iter()
        .any(|k| matches!(k, TokenKind::Ident(s) if s == "twoSum")));
    assert!(kinds
        .iter()
        .any(|k| matches!(k, TokenKind::Punct(Punct::PlusPlus))));
}

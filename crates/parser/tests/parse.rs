use rscpp_ast::*;
use rscpp_parser::parse;

#[test]
fn empty_tu() {
    let tu = parse("").unwrap();
    assert!(tu.items.is_empty());
}

#[test]
fn using_namespace() {
    let tu = parse("using namespace std;").unwrap();
    assert!(matches!(
        &tu.items[0],
        Item::UsingNamespace { path, .. } if path.segments[0].name == "std"
    ));
}

#[test]
fn simple_function() {
    let tu = parse("int main() { return 0; }").unwrap();
    match &tu.items[0] {
        Item::Function(f) => {
            assert_eq!(f.name.name, "main");
            assert!(matches!(
                f.return_type,
                Type::Builtin {
                    kind: BuiltinType::Int,
                    ..
                }
            ));
            assert_eq!(f.body.stmts.len(), 1);
        }
        _ => panic!("expected function"),
    }
}

#[test]
fn expr_precedence() {
    let tu = parse("int f() { return 1+2*3; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    let Stmt::Return {
        value:
            Some(Expr::Binary {
                op: BinaryOp::Add,
                right,
                ..
            }),
        ..
    } = &f.body.stmts[0]
    else {
        panic!("expected 1+(2*3)");
    };
    assert!(matches!(
        right.as_ref(),
        Expr::Binary {
            op: BinaryOp::Mul,
            ..
        }
    ));
}

#[test]
fn assign_right_assoc() {
    let tu = parse("int f() { a=b=c; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    let Stmt::Expr { expr, .. } = &f.body.stmts[0] else {
        panic!();
    };
    let Expr::Assign {
        left,
        right,
        op: AssignOp::Assign,
        ..
    } = expr
    else {
        panic!("{expr:?}");
    };
    assert!(matches!(left.as_ref(), Expr::Name(_)));
    assert!(matches!(right.as_ref(), Expr::Assign { .. }));
}

#[test]
fn call_index_member() {
    let tu = parse("int f() { return a->b()[i].c; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    let Stmt::Return {
        value: Some(expr), ..
    } = &f.body.stmts[0]
    else {
        panic!();
    };
    assert!(matches!(expr, Expr::Member { arrow: false, .. }));
}

#[test]
fn vector_type_nested_gtgt() {
    let tu = parse("vector<vector<int>> x;").unwrap();
    let Item::Decl(d) = &tu.items[0] else {
        panic!("{:?}", tu.items);
    };
    match &d.ty {
        Type::Named { path, args, .. } => {
            assert_eq!(path.segments[0].name, "vector");
            assert_eq!(args.len(), 1);
            match &args[0] {
                Type::Named { path, args, .. } => {
                    assert_eq!(path.segments[0].name, "vector");
                    assert!(matches!(
                        args[0],
                        Type::Builtin {
                            kind: BuiltinType::Int,
                            ..
                        }
                    ));
                }
                other => panic!("{other:?}"),
            }
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn class_solution_smoke() {
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
    let tu = parse(src).expect("should parse");
    assert_eq!(tu.items.len(), 1);
    let Item::Class(c) = &tu.items[0] else {
        panic!();
    };
    assert_eq!(c.name.name, "Solution");
    assert!(c
        .members
        .iter()
        .any(|m| matches!(m, Member::Access(AccessSpec::Public))));
    assert!(c
        .members
        .iter()
        .any(|m| matches!(m, Member::Function(f) if f.name.name == "twoSum")));
}

#[test]
fn if_else_while() {
    let tu = parse(
        r#"
int f() {
  if (1) { return 1; } else { return 0; }
  while (x) x = x - 1;
  do { x = x + 1; } while (x < 10);
}
"#,
    )
    .unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    assert!(f.body.stmts.len() >= 3);
}

#[test]
fn include_directives_skipped() {
    let tu = parse(
        r#"
#include <vector>
#pragma once
int main() { return 0; }
"#,
    )
    .unwrap();
    assert_eq!(tu.items.len(), 1);
}

#[test]
fn ternary_expression() {
    let tu = parse("int f() { return 1 ? 2 : 3; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    let Stmt::Return {
        value: Some(Expr::Conditional { .. }),
        ..
    } = &f.body.stmts[0]
    else {
        panic!("expected ternary");
    };
}

#[test]
fn range_for_stmt() {
    let tu = parse("int f() { for (int x : v) { } return 0; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    assert!(matches!(f.body.stmts[0], Stmt::ForRange { .. }));
}

#[test]
fn static_const_local_and_cast() {
    let src = r#"
    class Solution {
    public:
        int f() {
            static const int N = 5;
            return static_cast<int>(N);
        }
    };
    "#;
    let tu = parse(src).unwrap();
    assert_eq!(tu.items.len(), 1);
}

#[test]
fn vector_paren_init() {
    let tu = parse("int f() { vector<int> dp(6); return 0; }").unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    let Stmt::Decl(d) = &f.body.stmts[0] else {
        panic!();
    };
    assert!(matches!(d.declarators[0].init, Some(Expr::Call { .. })));
}

#[test]
fn lambda_and_structured_binding() {
    let src = r#"
    int f() {
        auto fn = [&](int x) { return x; };
        auto [a, b] = p;
        return 0;
    }
    "#;
    let tu = parse(src).unwrap();
    let Item::Function(f) = &tu.items[0] else {
        panic!();
    };
    assert!(matches!(
        &f.body.stmts[0],
        Stmt::Decl(d) if matches!(d.declarators[0].init, Some(Expr::Lambda { .. }))
    ));
    assert!(matches!(f.body.stmts[1], Stmt::Destructure { .. }));
}

#[test]
fn numeric_limits_template() {
    let tu = parse("int f() { return numeric_limits<int>::max(); }").unwrap();
    assert_eq!(tu.items.len(), 1);
}

#[test]
fn call_operator_member() {
    let tu = parse(
        r#"
    struct H {
        size_t operator()(const vector<int>& v) const { return 0; }
    };
    "#,
    )
    .unwrap();
    let Item::Class(c) = &tu.items[0] else {
        panic!();
    };
    assert!(matches!(c.members[0], Member::Function(_)));
}

#[test]
fn lookup_global_ctor_init_with_expr() {
    let tu = parse("vector<vector<int>> LOOKUP(5 + 1, vector<int>(5 + 1, -1));").unwrap();
    assert!(matches!(tu.items[0], Item::Decl(_)));
}

#[test]
fn local_anon_enum() {
    let tu = parse(
        r#"
int f() {
  enum { A, B };
  return A + B;
}
"#,
    )
    .unwrap();
    assert_eq!(tu.items.len(), 1);
}

#[test]
fn c_style_array_declarator_message() {
    let src = "int main() { int prime[]={2}; return 0; }";
    let err = parse(src).unwrap_err();
    assert!(
        err.message.contains("array declarator") && err.message.contains("vector"),
        "{}",
        err.message
    );
    let fmt = err.format_with_source(src);
    assert!(fmt.contains("--> 1:"), "{fmt}");
    assert!(fmt.contains('^'), "{fmt}");
}

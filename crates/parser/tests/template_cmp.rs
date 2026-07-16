use rscpp_parser::parse;

#[test]
fn lt_vs_numeric_limits_template() {
    parse(
        r#"
int f(int result, int x) {
  if (result < numeric_limits<int>::min() / 10) return 0;
  return 1;
}
"#,
    )
    .unwrap();
}

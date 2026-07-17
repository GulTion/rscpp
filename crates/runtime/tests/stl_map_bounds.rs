use rscpp_runtime::{Engine, Value};

#[test]
fn map_lower_upper_equal_range() {
    let src = r#"
int main() {
  map<int,int> m;
  m[1]=1; m[3]=3; m[5]=5;
  int lo = m.lower_bound(3);
  int hi = m.upper_bound(3);
  return lo * 10 + hi;
}
"#;
    // keys [1,3,5]: lower_bound(3)=1, upper_bound(3)=2 → 12
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(12)
    );
}

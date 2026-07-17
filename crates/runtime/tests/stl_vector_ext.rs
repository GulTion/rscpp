use rscpp_runtime::{Engine, Value};

#[test]
fn vector_at_resize_reserve() {
    let src = r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(2);
  v.reserve(10);
  int c = v.capacity();
  v.resize(4, 9);
  v.resize(3);
  return v.at(0) + v.at(2) * 10 + c * 0 + (int)v.size();
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    // 1 + 9*10 + 3 = 94
    assert_eq!(eng.run_main().unwrap(), Value::Int(94));
}

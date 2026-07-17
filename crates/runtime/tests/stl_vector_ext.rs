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

#[test]
fn vector_insert_erase() {
    let src = r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(3);
  v.insert(v.begin() + 1, 2);
  v.erase(v.begin());
  return (int)v.size() * 10 + v[0];
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(22));
}

#[test]
fn vector_erase_range() {
    let src = r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(2); v.push_back(3); v.push_back(4);
  v.erase(v.begin() + 1, v.begin() + 3);
  return (int)v.size() * 10 + v[0] + v[1];
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    // [1,4] → 20+1+4=25
    assert_eq!(eng.run_main().unwrap(), Value::Int(25));
}

#[test]
fn vector_from_rbegin_rend() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(3);
  vector<int> r(v.rbegin(), v.rend());
  return r[0] * 100 + r[1] * 10 + r[2];
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(321));
}

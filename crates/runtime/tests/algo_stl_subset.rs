use rscpp_runtime::{Engine, Value};

#[test]
fn nonmod_all_find_count() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(3);
  bool ok = all_of(v.begin(), v.end(), [](int x){ return x > 0; });
  int i = find(v.begin(), v.end(), 2);
  int c = count_if(v.begin(), v.end(), [](int x){ return x >= 2; });
  return (ok?1:0)*100 + i*10 + c;
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(112)
    );
}

#[test]
fn fill_remove_unique() {
    let src = r#"
int main() {
  vector<int> v; v.push_back(1); v.push_back(2); v.push_back(2); v.push_back(3);
  fill_n(v.begin(), 1, 9);
  auto it = remove(v.begin(), v.end(), 2);
  v.erase(it, v.end());
  return (int)v.size() * 10 + v[0];
}
"#;
    // after fill_n: 9,2,2,3; remove 2 → 9,3; size 2 → 29
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(29)
    );
}

#[test]
fn set_intersection_basic() {
    let src = r#"
int main() {
  vector<int> a; a.push_back(1); a.push_back(2); a.push_back(3);
  vector<int> b; b.push_back(2); b.push_back(3); b.push_back(4);
  vector<int> out; out.resize(3);
  set_intersection(a.begin(), a.end(), b.begin(), b.end(), out.begin());
  return out[0] * 10 + out[1];
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(23)
    );
}

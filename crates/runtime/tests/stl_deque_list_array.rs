use rscpp_runtime::{Engine, Value};

#[test]
fn deque_push_front_back() {
    let src = r#"
int main() {
  deque<int> d;
  d.push_back(2); d.push_front(1); d.push_back(3);
  return d.front() * 100 + d.back() * 10 + (int)d.size();
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(133));
}

#[test]
fn list_push_front() {
    let src = r#"
int main() {
  list<int> L; L.push_back(2); L.push_front(1);
  return L.front() + L.back() * 10;
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(21)
    );
}

#[test]
fn array_fill_and_index() {
    let src = r#"
int main() {
  array<int,3> a;
  a.fill(7);
  return a[0] + (int)a.size();
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(10)
    );
}

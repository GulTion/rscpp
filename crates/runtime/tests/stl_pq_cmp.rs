use rscpp_runtime::{Engine, Value};

#[test]
fn pq_default_max_heap() {
    let src = r#"
int main() {
  priority_queue<int> pq;
  pq.push(1); pq.push(3); pq.push(2);
  return pq.top();
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(3)
    );
}

#[test]
fn pq_with_greater() {
    let src = r#"
int main() {
  priority_queue<int, vector<int>, greater<int>> pq;
  pq.push(3); pq.push(1); pq.push(2);
  int a = pq.top(); pq.pop();
  int b = pq.top();
  return a * 10 + b;
}
"#;
    assert_eq!(
        Engine::from_source(src).unwrap().run_main().unwrap(),
        Value::Int(12)
    );
}

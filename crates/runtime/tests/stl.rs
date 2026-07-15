use rscpp_runtime::{Engine, Event, Value};

#[test]
fn map_index_assign_and_count() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  map<int, int> m;
  m[1] = 10;
  m[2] = 20;
  return m[1] + m[2] + m.count(1) + m.size();
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(33));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::ContainerMod { kind, .. } if kind == "map_assign")));
}

#[test]
fn unordered_map_insert_pair() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  unordered_map<int, int> m;
  m.insert(pair(3, 7));
  return m[3] + m.count(3);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(8));
}

#[test]
fn set_insert_count() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  set<int> s;
  s.insert(5);
  s.insert(5);
  return s.size() * 10 + s.count(5);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(11));
}

#[test]
fn stack_queue_priority() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  stack<int> st;
  st.push(1);
  st.push(2);
  int a = st.top();
  st.pop();
  queue<int> q;
  q.push(3);
  q.push(4);
  int b = q.front();
  q.pop();
  priority_queue<int> pq;
  pq.push(1);
  pq.push(9);
  pq.push(5);
  int c = pq.top();
  return a * 100 + b * 10 + c;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(239));
}

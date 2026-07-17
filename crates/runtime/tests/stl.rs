use rscpp_runtime::{Engine, Event, Value};

#[test]
fn vector_index_read_emits_container_lookup() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(2);
  v.push_back(7);
  return v[1];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(7));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::ContainerLookup {
            kind,
            key: Some(Value::Int(1)),
            result: Value::Int(7),
            ..
        } if kind == "index"
    )));
}

#[test]
fn map_count_emits_container_lookup() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  map<int, int> m;
  m[1] = 10;
  return m.count(1) + m.count(2);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(1));
    let lookups: Vec<_> = eng
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::ContainerLookup {
                kind, key, result, ..
            } if kind == "count" => Some((key.clone(), result.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(lookups.len(), 2);
    assert_eq!(lookups[0].0, Some(Value::Int(1)));
    assert_eq!(lookups[0].1, Value::Int(1));
    assert_eq!(lookups[1].0, Some(Value::Int(2)));
    assert_eq!(lookups[1].1, Value::Int(0));
}

#[test]
fn map_of_vector_index_method_and_assign() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  map<int, vector<int>> m;
  m[0].push_back(2);
  m[0][0] = 99;
  return m[0][0];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(99));
}

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
    assert!(eng.events().iter().any(|e| {
        matches!(
            e,
            Event::ContainerMod {
                kind,
                key: Some(Value::Int(1)),
                value: Some(Value::Int(10)),
                ..
            } if kind == "map_assign"
        )
    }));
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
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::ContainerMod {
            kind,
            key: Some(Value::Int(3)),
            value: Some(Value::Int(7)),
            ..
        } if kind == "unordered_map::insert"
    )));
}

#[test]
fn unordered_map_find_vs_end() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  unordered_map<int, int> m;
  m[1] = 10;
  int hit = m.find(1) != m.end() ? 1 : 0;
  int miss = m.find(2) == m.end() ? 1 : 0;
  return hit + miss;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(2));
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
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::ContainerMod {
            kind,
            key: Some(Value::Int(5)),
            ..
        } if kind == "set::insert"
    )));
}

#[test]
fn map_alloc_entries_and_assign_skips_index_lookup() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  map<int, int> m;
  m[2] = 9;
  return m[2];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(9));

    assert!(eng.events().iter().any(|e| {
        matches!(
            e,
            Event::Alloc {
                kind,
                size: 0,
                entries,
                ..
            } if kind == "map" && entries.is_empty()
        )
    }));

    // One index lookup for `return m[2]`; pure assign must not look up the LHS.
    let index_lookups = eng
        .events()
        .iter()
        .filter(|e| {
            matches!(
                e,
                Event::ContainerLookup {
                    kind,
                    key: Some(Value::Int(2)),
                    ..
                } if kind == "index"
            )
        })
        .count();
    assert_eq!(index_lookups, 1);

    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::ContainerMod {
            kind,
            key: Some(Value::Int(2)),
            ..
        } if kind == "map_assign"
    )));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::Write { .. })));
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

#[test]
fn map_range_for_kvp_emits_pair_alloc_before_varcreate() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  unordered_map<int, int> count;
  count[1] = 2;
  int s = 0;
  for (const auto& kvp : count) {
    s = s + kvp.first + kvp.second;
  }
  return s;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));

    let ev = eng.events();
    let mut saw_pair_alloc = false;
    for e in ev {
        match e {
            Event::Alloc { id, kind, .. } if kind == "pair" => {
                saw_pair_alloc = true;
                // Next VarCreate for kvp must reference this id.
                let pos = ev.iter().position(|x| matches!(x, Event::Alloc { id: i, kind: k, .. } if *i == *id && k == "pair")).unwrap();
                assert!(ev[pos + 1..].iter().any(|x| matches!(
                    x,
                    Event::VarCreate {
                        name,
                        value: Value::Object(oid),
                        ..
                    } if name == "kvp" && *oid == *id
                )));
            }
            Event::VarCreate {
                name,
                value: Value::Object(oid),
                ..
            } if name == "kvp" => {
                assert!(
                    saw_pair_alloc || ev.iter().any(|x| matches!(x, Event::Alloc { id, kind, .. } if *id == *oid && kind == "pair")),
                    "VarCreate kvp → Object({oid}) without prior Alloc pair"
                );
            }
            _ => {}
        }
    }
    assert!(
        saw_pair_alloc,
        "expected Alloc kind=pair for map range-for kvp"
    );
}

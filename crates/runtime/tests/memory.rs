use rscpp_runtime::{Engine, Event, Value};

#[test]
fn pointer_addr_deref_and_ptr_move() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int x = 1;
  int* p = &x;
  *p = 5;
  return x;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(5));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::PtrMove { name, .. } if name == "p")));
}

#[test]
fn reference_bind_and_write_through() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int x = 2;
  int& r = x;
  r = 9;
  return x;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(9));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::RefBind { name, .. } if name == "r")));
}

#[test]
fn ref_param_binds_heap_object() {
    let mut eng = Engine::from_source(
        r#"
int sum_first(vector<int>& v) {
  return v[0];
}
int main() {
  vector<int> v;
  v.push_back(42);
  return sum_first(v);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(42));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::RefBind { name, .. } if name == "v")));
}

#[test]
fn dealloc_on_frame_exit() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(1);
  return 0;
}
"#,
    )
    .unwrap();
    eng.run_main().unwrap();
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::Dealloc { .. })));
}

#[test]
fn nullptr_is_null_address() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int* p = nullptr;
  return p == nullptr;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Bool(true));
}

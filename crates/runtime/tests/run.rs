use rscpp_runtime::{Engine, Event, Slot, Value};

#[test]
fn run_main_returns() {
    let mut eng = Engine::from_source("int main() { return 42; }").unwrap();
    let v = eng.run_main().unwrap();
    assert_eq!(v, Value::Int(42));
    assert!(eng.events().iter().any(|e| matches!(e, Event::FnEnter { .. })));
    assert!(eng.events().iter().any(|e| matches!(e, Event::FnExit { .. })));
    assert!(eng.events().iter().any(|e| matches!(e, Event::Step { .. })));
}

#[test]
fn locals_and_assign_events() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int x = 1;
  x = x + 2;
  return x;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::VarCreate { name, .. } if name == "x")));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::VarAssign {
            name,
            old: Some(Value::Int(1)),
            value: Value::Int(3),
            ..
        } if name == "x"
    )));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::Write {
            slot: Slot::Local { name },
            value: Value::Int(3),
            ..
        } if name == "x"
    )));
}

#[test]
fn for_loop_sum() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int s = 0;
  for (int i = 0; i < 5; ++i) {
    s = s + i;
  }
  return s;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(10));
    assert!(
        eng.events()
            .iter()
            .filter(|e| matches!(e, Event::LoopIter { .. }))
            .count()
            >= 5
    );
}

#[test]
fn vector_push_and_index() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(10);
  v.push_back(20);
  return v[0] + v[1] + v.size();
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(32));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::ContainerMod {
            kind,
            index: Some(0),
            value: Some(Value::Int(10)),
            ..
        } if kind == "push_back"
    )));
}

#[test]
fn solution_two_sum_linear() {
    let src = r#"
class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        for (int i = 0; i < nums.size(); ++i) {
            if (nums[i] == target) return {i};
        }
        return {};
    }
};
"#;
    let mut eng = Engine::from_source(src).unwrap();
    let nums = eng.make_vector(vec![Value::Int(2), Value::Int(7), Value::Int(11)]);
    let ret = eng
        .call("Solution::twoSum", &[nums, Value::Int(7)])
        .unwrap();
    let idxs = eng.vector_as_ints(&ret).unwrap();
    assert_eq!(idxs, vec![1]);
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::Branch { then_taken: true, .. })));
}

#[test]
fn compare_emits_event() {
    let mut eng = Engine::from_source("int main() { return 1 < 2; }").unwrap();
    let v = eng.run_main().unwrap();
    assert_eq!(v, Value::Bool(true));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::Compare { result: true, .. })));
}

#[test]
fn swap_emits_event() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int a = 1;
  int b = 2;
  swap(a, b);
  return a * 10 + b;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(21));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::Swap {
            value_a: Value::Int(1),
            value_b: Value::Int(2),
            ..
        }
    )));
}

#[test]
fn branch_else_path() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  if (0) { return 1; } else { return 2; }
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(2));
    assert!(eng
        .events()
        .iter()
        .any(|e| matches!(e, Event::Branch { then_taken: false, .. })));
}

use rscpp_parser::parse;
use rscpp_runtime::{Event, Value};
use rscpp_vm::{compile, run_main, Vm};
#[test]
fn vm_main_return_arith() {
    let (v, events) = run_main("int main() { return 1+2*3; }").unwrap();
    assert_eq!(v, Value::Int(7));
    assert!(events.iter().any(|e| matches!(e, Event::Step { .. })));
}

#[test]
fn vm_for_loop_sum() {
    let (v, ev) = run_main(
        r#"
int main() {
  int s = 0;
  int i = 0;
  while (i < 5) {
    s = s + i;
    i = i + 1;
  }
  return s;
}
"#,
    )
    .unwrap();
    assert_eq!(v, Value::Int(10));
    assert!(ev.iter().any(|e| matches!(e, Event::Compare { .. })));
}

#[test]
fn vm_vector_push() {
    let (v, ev) = run_main(
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
    assert_eq!(v, Value::Int(32));
    assert!(ev
        .iter()
        .any(|e| matches!(e, Event::ContainerMod { kind, .. } if kind == "push_back")));
}

#[test]
fn vm_map_index() {
    let (v, _) = run_main(
        r#"
int main() {
  map<int, int> m;
  m[1] = 10;
  return m[1] + m.count(1);
}
"#,
    )
    .unwrap();
    assert_eq!(v, Value::Int(11));
}

#[test]
fn vm_call_function() {
    let src = r#"
int add(int a, int b) { return a + b; }
int main() { return add(2, 3); }
"#;
    let (v, _) = run_main(src).unwrap();
    assert_eq!(v, Value::Int(5));
}

#[test]
fn vm_solution_call() {
    let src = r#"
class Solution {
public:
    vector<int> twoSum(vector<int>& nums, int target) {
        int i = 0;
        while (i < nums.size()) {
            if (nums[i] == target) return {i};
            i = i + 1;
        }
        return {};
    }
};
"#;
    let tu = parse(src).unwrap();
    let program = compile(&tu).unwrap();
    let mut vm = Vm::new(program);
    let nums = vm.make_vector(vec![Value::Int(2), Value::Int(7), Value::Int(11)]);
    let ret = vm.call("Solution::twoSum", &[nums, Value::Int(7)]).unwrap();
    assert_eq!(vm.vector_as_ints(&ret).unwrap(), vec![1]);
}

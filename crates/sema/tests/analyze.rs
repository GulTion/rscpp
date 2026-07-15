use rscpp_parser::parse;
use rscpp_sema::analyze;

#[test]
fn accepts_simple_function() {
    let tu = parse("int main() { int x = 1; return x; }").unwrap();
    let r = analyze(&tu);
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn rejects_undeclared_ident() {
    let tu = parse("int main() { return y; }").unwrap();
    let r = analyze(&tu);
    assert!(!r.ok());
    assert!(r.errors.iter().any(|e| e.message.contains("undeclared")));
}

#[test]
fn rejects_bad_assign() {
    let tu = parse(
        r#"
int main() {
  int x = 1;
  int* p = nullptr;
  x = p;
  return 0;
}
"#,
    )
    .unwrap();
    let r = analyze(&tu);
    assert!(!r.ok());
    assert!(r.errors.iter().any(|e| e.message.contains("cannot assign")));
}

#[test]
fn vector_members_and_index() {
    let tu = parse(
        r#"
int f(vector<int>& nums) {
  int n = nums.size();
  int x = nums[0];
  nums.push_back(1);
  return n + x;
}
"#,
    )
    .unwrap();
    let r = analyze(&tu);
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn nested_templates_ok() {
    let tu = parse(
        r#"
int f() {
  vector<pair<int, int>> v;
  map<int, vector<int>> m;
  return 0;
}
"#,
    )
    .unwrap();
    let r = analyze(&tu);
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn class_method_smoke() {
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
    let tu = parse(src).unwrap();
    let r = analyze(&tu);
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn call_arity_mismatch() {
    let tu = parse(
        r#"
int add(int a, int b) { return a + b; }
int main() { return add(1); }
"#,
    )
    .unwrap();
    let r = analyze(&tu);
    assert!(!r.ok());
    assert!(r.errors.iter().any(|e| e.message.contains("expects")));
}

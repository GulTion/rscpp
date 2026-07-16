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
fn std_algorithm_and_comparator_stubs() {
    let tu = parse(
        r#"
int main() {
  vector<int> v;
  sort(v.begin(), v.end(), greater<int>());
  auto it = prev(v.end());
  transform(v.begin(), v.end(), v.begin(), [](int x) { return x; });
  assert(true);
  bitset<32> b(1);
  return b.count();
}
"#,
    )
    .unwrap();
    let r = analyze(&tu);
    assert!(r.ok(), "{:?}", r.errors);
}

#[test]
fn set_range_for_ok_but_no_subscript() {
    let tu = parse(
        r#"
int sum(unordered_set<int>& s) {
  int t = 0;
  for (const auto& x : s) { t += x; }
  return t;
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);

    let bad = parse(
        r#"
int f(set<int>& s) { return s[0]; }
"#,
    )
    .unwrap();
    let r = analyze(&bad);
    assert!(!r.ok());
    assert!(r
        .errors
        .iter()
        .any(|e| e.message.contains("does not provide operator[]")));
}

#[test]
fn bitset_subscript_ok() {
    let tu = parse(
        r#"
bool f() {
  bitset<8> b;
  b[0] = true;
  return b[0];
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
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

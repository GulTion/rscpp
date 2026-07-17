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
fn int64_t_is_numeric() {
    let tu = parse(
        r#"
long long f(int h, int n) {
  if (int64_t(h) * (h + 1) / 6 > n) return 1;
  return int64_t(h) + n;
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
}

#[test]
fn stl_set_map_string_members() {
    let tu = parse(
        r#"
int f(set<int>& s, unordered_map<int,int>& m, string& t) {
  if (m.empty()) return 0;
  auto a = m.cbegin();
  auto b = s.find(1);
  auto c = s.upper_bound(2);
  return t.compare("x") + (int)t.rfind("y") + (int)s.size();
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
}

#[test]
fn leetcode_node_fields_seeded() {
    let tu = parse(
        r#"
Node* f(Node* a) {
  if (a->isLeaf) return a->topLeft;
  a->random = a->next;
  a->parent = a;
  return a->left;
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
}

#[test]
fn rejects_unknown_node_field() {
    let tu = parse(r#"int f(Node* a) { return a->notARealField; }"#).unwrap();
    let r = analyze(&tu);
    assert!(!r.ok());
    assert!(r.errors.iter().any(|e| e.message.contains("no member")));
}

#[test]
fn recursive_function_lambda_and_template_method() {
    let tu = parse(
        r#"
class Solution {
public:
    int f(TreeNode* root) {
        const function<int(TreeNode*)> dfs = [&](TreeNode* curr) {
            if (!curr) return 0;
            return dfs(curr->left) + 1;
        };
        return dfs(root);
    }
    template<typename T>
    void compute(stack<T>* a, stack<char>* b) { a->pop(); b->pop(); }
    int g() {
        stack<int64_t> o; stack<char> p;
        compute(&o, &p);
        return 0;
    }
};
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
}

#[test]
fn array_subscript_and_vector_erase() {
    let tu = parse(
        r#"
int f() {
  array<int, 8> a;
  a[0] = 1;
  vector<int> v{1,2,3};
  v.erase(v.begin());
  v.assign(3, 0);
  v.insert(v.begin(), 1);
  return min(v, v).size();
}
"#,
    )
    .unwrap();
    assert!(analyze(&tu).ok(), "{:?}", analyze(&tu).errors);
}

#[test]
fn still_rejects_undeclared_free_helper() {
    let tu = parse(r#"int f() { return dfs(1); }"#).unwrap();
    let r = analyze(&tu);
    assert!(!r.ok());
    assert!(r.errors.iter().any(|e| e.message.contains("undeclared")));
}

#[test]
fn string_find_arity_map_at_range_and_auto_ptrs() {
    let tu = parse(
        r#"
int f(string& s, unordered_map<int, vector<int>>& G, Node* head) {
  auto pos = -1;
  while ((pos = s.find("x", pos + 1)) != string::npos) {}
  for (const auto& j : G.at(0)) { (void)j; }
  for (auto *curr = head, *copy = head; curr; curr = curr->next) {
    copy = copy->next;
  }
  return 0;
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

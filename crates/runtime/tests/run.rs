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
fn class_unqualified_method_call_and_recursion() {
    let src = r#"
class Solution {
public:
    void dfs(int u, vector<vector<int>>& adj, vector<int>& vis) {
        vis[u] = 1;
        int i = 0;
        while (i < adj[u].size()) {
            int v = adj[u][i];
            if (vis[v] == 0) {
                dfs(v, adj, vis);
            }
            i = i + 1;
        }
    }
    int countComponents(int n, vector<vector<int>>& adj) {
        vector<int> vis;
        int i = 0;
        while (i < n) {
            vis.push_back(0);
            i = i + 1;
        }
        int comps = 0;
        i = 0;
        while (i < n) {
            if (vis[i] == 0) {
                dfs(i, adj, vis);
                comps = comps + 1;
            }
            i = i + 1;
        }
        return comps;
    }
};
int main() {
    int n = 6;
    vector<vector<int>> adj = {{}, {}, {}, {}, {}, {}};
    adj[0].push_back(1);
    adj[1].push_back(0);
    adj[1].push_back(2);
    adj[2].push_back(1);
    adj[3].push_back(4);
    adj[4].push_back(3);
    Solution s;
    return s.countComponents(n, adj);
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::FnEnter { name, .. } if name == "Solution::dfs"
    )));
    // Recursion edges: child dfs.parent_id == parent dfs.call_id
    let dfs_enters: Vec<_> = eng
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::FnEnter {
                name,
                call_id,
                parent_id,
                ..
            } if name == "Solution::dfs" => Some((*call_id, *parent_id)),
            _ => None,
        })
        .collect();
    assert!(dfs_enters.len() >= 2);
    let (root_id, root_parent) = dfs_enters[0];
    assert!(root_parent.is_some()); // parent is countComponents
    assert!(dfs_enters.iter().skip(1).any(|(_, p)| *p == Some(root_id)));
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

#[test]
fn alloc_init_list_includes_size_and_elems() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<vector<int>> adj = {{0, 0, 0}, {0, 0, 0}, {0, 0, 0}};
  return adj.size();
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));

    let vector_allocs: Vec<_> = eng
        .events()
        .iter()
        .filter_map(|e| match e {
            Event::Alloc {
                kind,
                size,
                elems,
                id,
                ..
            } if kind == "vector" => Some((*id, *size, elems.clone())),
            _ => None,
        })
        .collect();

    // three rows + outer adj
    assert_eq!(vector_allocs.len(), 4);
    assert!(vector_allocs.iter().take(3).all(|(_, size, elems)| {
        *size == 3
            && elems
                == &[
                    Value::Int(0),
                    Value::Int(0),
                    Value::Int(0),
                ]
    }));
    let (outer_id, outer_size, outer_elems) = &vector_allocs[3];
    assert_eq!(*outer_size, 3);
    assert_eq!(outer_elems.len(), 3);
    assert!(matches!(outer_elems[0], Value::Object(_)));
    let inner_ids: Vec<_> = vector_allocs.iter().take(3).map(|(id, ..)| *id).collect();
    let pointed: Vec<_> = outer_elems
        .iter()
        .map(|v| match v {
            Value::Object(id) => *id,
            _ => panic!("expected object"),
        })
        .collect();
    assert_eq!(pointed, inner_ids);
    let _ = outer_id;
}

#[test]
fn ternary_and_range_for_and_sort() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(2);
  v.push_back(1);
  sort(v.begin(), v.end());
  int sum = 0;
  for (int x : v) {
    sum = sum + (x == 1 ? 10 : 0);
  }
  return sum + v[0];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(11));
}

#[test]
fn pair_brace_init() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  pair<int, int> p = {1, 2};
  return p.first + p.second;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));
}

#[test]
fn include_paste_ok() {
    let mut eng = Engine::from_source(
        r#"
#include <vector>
#include <map>
int main() {
  map<int, int> m;
  m[1] = 2;
  return m[1];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(2));
}

#[test]
fn leetcode_min_max_abs_and_climits() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int a = min(3, 7);
  int b = max(3, 7);
  int c = std::min(10, std::max(1, 5));
  int d = abs(-42);
  int lo = INT_MAX;
  int hi = INT_MIN;
  return a + b + c + d + (lo > hi);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(58));
}

#[test]
fn int_max_dp_style() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(5);
  v.push_back(2);
  v.push_back(8);
  int ans = INT_MAX;
  int i = 0;
  while (i < v.size()) {
    ans = min(ans, v[i]);
    i = i + 1;
  }
  return ans;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(2));
}

#[test]
fn min_max_emit_builtin_select_with_chosen_span() {
    let src = r#"
int main() {
  int a = 3;
  int b = 7;
  return max(a, b);
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(7));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::BuiltinSelect {
            name,
            chosen: 1,
            value: Value::Int(7),
            ..
        } if name == "max"
    )));
}

#[test]
fn locals_emit_vardestroy_on_scope_exit() {
    let src = r#"
int main() {
  int x = 1;
  {
    int y = 2;
    x = x + y;
  }
  return x;
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(3));
    assert!(eng.events().iter().any(|e| matches!(
        e,
        Event::VarDestroy {
            name,
            value: Value::Int(2),
            ..
        } if name == "y"
    )));
}

#[test]
fn leetcode_algorithms_from_issues_work() {
    let src = r#"
int main() {
  vector<int> v = {5, 2, 8, 2};
  reverse(v.begin(), v.end());
  int s = accumulate(v.begin(), v.end(), 0);
  int lo_idx = lower_bound(v.begin(), v.end(), 2);
  int hi_idx = upper_bound(v.begin(), v.end(), 2);
  bool has8 = binary_search(v.begin(), v.end(), 8);
  int mn = *min_element(v.begin(), v.end());
  int mx = *max_element(v.begin(), v.end());
  int p = __builtin_popcount(7);
  return s + lo_idx + hi_idx + (has8 ? 1 : 0) + mn + mx + p;
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    let _ = eng.run_main().unwrap();
}

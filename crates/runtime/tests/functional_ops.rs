use rscpp_runtime::{Engine, Value};

#[test]
fn greater_construct_and_call() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  auto g = greater<int>();
  return g(3, 1) ? 1 : 0;
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(1));
}

#[test]
fn plus_binary() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  auto p = plus<int>();
  return p(2, 5);
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(7));
}

#[test]
fn sort_with_greater() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(3); v.push_back(2);
  sort(v.begin(), v.end(), greater<int>());
  return v[0] * 100 + v[1] * 10 + v[2];
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(321));
}

#[test]
fn accumulate_with_plus() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  vector<int> v;
  v.push_back(1); v.push_back(2); v.push_back(3);
  return accumulate(v.begin(), v.end(), 0, plus<int>());
}
"#,
    )
    .unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(6));
}

#[test]
fn functional_ops_table() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  int n = 0;
  n += less<int>()(1, 2) ? 1 : 0;
  n += multiplies<int>()(3, 4) == 12 ? 10 : 0;
  n += divides<int>()(8, 2) == 4 ? 100 : 0;
  n += modulus<int>()(7, 4) == 3 ? 1000 : 0;
  n += negate<int>()(5) == -5 ? 10000 : 0;
  n += equal_to<int>()(2, 2) ? 100000 : 0;
  n += not_equal_to<int>()(1, 2) ? 1000000 : 0;
  n += greater_equal<int>()(3, 3) ? 10000000 : 0;
  n += less_equal<int>()(1, 2) ? 100000000 : 0;
  n += logical_and()(1, 1) ? 1000000000 : 0;
  // keep result in 32-bit-ish range; fold remaining into high bits via multiply
  int m = 0;
  m += logical_or()(0, 1) ? 1 : 0;
  m += logical_not()(0) ? 2 : 0;
  m += bit_and()(6, 3) == 2 ? 4 : 0;
  m += bit_or()(1, 2) == 3 ? 8 : 0;
  m += bit_xor()(1, 3) == 2 ? 16 : 0;
  m += bit_not()(0) == -1 ? 32 : 0;
  m += minus<int>()(9, 4) == 5 ? 64 : 0;
  return n + m;
}
"#,
    )
    .unwrap();
    // n = 1+10+100+1000+10000+100000+1000000+10000000+100000000+1000000000 = 1111111111
    // m = 1+2+4+8+16+32+64 = 127
    assert_eq!(eng.run_main().unwrap(), Value::Int(1_111_111_111 + 127));
}

#[test]
fn divides_by_zero_errors() {
    let mut eng = Engine::from_source(
        r#"
int main() {
  return divides<int>()(1, 0);
}
"#,
    )
    .unwrap();
    assert!(eng.run_main().unwrap_err().to_string().contains("division by zero"));
}

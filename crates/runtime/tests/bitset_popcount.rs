use rscpp_runtime::{Engine, Value};

#[test]
fn bitset_index_assign_and_test() {
    let src = r#"
int main() {
  bitset<8> b = 0;
  b[1] = 1;
  b[3] = 1;
  int sum = 0;
  if (b[1]) sum += 1;
  if (b[2]) sum += 10;
  if (b[3]) sum += 100;
  return sum + b.count();
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    // bits 1 and 3 set → sum=101, count=2 → 103
    assert_eq!(eng.run_main().unwrap(), Value::Int(103));
}

#[test]
fn popcount_and_prime_set_bits() {
    let src = r#"
class Solution {
public:
    static int countPrimeSetBits(int left, int right) {
        vector<int> prime={2, 3, 5, 7, 11, 13, 17, 19};
        bitset<21> isPrime=0;
        for(int p: prime) isPrime[p]=1;
        int sum=0;
        for(unsigned i=left; i<=right; i++){
            int b=popcount(i);
            if (isPrime[b]) sum++;
        }
        return sum;
    }
};
"#;
    let mut eng = Engine::from_source(src).unwrap();
    // LeetCode 762 sample: left=6, right=10 → 4
    assert_eq!(
        eng.call(
            "Solution::countPrimeSetBits",
            &[Value::Int(6), Value::Int(10)]
        )
        .unwrap(),
        Value::Int(4)
    );
}

#[test]
fn popcount_builtin() {
    let src = r#"
int main() {
  return popcount(7) + __builtin_popcount(15);
}
"#;
    let mut eng = Engine::from_source(src).unwrap();
    assert_eq!(eng.run_main().unwrap(), Value::Int(7)); // 3 + 4
}

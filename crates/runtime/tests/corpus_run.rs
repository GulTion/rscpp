//! Optional runtime smoke tests against local `testing/` fixtures (gitignored).
//! No-ops when `testing/` is absent so CI without fixtures still passes.

use rscpp_runtime::{args_from_json, Engine, Value};
use serde_json::json;
use std::path::PathBuf;

fn testing_root() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testing");
    root.is_dir().then_some(root)
}

fn load(name: &str) -> String {
    let path = testing_root().expect("testing/").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn call(src: &str, method: &str, args: &serde_json::Value) -> Result<Value, String> {
    let mut eng = Engine::from_source(src).map_err(|e| e.message)?;
    let vals = args_from_json(&mut eng, args).map_err(|e| e.message)?;
    eng.call(method, &vals).map_err(|e| e.message)
}

struct Case {
    file: &'static str,
    method: &'static str,
    args: serde_json::Value,
    check: fn(&Value) -> bool,
    label: &'static str,
}

#[test]
fn testing_folder_run_method_smoke() {
    let Some(_) = testing_root() else {
        eprintln!("skip: testing/ not present");
        return;
    };

    let cases = [
        Case {
            file: "add-two-integers.cpp",
            method: "Solution::sum",
            args: json!([12, 5]),
            check: |v| *v == Value::Int(17),
            label: "17",
        },
        Case {
            file: "add-digits.cpp",
            method: "Solution::addDigits",
            args: json!([38]),
            check: |v| *v == Value::Int(2),
            label: "2",
        },
        Case {
            file: "perfect-number.cpp",
            method: "Solution::checkPerfectNumber",
            args: json!([28]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "two-sum.cpp",
            method: "Solution::twoSum",
            args: json!([[2, 7, 11, 15], 9]),
            check: |v| matches!(v, Value::Object(_)),
            label: "vector",
        },
        Case {
            file: "missing-number.cpp",
            method: "Solution::missingNumber",
            args: json!([[3, 0, 1]]),
            check: |v| *v == Value::Int(2),
            label: "2",
        },
        Case {
            file: "1-bit-and-2-bit-characters.cpp",
            method: "Solution::isOneBitCharacter",
            args: json!([[1, 0, 0]]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "power-of-two.cpp",
            method: "Solution::isPowerOfTwo",
            args: json!([16]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "count-odd-numbers-in-an-interval-range.cpp",
            method: "Solution::countOdds",
            args: json!([3, 7]),
            check: |v| *v == Value::Int(3),
            label: "3",
        },
        Case {
            file: "subtract-the-product-and-sum-of-digits-of-an-integer.cpp",
            method: "Solution::subtractProductAndSum",
            args: json!([234]),
            check: |v| *v == Value::Int(15),
            label: "15",
        },
        Case {
            file: "number-of-steps-to-reduce-a-number-to-zero.cpp",
            method: "Solution::numberOfSteps",
            args: json!([14]),
            check: |v| *v == Value::Int(6),
            label: "6",
        },
        Case {
            file: "search-insert-position.cpp",
            method: "Solution::searchInsert",
            args: json!([[1, 3, 5, 6], 5]),
            check: |v| *v == Value::Int(2),
            label: "2",
        },
        Case {
            file: "plus-one.cpp",
            method: "Solution::plusOne",
            args: json!([[1, 2, 3]]),
            check: |v| matches!(v, Value::Object(_)),
            label: "vector",
        },
        Case {
            file: "jewels-and-stones.cpp",
            method: "Solution::numJewelsInStones",
            args: json!(["aA", "aAAbbbb"]),
            check: |v| *v == Value::Int(3),
            label: "3",
        },
        Case {
            file: "happy-number.cpp",
            method: "Solution::isHappy",
            args: json!([19]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "to-lower-case.cpp",
            method: "Solution::toLowerCase",
            args: json!(["Hello"]),
            check: |v| matches!(v, Value::Object(_)),
            label: "string",
        },
        Case {
            file: "contains-duplicate.cpp",
            method: "Solution::containsDuplicate",
            args: json!([[1, 2, 3, 1]]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "maximum-subarray.cpp",
            method: "Solution::maxSubArray",
            args: json!([[-2, 1, -3, 4, -1, 2, 1, -5, 4]]),
            check: |v| *v == Value::Int(6),
            label: "6",
        },
        Case {
            file: "best-time-to-buy-and-sell-stock.cpp",
            method: "Solution::maxProfit",
            args: json!([[7, 1, 5, 3, 6, 4]]),
            check: |v| *v == Value::Int(5),
            label: "5",
        },
        Case {
            file: "palindrome-number.cpp",
            method: "Solution::isPalindrome",
            args: json!([121]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
        Case {
            file: "reverse-integer.cpp",
            method: "Solution::reverse",
            args: json!([123]),
            check: |v| *v == Value::Int(321),
            label: "321",
        },
        Case {
            file: "running-sum-of-1d-array.cpp",
            method: "Solution::runningSum",
            args: json!([[1, 2, 3, 4]]),
            check: |v| matches!(v, Value::Object(_)),
            label: "vector",
        },
        Case {
            file: "roman-to-integer.cpp",
            method: "Solution::romanToInt",
            args: json!(["III"]),
            check: |v| *v == Value::Int(3),
            label: "3",
        },
        Case {
            file: "move-zeroes.cpp",
            method: "Solution::moveZeroes",
            args: json!([[0, 1, 0, 3, 12]]),
            check: |v| *v == Value::Void || *v == Value::Int(0),
            label: "void",
        },
        Case {
            file: "base-7.cpp",
            method: "Solution::convertToBase7",
            args: json!([100]),
            check: |v| matches!(v, Value::Object(_)),
            label: "string",
        },
        Case {
            file: "circular-sentence.cpp",
            method: "Solution::isCircularSentence",
            args: json!(["leetcode exercises sound delightful"]),
            check: |v| *v == Value::Bool(true),
            label: "true",
        },
    ];

    let mut failed = Vec::new();
    for c in &cases {
        let src = load(c.file);
        match call(&src, c.method, &c.args) {
            Ok(v) if (c.check)(&v) => eprintln!("ok {} -> {}", c.file, c.label),
            Ok(v) => failed.push(format!("{}: got {:?}, want {}", c.file, v, c.label)),
            Err(e) => failed.push(format!("{}: {e}", c.file)),
        }
    }

    assert!(
        failed.is_empty(),
        "{} failure(s):\n{}",
        failed.len(),
        failed.join("\n")
    );
}

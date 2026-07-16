use rscpp_runtime::{args_from_json, Engine};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Case {
    file: String,
    method: String,
    args: serde_json::Value,
}

#[test]
#[ignore = "optional: PROBE_LIMIT batch over local testing/; run with --ignored"]
fn batch_probe_testing() {
    let cases: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string("/tmp/testing_probe_fast.json").unwrap())
            .unwrap();
    let limit = 250usize;
    let fuel = 5000u64;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../testing");
    let mut ok = 0;
    let mut load_fail = 0;
    let mut run_fail = 0;
    let mut buckets: HashMap<String, (usize, String)> = HashMap::new();
    let mut undef: HashMap<String, (usize, String)> = HashMap::new();
    for c in cases.into_iter().take(limit) {
        let src = match std::fs::read_to_string(root.join(&c.file)) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let mut eng = match Engine::from_source_with_fuel(&src, fuel) {
            Ok(e) => e,
            Err(e) => {
                load_fail += 1;
                let key: String = e.message.chars().take(50).collect();
                buckets
                    .entry(format!("LOAD:{key}"))
                    .or_insert((0, c.file.clone()))
                    .0 += 1;
                continue;
            }
        };
        let args = match args_from_json(&mut eng, &c.args) {
            Ok(a) => a,
            Err(_) => {
                run_fail += 1;
                continue;
            }
        };
        match eng.call(&c.method, &args) {
            Ok(_) => ok += 1,
            Err(e) => {
                run_fail += 1;
                if e.message.starts_with("undefined function") {
                    undef
                        .entry(e.message.clone())
                        .or_insert((0, c.file.clone()))
                        .0 += 1;
                } else {
                    let key: String = e.message.chars().take(60).collect();
                    buckets
                        .entry(format!("RUN:{key}"))
                        .or_insert((0, c.file.clone()))
                        .0 += 1;
                }
            }
        }
    }
    let mut u: Vec<_> = undef.into_iter().collect();
    u.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    let mut b: Vec<_> = buckets.into_iter().collect();
    b.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    let mut out = format!("ok={ok} load={load_fail} run={run_fail}\nUNDEF:\n");
    for (m, (n, s)) in u.into_iter().take(25) {
        out += &format!("{n:3} {s} :: {m}\n");
    }
    out += "OTHER:\n";
    for (m, (n, s)) in b.into_iter().take(25) {
        out += &format!("{n:3} {s} :: {m}\n");
    }
    std::fs::write("/tmp/testing_probe_report.txt", &out).unwrap();
    eprintln!("{out}");
}

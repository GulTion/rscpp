
fn main() {
    let src = std::fs::read_to_string("examples/two_sum.cpp").expect("two_sum.cpp");
    let args = serde_json::json!([[2, 7, 11, 15], 9]);
    let r = rscpp_wasm::run_method_result(&src, "Solution::twoSum", &args);
    assert!(r.ok, "{:?}", r.error);
    let out = serde_json::json!({
        "source": src,
        "method": "Solution::twoSum",
        "args": [[2, 7, 11, 15], 9],
        "ok": r.ok,
        "value": r.value,
        "events": r.events,
    });
    let path = "packages/timeline/src/fixtures/two_sum.json";
    std::fs::create_dir_all("packages/timeline/src/fixtures").ok();
    std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("wrote {} events to {path}", r.events.len());
}

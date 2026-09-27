//! Dump complex example fixtures for the visualizer demo.

fn write_fixture(path: &str, source: &str, method: &str, args: serde_json::Value) {
    let r = rscpp_wasm::run_method_result(source, method, &args);
    assert!(r.ok, "{path}: {:?}", r.error);
    let out = serde_json::json!({
        "source": source,
        "method": method,
        "args": args,
        "ok": r.ok,
        "value": r.value,
        "events": r.events,
    });
    std::fs::create_dir_all("packages/timeline/src/fixtures").ok();
    std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("wrote {} events → {path}", r.events.len());
}

fn write_main_fixture(path: &str, source_path: &str) {
    let source = std::fs::read_to_string(source_path).expect(source_path);
    let r = rscpp_wasm::run_result(&source);
    assert!(r.ok, "{path}: {:?}", r.error);
    let out = serde_json::json!({
        "source": source,
        "method": null,
        "args": null,
        "ok": r.ok,
        "value": r.value,
        "events": r.events,
    });
    std::fs::create_dir_all("packages/timeline/src/fixtures").ok();
    std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    eprintln!("wrote {} events → {path}", r.events.len());
}

fn main() {
    let dfs = std::fs::read_to_string("examples/dfs.cpp").unwrap();
    // Graph: 0—1—2   3—4   5  → 3 components
    write_fixture(
        "packages/timeline/src/fixtures/dfs.json",
        &dfs,
        "Solution::countComponents",
        serde_json::json!([6, [[1], [0, 2], [1], [4], [3], []]]),
    );

    let paren = std::fs::read_to_string("examples/valid_parentheses.cpp").unwrap();
    write_fixture(
        "packages/timeline/src/fixtures/valid_parentheses.json",
        &paren,
        "Solution::isValid",
        serde_json::json!(["({[]})()[]{}"]),
    );

    let nqueens = std::fs::read_to_string("examples/n_queens.cpp").unwrap();
    write_fixture(
        "packages/timeline/src/fixtures/n_queens.json",
        &nqueens,
        "Solution::solveNQueens",
        serde_json::json!([4]),
    );

    // Full mains — more events (calls from main + setup)
    write_main_fixture(
        "packages/timeline/src/fixtures/dfs_main.json",
        "examples/dfs.cpp",
    );
    write_main_fixture(
        "packages/timeline/src/fixtures/valid_parentheses_main.json",
        "examples/valid_parentheses.cpp",
    );
    // n_queens main hits a temporary-lifetime edge case; method fixture is enough for the demo.
}

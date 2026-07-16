# Corpus Error Catalog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `rscpp-corpus`, a CLI that batch-scans `testing/*.cpp` (parse + sema only), processes one file at a time with `--offset`/`--limit`, and prints a ranked error catalog (optional JSON `--out`).

**Architecture:** Thin binary crate. Library logic in `crates/corpus/src/lib.rs` (list/slice files, run parse+sema, aggregate groups) so unit tests use small fixtures under `crates/corpus/tests/fixtures/` — never depend on the gitignored `testing/` tree. `main.rs` only parses CLI flags and prints/writes the report.

**Tech Stack:** Rust 2021 workspace crate, `rscpp-parser`, `rscpp-sema`, std only (no clap — match `rscpp-pipeline` hand-rolled args).

## Global Constraints

- Pipeline depth: **parse + sema only** (no runtime/vm).
- Memory: **one file at a time**; never preload the corpus.
- Defaults: `--dir testing`, `--limit 50`, `--offset 0`.
- File order: lexicographic sort of `*.cpp` paths (flat dir only in v1).
- Exit code `0` on completed scan even if files fail; non-zero only for tool errors.
- Sema: record **first** error only per file.
- Grouping key: `(kind, message)` where `kind` is `"parse"` | `"sema"`.
- `testing/` is gitignored — fixtures for tests live in-crate.

---

## File Structure

| Path | Responsibility |
|------|----------------|
| `crates/corpus/Cargo.toml` | Package `rscpp-corpus`, bin + lib, deps |
| `crates/corpus/src/lib.rs` | Types + `list_cpp_files`, `select_batch`, `check_file`, `run_corpus`, `format_report`, `report_to_json` |
| `crates/corpus/src/main.rs` | CLI arg parse, call `run_corpus`, print + optional write |
| `crates/corpus/tests/fixtures/*.cpp` | Tiny ok / parse-fail / sema-fail samples |
| `crates/corpus/tests/corpus.rs` | Integration tests for batch + ranking |
| `Cargo.toml` (workspace) | Add `crates/corpus` member |
| `README.md` | One-liner pointing at corpus CLI |
| `docs/wasm-build.md` | untouched |

---

### Task 1: Scaffold crate + file listing / batch select

**Files:**
- Create: `crates/corpus/Cargo.toml`
- Create: `crates/corpus/src/lib.rs`
- Create: `crates/corpus/src/main.rs` (stub `fn main() {}` for now)
- Create: `crates/corpus/tests/fixtures/ok.cpp`
- Create: `crates/corpus/tests/fixtures/parse_bad.cpp`
- Create: `crates/corpus/tests/fixtures/sema_bad.cpp`
- Create: `crates/corpus/tests/corpus.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces:**
- Produces:
  - `pub fn list_cpp_files(dir: &Path) -> Result<Vec<PathBuf>, String>`
  - `pub fn select_batch(files: &[PathBuf], offset: usize, limit: usize) -> Vec<PathBuf>`

- [ ] **Step 1: Add workspace member and crate files**

Root `Cargo.toml` — add `"crates/corpus"` to `members`.

`crates/corpus/Cargo.toml`:

```toml
[package]
name = "rscpp-corpus"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Batch parse+sema scanner for testing/ corpus; ranked error catalog"

[lib]
path = "src/lib.rs"

[[bin]]
name = "rscpp-corpus"
path = "src/main.rs"

[dependencies]
rscpp-parser = { path = "../parser" }
rscpp-sema = { path = "../sema" }
```

Stub `main.rs`:

```rust
fn main() {}
```

Fixtures:

`ok.cpp`:
```cpp
class Solution {
public:
    int foo() { return 1; }
};
```

`parse_bad.cpp`:
```cpp
class Solution {
public:
    int foo( {
};
```

`sema_bad.cpp` (parses but fails sema — use undefined name):
```cpp
class Solution {
public:
    int foo() { return unknown_name; }
};
```

- [ ] **Step 2: Write failing tests for list + batch**

`crates/corpus/tests/corpus.rs`:

```rust
use rscpp_corpus::{list_cpp_files, select_batch};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn list_cpp_files_sorted() {
    let mut files = list_cpp_files(&fixtures_dir()).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(names.iter().any(|n| n == "ok.cpp"));
}

#[test]
fn select_batch_offset_limit() {
    let files = list_cpp_files(&fixtures_dir()).unwrap();
    let batch = select_batch(&files, 0, 2);
    assert_eq!(batch.len(), 2);
    let batch2 = select_batch(&files, 100, 10);
    assert!(batch2.is_empty());
}
```

- [ ] **Step 3: Run tests — expect fail**

Run: `cargo test -p rscpp-corpus --test corpus list_cpp_files_sorted -- --nocapture`

Expected: compile fail or link fail (`list_cpp_files` not found).

- [ ] **Step 4: Implement list + select**

`crates/corpus/src/lib.rs`:

```rust
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_cpp_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let rd = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for ent in rd {
        let ent = ent.map_err(|e| e.to_string())?;
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) == Some("cpp") {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn select_batch(files: &[PathBuf], offset: usize, limit: usize) -> Vec<PathBuf> {
    files.iter().skip(offset).take(limit).cloned().collect()
}
```

- [ ] **Step 5: Run tests — expect pass**

Run: `cargo test -p rscpp-corpus --test corpus`

Expected: `list_cpp_files_sorted` and `select_batch_offset_limit` PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml crates/corpus
git commit -m "Add rscpp-corpus crate with sorted file listing and batch select."
```

---

### Task 2: Per-file check + corpus run + ranked groups

**Files:**
- Modify: `crates/corpus/src/lib.rs`
- Modify: `crates/corpus/tests/corpus.rs`

**Interfaces:**
- Consumes: `list_cpp_files`, `select_batch`
- Produces:
  - `pub enum FileKind { Ok, Parse, Sema }`
  - `pub struct FileResult { pub path: PathBuf, pub kind: FileKind, pub message: Option<String>, pub span_start: Option<usize>, pub span_end: Option<usize> }`
  - `pub struct ErrorGroup { pub kind: String, pub message: String, pub count: usize, pub samples: Vec<String> }`
  - `pub struct CorpusReport { pub dir: String, pub offset: usize, pub limit: usize, pub processed: usize, pub ok: usize, pub fail: usize, pub groups: Vec<ErrorGroup> }`
  - `pub fn check_file(path: &Path) -> Result<FileResult, String>`
  - `pub fn run_corpus(dir: &Path, offset: usize, limit: usize) -> Result<CorpusReport, String>`

- [ ] **Step 1: Write failing tests for check + ranking**

Append to `crates/corpus/tests/corpus.rs`:

```rust
use rscpp_corpus::{check_file, run_corpus, FileKind};

#[test]
fn check_file_ok_parse_sema() {
    let dir = fixtures_dir();
    let ok = check_file(&dir.join("ok.cpp")).unwrap();
    assert!(matches!(ok.kind, FileKind::Ok));

    let bad = check_file(&dir.join("parse_bad.cpp")).unwrap();
    assert!(matches!(bad.kind, FileKind::Parse));
    assert!(bad.message.is_some());

    let sema = check_file(&dir.join("sema_bad.cpp")).unwrap();
    assert!(matches!(sema.kind, FileKind::Sema));
    assert!(sema.message.is_some());
}

#[test]
fn run_corpus_ranks_errors() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    assert_eq!(report.processed, 3);
    assert_eq!(report.ok, 1);
    assert_eq!(report.fail, 2);
    assert!(!report.groups.is_empty());
    assert!(report.groups.iter().all(|g| g.count >= 1));
    // groups sorted by count descending
    for w in report.groups.windows(2) {
        assert!(w[0].count >= w[1].count);
    }
}
```

- [ ] **Step 2: Run tests — expect fail**

Run: `cargo test -p rscpp-corpus --test corpus check_file_ok_parse_sema`

Expected: FAIL (symbols missing).

- [ ] **Step 3: Implement check_file + run_corpus**

Append to `lib.rs`:

```rust
use rscpp_parser::parse;
use rscpp_sema::analyze;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileKind {
    Ok,
    Parse,
    Sema,
}

#[derive(Debug, Clone)]
pub struct FileResult {
    pub path: PathBuf,
    pub kind: FileKind,
    pub message: Option<String>,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ErrorGroup {
    pub kind: String,
    pub message: String,
    pub count: usize,
    pub samples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CorpusReport {
    pub dir: String,
    pub offset: usize,
    pub limit: usize,
    pub processed: usize,
    pub ok: usize,
    pub fail: usize,
    pub groups: Vec<ErrorGroup>,
}

pub fn check_file(path: &Path) -> Result<FileResult, String> {
    let src = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    match parse(&src) {
        Err(e) => Ok(FileResult {
            path: path.to_path_buf(),
            kind: FileKind::Parse,
            message: Some(e.message),
            span_start: Some(e.span.start),
            span_end: Some(e.span.end),
        }),
        Ok(tu) => {
            let sema = analyze(&tu);
            if sema.ok() {
                Ok(FileResult {
                    path: path.to_path_buf(),
                    kind: FileKind::Ok,
                    message: None,
                    span_start: None,
                    span_end: None,
                })
            } else {
                let e = &sema.errors[0];
                Ok(FileResult {
                    path: path.to_path_buf(),
                    kind: FileKind::Sema,
                    message: Some(e.message.clone()),
                    span_start: Some(e.span.start),
                    span_end: Some(e.span.end),
                })
            }
        }
    }
}

pub fn run_corpus(dir: &Path, offset: usize, limit: usize) -> Result<CorpusReport, String> {
    let files = list_cpp_files(dir)?;
    let batch = select_batch(&files, offset, limit);
    let mut ok = 0usize;
    let mut fail = 0usize;
    // key: (kind_str, message)
    let mut map: HashMap<(String, String), Vec<String>> = HashMap::new();

    for path in &batch {
        let res = check_file(path)?;
        match res.kind {
            FileKind::Ok => ok += 1,
            FileKind::Parse | FileKind::Sema => {
                fail += 1;
                let kind = match res.kind {
                    FileKind::Parse => "parse".to_string(),
                    FileKind::Sema => "sema".to_string(),
                    FileKind::Ok => unreachable!(),
                };
                let msg = res.message.unwrap_or_default();
                let sample = path.display().to_string();
                map.entry((kind, msg)).or_default().push(sample);
            }
        }
    }

    let mut groups: Vec<ErrorGroup> = map
        .into_iter()
        .map(|((kind, message), samples)| {
            let count = samples.len();
            let samples: Vec<_> = samples.into_iter().take(3).collect();
            ErrorGroup {
                kind,
                message,
                count,
                samples,
            }
        })
        .collect();
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.message.cmp(&b.message)));

    Ok(CorpusReport {
        dir: dir.display().to_string(),
        offset,
        limit,
        processed: batch.len(),
        ok,
        fail,
        groups,
    })
}
```

- [ ] **Step 4: Run tests — expect pass**

Run: `cargo test -p rscpp-corpus`

Expected: all PASS. If `sema_bad.cpp` unexpectedly passes sema, adjust fixture until `FileKind::Sema` (e.g. call an undefined free function).

- [ ] **Step 5: Commit**

```bash
git add crates/corpus
git commit -m "Add corpus parse+sema checking and ranked error groups."
```

---

### Task 3: Report formatting + JSON + CLI

**Files:**
- Modify: `crates/corpus/src/lib.rs`
- Modify: `crates/corpus/src/main.rs`
- Modify: `crates/corpus/tests/corpus.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: `CorpusReport`, `run_corpus`
- Produces:
  - `pub fn format_report(report: &CorpusReport) -> String`
  - `pub fn report_to_json(report: &CorpusReport) -> String` (hand-built JSON; no serde dep required)

- [ ] **Step 1: Write failing tests for formatters**

```rust
use rscpp_corpus::{format_report, report_to_json, run_corpus};

#[test]
fn format_report_contains_summary() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    let text = format_report(&report);
    assert!(text.contains("ok:"));
    assert!(text.contains("fail:"));
    assert!(text.contains("Top errors"));
}

#[test]
fn report_to_json_round_shape() {
    let report = run_corpus(&fixtures_dir(), 0, 50).unwrap();
    let json = report_to_json(&report);
    assert!(json.contains("\"processed\""));
    assert!(json.contains("\"groups\""));
}
```

- [ ] **Step 2: Run tests — expect fail**

Run: `cargo test -p rscpp-corpus --test corpus format_report_contains_summary`

Expected: FAIL (missing symbols).

- [ ] **Step 3: Implement formatters**

```rust
pub fn format_report(report: &CorpusReport) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "=== corpus: {} files (dir={}, offset={}, limit={}) ===\n",
        report.processed, report.dir, report.offset, report.limit
    ));
    s.push_str(&format!("ok: {}  fail: {}\n\n", report.ok, report.fail));
    s.push_str("Top errors:\n");
    for g in report.groups.iter().take(20) {
        s.push_str(&format!("  {}×  [{}] {}\n", g.count, g.kind, g.message));
        if let Some(sample) = g.samples.first() {
            s.push_str(&format!("         e.g. {sample}\n"));
        }
    }
    s
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out
}

pub fn report_to_json(report: &CorpusReport) -> String {
    let mut groups = String::new();
    for (i, g) in report.groups.iter().enumerate() {
        if i > 0 {
            groups.push(',');
        }
        let samples: Vec<_> = g
            .samples
            .iter()
            .map(|p| format!("\"{}\"", json_escape(p)))
            .collect();
        groups.push_str(&format!(
            "{{\"kind\":\"{}\",\"message\":\"{}\",\"count\":{},\"samples\":[{}]}}",
            json_escape(&g.kind),
            json_escape(&g.message),
            g.count,
            samples.join(",")
        ));
    }
    format!(
        "{{\"run\":{{\"dir\":\"{}\",\"offset\":{},\"limit\":{},\"processed\":{}}},\"summary\":{{\"ok\":{},\"fail\":{}}},\"groups\":[{}]}}",
        json_escape(&report.dir),
        report.offset,
        report.limit,
        report.processed,
        report.ok,
        report.fail,
        groups
    )
}
```

- [ ] **Step 4: Implement CLI `main.rs`**

```rust
use rscpp_corpus::{format_report, report_to_json, run_corpus};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "Usage: rscpp-corpus [--dir testing] [--limit 50] [--offset 0] [--out report.json]"
        );
        return ExitCode::from(2);
    }

    let mut dir = PathBuf::from("testing");
    let mut limit: usize = 50;
    let mut offset: usize = 0;
    let mut out: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                dir = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--limit" => {
                limit = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(50);
                i += 2;
            }
            "--offset" => {
                offset = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                i += 2;
            }
            "--out" => {
                out = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::from(2);
            }
        }
    }
    let _ = &mut args;

    let report = match run_corpus(&dir, offset, limit) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    if report.processed == 0 {
        println!(
            "no files in batch (dir={}, offset={}, limit={})",
            dir.display(),
            offset,
            limit
        );
        return ExitCode::SUCCESS;
    }

    print!("{}", format_report(&report));

    if let Some(path) = out {
        if let Err(e) = fs::write(&path, report_to_json(&report)) {
            eprintln!("failed to write {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
        println!("\nwrote {}", path.display());
    }

    ExitCode::SUCCESS
}
```

- [ ] **Step 5: Manual smoke on real corpus (if present)**

Run: `cargo run -p rscpp-corpus -- --dir testing --limit 5`

Expected: prints `=== corpus: …` with ok/fail and top errors (or “no files” if `testing/` missing).

Run: `cargo test -p rscpp-corpus`

Expected: all PASS.

- [ ] **Step 6: README one-liner**

In `README.md` Develop section, after pipeline / before or after wasm:

```markdown
# Corpus error catalog (parse+sema, batched)
cargo run -p rscpp-corpus -- --dir testing --limit 50
# see docs/superpowers/specs/2026-07-16-rscpp-corpus-design.md
```

- [ ] **Step 7: Commit**

```bash
git add crates/corpus README.md
git commit -m "Wire rscpp-corpus CLI with text/JSON reports and README usage."
```

---

## Spec coverage (self-review)

| Spec requirement | Task |
|------------------|------|
| Batch `--limit`/`--offset`, sorted | Task 1 |
| One file at a time parse+sema | Task 2 |
| Ranked catalog + samples | Task 2–3 |
| Optional JSON `--out` | Task 3 |
| Defaults dir/limit/offset | Task 3 |
| Exit 0 on scan complete | Task 3 |
| New workspace crate | Task 1 |
| README pointer | Task 3 |
| No runtime/vm | All (deps only parser+sema) |

**Placeholder scan:** none.  
**Type consistency:** `FileKind` / `CorpusReport` / `ErrorGroup` names match across tasks.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-16-rscpp-corpus.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks  
2. **Inline Execution** — execute tasks in this session with checkpoints  

Which approach?

# Corpus error catalog (rscpp-corpus)

**Date:** 2026-07-16  
**Status:** approved  
**Goal:** Drive an implement-what’s-missing loop over LeetCode-style C++ files in `testing/` without loading the whole corpus.

## Context

- `testing/` holds ~3500+ solution files (`class Solution { ... }`, almost no `main`).
- Existing `cargo test` crates cover unit behavior; they do not scan this corpus.
- v1 does **not** execute methods or check return values — only **parse + sema**, then catalog errors.

## Requirements

1. Process a **batch** of files (default 50), not the entire folder every run.
2. Stream **one file at a time** (read → parse → analyze → drop); never preload all sources.
3. Deterministic file order (sorted by path) with `--offset` / `--limit` slicing.
4. Emit a ranked error catalog (unique message → count → sample paths).
5. Optional JSON report for tooling / later UI.
6. New workspace crate; no dependency on runtime/vm for v1.

## Non-goals (v1)

- Calling `Solution::*` or inventing test args
- Expected-output / golden tests
- Default full-corpus scan
- WASM packaging of the corpus tool
- Auto-fixing code from errors (humans implement; tool only reports)

## Design

### Crate

- Path: `crates/corpus`
- Package name: `rscpp-corpus`
- Binary: `rscpp-corpus` (default bin from `src/main.rs`)
- Dependencies: `rscpp-parser`, `rscpp-sema` (and transitive `rscpp-ast` / lexer as needed)
- Workspace: add to root `Cargo.toml` `members`

### CLI

```bash
cargo run -p rscpp-corpus -- [OPTIONS]
```

| Flag | Default | Meaning |
|------|---------|---------|
| `--dir <path>` | `testing` | Directory of `.cpp` files (non-recursive is enough if all files are flat) |
| `--limit <n>` | `50` | Max files to process this run |
| `--offset <n>` | `0` | Skip first N entries of the sorted list |
| `--out <path>` | *(none)* | If set, write JSON report to this path |
| `-h` / `--help` | | Usage |

**File selection**

1. Collect `*.cpp` under `--dir` (v1: flat directory only, matching current `testing/` layout).
2. Sort paths lexicographically.
3. Take slice `[offset, offset + limit)`.
4. If the slice is empty, exit 0 with a short message.

**Per-file pipeline**

1. `fs::read_to_string`
2. `rscpp_parser::parse`
3. On parse success: `rscpp_sema::analyze`
4. Record outcome; drop AST / source before next file

**Outcomes**

- `Ok` — parse succeeded and `sema.ok()`
- `ParseError { message, span? }` — from parser / lexer via `ParseError` Display / fields
- `SemaError { message, span? }` — **first** sema error only (v1; enough for ranking)

Exit code:

- `0` if the run completed (even if some files failed — failures are the product)
- non-zero only for tool failures (bad args, unreadable `--dir`, write failure for `--out`)

### Report

**Stdout (always)**

```text
=== corpus: <n> files (dir=..., offset=..., limit=...) ===
ok: <a>  fail: <b>

Top errors:
  14×  <message>
         e.g. testing/foo.cpp
   8×  <message>
         e.g. testing/bar.cpp
```

Show up to ~20 groups (or all if fewer); each group lists up to 3 sample paths.

**JSON (`--out`)**

```json
{
  "run": { "dir": "testing", "offset": 0, "limit": 50, "processed": 50 },
  "summary": { "ok": 12, "fail": 38 },
  "groups": [
    {
      "kind": "parse" | "sema",
      "message": "...",
      "count": 14,
      "samples": ["testing/foo.cpp", "..."]
    }
  ]
}
```

Grouping key: `(kind, message)` with message taken as-is from the first error (no aggressive normalization in v1).

### Memory / performance

- One source string + one AST at a time.
- Default `--limit 50` keeps a full pass intentional (`--limit 10000` or repeated `--offset` walks).
- No parallel workers in v1 (simpler; optional later).

### Regression (light)

Earlier product choice was “CLI + light allowlist.” For this approved slice (**Approach A**), v1 ships **CLI only**. A tiny allowlist `cargo test` can be a fast follow-up without changing the catalog design.

### Workflow

1. `cargo run -p rscpp-corpus -- --limit 50`
2. Implement support for the top error in parser/sema
3. Re-run the same `--offset`/`--limit` (or advance offset)
4. Optionally write `testing/report.json` for inspection

## Open questions resolved

| Question | Decision |
|----------|----------|
| Pass criteria | Error catalog only (not correctness) |
| Pipeline depth | Parse + sema |
| Delivery | Streaming batch CLI (Approach A) |
| Corpus size | Batch with `--limit` / `--offset`; no full preload |

## Success criteria

- [ ] `cargo run -p rscpp-corpus -- --limit 5` finishes quickly and prints ranked errors
- [ ] Processing 50 files never requires loading all ~3500 sources
- [ ] Same `--dir/--offset/--limit` produces the same file set (sorted)
- [ ] Documented in README with a one-liner pointing at this tool

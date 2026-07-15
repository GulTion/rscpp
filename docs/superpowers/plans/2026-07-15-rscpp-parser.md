# rscpp Parser Implementation Plan

> **For agentic workers:** Implement task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Add `rscpp-ast` and `rscpp-parser` so LeetCode-subset C++ source parses to a typed AST.

**Architecture:** Hand-written recursive descent + Pratt expressions; AST crate holds types only; parser crate owns parsing.

**Tech Stack:** Rust 2021, workspace crates, `rscpp-lexer` dependency, no parser-generator crates.

## Global Constraints

- No mechanical JSCPP port
- No new third-party parse deps
- LeetCode subset only
- Every AST node carries `Span`
- `cargo test -p rscpp-ast -p rscpp-parser` must pass

---

### Task 1: Scaffold crates

- [ ] Add `crates/ast` and `crates/parser` to workspace
- [ ] Wire dependencies: parser → ast + lexer
- [ ] Commit scaffold if isolated; otherwise continue

### Task 2: AST types

- [ ] Define `TranslationUnit`, `Item`, `Stmt`, `Expr`, `Type`, `Decl`, helpers
- [ ] `cargo test -p rscpp-ast` (smoke / construction tests)

### Task 3: Parser skeleton

- [ ] `Parser` over `&[Token]` with peek/bump/expect
- [ ] `parse(source)` = tokenize + parse translation unit
- [ ] Empty / trivial TU tests

### Task 4: Expressions (Pratt)

- [ ] Prefix, infix, postfix (call, index, member)
- [ ] Tests for precedence (`1+2*3`, `a=b=c`, `a->b().c`)

### Task 5: Statements + declarations + items

- [ ] Blocks, control flow, return/break/continue
- [ ] Simple decls and function/class items, `using namespace`
- [ ] Types: builtins, `T*`, `T&`, `vector<int>`, nested `>>`
- [ ] LeetCode-like `Solution::twoSum` snippet smoke test

### Task 6: Verify + commit

- [ ] `cargo test` workspace green
- [ ] Update README roadmap
- [ ] Commit

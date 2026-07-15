# rscpp Lexer Design

**Date:** 2026-07-15  
**Crate:** `rscpp-lexer`  
**Phase:** 1 of 9

## Problem

JSCPP has no separate lexer. Tokenization is fused into a scannerless PEG grammar (`pegjs/ast.pegjs`). That works in JavaScript but mixes concerns, makes unit testing hard, and does not map cleanly to a typed Rust/WASM interpreter.

## Goals

- Turn UTF-8 C++ source into a flat token stream with byte spans.
- Support the LeetCode-common C++ subset.
- Stay free of the UI; pure `tokenize(source) -> Result<Vec<Token>, LexError>`.
- No mechanical port of JSCPP.

## Non-goals (this phase)

- Preprocessor (`#include`, macros, `#if`)
- Parser / AST
- Digraphs, trigraphs, UCNs
- Alternative operator keywords (`and`, `or`, `not`)
- Template `>>` disambiguation (parser concern)
- Wide / raw string literals (clear error if encountered)

## Design

### API

```rust
pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError>;
```

Every token carries a `Span { start, end }` of byte offsets into the original source for diagnostics and future visualizer mapping.

### Token model

- `Keyword` — fixed set used in LeetCode solutions
- `Ident(String)` — everything else that looks like an identifier (interning later)
- Integer / float literals with decoded values and optional suffixes
- Char / string literals with standard escapes
- `Punct` — operators and punctuation via maximal munch
- `#` as `Punct::Hash` so a future preprocessor can start from tokens
- `Eof` terminates the stream

Comments and whitespace are skipped (spans still advance correctly).

### Scanner

Hand-written byte cursor (`peek` / `bump`). Reasons over `logos` or PEG:

1. Maximal munch and C++ edge cases stay explicit.
2. Zero dependencies — WASM-friendly.
3. Tokenization is independently testable.

### Why better than JSCPP

| JSCPP | rscpp |
|-------|--------|
| Scannerless PEG | Explicit `TokenKind` + spans |
| Positions bolted onto AST objects | `Span` on every token |
| Dynamic JS structures | Copy-friendly Rust enums |
| Cannot test tokens alone | Pure function + unit tests |

## Error handling

`LexError { span, message }` for:

- Unterminated string, char, or block comment
- Bad escape / invalid numeric literal
- Unsupported wide/raw string prefixes (`L"`, `u8"`, `R"`, …)

## Future

Next crates in the workspace: preprocessor (optional feed into lexer), parser, AST, sema, runtime, STL, memory, VM, WASM.

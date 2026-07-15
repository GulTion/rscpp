# rscpp Parser + AST Design

**Date:** 2026-07-15  
**Crates:** `rscpp-ast`, `rscpp-parser`  
**Phase:** 2 (+ AST types as 3, same delivery)

## Problem

JSCPP builds an AST via a scannerless PEG grammar with dynamic objects. We already have a typed lexer; we need a typed AST and a parser that consumes tokens, not characters.

## Goals

- Typed AST with spans on every node.
- `parse(source) -> Result<TranslationUnit, ParseError>` for the LeetCode C++ subset.
- Recursive descent for declarations/statements; Pratt/precedence climbing for expressions.
- Independent of UI and runtime.

## Non-goals

- Preprocessor
- Full C++ declarator grammar
- Lambdas, exceptions, coroutines, concepts
- `new`/`delete` full expressions (defer)
- Complete template metaprogramming (only simple type template-ids like `vector<int>`)

## Architecture

```
source → rscpp-lexer → tokens → rscpp-parser → rscpp-ast::TranslationUnit
```

- **`rscpp-ast`**: data only (enums/structs). No parsing logic.
- **`rscpp-parser`**: depends on `rscpp-lexer` + `rscpp-ast`.

## AST shape (summary)

- `TranslationUnit { items: Vec<Item> }`
- `Item`: `Class`, `Function`, `UsingNamespace`, `Decl`
- `Stmt`: block, if/else, while, do-while, for, return/break/continue, expr, decl
- `Expr`: literals, name/path, unary/binary/assign, call, index, member (`.`/`->`)
- `Type`: builtin, named (+ optional template args), pointer, reference, const

## Why better than JSCPP

| JSCPP | rscpp |
|-------|--------|
| PEG fused with scanning | Token stream from dedicated lexer |
| Untyped `{type: ...}` objects | Exhaustive Rust enums |
| 15 expression nonterminals | One Pratt table |
| Hard to unit-test mid-parse | `parse` + focused expression/stmt tests |

## Error model

`ParseError { span, message }` — unexpected token, missing `)`, bad declarator, etc.

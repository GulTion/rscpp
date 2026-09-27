//! Dump each rscpp pipeline stage for a C++ source file.
//!
//! Usage:
//!   cargo run -p rscpp-pipeline -- path/to/main.cpp
//!   cargo run -p rscpp-pipeline -- path/to/main.cpp --phase lex

use rscpp_ast::{Expr, Item, Stmt};
use rscpp_lexer::{tokenize, TokenKind};
use rscpp_parser::parse;
use rscpp_runtime::{Engine, Event, Value};
use rscpp_sema::analyze;
use rscpp_vm::{compile, Vm};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("Usage: rscpp-pipeline <file.cpp> [--phase all|lex|parse|sema|run|vm|events]");
        return ExitCode::from(2);
    }

    let mut phase = "all".to_string();
    if let Some(i) = args.iter().position(|a| a == "--phase") {
        phase = args.get(i + 1).cloned().unwrap_or_else(|| "all".into());
        args.drain(i..=(i + 1).min(args.len() - 1));
    }

    let path = &args[0];
    let src = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!("=== file: {path} ({} bytes) ===\n", src.len());

    let want = |name: &str| phase == "all" || phase == name || (phase == "events" && name == "run");

    if want("lex") {
        section("1. LEXER");
        match tokenize(&src) {
            Ok(toks) => {
                for (i, t) in toks.iter().enumerate() {
                    if matches!(t.kind, TokenKind::Eof) {
                        println!("{i:>3}  Eof @ {}..{}", t.span.start, t.span.end);
                        continue;
                    }
                    println!(
                        "{i:>3}  {:?} @ {}..{}",
                        shorten_kind(&t.kind),
                        t.span.start,
                        t.span.end
                    );
                }
                println!("→ {} tokens (including Eof)\n", toks.len());
            }
            Err(e) => println!("LEX ERROR: {e}\n"),
        }
    }

    let tu = if want("parse") || want("sema") || want("run") || want("vm") || want("events") {
        section("2. PARSER (AST)");
        match parse(&src) {
            Ok(tu) => {
                println!("TranslationUnit with {} top-level item(s):", tu.items.len());
                for (i, item) in tu.items.iter().enumerate() {
                    println!("  [{i}] {}", summarize_item(item));
                }
                println!();
                Some(tu)
            }
            Err(e) => {
                println!("PARSE ERROR: {e}\n");
                None
            }
        }
    } else {
        None
    };

    if want("sema") {
        section("3. SEMANTIC ANALYSIS");
        if let Some(tu) = &tu {
            let r = analyze(tu);
            if r.ok() {
                println!("OK — no semantic errors\n");
            } else {
                println!("{} error(s):", r.errors.len());
                for e in &r.errors {
                    println!("  - {e}");
                }
                println!();
            }
        } else {
            println!("skipped (parse failed)\n");
        }
    }

    if want("run") || want("events") {
        section("4. RUNTIME (tree-walker)");
        match Engine::from_source(&src) {
            Ok(mut eng) => match eng.run_main() {
                Ok(v) => {
                    println!("main() → {v:?}");
                    let events = eng.events();
                    println!("events: {} total", events.len());
                    if want("events") || phase == "all" {
                        print_events(events, if phase == "all" { 40 } else { usize::MAX });
                    }
                    println!();
                }
                Err(e) => println!("RUNTIME ERROR: {e}\n"),
            },
            Err(e) => println!("RUNTIME LOAD ERROR: {e}\n"),
        }
    }

    if want("vm") {
        section("5. VM (bytecode)");
        match parse(&src) {
            Ok(tu) => match compile(&tu) {
                Ok(program) => {
                    println!(
                        "compiled {} function(s): {}",
                        program.functions.len(),
                        program
                            .functions
                            .iter()
                            .map(|c| c.name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                    if let Some(idx) = program.main_index {
                        let chunk = &program.functions[idx];
                        println!("main bytecode: {} ops", chunk.ops.len());
                        for (i, op) in chunk.ops.iter().take(30).enumerate() {
                            println!("  {i:>3}  {op:?}");
                        }
                        if chunk.ops.len() > 30 {
                            println!("  ... ({} more)", chunk.ops.len() - 30);
                        }
                    }
                    let mut vm = Vm::new(program);
                    match vm.run_main() {
                        Ok(v) => {
                            println!("vm main() → {v:?}");
                            println!("vm events: {}", vm.events().len());
                        }
                        Err(e) => println!("VM RUN ERROR: {e}"),
                    }
                    println!();
                }
                Err(e) => println!("VM COMPILE ERROR: {e}\n"),
            },
            Err(e) => println!("VM skipped (parse failed): {e}\n"),
        }
    }

    ExitCode::SUCCESS
}

fn section(title: &str) {
    println!("──────── {title} ────────");
}

fn shorten_kind(k: &TokenKind) -> String {
    let s = format!("{k:?}");
    if s.len() > 60 {
        format!("{}…", &s[..57])
    } else {
        s
    }
}

fn summarize_item(item: &Item) -> String {
    match item {
        Item::Function(f) => format!(
            "Function {}(...) body_stmts={}",
            f.name.name,
            f.body.stmts.len()
        ),
        Item::Class(c) => format!("Class {} members={}", c.name.name, c.members.len()),
        Item::UsingNamespace { path, .. } => format!(
            "using namespace {}",
            path.segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("::")
        ),
        Item::TypeAlias { name, .. } => format!("using {} = …", name.name),
        Item::Decl(d) => format!(
            "Decl {} name(s)={}",
            type_hint(&d.ty),
            d.declarators
                .iter()
                .map(|x| x.name.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn type_hint(ty: &rscpp_ast::Type) -> String {
    format!("{ty:?}").chars().take(40).collect::<String>()
}

fn print_events(events: &[Event], limit: usize) {
    for (i, e) in events.iter().take(limit).enumerate() {
        println!("  {i:>3}  {}", format_event(e));
    }
    if events.len() > limit {
        println!("  ... ({} more events)", events.len() - limit);
    }
}

fn format_event(e: &Event) -> String {
    match e {
        Event::Step { span, .. } => format!("Step @ {}..{}", span.start, span.end),
        Event::ScopeEnter { span, .. } => format!("ScopeEnter @ {}..{}", span.start, span.end),
        Event::ScopeExit { span, .. } => format!("ScopeExit @ {}..{}", span.start, span.end),
        Event::VarCreate { name, value, .. } => format!("VarCreate {name} = {value}"),
        Event::VarAssign {
            name, old, value, ..
        } => {
            format!("VarAssign {name}: {old:?} → {value}")
        }
        Event::VarDestroy { name, value, .. } => format!("VarDestroy {name} (was {value})"),
        Event::Write {
            slot,
            old,
            value,
            ..
        } => format!("Write {slot:?}: {old:?} → {value}"),
        Event::FnEnter {
            name,
            call_id,
            parent_id,
            args,
            ..
        } => format!("FnEnter {name}#{call_id} parent={parent_id:?} ({args:?})"),
        Event::Call {
            name,
            call_id,
            args,
            ..
        } => format!("Call {name}#{call_id} ({args:?})"),
        Event::FnExit {
            name,
            call_id,
            parent_id,
            ret,
            ..
        } => format!("FnExit {name}#{call_id} parent={parent_id:?} → {ret}"),
        Event::Branch { then_taken, .. } => format!("Branch then={then_taken}"),
        Event::LoopIter { loop_id, .. } => format!("LoopIter #{loop_id}"),
        Event::Continue { loop_id, .. } => format!("Continue #{loop_id}"),
        Event::Break { loop_id, .. } => format!("Break #{loop_id}"),
        Event::LoopEnd {
            loop_id, reason, ..
        } => format!("LoopEnd #{loop_id} ({reason})"),
        Event::Compare {
            op,
            left,
            right,
            result,
            ..
        } => format!("Compare {left} {op} {right} → {result}"),
        Event::BuiltinSelect {
            name,
            args,
            chosen,
            value,
            ..
        } => format!("BuiltinSelect {name} args={args:?} chose#{chosen} => {value}"),
        Event::Swap {
            value_a, value_b, ..
        } => format!("Swap {value_a} ↔ {value_b}"),
        Event::ContainerMod {
            kind,
            index,
            key,
            value,
            elems,
            ..
        } => format!("ContainerMod {kind} idx={index:?} key={key:?} val={value:?} elems={elems:?}"),
        Event::ContainerLookup {
            kind,
            key,
            result,
            ..
        } => format!("ContainerLookup {kind} key={key:?} → {result}"),
        Event::Alloc {
            id,
            kind,
            size,
            entries,
            ..
        } => format!(
            "Alloc #{id} ({kind}) size={size} entries={}",
            entries.len()
        ),
        Event::Dealloc { id, .. } => format!("Dealloc #{id}"),
        Event::RefBind { name, target, .. } => format!("RefBind {name} → {target:?}"),
        Event::PtrMove { name, to, .. } => format!("PtrMove {name} → {to}"),
    }
}

// keep stmt/expr imports quiet if unused later
#[allow(dead_code)]
fn _touch(_s: &Stmt, _e: &Expr, _v: &Value) {}

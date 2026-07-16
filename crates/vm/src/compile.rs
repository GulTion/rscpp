//! AST → bytecode.

use crate::chunk::{Chunk, Op, Program};
use crate::error::VmError;
use rscpp_ast::*;
use rscpp_runtime::Value;
use std::collections::HashMap;

type Result<T> = std::result::Result<T, VmError>;

pub fn compile(tu: &TranslationUnit) -> Result<Program> {
    let mut functions = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();

    // Collect function defs (top-level + class methods).
    for item in &tu.items {
        match item {
            Item::Function(f) => {
                let name = f.name.name.clone();
                let chunk = compile_function(f, &name)?;
                index.insert(name, functions.len());
                functions.push(chunk);
            }
            Item::Class(c) => {
                for m in &c.members {
                    if let Member::Function(f) = m {
                        let name = format!("{}::{}", c.name.name, f.name.name);
                        let chunk = compile_function(f, &name)?;
                        index.insert(name.clone(), functions.len());
                        functions.push(chunk);
                    }
                }
            }
            _ => {}
        }
    }

    // Resolve Call targets — second pass rewrite Call to use indices.
    // Stored as temporary string in string_pool via Call { func: 0xFFFF, ...} then fix:
    // Simpler: compile stores function name in string_pool and Call uses name index; VM resolves.

    let main_index = index.get("main").copied();
    Ok(Program {
        functions,
        main_index,
    })
}

struct Compiler {
    chunk: Chunk,
    locals: HashMap<String, u8>,
}

impl Compiler {
    fn new(name: &str, arity: u8) -> Self {
        Self {
            chunk: Chunk::new(name, arity),
            locals: HashMap::new(),
        }
    }

    fn local(&mut self, name: &str) -> u8 {
        if let Some(&i) = self.locals.get(name) {
            return i;
        }
        let i = self.locals.len() as u8;
        self.locals.insert(name.to_string(), i);
        self.chunk.local_names.push(name.to_string());
        i
    }

    fn emit(&mut self, op: Op, span: Span) {
        self.chunk.emit(op, span);
    }
}

fn compile_function(f: &FunctionDef, name: &str) -> Result<Chunk> {
    let arity = f.params.len() as u8;
    let mut c = Compiler::new(name, arity);
    for p in &f.params {
        if let Some(n) = &p.name {
            c.local(&n.name);
        } else {
            c.local(&format!("_arg{}", c.locals.len()));
        }
    }
    compile_block(&mut c, &f.body)?;
    // Implicit return 0 if path falls off.
    let z = c.chunk.add_const(Value::Int(0));
    c.emit(Op::LoadConst(z), f.span);
    c.emit(Op::Return, f.span);
    Ok(c.chunk)
}

fn compile_block(c: &mut Compiler, block: &Block) -> Result<()> {
    for stmt in &block.stmts {
        compile_stmt(c, stmt)?;
    }
    Ok(())
}

fn compile_stmt(c: &mut Compiler, stmt: &Stmt) -> Result<()> {
    let span = stmt.span();
    c.emit(Op::Step, span);
    match stmt {
        Stmt::Block(b) => compile_block(c, b),
        Stmt::Decl(d) => {
            for decl in &d.declarators {
                if let Some(init) = &decl.init {
                    compile_expr(c, init)?;
                } else {
                    // default: empty container or 0
                    emit_default_for_type(c, &d.ty, decl.span)?;
                }
                let slot = c.local(&decl.name.name);
                c.emit(Op::StoreLocal(slot), decl.span);
            }
            Ok(())
        }
        Stmt::Expr { expr, .. } => {
            compile_expr(c, expr)?;
            c.emit(Op::Pop, span);
            Ok(())
        }
        Stmt::Return { value, .. } => {
            if let Some(v) = value {
                compile_expr(c, v)?;
            } else {
                let z = c.chunk.add_const(Value::Void);
                c.emit(Op::LoadConst(z), span);
            }
            c.emit(Op::Return, span);
            Ok(())
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {
            Err(VmError::at(span, "break/continue in VM not implemented yet"))
        }
        Stmt::If {
            cond,
            then_branch,
            else_branch,
            span,
        } => {
            compile_expr(c, cond)?;
            let jf = c.chunk.emit(Op::JumpIfFalse(0), *span);
            compile_stmt(c, then_branch)?;
            if let Some(e) = else_branch {
                let j = c.chunk.emit(Op::Jump(0), *span);
                let else_ip = c.chunk.ops.len() as u16;
                c.chunk.patch_jump(jf, else_ip);
                compile_stmt(c, e)?;
                let end = c.chunk.ops.len() as u16;
                c.chunk.patch_jump(j, end);
            } else {
                let end = c.chunk.ops.len() as u16;
                c.chunk.patch_jump(jf, end);
            }
            Ok(())
        }
        Stmt::While {
            cond, body, span, ..
        } => {
            let loop_start = c.chunk.ops.len() as u16;
            compile_expr(c, cond)?;
            let jf = c.chunk.emit(Op::JumpIfFalse(0), *span);
            compile_stmt(c, body)?;
            c.emit(Op::Jump(loop_start), *span);
            let end = c.chunk.ops.len() as u16;
            c.chunk.patch_jump(jf, end);
            Ok(())
        }
        Stmt::DoWhile {
            body, cond, span, ..
        } => compile_do_while(c, body, cond, *span),
        Stmt::For {
            init,
            cond,
            step,
            body,
            span,
            ..
        } => {
            match init {
                Some(ForInit::Decl(d)) => compile_stmt(c, &Stmt::Decl(d.clone()))?,
                Some(ForInit::Expr(e)) => {
                    compile_expr(c, e)?;
                    c.emit(Op::Pop, *span);
                }
                None => {}
            }
            let loop_start = c.chunk.ops.len() as u16;
            let jf = if let Some(cond) = cond {
                compile_expr(c, cond)?;
                Some(c.chunk.emit(Op::JumpIfFalse(0), *span))
            } else {
                None
            };
            compile_stmt(c, body)?;
            if let Some(s) = step {
                compile_expr(c, s)?;
                c.emit(Op::Pop, *span);
            }
            c.emit(Op::Jump(loop_start), *span);
            let end = c.chunk.ops.len() as u16;
            if let Some(jf) = jf {
                c.chunk.patch_jump(jf, end);
            }
            Ok(())
        }
        Stmt::ForRange { span, .. } => {
            Err(VmError::at(*span, "range-for not supported in VM yet"))
        }
        Stmt::Destructure { span, .. } => {
            Err(VmError::at(*span, "structured bindings not supported in VM yet"))
        }
        Stmt::TypeAlias { .. } => Ok(()),
    }
}

fn compile_do_while(c: &mut Compiler, body: &Stmt, cond: &Expr, span: Span) -> Result<()> {
    let loop_start = c.chunk.ops.len() as u16;
    compile_stmt(c, body)?;
    compile_expr(c, cond)?;
    // if false, skip the Jump back
    let jf = c.chunk.emit(Op::JumpIfFalse(0), span);
    c.emit(Op::Jump(loop_start), span);
    let end = c.chunk.ops.len() as u16;
    c.chunk.patch_jump(jf, end);
    Ok(())
}

fn emit_default_for_type(c: &mut Compiler, ty: &Type, span: Span) -> Result<()> {
    match ty {
        Type::Named { path, .. } => {
            let name = path
                .segments
                .last()
                .map(|s| s.name.as_str())
                .unwrap_or("vector");
            let s = c.chunk.add_string(name);
            c.emit(Op::NewEmpty { type_name: s }, span);
        }
        Type::Builtin { kind: BuiltinType::Bool, .. } => {
            let i = c.chunk.add_const(Value::Bool(false));
            c.emit(Op::LoadConst(i), span);
        }
        Type::Pointer { .. } => {
            let i = c.chunk.add_const(Value::Nullptr);
            c.emit(Op::LoadConst(i), span);
        }
        _ => {
            let i = c.chunk.add_const(Value::Int(0));
            c.emit(Op::LoadConst(i), span);
        }
    }
    Ok(())
}

fn compile_expr(c: &mut Compiler, expr: &Expr) -> Result<()> {
    let span = expr.span();
    match expr {
        Expr::IntLit { value, .. } => {
            let i = c.chunk.add_const(Value::Int(*value as i64));
            c.emit(Op::LoadConst(i), span);
        }
        Expr::FloatLit { value, .. } => {
            let i = c.chunk.add_const(Value::Float(*value));
            c.emit(Op::LoadConst(i), span);
        }
        Expr::CharLit { value, .. } => {
            let i = c.chunk.add_const(Value::Char(*value));
            c.emit(Op::LoadConst(i), span);
        }
        Expr::BoolLit { value, .. } => {
            let i = c.chunk.add_const(Value::Bool(*value));
            c.emit(Op::LoadConst(i), span);
        }
        Expr::Nullptr { .. } => {
            let i = c.chunk.add_const(Value::Nullptr);
            c.emit(Op::LoadConst(i), span);
        }
        Expr::StringLit { value, .. } => {
            // allocate at runtime via native? store as const string object created later
            // For VM consts, use a marker — runtime string: put Rust String in pool and NewEmpty won't work.
            // Use LoadConst of a special approach: constants can be Object if we pre-alloc — skip, use CallMethod on temp.
            // Simple: put Value as... we can't put Object without heap. string methods tests not primary for VM first.
            let s = c.chunk.add_string(value);
            c.emit(Op::NewEmpty { type_name: s }, span); // wrong if type_name is content
            return Err(VmError::at(span, "string literals in VM not supported yet"));
        }
        Expr::Name(path) => {
            if path.segments.len() != 1 {
                return Err(VmError::at(span, "qualified names not supported in VM yet"));
            }
            let n = &path.segments[0].name;
            let slot = c
                .locals
                .get(n)
                .copied()
                .ok_or_else(|| VmError::at(span, format!("undefined local `{n}`")))?;
            c.emit(Op::LoadLocal(slot), span);
        }
        Expr::Unary { op, expr, span } => {
            compile_expr(c, expr)?;
            match op {
                UnaryOp::Minus | UnaryOp::Plus => {
                    if matches!(op, UnaryOp::Minus) {
                        c.emit(Op::Neg, *span);
                    }
                }
                UnaryOp::Not => c.emit(Op::Not, *span),
                UnaryOp::PreInc | UnaryOp::PostInc | UnaryOp::PreDec | UnaryOp::PostDec => {
                    return Err(VmError::at(*span, "++, -- in VM: use x = x + 1 for now"));
                }
                _ => return Err(VmError::at(*span, "unary op not supported in VM yet")),
            }
        }
        Expr::Binary {
            op,
            left,
            right,
            span,
        } => {
            compile_expr(c, left)?;
            compile_expr(c, right)?;
            let op = match op {
                BinaryOp::Add => Op::Add,
                BinaryOp::Sub => Op::Sub,
                BinaryOp::Mul => Op::Mul,
                BinaryOp::Div => Op::Div,
                BinaryOp::Rem => Op::Rem,
                BinaryOp::Lt => Op::CmpLt,
                BinaryOp::Le => Op::CmpLe,
                BinaryOp::Gt => Op::CmpGt,
                BinaryOp::Ge => Op::CmpGe,
                BinaryOp::Eq => Op::CmpEq,
                BinaryOp::Ne => Op::CmpNe,
                BinaryOp::And => {
                    return Err(VmError::at(*span, "&& short-circuit in VM later"))
                }
                BinaryOp::Or => return Err(VmError::at(*span, "|| short-circuit in VM later")),
                _ => return Err(VmError::at(*span, "binary op not supported in VM yet")),
            };
            c.emit(op, *span);
        }
        Expr::Assign {
            op,
            left,
            right,
            span,
        } => {
            if *op != AssignOp::Assign {
                return Err(VmError::at(*span, "compound assign in VM later"));
            }
            match left.as_ref() {
                Expr::Name(path) if path.segments.len() == 1 => {
                    compile_expr(c, right)?;
                    c.emit(Op::Dup, *span);
                    let slot = *c.locals.get(&path.segments[0].name).ok_or_else(|| {
                        VmError::at(*span, "assign to undefined local")
                    })?;
                    c.emit(Op::StoreLocal(slot), *span);
                }
                Expr::Index { base, index, .. } => {
                    compile_expr(c, base)?;
                    compile_expr(c, index)?;
                    compile_expr(c, right)?;
                    c.emit(Op::IndexSet, *span);
                }
                _ => return Err(VmError::at(*span, "unsupported assignment target in VM")),
            }
        }
        Expr::Call { callee, args, span } => {
            if let Expr::Member {
                base,
                field,
                arrow,
                ..
            } = callee.as_ref()
            {
                if *arrow {
                    return Err(VmError::at(*span, "-> not supported in VM"));
                }
                compile_expr(c, base)?;
                for a in args {
                    compile_expr(c, a)?;
                }
                let name = c.chunk.add_string(&field.name);
                c.emit(
                    Op::CallMethod {
                        name,
                        argc: args.len() as u8,
                    },
                    *span,
                );
                return Ok(());
            }
            if let Expr::Name(path) = callee.as_ref() {
                let fname = path
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join("::");
                if fname == "pair" {
                    if args.len() != 2 {
                        return Err(VmError::at(*span, "pair() needs 2 args"));
                    }
                    compile_expr(c, &args[0])?;
                    compile_expr(c, &args[1])?;
                    c.emit(Op::MakePair, *span);
                    return Ok(());
                }
                for a in args {
                    compile_expr(c, a)?;
                }
                let name = c.chunk.add_string(fname);
                // Call uses func field as string pool index until VM resolves
                c.emit(
                    Op::Call {
                        func: name,
                        argc: args.len() as u8,
                    },
                    *span,
                );
                return Ok(());
            }
            return Err(VmError::at(*span, "unsupported call"));
        }
        Expr::Index { base, index, span } => {
            compile_expr(c, base)?;
            compile_expr(c, index)?;
            c.emit(Op::IndexGet, *span);
        }
        Expr::InitList { elems, span } => {
            // brace init → vector
            let s = c.chunk.add_string("vector");
            c.emit(Op::NewEmpty { type_name: s }, *span);
            for e in elems {
                c.emit(Op::Dup, *span);
                compile_expr(c, e)?;
                let pb = c.chunk.add_string("push_back");
                c.emit(Op::CallMethod { name: pb, argc: 1 }, *span);
                c.emit(Op::Pop, *span); // push_back returns void
            }
        }
        Expr::Member { .. } => {
            return Err(VmError::at(span, "member load without call not in VM yet"));
        }
        Expr::Cast { expr, .. } => compile_expr(c, expr)?,
        Expr::Conditional { span, .. } => {
            return Err(VmError::at(*span, "ternary ?: not supported in VM yet"));
        }
        Expr::Lambda { span, .. } => {
            return Err(VmError::at(*span, "lambda not supported in VM yet"));
        }
        Expr::New { span, .. } => {
            return Err(VmError::at(*span, "new not supported in VM yet"));
        }
        Expr::Sizeof { span, .. } => {
            let i = c.chunk.add_const(Value::Int(8));
            c.emit(Op::LoadConst(i), *span);
        }
        Expr::Delete { span, .. } => {
            return Err(VmError::at(*span, "delete not supported in VM yet"));
        }
    }
    Ok(())
}

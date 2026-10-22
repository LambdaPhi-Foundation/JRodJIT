use crate::ast::{Expr, ExprKind};
use crate::ir::IR;
use crate::codegen::X86_64Emitter;

pub struct JITCompiler {
    ir: Vec<IR>,
    var_offsets: std::collections::HashMap<String, usize>,
    next_var_offset: usize,
    label_counter: usize,
    runtime_addrs: std::collections::HashMap<String, usize>,
}

impl JITCompiler {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            ir: Vec::new(),
            var_offsets: std::collections::HashMap::new(),
            next_var_offset: 8,
            label_counter: 0,
            runtime_addrs: std::collections::HashMap::new(),
        })
    }

    pub fn register_runtime(&mut self, name: &str, addr: usize) {
        self.runtime_addrs.insert(name.to_string(), addr);
    }

    fn new_label(&mut self) -> usize {
        let idx = self.label_counter;
        self.label_counter += 1;
        idx
    }

    fn ensure_var(&mut self, name: &str) -> usize {
        if let Some(&off) = self.var_offsets.get(name) {
            return off;
        }
        let off = self.next_var_offset;
        self.next_var_offset += 8;
        self.var_offsets.insert(name.to_string(), off);
        off
    }

    fn collect_vars(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Id(name) => {
                self.ensure_var(name);
            }
            ExprKind::TypedId(name, _) => {
                self.ensure_var(name);
            }
            ExprKind::List(children) => {
                for child in children.iter().skip(1) {
                    self.collect_vars(child);
                }
            }
            _ => {}
        }
    }

    pub fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match &expr.kind {
            ExprKind::Int(v) => {
                self.ir.push(IR::ConstInt(*v));
            }
            ExprKind::Float(v) => {
                self.ir.push(IR::ConstFloat(*v));
            }
            ExprKind::Bool(v) => {
                self.ir.push(IR::ConstBool(*v));
            }
            ExprKind::Id(name) => {
                self.ir.push(IR::LoadVar(name.clone()));
            }
            ExprKind::TypedId(_, _) => {}
            ExprKind::String(_) => {
                self.ir.push(IR::ConstInt(0));
            }
            ExprKind::Array(elements) => {
                for el in elements {
                    self.compile_expr(el)?;
                }
            }
            ExprKind::Object(_) => {
                self.ir.push(IR::ConstInt(0));
            }
            ExprKind::List(children) => {
                if children.is_empty() {
                    return Ok(());
                }
                let first = &children[0];
                if let ExprKind::Id(cmd) = &first.kind {
                    match cmd.as_str() {
                        "set" => {
                            if children.len() != 3 {
                                return Err("set requires 2 arguments".into());
                            }
                            let target = &children[1];
                            let value = &children[2];
                            self.compile_expr(value)?;
                            if let ExprKind::Id(name) = &target.kind {
                                self.ir.push(IR::StoreVar(name.clone()));
                            } else if let ExprKind::TypedId(name, _) = &target.kind {
                                self.ir.push(IR::StoreVar(name.clone()));
                            } else {
                                return Err("set target must be identifier".into());
                            }
                        }
                        "add" => {
                            if children.len() != 3 {
                                return Err("add requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Add);
                        }
                        "sub" => {
                            if children.len() != 3 {
                                return Err("sub requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Sub);
                        }
                        "mult" => {
                            if children.len() != 3 {
                                return Err("mult requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Mul);
                        }
                        "div" => {
                            if children.len() != 3 {
                                return Err("div requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Div);
                        }
                        "lt" => {
                            if children.len() != 3 {
                                return Err("lt requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::CmpLt);
                        }
                        "gt" => {
                            if children.len() != 3 {
                                return Err("gt requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::CmpGt);
                        }
                        "eq" => {
                            if children.len() != 3 {
                                return Err("eq requires 2 arguments".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::CmpEq);
                        }
                        "println" | "print" => {
                            if children.len() != 2 {
                                return Err("println/print requires 1 argument".into());
                            }
                            self.compile_expr(&children[1])?;
                            if let Some(&addr) = self.runtime_addrs.get("jr_print_int") {
                                self.ir.push(IR::CallFFI(addr, 1));
                            } else {
                                return Err("Runtime function jr_print_int not registered".into());
                            }
                        }
                        "if" => {
                            if children.len() < 3 {
                                return Err("if requires condition and branches".into());
                            }
                            self.compile_expr(&children[1])?;
                            let else_label = self.new_label();
                            let end_label = self.new_label();
                            self.ir.push(IR::JmpIfFalse(else_label));
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Jmp(end_label));
                            self.ir.push(IR::Label(else_label));
                            if children.len() > 3 {
                                self.compile_expr(&children[3])?;
                            }
                            self.ir.push(IR::Label(end_label));
                        }
                        "while" => {
                            if children.len() < 3 {
                                return Err("while requires condition and body".into());
                            }
                            let start_label = self.new_label();
                            let end_label = self.new_label();
                            self.ir.push(IR::Label(start_label));
                            self.compile_expr(&children[1])?;
                            self.ir.push(IR::JmpIfFalse(end_label));
                            for body in &children[2..] {
                                self.compile_expr(body)?;
                            }
                            self.ir.push(IR::Jmp(start_label));
                            self.ir.push(IR::Label(end_label));
                        }
                        "for" => {
                            return Err("for loops should be desugared before JIT".into());
                        }
                        "def" => {
                            return Err("def should be handled at program level".into());
                        }
                        "c-lib" | "c-bind" | "c-call" => {
                            return Err("C FFI not yet implemented in JIT compiler".into());
                        }
                        "peek-i64" => {
                            if children.len() != 3 {
                                return Err("peek-i64 requires ptr and offset".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.ir.push(IR::Add);
                            self.ir.push(IR::PeekI64);
                        }
                        "poke-i64" => {
                            if children.len() != 4 {
                                return Err("poke-i64 requires ptr, offset, value".into());
                            }
                            self.compile_expr(&children[1])?;
                            self.compile_expr(&children[2])?;
                            self.compile_expr(&children[3])?;
                            self.ir.push(IR::Add);
                            self.ir.push(IR::PokeI64);
                        }
                        _ => {
                            for arg in &children[1..] {
                                self.compile_expr(arg)?;
                            }
                            self.ir.push(IR::CallNative(cmd.clone(), children.len() - 1));
                        }
                    }
                }
            }
            ExprKind::Comment(_) => {}
            ExprKind::Quote(_) => {}
        }
        Ok(())
    }

    pub fn compile_program(&mut self, program: &[Expr]) -> Result<*mut u8, String> {
        for expr in program {
            self.collect_vars(expr);
        }

        let mut emitter = X86_64Emitter::new(4096, self.var_offsets.clone())?;

        for expr in program {
            self.compile_expr(expr)?;
        }

        self.ir.push(IR::Ret);

        let stack_size = self.next_var_offset;
        let entry = emitter.compile_block(&self.ir, stack_size)?;

        std::mem::forget(emitter);

        Ok(entry)
    }
}

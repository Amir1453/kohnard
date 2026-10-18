// The following code is absolutely garbage for the time being.
// And update is direly needed. What are those CompactStrings everywhere ??
// Should probably intern all the x86 assembly grammar, and just emit the slices

use compact_str::CompactString;
use std::collections::HashMap;

use crate::ir::tac::*;
use crate::types::Name;

pub struct Asm {
    var_asm: Vec<CompactString>,
    asm: Vec<CompactString>,
    temps: HashMap<u32, usize>,
    named: HashMap<CompactString, usize>,
    tparams: HashMap<CompactString, usize>,
    params: Vec<TACTemp>,
}

impl Asm {
    fn new() -> Self {
        Self {
            var_asm: Vec::new(),
            asm: Vec::new(),
            temps: HashMap::new(),
            named: HashMap::new(),
            tparams: HashMap::new(),
            params: Vec::new(),
        }
    }

    pub fn lower(cutac: CUTAC) -> String {
        let mut asm = Asm::new();
        asm.lower_cutac(cutac);
        let mut out: Vec<CompactString> = Vec::new();
        out.extend(asm.var_asm);
        out.extend(asm.asm);
        let mut asmuk = out.join("\n");
        asmuk.push('\n');
        asmuk
    }

    fn lower_cutac(&mut self, cutac: CUTAC) {
        for decl in cutac.0 {
            match decl {
                TACDeclaration::GlobalVarDecl(var) => self.lower_global_var_decl(var),
                TACDeclaration::ProcDecl(proc_decl) => self.lower_proc_decl(proc_decl),
            }
        }
    }

    fn lower_global_var_decl(&mut self, var: GlobalVarDecl) {
        self.emit_var(".data");
        self.emit_var(format!(".globl {}", var.name));
        self.emit_var(format!("{}:", var.name));
        self.emit_var(format!(".quad {}", var.value));
    }

    fn lower_proc_decl(&mut self, proc: ProcDecl) {
        // reset per-proc state
        self.temps.clear();
        self.named.clear();
        self.tparams.clear();
        self.params.clear();

        // Use a fresh buffer for this proc's body so nothing else gets captured.
        let mut body_buf: Vec<CompactString> = Vec::new();

        // helper closures to emit into body_buf
        let emit_body = |s: &str, buf: &mut Vec<CompactString>| {
            buf.push(CompactString::from(s));
        };

        // Reserve and move first up to 6 args into named temps (if arguments provided)
        let param_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];

        if let Some(args) = &proc.arguments {
            for (i, arg) in args.iter().enumerate().take(6) {
                let arg = CompactString::from(arg.as_str());
                let slot = self.alloc_named_temp(arg.clone());
                emit_body(
                    &format!("\tmovq\t{}, {}", param_regs[i], self.format_temp_slot(slot)),
                    &mut body_buf,
                );
            }
            for (i, arg) in args.iter().enumerate().skip(6) {
                let arg = CompactString::from(arg.as_str());
                self.tparams.insert(arg.clone(), i - 6);
            }
        }

        // Temporarily redirect emits: we need lower_inst to write into body_buf.
        // To avoid changing lower_inst signature, collect lowered lines by calling helpers here:
        // Re-implement lowering loop locally using lower_inst-like logic but emitting into body_buf.
        // Simpler: swap self.asm with body_buf so existing lower_inst emits go into body_buf.
        let prev_asm = std::mem::take(&mut self.asm);
        self.asm = body_buf;
        // lower body instructions (this will push into self.asm which is the body_buf)
        for instr in proc.instructions {
            self.lower_inst(instr);
        }
        // take the produced body
        body_buf = std::mem::take(&mut self.asm);
        // restore previous asm buffer (append prologue+body to it)
        self.asm = prev_asm;

        // compute stack frame size (number of slots for temps + named)
        let mut nvars = self.temps.len() + self.named.len();
        if nvars % 2 == 1 {
            nvars += 1;
        }

        // build prologue + body and append to global asm
        self.asm.push(CompactString::from(".text"));
        self.asm
            .push(CompactString::from(format!(".globl {}", proc.name)));
        self.asm
            .push(CompactString::from(format!("{}:", proc.name)));
        self.asm.push(CompactString::from("\tpushq\t%rbp"));
        self.asm.push(CompactString::from("\tmovq\t%rsp, %rbp"));
        self.asm
            .push(CompactString::from(format!("\tsubq\t${}, %rsp", 8 * nvars)));
        self.asm.extend(body_buf);
    }

    fn lower_inst(&mut self, instr: TACInst) {
        match instr {
            TACInst::Const {
                destination,
                constant,
            } => {
                let dst = self.temp_for(destination);
                self.emit(format!("\tmovq\t${}, {}", constant, dst));
            }

            TACInst::Copi {
                destination,
                source,
            } => {
                let dst = self.temp_for(destination);
                let src = self.temp_for(source);
                self.emit(format!("\tmovq\t{}, %r11", src));
                self.emit(format!("\tmovq\t%r11, {}", dst));
            }

            TACInst::LabelDecl(lbl) => {
                self.emit(format!("{}:", self.format_label(&lbl)));
            }

            TACInst::UnconditionalJump(lbl) => {
                self.emit(format!("\tjmp\t{}", self.format_label(&lbl)));
            }

            TACInst::ConditionalJump {
                opcode,
                condition,
                destination,
            } => {
                let cond = self.temp_for(condition);
                self.emit(format!("\tcmpq\t$0, {}", cond));
                let j = match opcode {
                    TACJumpOpcode::JZ => "jz",
                    TACJumpOpcode::JNZ => "jnz",
                    TACJumpOpcode::JL => "jl",
                    TACJumpOpcode::JNL => "jge",
                    TACJumpOpcode::JLE => "jle",
                    TACJumpOpcode::JNLE => "jg",
                };
                self.emit(format!("\t{}\t{}", j, self.format_label(&destination)));
            }

            TACInst::UnaryOperation {
                opcode,
                operand,
                result,
            } => {
                let opnd = self.temp_for(operand);
                let dst = self.temp_for(result);
                match opcode {
                    TACUnaryOpcode::NEG => {
                        self.emit(format!("\tmovq\t{}, %r11", opnd));
                        self.emit("\tnegq\t%r11");
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACUnaryOpcode::NOT => {
                        self.emit(format!("\tmovq\t{}, %r11", opnd));
                        self.emit("\tnotq\t%r11");
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                }
            }

            TACInst::BinaryOperation {
                opcode,
                lhs,
                rhs,
                result,
            } => {
                let l = self.temp_for(lhs);
                let r = self.temp_for(rhs);
                let dst = self.temp_for(result);
                match opcode {
                    TACBinaryOpcode::ADD => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\taddq\t{}, %r11", r));
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::SUB => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\tsubq\t{}, %r11", r));
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::MUL => {
                        self.emit(format!("\tmovq\t{}, %rax", l));
                        self.emit(format!("\timulq\t{}", r));
                        self.emit(format!("\tmovq\t%rax, {}", dst));
                    }
                    TACBinaryOpcode::DIV => {
                        self.emit(format!("\tmovq\t{}, %rax", l));
                        self.emit("\tcqto");
                        self.emit(format!("\tidivq\t{}", r));
                        self.emit(format!("\tmovq\t%rax, {}", dst));
                    }
                    TACBinaryOpcode::MOD => {
                        self.emit(format!("\tmovq\t{}, %rax", l));
                        self.emit("\tcqto");
                        self.emit(format!("\tidivq\t{}", r));
                        self.emit(format!("\tmovq\t%rdx, {}", dst));
                    }
                    TACBinaryOpcode::AND => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\tandq\t{}, %r11", r));
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::OR => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\torq\t{}, %r11", r));
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::XOR => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\txorq\t{}, %r11", r));
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::SHL => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\tmovq\t{}, %rcx", r));
                        self.emit("\tsalq\t%cl, %r11");
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                    TACBinaryOpcode::SHR => {
                        self.emit(format!("\tmovq\t{}, %r11", l));
                        self.emit(format!("\tmovq\t{}, %rcx", r));
                        self.emit("\tsarq\t%cl, %r11");
                        self.emit(format!("\tmovq\t%r11, {}", dst));
                    }
                }
            }

            TACInst::Parameter {
                nth_param,
                source_temp,
            } => {
                assert!(self.params.len() + 1 == nth_param);
                self.params.push(source_temp);
            }

            TACInst::ProcCall {
                proc_name,
                arg_count,
                result,
            } => {
                let param_regs = ["%rdi", "%rsi", "%rdx", "%rcx", "%r8", "%r9"];
                assert!(arg_count == self.params.len());

                // first up to 6 into regs
                for i in 0..self.params.len().min(6) {
                    let a = self.params[i].clone();
                    let src = self.temp_for(a);
                    self.emit(format!("\tmovq\t{}, {}", src, param_regs[i]));
                }

                // rest pushed (reverse)
                if self.params.len() > 6 {
                    for idx in (6..self.params.len()).rev() {
                        let a = self.params[idx].clone();
                        let src = self.temp_for(a);
                        self.emit(format!("\tpushq\t{}", src));
                    }
                }

                let qarg = arg_count.saturating_sub(6);
                if qarg & 0x1 == 1 {
                    self.emit("\tsubq\t$8, %rsp");
                }

                self.emit(format!("\tcallq\t{}", proc_name));

                if qarg > 0 {
                    let restore = qarg + (qarg & 0x1);
                    self.emit(format!("\taddq\t${}, %rsp", restore * 8));
                }

                // result is a TACTemp
                let dst = self.temp_for(result);
                self.emit(format!("\tmovq\t%rax, {}", dst));

                self.params.clear();
            }

            TACInst::Return(opt) => {
                if let Some(t) = opt {
                    let src = self.temp_for(t);
                    self.emit(format!("\tmovq\t{}, %rax", src));
                }
                // emit an actual return here; TAC's shared epilogue (label + ret) will be emitted where present
                self.emit("\tmovq\t%rbp, %rsp");
                self.emit("\tpopq\t%rbp");
                self.emit("\tretq");
            }

            TACInst::Nop => {
                self.emit("nop");
            }
        }
    }

    fn alloc_named_temp(&mut self, name: Name) -> usize {
        if let Some(&s) = self.named.get(&name) {
            s
        } else {
            let s = self.temps.len() + self.named.len();
            self.named.insert(name, s);
            s
        }
    }

    fn temp_for(&mut self, t: TACTemp) -> String {
        match t {
            TACTemp::Temp(i) => {
                if let Some(&slot) = self.temps.get(&i) {
                    self.format_temp_slot(slot)
                } else {
                    let s = self.temps.len() + self.named.len();
                    self.temps.insert(i, s);
                    self.format_temp_slot(s)
                }
            }
            TACTemp::NamedTemp(name) => {
                let k = CompactString::from(name.as_str());
                if let Some(&s) = self.named.get(&k) {
                    self.format_temp_slot(s)
                } else if let Some(&sp) = self.tparams.get(&k) {
                    // spilled parameter at 8*(sp+2)(%rbp) -> represent as immediate stack address string
                    format!("{}(%rbp)", 8 * (sp + 2))
                } else {
                    let s = self.temps.len() + self.named.len();
                    self.named.insert(k.clone(), s);
                    self.format_temp_slot(s)
                }
            }
            TACTemp::GlobalVar(name) => format!("{}(%rip)", name),
        }
    }

    fn format_temp_slot(&self, slot: usize) -> String {
        format!("-{}(%rbp)", 8 * (slot + 1))
    }

    fn format_label(&self, lbl: &Label) -> CompactString {
        CompactString::from(lbl.to_string())
    }

    fn emit<S: AsRef<str>>(&mut self, a: S) {
        self.asm.push(CompactString::from(a.as_ref()));
    }

    fn emit_var<S: AsRef<str>>(&mut self, a: S) {
        self.var_asm.push(CompactString::from(a.as_ref()));
    }
}

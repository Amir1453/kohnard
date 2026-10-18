use std::collections::HashMap;

use crate::frontend::{Symbol, ast::*};
use crate::ir::tac::*;
use crate::structs::Stack;
use crate::types::Type;

pub struct MM {
    cutac: CUTAC,
    proc_instructions: InstBlock,

    vars: Stack<HashMap<Symbol, TACTemp>>,
    loop_stack: Stack<(Label, Label)>,

    temp_counter: u32,
    label_counter: u32,
}

impl MM {
    fn new() -> Self {
        Self {
            cutac: CUTAC::new(),
            proc_instructions: InstBlock::new(),
            vars: Stack::new_with(HashMap::new()),
            loop_stack: Stack::default(),
            temp_counter: 0,
            label_counter: 0,
        }
    }

    pub fn munch(program: Program) -> CUTAC {
        let mm = MM::new();
        mm.munch_program(program)
    }

    fn munch_program(mut self, program: Program) -> CUTAC {
        use TACInst::*;

        for decl in program.0 {
            match decl {
                Declaration::Variable(var) => {
                    self.munch_global_variable(*var);
                }

                Declaration::Proc {
                    proc_name,
                    proc_args,
                    return_type,
                    block,
                } => {
                    self.vars.push(HashMap::new());

                    let arguments: Option<Vec<Symbol>> =
                        proc_args.map(|args| args.into_iter().map(|(name, _ty)| name).collect());

                    arguments.iter().flat_map(|v| v.iter()).for_each(|name| {
                        self.current_vars_insert(*name, TACTemp::NamedTemp(*name))
                    });

                    self.munch_block(*block);
                    let mut instructions = std::mem::take(&mut self.proc_instructions);

                    let ret_label = self.fresh_label();

                    if return_type.is_some() {
                        let ret_temp = self.fresh_temp();
                        let mut copy_inst: Vec<(usize, TACInst)> =
                            Vec::with_capacity(instructions.len() / 4);

                        for (i, inst) in instructions.iter().enumerate() {
                            if let Return(Some(tmp)) = inst {
                                let copy = Copi {
                                    destination: ret_temp.clone(),
                                    source: tmp.clone(),
                                };
                                copy_inst.push((i, copy));
                            }
                        }

                        copy_inst.into_iter().rev().for_each(|(pos, inst)| {
                            instructions.insert(pos, inst);
                            instructions[pos + 1] = UnconditionalJump(ret_label.clone());
                        });

                        instructions.push(LabelDecl(ret_label));
                        instructions.push(Return(Some(ret_temp.clone())));
                    } else {
                        for inst in instructions.iter_mut() {
                            if let Return(None) = inst {
                                *inst = UnconditionalJump(ret_label.clone())
                            }
                        }

                        instructions.push(LabelDecl(ret_label));
                        instructions.push(Return(None));
                    }

                    let proc_decl = ProcDecl::new(proc_name, arguments, instructions);
                    self.cutac.push(TACDeclaration::ProcDecl(proc_decl));

                    self.vars.pop();
                }
            }
        }
        self.cutac
    }

    fn munch_global_variable(&mut self, var: Variable) {
        for (name, value_expr) in var.names.into_iter().zip(var.values) {
            let value = match *value_expr {
                Expression::Number(num) => num,
                Expression::Bool(boo) => boo.into(),
                _ => -42,
            };
            let global = GlobalVarDecl::new(name, value);
            self.cutac.push(TACDeclaration::GlobalVarDecl(global));

            self.current_vars_insert(name, TACTemp::GlobalVar(name));
        }
    }

    fn munch_block(&mut self, block: Block) {
        self.vars.push(HashMap::new());
        for stmt in block.0 {
            self.munch_statement(stmt);
        }
        self.vars.pop();
    }

    fn munch_statement(&mut self, stmt: Statement) {
        use TACInst::*;

        match stmt {
            Statement::Variable(variable) => {
                self.munch_variable(*variable);
            }

            Statement::Assignment { name, value } => {
                let temp = self.munch_expression(*value);
                self.emit(Copi {
                    destination: self.current_var_get(&name),
                    source: temp,
                });
            }

            Statement::Block(block) => {
                self.munch_block(*block);
            }

            Statement::Eval(expression) => {
                self.munch_expression(*expression);
            }

            Statement::Return(expression) => match expression {
                Some(exp) => {
                    let temp = self.munch_expression(*exp);
                    self.emit(Return(Some(temp)));
                }
                None => self.emit(Return(None)),
            },

            Statement::If {
                condition,
                then_block,
                else_branch,
            } => {
                let tlabel = self.fresh_label();
                let flabel = self.fresh_label();
                let olabel = self.fresh_label();

                self.munch_boolean_expression(*condition, tlabel.clone(), flabel.clone());
                self.emit_label(tlabel);

                self.munch_block(*then_block);
                self.emit(UnconditionalJump(olabel.clone()));

                self.emit_label(flabel);
                if let Some(elise) = else_branch {
                    self.munch_statement(*elise);
                }
                self.emit_label(olabel);
            }

            Statement::While { condition, block } => {
                let clabel = self.fresh_label();
                let blabel = self.fresh_label();
                let olabel = self.fresh_label();

                self.loop_stack.push((clabel.clone(), olabel.clone()));

                self.emit_label(clabel.clone());
                self.munch_boolean_expression(*condition, blabel.clone(), olabel.clone());

                self.emit_label(blabel);
                self.munch_block(*block);

                self.emit(UnconditionalJump(clabel));

                self.emit_label(olabel);

                self.loop_stack.pop();
            }

            Statement::Jump(state) => match state {
                JumpState::Break => {
                    if let Some((_, brk)) = self.loop_stack.top().cloned() {
                        self.emit(UnconditionalJump(brk))
                    }
                }

                JumpState::Continue => {
                    if let Some((cont, _)) = self.loop_stack.top().cloned() {
                        self.emit(UnconditionalJump(cont))
                    }
                }
            },
        }
    }

    fn munch_variable(&mut self, var: Variable) {
        use TACInst::Copi;

        for (name, value_expr) in var.names.into_iter().zip(var.values) {
            let fresh = self.fresh_temp();
            self.current_vars_insert(name, fresh.clone());

            let temp = self.munch_expression(*value_expr);
            self.emit(Copi {
                destination: fresh,
                source: temp,
            });
        }
    }

    fn munch_expression(&mut self, expr: Expression) -> TACTemp {
        use TACInst::*;

        if expr.get_type() == Type::Bool {
            let temp = self.fresh_temp();
            let tlabel = self.fresh_label();
            let flabel = self.fresh_label();

            self.emit(Const {
                destination: temp.clone(),
                constant: 0,
            });

            self.munch_boolean_expression(expr, tlabel.clone(), flabel.clone());
            self.emit_label(tlabel);

            self.emit(Const {
                destination: temp.clone(),
                constant: 1,
            });
            self.emit_label(flabel);

            temp
        } else {
            match expr {
                Expression::Variable(name, ..) => self.current_var_get(&name),

                Expression::Number(num) => {
                    let temp = self.fresh_temp();
                    self.emit(Const {
                        destination: temp.clone(),
                        constant: num,
                    });
                    temp
                }

                Expression::BinaryOperation {
                    lhs,
                    operator,
                    rhs,
                    ty: _,
                } => {
                    let temp = self.fresh_temp();
                    let ltmp = self.munch_expression(*lhs);
                    let rtmp = self.munch_expression(*rhs);

                    self.emit(BinaryOperation {
                        opcode: operator.into(),
                        lhs: ltmp,
                        rhs: rtmp,
                        result: temp.clone(),
                    });

                    temp
                }

                Expression::UnaryOperation {
                    operator,
                    value,
                    ty: _,
                } => {
                    let temp = self.fresh_temp();
                    let utmp = self.munch_expression(*value);

                    self.emit(UnaryOperation {
                        opcode: operator.into(),
                        operand: utmp,
                        result: temp.clone(),
                    });

                    temp
                }

                Expression::ProcCall {
                    proc_name,
                    proc_args,
                    ..
                } => {
                    let arg_count = proc_args.as_ref().map(|v| v.len()).unwrap_or(0);

                    let mut iexp = proc_args.into_iter().flatten().enumerate();
                    let mut first_exp_type = Type::Error;

                    if let Some((i, expr)) = iexp.next() {
                        let exp = *expr;
                        first_exp_type = exp.get_type();

                        let temp = self.munch_expression(exp);
                        self.emit(Parameter {
                            nth_param: i + 1,
                            source_temp: temp,
                        });
                    }

                    for (i, exp) in iexp {
                        let temp = self.munch_expression(*exp);
                        self.emit(Parameter {
                            nth_param: i + 1,
                            source_temp: temp,
                        });
                    }

                    let result = self.fresh_temp();
                    if proc_name.as_str() == "print" {
                        match first_exp_type {
                            Type::Int => {
                                self.emit(ProcCall {
                                    proc_name: Symbol::intern("__print_int"),
                                    arg_count,
                                    result: result.clone(),
                                });
                            }
                            Type::Bool => {
                                self.emit(ProcCall {
                                    proc_name: Symbol::intern("__print_bool"),
                                    arg_count,
                                    result: result.clone(),
                                });
                            }
                            _ => {}
                        }
                    } else {
                        self.emit(ProcCall {
                            proc_name,
                            arg_count,
                            result: result.clone(),
                        });
                    }

                    result
                }

                _ => unreachable!(),
            }
        }
    }

    fn munch_boolean_expression(&mut self, expr: Expression, tlabel: Label, flabel: Label) {
        use TACInst::*;
        use TACJumpOpcode::*;

        match expr {
            Expression::Variable(name, ..) => {
                let temp = self.current_var_get(&name);

                self.emit(ConditionalJump {
                    opcode: JZ,
                    condition: temp,
                    destination: flabel,
                });

                self.emit(UnconditionalJump(tlabel));
            }

            Expression::Bool(boo) => match boo {
                true => self.emit(UnconditionalJump(tlabel)),
                false => self.emit(UnconditionalJump(flabel)),
            },

            Expression::BinaryOperation {
                lhs, operator, rhs, ..
            } => {
                use Operator::*;
                use TACBinaryOpcode::SUB;

                match operator {
                    Equal | NEqual | L | LTE | G | GTE => {
                        let ltmp = self.munch_expression(*lhs);
                        let rtmp = self.munch_expression(*rhs);
                        let temp = self.fresh_temp();

                        self.emit(BinaryOperation {
                            opcode: SUB,
                            lhs: ltmp,
                            rhs: rtmp,
                            result: temp.clone(),
                        });

                        self.emit(ConditionalJump {
                            opcode: operator.into(),
                            condition: temp,
                            destination: tlabel,
                        });
                        self.emit(UnconditionalJump(flabel));
                    }

                    LAnd => {
                        let olabel = self.fresh_label();

                        self.munch_boolean_expression(*lhs, olabel.clone(), flabel.clone());
                        self.emit_label(olabel);
                        self.munch_boolean_expression(*rhs, tlabel, flabel);
                    }

                    LOr => {
                        let olabel = self.fresh_label();
                        self.munch_boolean_expression(*lhs, tlabel.clone(), olabel.clone());
                        self.emit_label(olabel);
                        self.munch_boolean_expression(*rhs, tlabel, flabel);
                    }

                    _ => {}
                }
            }

            Expression::UnaryOperation {
                operator, value, ..
            } => {
                use Operator::*;
                if let LNot = &operator {
                    self.munch_boolean_expression(*value, flabel, tlabel);
                }
            }

            Expression::ProcCall {
                proc_name,
                proc_args,
                ..
            } => {
                let arg_count = proc_args.as_ref().map(|v| v.len()).unwrap_or(0);

                for (i, exp) in proc_args.into_iter().flatten().enumerate() {
                    let temp = self.munch_expression(*exp);
                    self.emit(Parameter {
                        nth_param: i + 1,
                        source_temp: temp,
                    });
                }

                let result = self.fresh_temp();
                self.emit(ProcCall {
                    proc_name,
                    arg_count,
                    result: result.clone(),
                });

                self.emit(ConditionalJump {
                    opcode: JZ,
                    condition: result,
                    destination: flabel,
                });
                self.emit(UnconditionalJump(tlabel));
            }

            _ => {}
        }
    }

    fn fresh_temp(&mut self) -> TACTemp {
        let tmp = TACTemp::Temp(self.temp_counter);
        self.temp_counter += 1;
        tmp
    }

    fn fresh_label(&mut self) -> Label {
        let lbl = Label(self.label_counter);
        self.label_counter += 1;
        lbl
    }

    fn emit(&mut self, inst: TACInst) {
        self.proc_instructions.push(inst);
    }

    fn emit_label(&mut self, label: Label) {
        self.proc_instructions.push(TACInst::LabelDecl(label));
    }

    // Stack helpers

    pub fn current_vars_insert(&mut self, name: Symbol, temp: TACTemp) {
        if let Some(current) = self.vars.top_mut() {
            current.insert(name, temp);
        }
    }

    pub fn current_var_get(&self, name: &Symbol) -> TACTemp {
        for scope in self.vars.iter().rev() {
            if let Some(t) = scope.get(name) {
                return t.clone();
            }
        }
        TACTemp::GlobalVar(*name)
        // panic!("undefined variable {:?}", name);
    }
}

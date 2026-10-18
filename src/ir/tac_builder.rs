use std::collections::HashMap;

use crate::frontend::Symbol;
use crate::ir::tac::{
    InstBlock, Label, ProcDecl, TACBinaryOpcode, TACInst, TACJumpOpcode, TACTemp,
};

pub struct ProcBuilder {
    pub name: Symbol,
    pub arguments: Option<Vec<Symbol>>,
    pub instructions: InstBlock,

    variables: HashMap<Symbol, TACTemp>,
    labels: HashMap<Symbol, Label>,
    next_temp: u32,
    next_label: u32,
}

impl ProcBuilder {
    pub fn new(name: &str) -> Self {
        let name = Symbol::intern(name);

        Self {
            name,
            arguments: None,
            instructions: InstBlock::new(),
            variables: HashMap::new(),
            labels: HashMap::new(),
            next_temp: 0,
            next_label: 0,
        }
    }

    pub fn build(self) -> ProcDecl {
        ProcDecl {
            name: self.name,
            arguments: self.arguments,
            instructions: self.instructions,
        }
    }

    pub fn add_argument(&mut self, arg: &str) {
        self.define(arg);
        self.arguments
            .get_or_insert_with(Vec::new)
            .push(Symbol::intern(arg));
    }

    pub fn add_instruction(&mut self, instr: TACInst) -> &mut Self {
        self.instructions.push(instr);
        self
    }

    pub fn constant(&mut self, name: &str, value: i64) -> &mut Self {
        let destination = self.define(name);

        self.add_instruction(TACInst::Const {
            destination,
            constant: value,
        })
    }

    pub fn copy(&mut self, source: &str, destination: &str) -> &mut Self {
        let source = self.resolve(source);
        let destination = self.define(destination);

        self.add_instruction(TACInst::Copi {
            destination,
            source,
        })
    }

    pub fn binary(
        &mut self,
        opcode: TACBinaryOpcode,
        lhs: &str,
        rhs: &str,
        destination: &str,
    ) -> &mut Self {
        let lhs = self.resolve(lhs);
        let rhs = self.resolve(rhs);
        let result = self.define(destination);

        self.add_instruction(TACInst::BinaryOperation {
            opcode,
            lhs,
            rhs,
            result,
        })
    }

    pub fn add(&mut self, lhs: &str, rhs: &str, destination: &str) -> &mut Self {
        self.binary(TACBinaryOpcode::ADD, lhs, rhs, destination)
    }

    pub fn sub(&mut self, lhs: &str, rhs: &str, destination: &str) -> &mut Self {
        self.binary(TACBinaryOpcode::SUB, lhs, rhs, destination)
    }

    pub fn mul(&mut self, lhs: &str, rhs: &str, destination: &str) -> &mut Self {
        self.binary(TACBinaryOpcode::MUL, lhs, rhs, destination)
    }

    pub fn div(&mut self, lhs: &str, rhs: &str, destination: &str) -> &mut Self {
        self.binary(TACBinaryOpcode::DIV, lhs, rhs, destination)
    }

    pub fn label_decl(&mut self, label: &str) -> &mut Self {
        let label = self.resolve_label(label);
        self.add_instruction(TACInst::LabelDecl(label))
    }

    pub fn jump(&mut self, destination: &str) -> &mut Self {
        let destination = self.resolve_label(destination);
        self.add_instruction(TACInst::UnconditionalJump(destination))
    }

    pub fn branch_named(
        &mut self,
        opcode: TACJumpOpcode,
        condition: &str,
        destination: &str,
    ) -> &mut Self {
        let condition = self.resolve(condition);
        let destination = self.resolve_label(destination);

        self.add_instruction(TACInst::ConditionalJump {
            opcode,
            condition,
            destination,
        })
    }

    pub fn return_void(&mut self) -> &mut Self {
        self.add_instruction(TACInst::Return(None))
    }

    pub fn return_value(&mut self, value: TACTemp) -> &mut Self {
        self.add_instruction(TACInst::Return(Some(value)))
    }

    fn define(&mut self, name: &str) -> TACTemp {
        let name = Symbol::intern(name);
        let temp = self.fresh_temp();

        self.variables.insert(name, temp.clone());

        temp
    }

    fn resolve(&self, name: &str) -> TACTemp {
        let name = Symbol::intern(name);

        self.variables
            .get(&name)
            .cloned()
            .unwrap_or_else(|| panic!("use of undefined variable `{}`", name))
    }

    fn define_label(&mut self, name: &str) -> Label {
        let name = Symbol::intern(name);
        let label = self.fresh_label();

        self.labels.insert(name, label.clone());

        label
    }

    fn resolve_label(&mut self, name: &str) -> Label {
        let sym = Symbol::intern(name);

        match self.labels.get(&sym) {
            Some(label) => label.clone(),
            None => self.define_label(name),
        }
    }

    fn fresh_temp(&mut self) -> TACTemp {
        let temp = TACTemp::Temp(self.next_temp);
        self.next_temp += 1;
        temp
    }

    fn fresh_label(&mut self) -> Label {
        let label = Label(self.next_label);
        self.next_label += 1;
        label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::create_session_globals_then;

    #[test]
    fn builds_addition() {
        create_session_globals_then(|| {
            let mut proc = ProcBuilder::new("main");

            proc.constant("rhs", 10)
                .constant("lhs", 20)
                .add("lhs", "rhs", "result");

            let procedure = proc.build();

            eprintln!("{}", procedure);
        })
    }
}

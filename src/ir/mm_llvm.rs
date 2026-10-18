use std::collections::HashMap;

use inkwell::builder::BuilderError;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::BasicType;

use crate::frontend::ast::*;

use crate::types::{Name, ProcType, SemanticType, Type};

pub struct MMLLVM<'ctx> {
    context: &'ctx inkwell::context::Context,
    module: inkwell::module::Module<'ctx>,
    builder: inkwell::builder::Builder<'ctx>,
}

impl<'ctx> MMLLVM<'ctx> {
    fn new(context: &'ctx Context, module: Module<'ctx>) -> Self {
        Self {
            context,
            module,
            builder: context.create_builder(),
        }
    }

    pub fn munch(
        context: &'ctx Context,
        module: Module<'ctx>,
        program: Program,
        global_decls: HashMap<Name, SemanticType>,
    ) -> Result<Module<'ctx>, BuilderError> {
        let mm = MMLLVM::new(context, module);

        mm.populate_module(global_decls);
        mm.munch_program(program)
    }

    fn populate_module(&self, global_decls: HashMap<Name, SemanticType>) {
        for (key, value) in global_decls.into_iter() {
            match value {
                SemanticType::SimpleType(ty) => {
                    self.module.add_global(self.map_basic_type(ty), None, &key);
                }
                SemanticType::ProcType(proc_type) => {
                    self.module
                        .add_function(&key, self.map_proc_type(proc_type), None);
                }
            }
        }
    }

    fn map_basic_type(&self, ty: Type) -> inkwell::types::BasicTypeEnum<'ctx> {
        match ty {
            Type::Int => self.context.i64_type().into(),
            Type::Bool => self.context.bool_type().into(),
            _ => unreachable!(),
        }
    }

    fn map_proc_type(&self, proc_type: ProcType) -> inkwell::types::FunctionType<'ctx> {
        let args: Vec<inkwell::types::BasicMetadataTypeEnum<'ctx>> = proc_type
            .args_type
            .into_iter()
            .flatten()
            .map(|(_, ty)| self.map_basic_type(ty))
            .map(Into::into)
            .collect();

        match proc_type.return_type.as_ref() {
            Some(return_type) => {
                let return_type = self.map_basic_type(*return_type);
                return_type.fn_type(&args, false)
            }

            None => self.context.void_type().fn_type(&args, false),
        }
    }

    pub fn munch_program(self, program: Program) -> Result<Module<'ctx>, BuilderError> {
        for global in self.module.get_globals() {
            println!("{}", global);
        }

        for func in self.module.get_functions() {
            println!("{}", func);
        }

        for decl in program.0 {
            match decl {
                Declaration::Variable(variable) => todo!(),
                Declaration::Proc {
                    proc_name,
                    proc_args,
                    return_type,
                    block,
                } => todo!(),
            }
        }

        Ok(self.module)
    }

    fn munch_global_variable(&self, var: Variable) -> Result<(), BuilderError> {
        for (name, value_expr) in var.names.into_iter().zip(var.values) {
            let value = match *value_expr {
                Expression::Number(num) => num,
                Expression::Bool(boo) => boo.into(),
                _ => -42,
            };

            let global = self
                .module
                .get_global(&name)
                .ok_or(BuilderError::UnsetPosition)?;

            // let init = self.context.i64_type().const_int(value, sign_extend)
            // global.set_initializer(value);
        }

        Ok(())
    }
}

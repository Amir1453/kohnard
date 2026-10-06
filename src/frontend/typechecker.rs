use std::collections::HashMap;
use std::iter::zip;

use thin_vec::ThinVec;

use crate::frontend::{Symbol, ast::*};
use crate::structs::{ErrorAggregate, Stack};
use crate::types::{ProcType, SemanticType, Span, Type};

pub struct TypeChecker {
    scopes: Stack<HashMap<Symbol, SemanticType>>,
    current_proc_name: Option<Symbol>,
    errors: ErrorAggregate<TypeErrorType>,
}

impl TypeChecker {
    fn new() -> Self {
        Self {
            scopes: Stack::new(),
            current_proc_name: None,
            errors: ErrorAggregate::new(),
        }
    }

    pub fn check(
        program: &mut Program,
        global_decls: HashMap<Symbol, SemanticType>,
    ) -> Result<(), ErrorAggregate<TypeErrorType>> {
        let mut tc = TypeChecker::new();
        tc.scopes.push(global_decls);
        tc.check_program(program);
        tc.errors.resolve()
    }

    fn check_program(&mut self, program: &mut Program) {
        for decl in &mut program.0 {
            match decl {
                Declaration::Proc {
                    proc_name,
                    block,
                    proc_args,
                    ..
                } => {
                    self.current_proc_name = Some(*proc_name);
                    self.scopes.push(HashMap::new());

                    for (name, ty) in proc_args.iter_mut().flatten() {
                        self.current_scope_insert_simple(*name, *ty);
                    }

                    self.check_block(block);

                    self.scopes.pop();
                    self.current_proc_name = None;
                }

                Declaration::Variable(var) => self.check_global_variable(var),
            }
        }
    }

    fn check_global_variable(&mut self, var: &mut Variable) {
        let var_type = var.ty;
        for (_, expr) in zip(&var.names, &mut var.values) {
            let value_type = expr.get_type();
            if value_type != var_type {
                self.collect_error(TypeErrorType::TypeMismatchGlobalDecl, None);
            }

            // self.current_scope_insert_simple(name.clone(), value_type);
        }
    }

    fn check_block(&mut self, block: &mut Block) {
        self.scopes.push(HashMap::new());
        for stmt in &mut block.0 {
            self.check_statement(stmt);
        }
        self.scopes.pop();
    }

    fn check_statement(&mut self, stmt: &mut Statement) {
        match stmt {
            Statement::Variable(var) => self.check_variable(var),

            Statement::Block(block) => {
                self.check_block(block);
            }

            Statement::Assignment { name, value } => {
                self.check_expression(value);
                let value_type = value.get_type();
                if self.find_simple_type(name) != value_type {
                    self.collect_error(TypeErrorType::TypeMismatchAssign, None);
                }
            }

            Statement::Eval(expr) => self.check_expression(expr),

            Statement::Return(expr) => {
                match self
                    .find_proc_type(&self.current_proc_name.unwrap())
                    .return_type
                {
                    Some(expected) => {
                        if let Some(exp) = expr {
                            self.check_expression(exp);
                            let ty = exp.get_type();
                            if ty != expected {
                                self.collect_error(TypeErrorType::ReturnTypeMismatch, None);
                            }
                        } else {
                            self.collect_error(TypeErrorType::ReturnValueMissing, None);
                        }
                    }

                    None => {
                        if let Some(exp) = expr {
                            self.check_expression(exp);
                            let ty = exp.get_type();
                            if ty != Type::Void {
                                self.collect_error(TypeErrorType::ReturnTypeMismatchSub, None);
                            }
                        }
                    }
                }
            }

            Statement::If {
                condition,
                then_block,
                else_branch,
            } => {
                self.check_expression(condition);
                if condition.get_type() != Type::Bool {
                    self.collect_error(TypeErrorType::ConditionNotBool, None);
                }

                self.check_block(then_block);

                if let Some(ebr) = else_branch {
                    self.check_statement(ebr);
                }
            }

            Statement::While { condition, block } => {
                self.check_expression(condition);
                if condition.get_type() != Type::Bool {
                    self.collect_error(TypeErrorType::ConditionNotBool, None);
                }

                self.check_block(block);
            }

            Statement::Jump(_) => {}
        }
    }

    fn check_variable(&mut self, var: &mut Variable) {
        let var_type = var.ty;
        for (name, expr) in zip(&var.names, &mut var.values) {
            self.check_expression(expr);
            let value_type = expr.get_type();
            if value_type != var_type {
                self.collect_error(TypeErrorType::TypeMismatchDecl, None);
            }

            self.current_scope_insert_simple(*name, value_type);
        }
    }

    fn check_expression(&mut self, expr: &mut Expression) {
        match expr {
            Expression::Variable(name, ty) => {
                *ty = Some(self.find_simple_type(name));
            }

            Expression::UnaryOperation {
                operator,
                value,
                ty,
            } => {
                self.check_expression(value);
                let value_type = value.get_type();
                match operator {
                    Operator::Neg => {
                        if value_type != Type::Int {
                            self.collect_error(TypeErrorType::OpNegMismatch, None);
                            *ty = Some(Type::Error);
                        } else {
                            *ty = Some(Type::Int);
                        }
                    }

                    Operator::Tilde => {
                        if value_type != Type::Int {
                            self.collect_error(TypeErrorType::OpTildeMismatch, None);
                            *ty = Some(Type::Error);
                        } else {
                            *ty = Some(Type::Int);
                        }
                    }

                    Operator::LNot => {
                        if value_type != Type::Bool {
                            self.collect_error(TypeErrorType::OpLNotMismatch, None);
                            *ty = Some(Type::Error);
                        } else {
                            *ty = Some(Type::Bool);
                        }
                    }

                    _ => {}
                }
            }

            Expression::BinaryOperation {
                lhs,
                operator,
                rhs,
                ty,
            } => {
                self.check_expression(lhs);
                self.check_expression(rhs);
                let lhs_type = lhs.get_type();
                let rhs_type = rhs.get_type();

                use Operator::*;
                let exp_type = match operator {
                    Plus | Minus | Star | Slash | Mod => {
                        if lhs_type == Type::Int && rhs_type == Type::Int {
                            Some(Type::Int)
                        } else {
                            self.collect_error(TypeErrorType::OpArithMismatch, None);
                            Some(Type::Error)
                        }
                    }

                    Pipe | Caret | Ampersand | LShift | RShift => {
                        if lhs_type == Type::Int && rhs_type == Type::Int {
                            Some(Type::Int)
                        } else {
                            self.collect_error(TypeErrorType::OpBitwiseMismatch, None);
                            Some(Type::Error)
                        }
                    }

                    Equal | NEqual | L | LTE | G | GTE => {
                        if lhs_type == Type::Int && rhs_type == Type::Int {
                            Some(Type::Bool)
                        } else {
                            self.collect_error(TypeErrorType::OpCompareMismatch, None);
                            Some(Type::Error)
                        }
                    }

                    LAnd | LOr => {
                        if lhs_type == Type::Bool && rhs_type == Type::Bool {
                            Some(Type::Bool)
                        } else {
                            self.collect_error(TypeErrorType::OpLogicalMismatch, None);
                            Some(Type::Error)
                        }
                    }

                    _ => None,
                };
                *ty = exp_type;
            }

            Expression::ProcCall {
                proc_name,
                proc_args,
                ty,
            } => {
                if proc_name.as_str() == "print" {
                    let arg = proc_args.as_mut().unwrap().first_mut().unwrap();
                    self.check_expression(arg);
                    let arg_type = arg.get_type();
                    if arg_type != Type::Int && arg_type != Type::Bool {
                        self.collect_error(TypeErrorType::TypeMismatchProcArgument, None);
                    }
                } else {
                    let proc_type = self.find_proc_type(proc_name);
                    *ty = Some(proc_type.return_type.unwrap_or(Type::Void));

                    if let (Some(args), Some(arg_supposed_types)) =
                        (proc_args, proc_type.args_type.as_ref())
                    {
                        for (arg, (_name, expected_ty)) in
                            args.iter_mut().zip(arg_supposed_types.iter())
                        {
                            self.check_expression(arg);
                            let arg_type = arg.get_type();
                            if arg_type != *expected_ty {
                                self.collect_error(TypeErrorType::TypeMismatchProcArgument, None);
                            }
                        }
                    }
                }
            }

            _ => {}
        }
    }

    // Stack helpers

    pub fn current_scope_insert_simple(&mut self, name: Symbol, ty: Type) {
        if let Some(current) = self.scopes.top_mut() {
            current.insert(name, SemanticType::SimpleType(ty));
        }
    }

    #[allow(dead_code)]
    pub fn current_scope_insert_proc(
        &mut self,
        name: Symbol,
        args_type: Option<ThinVec<(Symbol, Type)>>,
        return_type: Option<Type>,
    ) {
        if let Some(current) = self.scopes.top_mut() {
            current.insert(
                name,
                SemanticType::ProcType(ProcType {
                    args_type,
                    return_type,
                }),
            );
        }
    }

    pub fn find_simple_type(&self, name: &Symbol) -> Type {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.get(name)
                && let SemanticType::SimpleType(ty) = t
            {
                return *ty;
            }
        }
        Type::Error
    }

    pub fn find_proc_type(&self, name: &Symbol) -> ProcType {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.get(name)
                && let SemanticType::ProcType(pt) = t
            {
                return pt.clone();
            }
        }
        ProcType::default()
    }

    // Error helper

    fn collect_error(&mut self, error_type: TypeErrorType, _span: Option<Span>) {
        self.errors.add_error(error_type);
    }
}

// Type Errors

#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub enum TypeErrorType {
    #[error("Type mismatch in declaration")]
    TypeMismatchDecl,
    #[error("Type mismatch in global declaration")]
    TypeMismatchGlobalDecl,
    #[error("Type mismatch in assignment")]
    TypeMismatchAssign,
    #[error("Type mismatch in procedure argument")]
    TypeMismatchProcArgument,

    #[error("Condition is not boolean")]
    ConditionNotBool,

    #[error("Wrong type used with -")]
    OpNegMismatch,
    #[error("Wrong type used with ~")]
    OpTildeMismatch,
    #[error("Wrong type used with !")]
    OpLNotMismatch,
    #[error("Wrong type used with arithmetics")]
    OpArithMismatch,
    #[error("Wrong type used with bitwise")]
    OpBitwiseMismatch,
    #[error("Wrong type used with comparasion")]
    OpCompareMismatch,
    #[error("Wrong type used with negation")]
    OpLogicalMismatch,

    #[error("Return value is missing")]
    ReturnValueMissing,
    #[error("Return type is mismatching")]
    ReturnTypeMismatch,
    #[error("Return type should be void")]
    ReturnTypeMismatchSub,
}

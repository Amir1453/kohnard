use std::collections::HashMap;
use std::iter::zip;

use crate::Symbol;
use crate::frontend::ast::*;
use crate::span::Span;
use crate::structs::{ErrorAggregate, Stack};
use crate::types::{ProcType, SemanticType, Ty, TyKind};

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
                Declaration::Proc(proc) => {
                    self.current_proc_name = Some(proc.proc_name);
                    self.scopes.push(HashMap::new());

                    for (name, ty) in proc.proc_args.iter_mut().flatten() {
                        self.current_scope_insert_simple(*name, *ty);
                    }

                    self.check_block(&mut proc.block);

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
        for stmt in &mut block.statements {
            self.check_statement(stmt);
        }
        self.scopes.pop();
    }

    fn check_statement(&mut self, stmt: &mut Statement) {
        match &mut stmt.kind {
            StatementKind::Variable(var) => self.check_variable(var),

            StatementKind::Block(block) => {
                self.check_block(block);
            }

            StatementKind::Assignment { target, value } => {
                self.check_expression(target);
                self.check_expression(value);
                let value_type = value.get_type();
                if target.get_type() != value_type {
                    self.collect_error(TypeErrorType::TypeMismatchAssign, None);
                }
            }

            StatementKind::Eval(expr) => self.check_expression(expr),

            StatementKind::Return(expr) => {
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
                            if ty != Ty::VOID {
                                self.collect_error(TypeErrorType::ReturnTypeMismatchSub, None);
                            }
                        }
                    }
                }
            }

            StatementKind::If {
                condition,
                then_block,
                else_branch,
            } => {
                self.check_expression(condition);
                if condition.get_type() != Ty::BOOL {
                    self.collect_error(TypeErrorType::ConditionNotBool, None);
                }

                self.check_block(then_block);

                if let Some(ebr) = else_branch {
                    self.check_statement(ebr);
                }
            }

            StatementKind::While { condition, block } => {
                self.check_expression(condition);
                if condition.get_type() != Ty::BOOL {
                    self.collect_error(TypeErrorType::ConditionNotBool, None);
                }

                self.check_block(block);
            }

            StatementKind::Jump(_) => {}
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
        match &mut expr.kind {
            ExpressionKind::Variable(name, ty) => {
                *ty = Some(self.find_simple_type(name));
            }

            ExpressionKind::UnaryOperation {
                operator,
                value,
                ty,
            } => {
                self.check_expression(value);
                let value_type = value.get_type();
                match operator {
                    Operator::Neg => {
                        if value_type != Ty::INT {
                            self.collect_error(TypeErrorType::OpNegMismatch, None);
                            *ty = Some(Ty::ERROR);
                        } else {
                            *ty = Some(Ty::INT);
                        }
                    }

                    Operator::Tilde => {
                        if value_type != Ty::INT {
                            self.collect_error(TypeErrorType::OpTildeMismatch, None);
                            *ty = Some(Ty::ERROR);
                        } else {
                            *ty = Some(Ty::INT);
                        }
                    }

                    Operator::LNot => {
                        if value_type != Ty::BOOL {
                            self.collect_error(TypeErrorType::OpLNotMismatch, None);
                            *ty = Some(Ty::ERROR);
                        } else {
                            *ty = Some(Ty::BOOL);
                        }
                    }

                    Operator::Ref => {
                        *ty = Some(Ty::intern(TyKind::Ref(value_type)));
                    }

                    Operator::MutRef => {
                        *ty = Some(Ty::intern(TyKind::MutRef(value_type)));
                    }

                    Operator::Deref => {
                        *ty = Some(match value_type.kind() {
                            TyKind::Ref(inner)
                            | TyKind::MutRef(inner)
                            | TyKind::Ptr(inner)
                            | TyKind::MutPtr(inner) => inner,
                            TyKind::Error => Ty::ERROR,
                            _ => {
                                self.collect_error(TypeErrorType::OpDerefMismatch, None);
                                Ty::ERROR
                            }
                        });
                    }

                    _ => {}
                }
            }

            ExpressionKind::BinaryOperation {
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
                        if lhs_type == Ty::INT && rhs_type == Ty::INT {
                            Some(Ty::INT)
                        } else {
                            self.collect_error(TypeErrorType::OpArithMismatch, None);
                            Some(Ty::ERROR)
                        }
                    }

                    Pipe | Caret | Ampersand | LShift | RShift => {
                        if lhs_type == Ty::INT && rhs_type == Ty::INT {
                            Some(Ty::INT)
                        } else {
                            self.collect_error(TypeErrorType::OpBitwiseMismatch, None);
                            Some(Ty::ERROR)
                        }
                    }

                    Equal | NEqual | L | LTE | G | GTE => {
                        if lhs_type == Ty::INT && rhs_type == Ty::INT {
                            Some(Ty::BOOL)
                        } else {
                            self.collect_error(TypeErrorType::OpCompareMismatch, None);
                            Some(Ty::ERROR)
                        }
                    }

                    LAnd | LOr => {
                        if lhs_type == Ty::BOOL && rhs_type == Ty::BOOL {
                            Some(Ty::BOOL)
                        } else {
                            self.collect_error(TypeErrorType::OpLogicalMismatch, None);
                            Some(Ty::ERROR)
                        }
                    }

                    _ => None,
                };
                *ty = exp_type;
            }

            ExpressionKind::ProcCall {
                proc_name,
                proc_args,
                ty,
            } => {
                if proc_name.as_str() == "print" {
                    *ty = Some(Ty::VOID);
                    let arg = proc_args.as_mut().unwrap().first_mut().unwrap();
                    self.check_expression(arg);
                    let arg_type = arg.get_type();
                    if arg_type != Ty::INT && arg_type != Ty::BOOL {
                        self.collect_error(TypeErrorType::TypeMismatchProcArgument, None);
                    }
                } else {
                    let proc_type = self.find_proc_type(proc_name);
                    *ty = Some(proc_type.return_type.unwrap_or(Ty::VOID));

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

    pub fn current_scope_insert_simple(&mut self, name: Symbol, ty: Ty) {
        if let Some(current) = self.scopes.top_mut() {
            current.insert(name, SemanticType::SimpleType(ty));
        }
    }

    pub fn find_simple_type(&self, name: &Symbol) -> Ty {
        for s in self.scopes.iter().rev() {
            if let Some(t) = s.get(name)
                && let SemanticType::SimpleType(ty) = t
            {
                return *ty;
            }
        }
        Ty::ERROR
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
    #[error("Cannot dereference this type")]
    OpDerefMismatch,
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

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
                            if ty.is_void() {
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
                if condition.get_type().is_bool() {
                    self.collect_error(TypeErrorType::ConditionNotBool, None);
                }

                self.check_block(then_block);

                if let Some(ebr) = else_branch {
                    self.check_statement(ebr);
                }
            }

            StatementKind::While { condition, block } => {
                self.check_expression(condition);
                if condition.get_type().is_bool() {
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
        let ty = &mut expr.ty;
        match &mut expr.kind {
            ExpressionKind::Variable(name) => {
                *ty = Some(self.find_simple_type(name));
            }

            ExpressionKind::UnaryOperation { operator, value } => {
                self.check_expression(value);
                let value_type = value.get_type();
                match self.check_unary_operation(*operator, value_type) {
                    Ok(typ) => *ty = Some(typ),
                    Err(err) => {
                        self.collect_error(err, None);
                        *ty = Some(Ty::ERROR)
                    }
                }
            }

            ExpressionKind::BinaryOperation { lhs, operator, rhs } => {
                self.check_expression(lhs);
                self.check_expression(rhs);
                let lhs_type = lhs.get_type();
                let rhs_type = rhs.get_type();
                match self.check_binary_operation(*operator, lhs_type, rhs_type) {
                    Ok(typ) => *ty = Some(typ),
                    Err(err) => {
                        self.collect_error(err, None);
                        *ty = Some(Ty::ERROR)
                    }
                }
            }

            ExpressionKind::ProcCall {
                proc_name,
                proc_args,
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

    fn check_unary_operation(&self, op: UnOp, operand_ty: Ty) -> Result<Ty, TypeErrorType> {
        use TypeErrorType::*;
        use UnOp::*;

        match op {
            Neg | Tilde if operand_ty.is_integral() => Ok(operand_ty),
            LNot if operand_ty.is_bool() => Ok(Ty::BOOL),
            Ref => Ok(Ty::intern(TyKind::Ref(operand_ty))),
            MutRef => Ok(Ty::intern(TyKind::MutRef(operand_ty))),
            Deref => operand_ty.deref().ok_or(OpDerefMismatch),
            _ => Err(UnOpMismatch),
        }
    }

    fn check_binary_operation(&self, op: BinOp, lhs: Ty, rhs: Ty) -> Result<Ty, TypeErrorType> {
        use BinOp::*;
        use TypeErrorType::*;

        let both_int = lhs == Ty::INT && rhs == Ty::INT;

        match op {
            Plus | Minus | Star | Slash | Mod if both_int => Ok(Ty::INT),
            Plus | Minus | Star | Slash | Mod => Err(OpArithMismatch),

            Pipe | Caret | Ampersand | LShift | RShift if both_int => Ok(Ty::INT),
            Pipe | Caret | Ampersand | LShift | RShift => Err(OpBitwiseMismatch),

            Equal | NEqual | L | LTE | G | GTE if both_int => Ok(Ty::BOOL),
            Equal | NEqual | L | LTE | G | GTE => Err(OpCompareMismatch),

            LAnd | LOr if lhs == Ty::BOOL && rhs == Ty::BOOL => Ok(Ty::BOOL),
            LAnd | LOr => Err(OpLogicalMismatch),
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

    #[error("Mismatch on binary operation types")]
    BinOpMismatch,
    #[error("Mismatch on unary operation type")]
    UnOpMismatch,

    #[error("Return value is missing")]
    ReturnValueMissing,
    #[error("Return type is mismatching")]
    ReturnTypeMismatch,
    #[error("Return type should be void")]
    ReturnTypeMismatchSub,
}

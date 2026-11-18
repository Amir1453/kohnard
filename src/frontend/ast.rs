use crate::Symbol;
use crate::types::{Promise, Ty};

use thin_vec::ThinVec;

pub struct Program(pub Vec<Declaration>);

pub enum Declaration {
    Variable(Box<Variable>),

    Proc {
        proc_name: Symbol,
        proc_args: Option<ThinVec<(Symbol, Ty)>>,
        return_type: Option<Ty>,
        block: Box<Block>,
    },
}

pub struct Variable {
    pub names: ThinVec<Symbol>,
    pub values: ThinVec<Box<Expression>>,
    pub ty: Ty,
    #[allow(unused)]
    pub scope: ScopeState,
}

// pub struct ProcDecl {
//     pub proc_name: (),
//     pub proc_args: (),
//     pub return_type: (),
//     pub block: (),
//     pub span: (),
// }

pub struct Block(pub ThinVec<Statement>);

// pub struct Statement {
//     pub kind: (),
//     pub span: (),
// }

pub enum Statement {
    Variable(Box<Variable>),

    Block(Box<Block>),

    Assignment {
        name: Symbol,
        value: Box<Expression>,
    },

    // DerefAssignment {
    //     name: Symbol,
    //     value: Box<Expression>,
    // },
    Eval(Box<Expression>),

    Return(Option<Box<Expression>>),

    If {
        condition: Box<Expression>,
        then_block: Box<Block>,
        else_branch: Option<Box<Statement>>,
    },

    While {
        condition: Box<Expression>,
        block: Box<Block>,
    },

    Jump(JumpState),
}

// pub struct Expr {
//     pub id: (),
//     pub kind: (),
//     pub span: (),
// }

pub enum Expression {
    Variable(Symbol, Promise<Ty>),
    Number(i64),
    Bool(bool),

    BinaryOperation {
        lhs: Box<Expression>,
        operator: Operator,
        rhs: Box<Expression>,
        ty: Promise<Ty>,
    },

    UnaryOperation {
        operator: Operator,
        value: Box<Expression>,
        ty: Promise<Ty>,
    },

    ProcCall {
        proc_name: Symbol,
        proc_args: Option<ThinVec<Box<Expression>>>,
        ty: Promise<Ty>,
    },
}

impl Expression {
    pub fn get_type(&self) -> Ty {
        match self {
            Expression::Variable(_, ty) => ty.unwrap_or(Ty::PROMISED),
            Expression::Number(_) => Ty::INT,
            Expression::Bool(_) => Ty::BOOL,
            Expression::UnaryOperation { ty, .. } => ty.unwrap_or(Ty::PROMISED),
            Expression::BinaryOperation { ty, .. } => ty.unwrap_or(Ty::PROMISED),
            Expression::ProcCall { ty, .. } => ty.unwrap_or(Ty::PROMISED),
        }
    }
}

pub enum Operator {
    Plus,
    Minus,
    Star,
    Slash,
    Mod,
    Neg,

    Pipe,
    Caret,
    Ampersand,
    Tilde,
    LShift,
    RShift,

    Equal,
    NEqual,
    L,
    LTE,
    G,
    GTE,
    LAnd,
    LOr,
    LNot,

    Ref,
    MutRef,
    Deref,
}

pub enum ScopeState {
    Global,
    Local,
}

pub enum JumpState {
    Break,
    Continue,
}

use crate::Symbol;
use crate::span::Span;
use crate::types::{Promise, Ty};

use thin_vec::ThinVec;

pub struct Program(pub Vec<Declaration>);

pub enum Declaration {
    Variable(Box<Variable>),

    Proc(Box<ProcDecl>),
}

pub struct Variable {
    pub names: ThinVec<Symbol>,
    pub values: ThinVec<Box<Expression>>,
    pub ty: Ty,
    pub span: Span,
}

pub struct ProcDecl {
    pub proc_name: Symbol,
    pub proc_args: Option<ThinVec<(Symbol, Ty)>>,
    pub return_type: Option<Ty>,
    pub block: Box<Block>,
    pub span: Span,
}

pub struct Block {
    pub statements: ThinVec<Statement>,
    pub span: Span,
}

pub struct Statement {
    pub kind: StatementKind,
    pub span: Span,
}

pub enum StatementKind {
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

pub struct Expression {
    // pub id: (),
    pub kind: ExpressionKind,
    pub span: Span,
}

pub enum ExpressionKind {
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
        self.kind.get_type()
    }
}

impl ExpressionKind {
    pub fn get_type(&self) -> Ty {
        match self {
            Self::Variable(_, ty) => ty.unwrap_or(Ty::PROMISED),
            Self::Number(_) => Ty::INT,
            Self::Bool(_) => Ty::BOOL,
            Self::UnaryOperation { ty, .. } => ty.unwrap_or(Ty::PROMISED),
            Self::BinaryOperation { ty, .. } => ty.unwrap_or(Ty::PROMISED),
            Self::ProcCall { ty, .. } => ty.unwrap_or(Ty::PROMISED),
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

pub enum JumpState {
    Break,
    Continue,
}

use crate::Symbol;
use crate::span::Span;
use crate::types::{Promise, Ty};

use thin_vec::ThinVec;

/// Represents the overall program structure for one compilation unit.
pub struct Program(pub Vec<Declaration>);

/// In general, at the high level we basically only have declarations.
pub enum Declaration {
    /// This represents a global variable declaration.
    Variable(Box<Variable>),
    // /// This represents a structure declaration.
    // Structure(Box<Variable>),
    /// This represents a procedure declaration.
    Proc(Box<ProcDecl>),
}

pub struct Variable {
    pub names: ThinVec<Symbol>,
    pub values: ThinVec<Box<Expression>>,
    pub ty: Ty,
    pub span: Span,
}

/// Represents a structure declaration..
pub struct Structure {
    /// The name of the structure.
    pub name: Symbol,
    /// The span of the structure declaration.
    pub span: Span,
    /// The fields of the declaration.
    /// An empty vector represents no fields.
    pub fields: ThinVec<FieldDef>,
}

/// Represents a field of a struct.
pub struct FieldDef {
    pub name: Symbol,
    pub span: Span,
    pub ty: Ty,
}

pub struct ProcDecl {
    pub proc_name: Symbol,
    pub proc_args: Option<ThinVec<(Symbol, Ty)>>,
    pub return_type: Option<Ty>,
    pub block: Box<Block>,
    pub span: Span,
}

#[derive(Clone, Copy, Debug)]
pub struct Ident {
    pub name: Symbol,
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
        target: Box<Expression>,
        value: Box<Expression>,
    },

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
    pub ty: Promise<Ty>,
}

pub enum ExpressionKind {
    Variable(Symbol),
    Number(i64),
    Bool(bool),

    BinaryOperation {
        lhs: Box<Expression>,
        operator: Operator,
        rhs: Box<Expression>,
    },

    UnaryOperation {
        operator: Operator,
        value: Box<Expression>,
    },

    ProcCall {
        proc_name: Symbol,
        proc_args: Option<ThinVec<Box<Expression>>>,
    },
}

impl Expression {
    pub fn get_type(&self) -> Ty {
        self.ty.unwrap_or(Ty::PROMISED)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum JumpState {
    Break,
    Continue,
}

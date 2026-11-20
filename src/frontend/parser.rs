use thin_vec::ThinVec;

use crate::Symbol;
use crate::frontend::ast::{BinOp, UnOp};
use crate::frontend::ast::{Block, Declaration, ProcDecl};
use crate::frontend::ast::{Expression, ExpressionKind};
use crate::frontend::ast::{Statement, StatementKind};
use crate::span::Span;
use crate::types::Ty;

/// The parsing context that is passed on to the parser.
pub(crate) struct ParseCtxt {
    offset: u32,
    // next_expr: Cell<usize>,
}

impl ParseCtxt {
    /// Constructs a new ParseCtxt, with an offset.
    pub const fn new(offset: u32) -> Self {
        Self { offset }
    }

    /// Converts a file-local byte offset into a global SourceMap byte offset by adding the file’s base offset.
    pub fn position(&self, local: usize) -> u32 {
        let local = u32::try_from(local).expect("source position exceeds u32 range");
        self.offset
            .checked_add(local)
            .expect("source position exceeds u32 address space")
    }

    /// Constructs a Span.
    pub fn span(&self, start: usize, end: usize) -> Span {
        Span::new(self.position(start), self.position(end))
    }

    pub fn expr(&self, kind: ExpressionKind, span: Span, ty: Option<Ty>) -> Box<Expression> {
        Box::new(Expression { kind, span, ty })
    }

    pub fn unary(&self, operator: UnOp, value: Box<Expression>, span: Span) -> Box<Expression> {
        self.expr(
            ExpressionKind::UnaryOperation { operator, value },
            span,
            None,
        )
    }

    pub fn binary(
        &self,
        operator: BinOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
        span: Span,
    ) -> Box<Expression> {
        self.expr(
            ExpressionKind::BinaryOperation { lhs, operator, rhs },
            span,
            None,
        )
    }

    pub fn proc_decl(
        &self,
        proc_name: Symbol,
        proc_args: Option<ThinVec<(Symbol, Ty)>>,
        return_type: Option<Ty>,
        block: Box<Block>,
        span: Span,
    ) -> Declaration {
        Declaration::Proc(Box::new(ProcDecl {
            proc_name,
            proc_args,
            return_type,
            block,
            span,
        }))
    }

    pub fn statement(&self, kind: StatementKind, span: Span) -> Statement {
        Statement { kind, span }
    }
}

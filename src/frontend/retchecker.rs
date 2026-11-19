use compact_str::CompactString;

use crate::frontend::ast::{Block, Program, Statement, StatementKind};
use crate::structs::ErrorAggregate;

pub struct RetChecker {}

impl RetChecker {
    pub fn check(program: &Program) -> Result<(), ErrorAggregate<NoReturnError>> {
        let mut errors = ErrorAggregate::<NoReturnError>::new();

        for decl in &program.0 {
            if let super::ast::Declaration::Proc(proc) = decl {
                if proc.proc_name.as_str() == "main" {
                    continue;
                }

                if !Self::check_block(&proc.block) {
                    errors.add_error(NoReturnError(proc.proc_name.as_str().into()));
                }
            }
        }

        errors.resolve()
    }

    fn check_block(block: &Block) -> bool {
        for statement in &block.statements {
            if Self::check_statement(statement) {
                return true;
            }
        }
        false
    }

    fn check_statement(statement: &Statement) -> bool {
        match &statement.kind {
            StatementKind::Return(_) => true,

            StatementKind::Block(block) => Self::check_block(block),

            StatementKind::If {
                then_block,
                else_branch: Some(else_branch),
                ..
            } => Self::check_block(then_block) && Self::check_statement(else_branch),

            // Assume that while statements return for now
            // Needs return analysis
            StatementKind::While { .. } => true,

            _ => false,
        }
    }
}

#[derive(thiserror::Error, Clone, Debug, PartialEq)]
#[error("function {0} does not return")]
pub struct NoReturnError(CompactString);

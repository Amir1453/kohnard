pub mod basic_block;
pub mod mm;
pub mod tac;

#[cfg(test)]
pub mod tac_builder;

pub use basic_block::{BasicBlock, BasicBlocks};
pub use mm::MM;

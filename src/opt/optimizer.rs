use petgraph::visit::Dfs;

use crate::ir::BasicBlocks;
use crate::ir::basic_block::{BlockIdx, BlockTerminator};
use crate::ir::tac::{CUTAC, InstBlock, TACDeclaration};

pub struct Optimizer {}

impl Optimizer {
    pub fn optimize(cutac: CUTAC) -> CUTAC {
        use TACDeclaration::*;
        let mut optimized_cutac: CUTAC = CUTAC(Vec::with_capacity(cutac.len()));

        for decl in cutac.0.into_iter() {
            if let ProcDecl(proc) = decl {
                let instructions = Self::optimize_instructions(proc.instructions);

                optimized_cutac.push(ProcDecl(crate::ir::tac::ProcDecl {
                    name: proc.name,
                    arguments: proc.arguments,
                    instructions,
                }));
            } else {
                optimized_cutac.push(decl);
            }
        }
        optimized_cutac
    }

    fn optimize_instructions(instructions: InstBlock) -> InstBlock {
        let mut bb = BasicBlocks::from_instrs(instructions);

        mark_unreachable(&mut bb);
        coalesce_blocks(&mut bb);

        bb.serialize()
    }
}

/// Marks unreachable nodes in the Control Flow Graph by doing a depth first search,
/// marking all the unvisited nodes, and setting their terminators to Nop.
fn mark_unreachable(bb: &mut BasicBlocks) {
    let entry = BlockIdx::new(0);
    let mut dfs = Dfs::new(&*bb, entry);

    while dfs.next(&*bb).is_some() {}

    let unreachable_nodes: Vec<BlockIdx> = bb
        .blocks
        .iter_enumerated()
        .filter(|(idx, _)| !dfs.discovered.contains(idx))
        .map(|(idx, _)| idx)
        .collect();

    for unreachable_node in unreachable_nodes {
        bb.blocks[unreachable_node].set_terminator(BlockTerminator::Nop);
    }
}

/// Given two blocks, A and B, combines them if succ(A) = {B}, and pred(B) = {A}.
/// Where succ(A) denotes the successor blocks of A, and pred(B) the predecessors of B.
fn coalesce_blocks(bb: &mut BasicBlocks) {
    let predecessors = bb.predecessors();
    let mut tb_merged: Vec<(BlockIdx, BlockIdx)> = Vec::new();

    for (idx, block) in bb.blocks.iter_enumerated() {
        let block_succs: Vec<BlockIdx> = block.successors().collect();

        if block_succs.len() != 1 {
            continue;
        }

        let succ_idx = block_succs[0];
        let succ_preds = &predecessors[succ_idx];

        if succ_preds.len() != 1 {
            continue;
        }

        tb_merged.push((idx, succ_idx));
    }
}

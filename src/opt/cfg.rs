use std::collections::HashMap;

/// The Control Flow Graph of a procedure. Contains a graph over basic blocks of the program.
/// Multiple code optimizations are possible with this representation.
pub struct CFG {
    /// The graph of basic blocks. Each node is a basic block, and each edge weight represents an
    /// index of the instruction that diverts the control flow to the respective block.
    pub graph: StableGraph<BasicBlock, usize>,
    /// The procedure name from which the CFG was constructed.
    pub name: Symbol,
    /// The arguments of the procedure from which the CFG was constructed.
    pub arguments: Option<Vec<Symbol>>,
}

impl CFG {
    /// Given two blocks A and B, if A has a conditional jump edge to B, and B has
    /// a conditional jump edge to some block C, unconditionally jump from B to C if
    /// the conditional edge from A has some sort of implication on the conditional
    /// jump to C.
    pub fn jump_threading(&mut self) {
        use TACInst::*;

        let nodes: Vec<NodeIndex> = self.graph.node_indices().collect();

        let mut i = 0usize;
        while i < nodes.len() {
            let idx = nodes[i];
            if self.graph.node_weight(idx).is_none() {
                i += 1;
                continue;
            }

            // Get the outgoing edges of our current block.
            let out_edges: Vec<(NodeIndex, usize)> = self
                .graph
                .edges_directed(idx, Direction::Outgoing)
                .map(|e| (e.target(), *e.weight()))
                .collect();

            // We continue for every edge instruction that is a ConditionalJump.
            for (successor_idx, pred_instr_idx) in out_edges {
                let instr = self.graph[idx].instructions[pred_instr_idx].clone();
                let (opcode1, cond1, _dest1) = match instr {
                    ConditionalJump {
                        opcode,
                        condition,
                        destination,
                    } => (opcode, condition, destination),

                    _ => continue,
                };

                // If the edge is a ConditionalJump, get the predecessors of the block the
                // ConditionalJump points to (successor block).
                // TODO We only work with one predecessor for now.
                if self
                    .graph
                    .neighbors_directed(successor_idx, Direction::Incoming)
                    .count()
                    != 1
                {
                    continue;
                }

                // Now, get the outgoing edges of our successor block. We will be changing all the
                // edges of the successor block that can be changed via opcode matching.
                let out_edges_successor: Vec<(NodeIndex, usize)> = self
                    .graph
                    .edges_directed(successor_idx, Direction::Outgoing)
                    .map(|e| (e.target(), *e.weight()))
                    .collect();

                // Go through the successor edges, and once again match against ConditionalJump
                // instructions.
                for (_successor2_idx, succ_instr_idx) in out_edges_successor {
                    let suc_instr = self.graph[successor_idx].instructions[succ_instr_idx].clone();
                    let (opcode2, cond2, dest2) = match suc_instr {
                        ConditionalJump {
                            opcode,
                            condition,
                            destination,
                        } => (opcode, condition, destination),

                        _ => continue,
                    };

                    // If the conditions are not the same, or if the successor block is modifying
                    // the condition, we cannot make this optimization.
                    // TODO Direct equality does not suffice, as we have no SSA.
                    if cond1 != cond2
                        || self.graph[successor_idx]
                            .instructions
                            .iter()
                            .any(|instr| instr.modifies_temp(&cond2))
                    {
                        continue;
                    }

                    // Match against the opcodes we have to see what we can do. We will have to
                    // modify the instructions inside of the successor block, as well as updating
                    // the edges.
                    let taken_or_not = jthread_match_opcode(opcode1, opcode2);

                    // This is the branch where the ConditionalJump is always taken.
                    if matches!(taken_or_not, TakenOrNot::AlwaysTaken) {
                        let successor_block = &mut self.graph[successor_idx];

                        // We replace our ConditionalJump instruction with an UnconditionalJump.
                        // Now, we can get rid of the rest of the block body after the
                        // ConditionalJump, since it will not be reached.
                        let new_instr = UnconditionalJump(dest2);
                        successor_block.instructions[succ_instr_idx] = new_instr;
                        successor_block.instructions.truncate(succ_instr_idx + 1);

                        // Since we got rid of the instructions, we get rid of the edges associated
                        // to those instructions as well.
                        let edges_to_remove: Vec<_> = self
                            .graph
                            .edges_directed(successor_idx, Direction::Outgoing)
                            .filter(|edge| *edge.weight() > succ_instr_idx)
                            .map(|edge| edge.id())
                            .collect();

                        for edge_idx in edges_to_remove {
                            self.graph.remove_edge(edge_idx);
                        }
                    }

                    // This is the branch where the ConditionalJump is never taken.
                    if matches!(taken_or_not, TakenOrNot::NeverTaken) {
                        let successor_block = &mut self.graph[successor_idx];

                        // We remove the ConditionalJump instruction, since it will never be taken.
                        successor_block.instructions.remove(succ_instr_idx);

                        // We have to remove the edge associated to this ConditionalJump insruction.
                        let edges_to_remove: Vec<_> = self
                            .graph
                            .edges_directed(successor_idx, Direction::Outgoing)
                            .filter(|edge| *edge.weight() == succ_instr_idx)
                            .map(|edge| edge.id())
                            .collect();

                        for edge_idx in edges_to_remove {
                            self.graph.remove_edge(edge_idx);
                        }

                        // Now we have to update all the affected edges.
                        let edges_to_update: Vec<_> = self
                            .graph
                            .edges_directed(successor_idx, Direction::Outgoing)
                            .filter(|edge| *edge.weight() > succ_instr_idx)
                            .map(|edge| edge.id())
                            .collect();

                        for edge_idx in edges_to_update {
                            if let Some(edge) = self.graph.edge_weight_mut(edge_idx) {
                                *edge -= 1;
                            }
                        }
                    }

                    break;
                }
            }

            i += 1;
        }
    }
}

/// Matches two conditional jump codes to see if some sort of implication can be reached.
fn jthread_match_opcode(opcode1: TACJumpOpcode, opcode2: TACJumpOpcode) -> TakenOrNot {
    // Same opcode implies always taken
    if opcode1 == opcode2 {
        return TakenOrNot::AlwaysTaken;
    }

    use TACJumpOpcode::*;
    match (opcode1, opcode2) {
        // x < 0 -> x != 0, x <= 0.
        // x > 0 -> x != 0, x >= 0.
        (JL, JNZ | JLE) | (JNLE, JNZ | JNL) => TakenOrNot::AlwaysTaken,

        // x = 0 -> x <= 0, x >= 0.
        (JZ, JLE | JNL) => TakenOrNot::AlwaysTaken,

        // x < 0 -> not x = 0, not x > 0, not x >= 0.
        // x > 0 -> not x = 0, not x < 0, not x <= 0.
        (JL, JZ | JNL | JNLE) | (JNLE, JZ | JL | JLE) => TakenOrNot::NeverTaken,

        // x >= 0 -> not x < 0.
        // x <= 0 -> not x > 0.
        (JNL, JL) | (JLE, JNLE) => TakenOrNot::NeverTaken,

        // x = 0 => not x < 0, not x != 0, not x > 0.
        // x != 0 => not x = 0.
        (JZ, JL | JNZ | JNLE) | (JNZ, JZ) => TakenOrNot::NeverTaken,

        // We cannot make deductions about the rest.
        _ => TakenOrNot::Unknown,
    }
}

/// The enum which represents possible matchings of conditional jump opcodes.
enum TakenOrNot {
    /// The conditional branch is always taken.
    AlwaysTaken,
    /// The conditional branch is never taken.
    NeverTaken,
    /// A deduction cannot be made.
    Unknown,
}

// #[cfg(test)]
// mod tests {
//     use crate::globals::create_session_globals_then;
//     use crate::ir::tac::TACJumpOpcode;
//     use crate::ir::tac_builder::ProcBuilder;
//
//     #[test]
//     fn jthreading_always_taken() {
//         create_session_globals_then(|| {
//             let mut proc = ProcBuilder::new("class");
//             proc.add_argument("x");
//
//             proc.constant("zero_1", 0)
//                 .branch_named(TACJumpOpcode::JNLE, "zero_1", "l0")
//                 .jump("l1");
//
//             proc.label_decl("l1");
//
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("l0");
//
//             proc.branch_named(TACJumpOpcode::JNLE, "zero_1", "l3")
//                 .jump("ret");
//
//             proc.label_decl("l3");
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("ret");
//             proc.return_void();
//
//             let procedure = proc.build();
//
//             eprintln!("{procedure}");
//
//             let mut cfg = super::CFG::from(procedure);
//             cfg.jump_threading();
//
//             eprintln!("{}", super::ProcDecl::from(cfg));
//         })
//     }
//
//     #[test]
//     fn jthreading_never_taken() {
//         create_session_globals_then(|| {
//             let mut proc = ProcBuilder::new("class");
//             proc.add_argument("x");
//
//             proc.constant("zero_1", 0)
//                 .branch_named(TACJumpOpcode::JNLE, "zero_1", "l0")
//                 .jump("l1");
//
//             proc.label_decl("l1");
//
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("l0");
//
//             proc.branch_named(TACJumpOpcode::JZ, "zero_1", "l3")
//                 .jump("ret");
//
//             proc.label_decl("l3");
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("ret");
//             proc.return_void();
//
//             let procedure = proc.build();
//
//             eprintln!("{procedure}");
//
//             let mut cfg = super::CFG::from(procedure);
//             cfg.jump_threading();
//
//             eprintln!("{}", super::ProcDecl::from(cfg));
//         })
//     }
//
//     #[test]
//     fn jthreading_modified_condition() {
//         create_session_globals_then(|| {
//             let mut proc = ProcBuilder::new("class");
//             proc.add_argument("x");
//
//             proc.constant("zero_1", 0)
//                 .branch_named(TACJumpOpcode::JNLE, "zero_1", "l0")
//                 .jump("l1");
//
//             proc.label_decl("l1");
//
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("l0");
//
//             proc.add("zero_1", "error_value", "zero_1");
//
//             proc.branch_named(TACJumpOpcode::JZ, "zero_1", "l3")
//                 .jump("ret");
//
//             proc.label_decl("l3");
//             proc.constant("error_value", -69)
//                 .copy("error_value", "returned_value")
//                 .jump("ret");
//
//             proc.label_decl("ret");
//             proc.return_void();
//
//             let procedure = proc.build();
//
//             eprintln!("{procedure}");
//
//             let mut cfg = super::CFG::from(procedure);
//             cfg.jump_threading();
//
//             eprintln!("{}", super::ProcDecl::from(cfg));
//         })
//     }
// }

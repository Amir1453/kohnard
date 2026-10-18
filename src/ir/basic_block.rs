#![allow(dead_code)]

use hashbrown::HashMap;
use typed_index_collections::TiVec as IndexVec;

use crate::ir::tac::{InstBlock, Label, TACInst, TACJumpOpcode, TACTemp};

/// Represents a collection of BasicBlocks.
pub struct BasicBlocks {
    /// The collection of basic blocks.
    pub blocks: IndexVec<BlockIdx, BasicBlock>,
}

impl BasicBlocks {
    /// Creates an empty BasicBlocks collection.
    const fn empty() -> Self {
        Self {
            blocks: IndexVec::new(),
        }
    }

    /// Given a sequence of instructions, converts them into basic block form.
    /// A basic block starts with a leader. The firs instructions is a leader.
    /// A basic block ends with a control flow divergence (jumps and return).
    ///
    /// To infer a collection of basic blocks from a sequence of instructions:
    /// 1. Start a BasicBlock.
    /// 2. Insert a label if there is not one.
    /// 3. Accumulate instructions until a jump or return.
    /// 4. If there is a fall-through instruction, add an unconditional jump.
    pub fn from_instrs(instructions: InstBlock) -> Self {
        use TACInst::*;

        // TODO Change usize -> InstrIdx
        let mut basic_blocks: BasicBlocks = BasicBlocks::empty();

        // Set of current leaders, stored as InstrIdx of instructions.
        let mut leaders: Vec<usize> = Vec::with_capacity(5);
        // Denotes whether the next instruction is a leader. First one is.
        let mut next_is_leader = true;

        // Maps each Label in instructions to a leader.
        let mut label_to_leader: HashMap<Label, usize> = HashMap::new();
        // Stores consecutive labels.
        let mut consecutive_labels: Vec<Label> = Vec::with_capacity(4);

        let instr_len = instructions.len();
        for (instr_idx, instr) in instructions.iter().enumerate() {
            // The instructions after n-consecutive labels is a leader.
            if let LabelDecl(label) = instr {
                next_is_leader = true;
                consecutive_labels.push(*label);
                continue;
            }

            // If next one is leader, either it is after n-consecutive labels,
            // or after a terminator.
            if next_is_leader {
                leaders.push(instr_idx);
            }

            // instr_idx is guaranteed to be a leader.
            for label in consecutive_labels.drain(..) {
                label_to_leader.insert(label, instr_idx);
            }

            next_is_leader = instr.is_terminator();
        }

        // Maps leaders to BlockIdx.
        let mut leader_to_block: HashMap<usize, BlockIdx> = HashMap::new();
        for leader in &leaders {
            let block_idx = basic_blocks.push_block();
            leader_to_block.insert(*leader, block_idx);
        }

        // Maps Labels to BlockIdx.
        let label_to_block: HashMap<Label, BlockIdx> = label_to_leader
            .into_iter()
            .map(|(label, idx)| (label, leader_to_block[&idx]))
            .collect();

        // For every pair (start, end) in leaders, go over the instructions and add them to
        // BasicBlocks, specified by the map leader_to_block.
        for (start, end) in leaders
            .windows(2)
            .map(|w| (w[0], w[1]))
            .chain(leaders.last().copied().map(|x| (x, instr_len)))
        {
            if let Some(&current_block) = leader_to_block.get(&start) {
                use BlockTerminator::*;

                let bb = &mut basic_blocks.blocks[current_block];

                for &inst in &instructions[start..end] {
                    match inst {
                        UnconditionalJump(label) => bb.set_terminator(Jump(label_to_block[&label])),

                        ConditionalJump {
                            opcode,
                            condition,
                            destination,
                        } => bb.set_terminator(Branch {
                            opcode,
                            condition,
                            taken: label_to_block[&destination],
                            not_taken: leader_to_block[&end],
                        }),

                        TACInst::Return(tactemp) => bb.set_terminator(Return(tactemp)),

                        LabelDecl(_) | TACInst::Nop => continue,

                        _ => bb.push_instruction(inst),
                    }
                }

                if matches!(bb.terminator, Nop) {
                    bb.set_terminator(Jump(leader_to_block[&end]));
                }
            }
        }

        basic_blocks
    }

    /// Pushes a new block and returns its BlockIdx.
    fn push_block(&mut self) -> BlockIdx {
        let idx = BlockIdx(self.blocks.len() as u32);
        self.blocks.push(BasicBlock::new());
        idx
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockIdx(u32);

impl From<usize> for BlockIdx {
    fn from(value: usize) -> Self {
        debug_assert!(value <= u32::MAX as usize);
        Self(value as u32)
    }
}

impl From<BlockIdx> for usize {
    fn from(value: BlockIdx) -> Self {
        value.0 as usize
    }
}

/// Represents a sequence of instructions that start with a label and end with
/// an unconditional jump or a return.
pub struct BasicBlock {
    /// Instructions that are contained in the block.
    pub instructions: InstBlock,
    /// The terminator of the block.
    pub terminator: BlockTerminator,
}

impl BasicBlock {
    /// Constructs an empty block.
    pub const fn new() -> Self {
        Self {
            instructions: InstBlock::new(),
            terminator: BlockTerminator::Nop,
        }
    }

    /// Pushes an instruction into the block.
    pub fn push_instruction(&mut self, inst: TACInst) {
        self.instructions.push(inst);
    }

    /// Pops the last instruction from the block.
    pub fn pop_instruction(&mut self) -> Option<TACInst> {
        self.instructions.pop()
    }

    pub fn set_terminator(&mut self, term: BlockTerminator) {
        self.terminator = term;
    }
}

/// All Basic Blocks must end in a BlockTerminator.
#[derive(Copy, Clone, Hash, PartialEq, Default)]
pub enum BlockTerminator {
    Return(Option<TACTemp>),

    Jump(BlockIdx),

    Branch {
        opcode: TACJumpOpcode,
        condition: TACTemp,
        taken: BlockIdx,
        not_taken: BlockIdx,
    },

    #[default]
    Nop,
}

impl std::fmt::Display for BasicBlocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, block) in self.blocks.iter_enumerated() {
            writeln!(f, "{}:", idx)?;
            writeln!(f, "{}", block)?
        }
        Ok(())
    }
}

impl std::fmt::Display for BlockIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for inst in &self.instructions {
            writeln!(f, "   {}", inst)?;
        }
        writeln!(f, "   {}", self.terminator)
    }
}

impl std::fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for BlockTerminator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BlockTerminator::*;

        match self {
            Return(tactemp) => match tactemp {
                Some(temp) => write!(f, "ret {}", temp),
                None => write!(f, "ret"),
            },

            Jump(block_idx) => write!(f, "br label %{}", block_idx),

            BlockTerminator::Branch {
                opcode,
                condition,
                taken,
                not_taken,
            } => write!(
                f,
                "br {} {}, label %{}, label %{}",
                opcode, condition, taken, not_taken
            ),

            Nop => write!(f, ""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::globals::create_session_globals_then;
    use crate::ir::basic_block::BasicBlocks;
    use crate::ir::tac_builder::ProcBuilder;

    fn dump_blocks(name: &str, build: impl FnOnce(&mut ProcBuilder)) {
        create_session_globals_then(|| {
            let mut proc = ProcBuilder::new(name);
            build(&mut proc);

            let blocks = BasicBlocks::from_instrs(proc.build().instructions);

            eprintln!("\n========== {name} ==========");
            eprintln!("{blocks}");
        });
    }

    #[test]
    fn single_basic_block() {
        dump_blocks("single_block", |p| {
            p.constant("a", 10)
                .constant("b", 20)
                .add("a", "b", "c")
                .return_void();
        });
    }

    #[test]
    fn unconditional_forward_jump() {
        dump_blocks("forward_jump", |p| {
            p.constant("a", 10)
                .jump("end")
                .constant("unreachable", 99)
                .label_decl("end")
                .return_void();
        });
    }

    #[test]
    fn conditional_branch() {
        dump_blocks("conditional_branch", |p| {
            p.constant("condition", 1)
                .branch_named(TACJumpOpcode::JNZ, "condition", "then")
                .constant("x", 10)
                .jump("end")
                .label_decl("then")
                .constant("x", 20)
                .label_decl("end")
                .return_void();
        });
    }

    #[test]
    fn consecutive_labels() {
        dump_blocks("consecutive_labels", |p| {
            p.jump("second")
                .label_decl("first")
                .label_decl("second")
                .label_decl("third")
                .constant("x", 42)
                .return_void();
        });
    }

    #[test]
    fn implicit_fallthrough() {
        dump_blocks("implicit_fallthrough", |p| {
            p.constant("a", 10)
                .label_decl("next")
                .constant("b", 20)
                .return_void();
        });
    }

    #[test]
    fn backward_jump_loop() {
        dump_blocks("backward_jump_loop", |p| {
            p.constant("counter", 5)
                .label_decl("loop")
                .constant("one", 1)
                .sub("counter", "one", "next")
                .jump("loop");
        });
    }

    #[test]
    fn conditional_loop() {
        dump_blocks("conditional_loop", |p| {
            p.constant("condition", 1)
                .label_decl("loop")
                .constant("x", 10)
                .branch_named(TACJumpOpcode::JNZ, "condition", "loop")
                .return_void();
        });
    }

    #[test]
    fn multiple_returns() {
        dump_blocks("multiple_returns", |p| {
            p.constant("condition", 1)
                .branch_named(TACJumpOpcode::JNZ, "condition", "early")
                .constant("x", 10)
                .return_void()
                .label_decl("early")
                .return_void();
        });
    }

    #[test]
    fn empty_procedure() {
        dump_blocks("empty_procedure", |_p| {});
    }

    #[test]
    fn label_at_entry() {
        dump_blocks("label_at_entry", |p| {
            p.label_decl("entry").constant("x", 10).return_void();
        });
    }

    #[test]
    fn arithmetic_with_arguments() {
        dump_blocks("arithmetic_with_arguments", |p| {
            p.add_argument("x");
            p.add_argument("y");
            p.copy("x", "a")
                .mul("a", "y", "b")
                .div("b", "y", "c")
                .return_value(TACTemp::Temp(4));
        });
    }
}

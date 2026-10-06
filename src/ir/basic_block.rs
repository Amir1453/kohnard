use std::collections::HashSet;

use crate::frontend::Symbol;
use crate::ir::tac::{InstBlock, Label, TACInst};

/// Represents a collection of BasicBlocks.
pub struct BasicBlocks {
    /// The collection of basic blocks.
    pub blocks: Vec<BasicBlock>,
}

impl BasicBlocks {
    /// Creates an empty BasicBlocks collection.
    const fn empty() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Given a sequence of instructions, converts them into basic block form.
    /// A basic block must start with (possibly multiple) labels.
    /// A basic block ends with a control flow divergence (jumps and return).
    ///
    /// To infer a collection of basic blocks from a sequence of instructions:
    /// 1. Start a BasicBlock.
    /// 2. Insert a label if there is not one.
    /// 3. Accumulate instructions until a jump or return.
    /// 4. If there is a fall-through instruction, add an unconditional jump.
    pub fn from_instrs(instructions: InstBlock, entry_name: &str) -> Self {
        use TACInst::*;

        let mut used_labels: HashSet<Label> = instructions
            .iter()
            .filter_map(|instruction| match instruction {
                LabelDecl(label) | UnconditionalJump(label) => Some(label.clone()),
                ConditionalJump { destination, .. } => Some(destination.clone()),
                _ => None,
            })
            .collect();

        let mut counter = 0u64;
        let mut fresh_label = || -> Label {
            loop {
                let label = Label::Named(Symbol::intern(&format!("{entry_name}.B{counter}")));
                counter += 1;
                if used_labels.insert(label.clone()) {
                    return label;
                }
            }
        };

        let mut basic_blocks: BasicBlocks = BasicBlocks::empty();
        let mut inst_iter = instructions.into_iter().peekable();
        let mut terminated = true;

        while let Some(instruction) = inst_iter.next() {
            if let LabelDecl(label) = instruction {
                if !terminated {
                    basic_blocks.push_instruction_last_block(UnconditionalJump(label.clone()));
                }

                basic_blocks.push_block(label);
                terminated = false;
                continue;
            }

            if terminated {
                basic_blocks.push_block(fresh_label());
                terminated = false;
            }

            match instruction {
                UnconditionalJump(_) | Return(_) => {
                    basic_blocks.push_instruction_last_block(instruction);
                    terminated = true;
                }

                ConditionalJump { .. } => {
                    basic_blocks.push_instruction_last_block(instruction);

                    match inst_iter.peek() {
                        Some(UnconditionalJump(_)) => {}

                        Some(LabelDecl(label)) => {
                            basic_blocks
                                .push_instruction_last_block(UnconditionalJump(label.clone()));
                            terminated = true;
                        }

                        Some(_) => {
                            let false_label = fresh_label();
                            basic_blocks.push_instruction_last_block(UnconditionalJump(
                                false_label.clone(),
                            ));
                            basic_blocks.push_block(false_label);
                        }

                        None => panic!("unterminated basic block in procedure {entry_name}"),
                    }
                }

                _ => {
                    basic_blocks.push_instruction_last_block(instruction);
                }
            }
        }

        assert!(
            terminated,
            "unterminated basic block in procedure {entry_name}"
        );

        basic_blocks
    }

    /// Pushes a new block with a label.
    fn push_block(&mut self, block_label: Label) {
        self.blocks.push(BasicBlock::new(block_label));
    }

    /// Pushes an instruction to the last block.
    fn push_instruction_last_block(&mut self, inst: TACInst) {
        self.blocks.last_mut().unwrap().push_instruction(inst);
    }
}

/// Represents a sequence of instructions that start with a label and end with
/// an unconditional jump or a return.
pub struct BasicBlock {
    /// The start label of the block.
    pub block_label: Label,
    /// Instructions that are contained in the block.
    pub instructions: InstBlock,
}

impl BasicBlock {
    /// Constructs an empty block with specified label.
    pub fn new(block_label: Label) -> Self {
        Self {
            block_label,
            instructions: Vec::new(),
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
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.block_label)?;
        for inst in &self.instructions {
            writeln!(f, "{}", inst)?;
        }
        write!(f, "")
    }
}

impl std::fmt::Debug for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

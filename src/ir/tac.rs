use std::fmt;
use std::ops::{Deref, DerefMut};

use thin_vec::ThinVec;

use crate::frontend::Symbol;
use crate::frontend::ast::Operator;

#[derive(Clone, Hash, PartialEq)]
pub struct CUTAC(pub Vec<TACDeclaration>);

#[derive(Clone, Hash, PartialEq)]
pub enum TACDeclaration {
    GlobalVarDecl(GlobalVarDecl),
    ProcDecl(ProcDecl),
}

#[derive(Clone, Hash, PartialEq)]
pub struct GlobalVarDecl {
    pub name: Symbol,
    pub value: i64,
}

#[derive(Clone, Hash, PartialEq)]
pub struct ProcDecl {
    pub name: Symbol,
    pub arguments: Option<Vec<Symbol>>,
    pub instructions: InstBlock,
}

pub type InstBlock = ThinVec<TACInst>;

#[derive(Copy, Clone, Hash, PartialEq, Default)]
pub enum TACInst {
    Const {
        destination: TACTemp,
        constant: i64,
    },

    Copi {
        destination: TACTemp,
        source: TACTemp,
    },

    LabelDecl(Label),

    UnconditionalJump(Label),

    ConditionalJump {
        opcode: TACJumpOpcode,
        condition: TACTemp,
        destination: Label,
    },

    UnaryOperation {
        opcode: TACUnaryOpcode,
        operand: TACTemp,
        result: TACTemp,
    },

    BinaryOperation {
        opcode: TACBinaryOpcode,
        lhs: TACTemp,
        rhs: TACTemp,
        result: TACTemp,
    },

    Parameter {
        nth_param: usize,
        source_temp: TACTemp,
    },

    ProcCall {
        proc_name: Symbol,
        arg_count: usize,
        result: TACTemp,
    },

    Return(Option<TACTemp>),

    #[default]
    Nop,
}

/// TODO Remove this Temp enum type, since it is not necessary,
/// and i32 can be represented as a Symbol as well. Having this
/// as an enum bloats the size to 8 bytes.
///
/// One can make a HashMap of Symbol -> TempType, but not sure
/// how the performance compares.
#[derive(Copy, Clone, Hash, PartialEq)]
pub enum TACTemp {
    Temp(u32),
    NamedTemp(Symbol),
    GlobalVar(Symbol),
}

#[derive(Copy, Clone, PartialEq, Hash, Eq)]
pub struct Label(pub u32);

#[derive(Copy, Clone, Hash, PartialEq)]
pub enum TACJumpOpcode {
    JZ,
    JNZ,
    JL,
    JNL,
    JLE,
    JNLE,
}

#[derive(Copy, Clone, Hash, PartialEq)]
pub enum TACUnaryOpcode {
    NEG,
    NOT,
}

#[derive(Copy, Clone, Hash, PartialEq)]
pub enum TACBinaryOpcode {
    ADD,
    SUB,
    MUL,
    DIV,
    MOD,

    AND,
    OR,
    XOR,
    SHL,
    SHR,
}

impl CUTAC {
    pub fn new() -> Self {
        Self(Vec::new())
    }
}

impl Deref for CUTAC {
    type Target = Vec<TACDeclaration>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CUTAC {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl fmt::Display for CUTAC {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for decl in &self.0 {
            write!(f, "{}", decl)?;
        }
        write!(f, "")
    }
}

impl fmt::Display for TACDeclaration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TACDeclaration::GlobalVarDecl(decl) => write!(f, "{}", decl),
            TACDeclaration::ProcDecl(decl) => write!(f, "{}", decl),
        }
    }
}

impl GlobalVarDecl {
    pub fn new(name: Symbol, value: i64) -> Self {
        Self { name, value }
    }
}

impl fmt::Display for GlobalVarDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "var @{} = {};", self.name, self.value)
    }
}

impl ProcDecl {
    pub fn new(name: Symbol, arguments: Option<Vec<Symbol>>, instructions: InstBlock) -> Self {
        Self {
            name,
            arguments,
            instructions,
        }
    }
}

impl fmt::Display for ProcDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.arguments {
            Some(args) if !args.is_empty() => {
                let names: Vec<String> = args.iter().map(|nt| format!("%{}", nt)).collect();
                writeln!(f, "define @{}({}):", self.name, names.join(", "))?;
            }
            _ => writeln!(f, "define @{}():", self.name)?,
        }

        for inst in &self.instructions {
            if matches!(inst, TACInst::LabelDecl(_)) {
                writeln!(f, "  {}", inst)?;
            } else {
                writeln!(f, "    {}", inst)?;
            }
        }

        write!(f, "")
    }
}

impl TACInst {
    pub fn modifies_temp(&self, temp: &TACTemp) -> bool {
        use TACInst::*;

        match self {
            Const { destination, .. }
            | Copi { destination, .. }
            | ProcCall {
                result: destination,
                ..
            } => destination == temp,

            UnaryOperation { result, .. } | BinaryOperation { result, .. } => result == temp,

            _ => false,
        }
    }

    /// Should this return Option<Terminator> instead ?
    /// But it cannot, since it Terminator is via BlockIdx.
    /// How about Option<TACInst> then ?
    pub fn is_terminator(&self) -> bool {
        use TACInst::*;

        match self {
            UnconditionalJump(_) | ConditionalJump { .. } | Return(_) => true,
            _ => false,
        }
    }
}

impl fmt::Debug for TACInst {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for TACInst {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TACInst::Const {
                destination,
                constant,
            } => write!(f, "{} = const {};", destination, constant),

            TACInst::Copi {
                destination,
                source,
            } => write!(f, "{} = copy {};", destination, source),

            TACInst::LabelDecl(label) => write!(f, "{}:", label),

            TACInst::UnconditionalJump(label) => write!(f, "jmp {};", label),

            TACInst::ConditionalJump {
                opcode,
                condition,
                destination,
            } => write!(f, "{} {}, {};", opcode, condition, destination),

            TACInst::UnaryOperation {
                opcode,
                operand,
                result,
            } => write!(f, "{} = {} {}", result, opcode, operand),

            TACInst::BinaryOperation {
                opcode,
                lhs,
                rhs,
                result,
            } => write!(f, "{} = {} {}, {}", result, opcode, lhs, rhs),

            TACInst::Parameter {
                nth_param,
                source_temp,
            } => write!(f, "param {}, {}", nth_param, source_temp),

            TACInst::ProcCall {
                proc_name,
                arg_count,
                result,
            } => write!(f, "{} = call @{}, {}", result, proc_name, arg_count),

            TACInst::Return(tactemp) => match tactemp {
                Some(temp) => write!(f, "ret {};", temp),
                None => write!(f, "ret;"),
            },

            TACInst::Nop => write!(f, "nop;"),
        }
    }
}

impl fmt::Display for TACTemp {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TACTemp::Temp(temp) => write!(f, "%{}", temp),
            TACTemp::NamedTemp(ntemp) => write!(f, "%{}", ntemp),
            TACTemp::GlobalVar(gtemp) => write!(f, "@{}", gtemp),
        }
    }
}

impl fmt::Display for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".L{}", self.0)
    }
}

impl From<Operator> for TACJumpOpcode {
    fn from(value: Operator) -> Self {
        use Operator::*;
        use TACJumpOpcode::*;
        match value {
            Equal => JZ,
            NEqual => JNZ,
            L => JL,
            LTE => JLE,
            G => JNLE,
            GTE => JNL,
            _ => unreachable!(),
        }
    }
}

impl From<Operator> for TACUnaryOpcode {
    fn from(value: Operator) -> Self {
        use Operator::*;
        use TACUnaryOpcode::*;
        match value {
            Neg => NEG,
            Tilde => NOT,
            _ => unreachable!(),
        }
    }
}

impl From<Operator> for TACBinaryOpcode {
    fn from(value: Operator) -> Self {
        use Operator::*;
        use TACBinaryOpcode::*;
        match value {
            Plus => ADD,
            Minus => SUB,
            Star => MUL,
            Slash => DIV,
            Mod => MOD,
            Ampersand => AND,
            Pipe => OR,
            Caret => XOR,
            LShift => SHL,
            RShift => SHR,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for TACJumpOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::JZ => "jz",
            Self::JNZ => "jnz",
            Self::JL => "jl",
            Self::JNL => "jnl",
            Self::JLE => "jle",
            Self::JNLE => "jnle",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for TACUnaryOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::NEG => "neg",
            Self::NOT => "not",
        };

        write!(f, "{}", s)
    }
}

impl fmt::Display for TACBinaryOpcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ADD => "add",
            Self::SUB => "sub",
            Self::MUL => "mul",
            Self::DIV => "div",
            Self::MOD => "mod",
            Self::AND => "and",
            Self::OR => "or",
            Self::XOR => "xor",
            Self::SHL => "shl",
            Self::SHR => "shr",
        };

        write!(f, "{}", s)
    }
}

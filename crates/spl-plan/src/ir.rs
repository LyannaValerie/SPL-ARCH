//! `P` — the Plan IR.
//!
//! The Plan IR is an operational representation and is deliberately unlike `K`.
//! Where `K` is a tree of expressions whose checked operations *raise*, a plan
//! is a set of basic blocks over typed registers where a checked operation
//! produces two results — the truncated value and an overflow flag — and the
//! plan must branch on the flag itself. Nothing in a plan propagates anything
//! implicitly.
//!
//! Core A has no loops, so the block graph must be acyclic; the validator
//! enforces that rather than the VM guessing at runtime.

use spl_core::outcome::SemanticErrorKind;
use spl_core::prims::{ArithOp, CompareOp};
use spl_core::value::{Type, Value};

/// A typed register. Registers are assigned exactly once per plan.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Reg(pub u32);

/// Index of a basic block. Block 0 is the entry block.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BlockId(pub u32);

/// Structural bounds. They exist so that decoding and executing an untrusted
/// plan has an explicit budget instead of an implicit one.
pub const MAX_REGISTERS: u32 = 1024;
/// Maximum number of basic blocks in one plan.
pub const MAX_BLOCKS: u32 = 256;
/// Maximum number of instructions in one basic block.
pub const MAX_INSTRUCTIONS_PER_BLOCK: usize = 1024;

/// A straight-line instruction.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Instruction {
    /// Bind a declared input to a register.
    Input { dst: Reg, index: u32 },
    /// Bind a literal to a register.
    Const { dst: Reg, value: Value },
    /// Compare two registers, producing a `Bool`.
    Compare {
        dst: Reg,
        op: CompareOp,
        lhs: Reg,
        rhs: Reg,
    },
    /// Wrapping arithmetic. One result, no flag: wrapping cannot overflow in
    /// any observable sense.
    Wrapping {
        dst: Reg,
        op: ArithOp,
        lhs: Reg,
        rhs: Reg,
    },
    /// Checked arithmetic, in its operational form: `(value, overflow) = op lhs, rhs`.
    ///
    /// The overflow flag is an ordinary `Bool` register. A plan that never
    /// branches on it is still structurally valid — and semantically wrong,
    /// which is precisely the kind of realization Stage 1 must be able to
    /// build and detect.
    Checked {
        value: Reg,
        overflow: Reg,
        op: ArithOp,
        lhs: Reg,
        rhs: Reg,
    },
}

/// How a basic block ends. Every block has exactly one terminator.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Terminator {
    Jump {
        target: BlockId,
    },
    Branch {
        cond: Reg,
        if_true: BlockId,
        if_false: BlockId,
    },
    /// Finish the execution with `Return(value)`.
    Return {
        value: Reg,
    },
    /// Finish the execution with `SemanticError(kind)`.
    Error {
        kind: SemanticErrorKind,
    },
}

/// A basic block: straight-line instructions and one terminator.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Block {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

/// An execution plan.
///
/// A `Plan` is a *candidate* representation. Constructing one asserts nothing:
/// it is not structurally checked until it goes through the validator, and it
/// is never semantically authorized at all in Stage 1, because Stage 1 has no
/// admission.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Plan {
    pub name: String,
    pub params: Vec<Type>,
    pub result: Type,
    /// Block 0 is the entry block.
    pub blocks: Vec<Block>,
}

impl Block {
    pub fn new(instructions: Vec<Instruction>, terminator: Terminator) -> Self {
        Block {
            instructions,
            terminator,
        }
    }
}

impl Instruction {
    /// The registers this instruction defines.
    pub fn defs(&self) -> Vec<Reg> {
        match self {
            Instruction::Input { dst, .. }
            | Instruction::Const { dst, .. }
            | Instruction::Compare { dst, .. }
            | Instruction::Wrapping { dst, .. } => vec![*dst],
            Instruction::Checked {
                value, overflow, ..
            } => vec![*value, *overflow],
        }
    }

    /// The registers this instruction reads.
    pub fn uses(&self) -> Vec<Reg> {
        match self {
            Instruction::Input { .. } | Instruction::Const { .. } => Vec::new(),
            Instruction::Compare { lhs, rhs, .. }
            | Instruction::Wrapping { lhs, rhs, .. }
            | Instruction::Checked { lhs, rhs, .. } => vec![*lhs, *rhs],
        }
    }
}

impl Terminator {
    /// The blocks this terminator can transfer control to.
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Terminator::Jump { target } => vec![*target],
            Terminator::Branch {
                if_true, if_false, ..
            } => vec![*if_true, *if_false],
            Terminator::Return { .. } | Terminator::Error { .. } => Vec::new(),
        }
    }

    /// The registers this terminator reads.
    pub fn uses(&self) -> Vec<Reg> {
        match self {
            Terminator::Branch { cond, .. } => vec![*cond],
            Terminator::Return { value } => vec![*value],
            Terminator::Jump { .. } | Terminator::Error { .. } => Vec::new(),
        }
    }
}

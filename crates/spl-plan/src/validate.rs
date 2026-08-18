//! Structural validation of a plan.
//!
//! Validation is about *form*, never about meaning. A plan that passes every
//! check here can still compute something the semantic contract does not
//! authorize — Stage 1 depends on that being possible, because the whole point
//! is to exhibit a well-formed, well-typed, executable realization that is
//! semantically wrong.
//!
//! What validation does guarantee is that the VM never has to improvise: every
//! register read is defined on every path that reaches it, every type lines up,
//! every block target exists, and the block graph is acyclic so execution
//! terminates.

use std::collections::{BTreeMap, BTreeSet};

use spl_core::prims::CompareOp;
use spl_core::value::Type;

use crate::ir::{
    Block, BlockId, Instruction, Plan, Reg, Terminator, MAX_BLOCKS, MAX_INSTRUCTIONS_PER_BLOCK,
    MAX_REGISTERS,
};

/// Why a plan is not well formed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PlanError {
    /// A plan must have an entry block.
    NoBlocks,
    /// A structural bound was exceeded.
    TooLarge { what: &'static str, limit: usize },
    /// A register index is outside the addressable range.
    RegisterOutOfRange { reg: Reg, limit: u32 },
    /// A register is assigned more than once.
    RegisterRedefined { reg: Reg },
    /// A register is read but never defined anywhere in the plan.
    RegisterUndefined { reg: Reg },
    /// A register is read where it is not defined on every path.
    RegisterNotAvailable { reg: Reg, block: BlockId },
    /// A branch or jump names a block that does not exist.
    InvalidBlockTarget { target: BlockId, blocks: usize },
    /// A block cannot be reached from the entry block.
    UnreachableBlock { block: BlockId },
    /// The block graph contains a cycle. Core A has no loops.
    CyclicControlFlow { block: BlockId },
    /// An input index has no corresponding declared parameter.
    UnknownInput { index: u32, params: usize },
    /// Two operands of one instruction disagree.
    OperandTypeMismatch { lhs: Type, rhs: Type },
    /// An operation is not defined for the operand type.
    UndefinedForType { op_name: &'static str, ty: Type },
    /// A register is used where a different type is required.
    RegisterTypeMismatch {
        reg: Reg,
        expected: Type,
        found: Type,
    },
    /// A returned register does not have the declared result type.
    ResultTypeMismatch { declared: Type, found: Type },
    /// A destination register would receive a type that contradicts the one it
    /// is required to have.
    DestinationTypeMismatch {
        reg: Reg,
        expected: Type,
        found: Type,
    },
}

impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PlanError::NoBlocks => write!(f, "a plan must have at least an entry block"),
            PlanError::TooLarge { what, limit } => write!(f, "too many {what}; limit is {limit}"),
            PlanError::RegisterOutOfRange { reg, limit } => {
                write!(f, "register r{} is out of range; limit is {limit}", reg.0)
            }
            PlanError::RegisterRedefined { reg } => {
                write!(f, "register r{} is assigned more than once", reg.0)
            }
            PlanError::RegisterUndefined { reg } => {
                write!(f, "register r{} is read but never defined", reg.0)
            }
            PlanError::RegisterNotAvailable { reg, block } => write!(
                f,
                "register r{} is read in block {} but is not defined on every path to it",
                reg.0, block.0
            ),
            PlanError::InvalidBlockTarget { target, blocks } => write!(
                f,
                "block target {} does not exist; the plan has {blocks} block(s)",
                target.0
            ),
            PlanError::UnreachableBlock { block } => {
                write!(f, "block {} is unreachable from the entry block", block.0)
            }
            PlanError::CyclicControlFlow { block } => {
                write!(
                    f,
                    "control flow re-enters block {}; Core A has no loops",
                    block.0
                )
            }
            PlanError::UnknownInput { index, params } => write!(
                f,
                "input {index} out of range; the plan declares {params} parameter(s)"
            ),
            PlanError::OperandTypeMismatch { lhs, rhs } => {
                write!(f, "operand type mismatch: {lhs} and {rhs}")
            }
            PlanError::UndefinedForType { op_name, ty } => {
                write!(f, "operation `{op_name}` is not defined for {ty}")
            }
            PlanError::RegisterTypeMismatch {
                reg,
                expected,
                found,
            } => write!(f, "register r{} is {found}, expected {expected}", reg.0),
            PlanError::ResultTypeMismatch { declared, found } => {
                write!(f, "plan declares result {declared} but returns {found}")
            }
            PlanError::DestinationTypeMismatch {
                reg,
                expected,
                found,
            } => write!(
                f,
                "destination r{} would be {found}, but is required to be {expected}",
                reg.0
            ),
        }
    }
}

/// A plan that passed structural validation, together with the register types
/// the validator derived.
///
/// Only [`validate`] constructs one. The Plan VM accepts nothing else, which is
/// how "the VM never runs an unvalidated plan" is enforced by the type system
/// rather than by convention.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValidatedPlan {
    plan: Plan,
    register_types: BTreeMap<u32, Type>,
    highest_register: u32,
}

impl ValidatedPlan {
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    pub fn register_types(&self) -> &BTreeMap<u32, Type> {
        &self.register_types
    }

    /// Number of register slots an execution needs.
    pub fn register_slots(&self) -> usize {
        self.highest_register as usize + 1
    }

    /// Every type the plan mentions, for checking against a configuration.
    pub fn required_types(&self) -> BTreeSet<Type> {
        let mut types: BTreeSet<Type> = self.register_types.values().copied().collect();
        types.extend(self.plan.params.iter().copied());
        types.insert(self.plan.result);
        types
    }

    /// Consume the wrapper and return the underlying plan.
    pub fn into_plan(self) -> Plan {
        self.plan
    }
}

/// Validate a plan.
pub fn validate(plan: Plan) -> Result<ValidatedPlan, PlanError> {
    if plan.blocks.is_empty() {
        return Err(PlanError::NoBlocks);
    }
    if plan.blocks.len() > MAX_BLOCKS as usize {
        return Err(PlanError::TooLarge {
            what: "blocks",
            limit: MAX_BLOCKS as usize,
        });
    }
    for block in &plan.blocks {
        if block.instructions.len() > MAX_INSTRUCTIONS_PER_BLOCK {
            return Err(PlanError::TooLarge {
                what: "instructions in a block",
                limit: MAX_INSTRUCTIONS_PER_BLOCK,
            });
        }
    }

    check_block_targets(&plan)?;
    let order = topological_order(&plan)?;
    let register_types = collect_definitions(&plan, &order)?;
    check_availability(&plan, &order)?;

    let highest_register = register_types.keys().copied().max().unwrap_or(0);
    Ok(ValidatedPlan {
        plan,
        register_types,
        highest_register,
    })
}

fn check_block_targets(plan: &Plan) -> Result<(), PlanError> {
    let count = plan.blocks.len();
    for block in &plan.blocks {
        for target in block.terminator.successors() {
            if target.0 as usize >= count {
                return Err(PlanError::InvalidBlockTarget {
                    target,
                    blocks: count,
                });
            }
        }
    }
    Ok(())
}

/// Reachable blocks in an order where every predecessor precedes its successors.
///
/// Rejects unreachable blocks and any cycle. Both are structural: an
/// unreachable block is dead weight that would still occupy artifact bytes and
/// perturb identity, and a cycle is a loop, which Core A does not have.
fn topological_order(plan: &Plan) -> Result<Vec<BlockId>, PlanError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unvisited,
        InProgress,
        Done,
    }

    let count = plan.blocks.len();
    let mut marks = vec![Mark::Unvisited; count];
    let mut postorder = Vec::with_capacity(count);
    // Explicit stack: a deeply nested plan must not consume host stack.
    let mut stack: Vec<(usize, usize)> = vec![(0, 0)];
    marks[0] = Mark::InProgress;

    while let Some((block_index, next_successor)) = stack.pop() {
        let successors = plan.blocks[block_index].terminator.successors();
        if next_successor < successors.len() {
            stack.push((block_index, next_successor + 1));
            let successor = successors[next_successor].0 as usize;
            match marks[successor] {
                Mark::Unvisited => {
                    marks[successor] = Mark::InProgress;
                    stack.push((successor, 0));
                }
                Mark::InProgress => {
                    return Err(PlanError::CyclicControlFlow {
                        block: BlockId(successor as u32),
                    })
                }
                Mark::Done => {}
            }
        } else {
            marks[block_index] = Mark::Done;
            postorder.push(BlockId(block_index as u32));
        }
    }

    for (index, mark) in marks.iter().enumerate() {
        if *mark != Mark::Done {
            return Err(PlanError::UnreachableBlock {
                block: BlockId(index as u32),
            });
        }
    }

    postorder.reverse();
    Ok(postorder)
}

/// Collect every register definition and its type, rejecting redefinitions and
/// type errors in the defining instruction.
fn collect_definitions(plan: &Plan, order: &[BlockId]) -> Result<BTreeMap<u32, Type>, PlanError> {
    let mut types: BTreeMap<u32, Type> = BTreeMap::new();

    // First pass: assign a type to every defined register. Blocks are walked in
    // topological order, so an instruction whose result type is inherited from
    // an operand always sees that operand's type already recorded, whatever
    // order the blocks happen to be stored in. Operand *agreement* is checked in
    // the second pass, once every definition is known.
    for block_id in order {
        let block = &plan.blocks[block_id.0 as usize];
        for instruction in &block.instructions {
            for reg in instruction.defs() {
                if reg.0 >= MAX_REGISTERS {
                    return Err(PlanError::RegisterOutOfRange {
                        reg,
                        limit: MAX_REGISTERS,
                    });
                }
            }
            match instruction {
                Instruction::Input { dst, index } => {
                    let ty = plan.params.get(*index as usize).copied().ok_or(
                        PlanError::UnknownInput {
                            index: *index,
                            params: plan.params.len(),
                        },
                    )?;
                    define(&mut types, *dst, ty)?;
                }
                Instruction::Const { dst, value } => define(&mut types, *dst, value.ty())?,
                Instruction::Compare { dst, .. } => define(&mut types, *dst, Type::Bool)?,
                Instruction::Wrapping { dst, lhs, .. } => {
                    // The result type is the operand type; agreement between the
                    // two operands is checked in the second pass.
                    let ty = operand_type(&types, *lhs)?;
                    define(&mut types, *dst, ty)?;
                }
                Instruction::Checked {
                    value,
                    overflow,
                    lhs,
                    ..
                } => {
                    let ty = operand_type(&types, *lhs)?;
                    define(&mut types, *value, ty)?;
                    define(&mut types, *overflow, Type::Bool)?;
                }
            }
        }
    }

    // Second pass: operand and terminator typing.
    for block in &plan.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Input { .. } | Instruction::Const { .. } => {}
                Instruction::Compare { op, lhs, rhs, .. } => {
                    let ty = binary_operand_type(&types, *lhs, *rhs)?;
                    if op.needs_order() && !ty.is_ordered() {
                        return Err(PlanError::UndefinedForType {
                            op_name: CompareOp::name(*op),
                            ty,
                        });
                    }
                }
                Instruction::Wrapping { op, dst, lhs, rhs } => {
                    let ty = binary_operand_type(&types, *lhs, *rhs)?;
                    if !ty.is_arithmetic() {
                        return Err(PlanError::UndefinedForType {
                            op_name: op.name(),
                            ty,
                        });
                    }
                    expect_register_type(&types, *dst, ty)?;
                }
                Instruction::Checked {
                    op,
                    value,
                    overflow,
                    lhs,
                    rhs,
                } => {
                    let ty = binary_operand_type(&types, *lhs, *rhs)?;
                    if !ty.is_arithmetic() {
                        return Err(PlanError::UndefinedForType {
                            op_name: op.name(),
                            ty,
                        });
                    }
                    expect_register_type(&types, *value, ty)?;
                    expect_register_type(&types, *overflow, Type::Bool)?;
                }
            }
        }
        match &block.terminator {
            Terminator::Branch { cond, .. } => {
                let ty = operand_type(&types, *cond)?;
                if ty != Type::Bool {
                    return Err(PlanError::RegisterTypeMismatch {
                        reg: *cond,
                        expected: Type::Bool,
                        found: ty,
                    });
                }
            }
            Terminator::Return { value } => {
                let ty = operand_type(&types, *value)?;
                if ty != plan.result {
                    return Err(PlanError::ResultTypeMismatch {
                        declared: plan.result,
                        found: ty,
                    });
                }
            }
            Terminator::Jump { .. } | Terminator::Error { .. } => {}
        }
    }

    Ok(types)
}

fn define(types: &mut BTreeMap<u32, Type>, reg: Reg, ty: Type) -> Result<(), PlanError> {
    if types.insert(reg.0, ty).is_some() {
        return Err(PlanError::RegisterRedefined { reg });
    }
    Ok(())
}

fn operand_type(types: &BTreeMap<u32, Type>, reg: Reg) -> Result<Type, PlanError> {
    types
        .get(&reg.0)
        .copied()
        .ok_or(PlanError::RegisterUndefined { reg })
}

fn binary_operand_type(types: &BTreeMap<u32, Type>, lhs: Reg, rhs: Reg) -> Result<Type, PlanError> {
    let lhs_ty = operand_type(types, lhs)?;
    let rhs_ty = operand_type(types, rhs)?;
    if lhs_ty == rhs_ty {
        Ok(lhs_ty)
    } else {
        Err(PlanError::OperandTypeMismatch {
            lhs: lhs_ty,
            rhs: rhs_ty,
        })
    }
}

fn expect_register_type(
    types: &BTreeMap<u32, Type>,
    reg: Reg,
    expected: Type,
) -> Result<(), PlanError> {
    let found = operand_type(types, reg)?;
    if found == expected {
        Ok(())
    } else {
        Err(PlanError::DestinationTypeMismatch {
            reg,
            expected,
            found,
        })
    }
}

/// Every read must be of a register defined on *every* path reaching it.
///
/// Registers are single-assignment and the graph is acyclic, so the set
/// available on entry to a block is the intersection of the sets available on
/// exit from its predecessors, and one pass in topological order suffices.
fn check_availability(plan: &Plan, order: &[BlockId]) -> Result<(), PlanError> {
    let mut predecessors: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (index, block) in plan.blocks.iter().enumerate() {
        for successor in block.terminator.successors() {
            predecessors
                .entry(successor.0)
                .or_default()
                .push(index as u32);
        }
    }

    let mut on_exit: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();

    for block_id in order {
        let block: &Block = &plan.blocks[block_id.0 as usize];
        let mut available: BTreeSet<u32> = match predecessors.get(&block_id.0) {
            None => BTreeSet::new(),
            Some(preds) => {
                let mut iter = preds.iter().filter_map(|p| on_exit.get(p));
                match iter.next() {
                    None => BTreeSet::new(),
                    Some(first) => iter.fold(first.clone(), |acc, next| {
                        acc.intersection(next).copied().collect()
                    }),
                }
            }
        };

        for instruction in &block.instructions {
            for reg in instruction.uses() {
                if !available.contains(&reg.0) {
                    return Err(PlanError::RegisterNotAvailable {
                        reg,
                        block: *block_id,
                    });
                }
            }
            for reg in instruction.defs() {
                available.insert(reg.0);
            }
        }
        for reg in block.terminator.uses() {
            if !available.contains(&reg.0) {
                return Err(PlanError::RegisterNotAvailable {
                    reg,
                    block: *block_id,
                });
            }
        }

        on_exit.insert(block_id.0, available);
    }

    Ok(())
}

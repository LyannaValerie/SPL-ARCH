//! The Stage-1 realizations.
//!
//! Three plans for the one contract in `spl_core::demo`, written by hand:
//!
//! * [`p_reference`] performs the checked operations operationally — compute,
//!   inspect the overflow flag, branch;
//! * [`p_specialized`] skips the arithmetic entirely and decides from `x > 0`;
//! * [`p_bad`] returns the input.
//!
//! All three are well formed, well typed and executable. Two of them agree with
//! the contract on every `u8` input and one does not, and nothing structural
//! distinguishes the wrong one from the right ones — which is the point.
//! Stage 1 has no admission, so nothing here is authorized; the tests observe
//! the disagreement, they do not act on it.

use spl_core::outcome::SemanticErrorKind;
use spl_core::prims::{ArithOp, CompareOp};
use spl_core::value::{Type, Value};

use crate::ir::{Block, BlockId, Instruction, Plan, Reg, Terminator};

/// The checked arithmetic written out operationally.
///
/// ```text
/// entry:      r0 = input 0
///             r1 = const 255
///             (r2, r3) = checked_add r0, r1
///             branch r3 -> overflow, continue
/// continue:   (r4, r5) = checked_sub r2, r1
///             branch r5 -> overflow, done
/// overflow:   error Overflow
/// done:       return r4
/// ```
pub fn p_reference() -> Plan {
    Plan {
        name: "p_reference_checked".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks: vec![
            Block::new(
                vec![
                    Instruction::Input {
                        dst: Reg(0),
                        index: 0,
                    },
                    Instruction::Const {
                        dst: Reg(1),
                        value: Value::U8(255),
                    },
                    Instruction::Checked {
                        value: Reg(2),
                        overflow: Reg(3),
                        op: ArithOp::Add,
                        lhs: Reg(0),
                        rhs: Reg(1),
                    },
                ],
                Terminator::Branch {
                    cond: Reg(3),
                    if_true: BlockId(2),
                    if_false: BlockId(1),
                },
            ),
            Block::new(
                vec![Instruction::Checked {
                    value: Reg(4),
                    overflow: Reg(5),
                    op: ArithOp::Sub,
                    lhs: Reg(2),
                    rhs: Reg(1),
                }],
                Terminator::Branch {
                    cond: Reg(5),
                    if_true: BlockId(2),
                    if_false: BlockId(3),
                },
            ),
            Block::new(
                Vec::new(),
                Terminator::Error {
                    kind: SemanticErrorKind::Overflow,
                },
            ),
            Block::new(Vec::new(), Terminator::Return { value: Reg(4) }),
        ],
    }
}

/// The specialization: decide from `x > 0` and never do the arithmetic.
///
/// Structurally this shares nothing with [`p_reference`] beyond having blocks:
/// no checked instruction, no overflow flag, a comparison the reference does
/// not have, and a different number of blocks and registers.
pub fn p_specialized() -> Plan {
    Plan {
        name: "p_specialized_compare".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks: vec![
            Block::new(
                vec![
                    Instruction::Input {
                        dst: Reg(0),
                        index: 0,
                    },
                    Instruction::Const {
                        dst: Reg(1),
                        value: Value::U8(0),
                    },
                    Instruction::Compare {
                        dst: Reg(2),
                        op: CompareOp::Gt,
                        lhs: Reg(0),
                        rhs: Reg(1),
                    },
                ],
                Terminator::Branch {
                    cond: Reg(2),
                    if_true: BlockId(1),
                    if_false: BlockId(2),
                },
            ),
            Block::new(
                Vec::new(),
                Terminator::Error {
                    kind: SemanticErrorKind::Overflow,
                },
            ),
            Block::new(
                vec![Instruction::Const {
                    dst: Reg(3),
                    value: Value::U8(0),
                }],
                Terminator::Return { value: Reg(3) },
            ),
        ],
    }
}

/// A well-typed, executable, semantically wrong realization: `Return(x)`.
///
/// It agrees with the contract at `x == 0` and disagrees everywhere else. The
/// validator has no reason to reject it, and does not.
pub fn p_bad() -> Plan {
    Plan {
        name: "p_bad_identity".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks: vec![Block::new(
            vec![Instruction::Input {
                dst: Reg(0),
                index: 0,
            }],
            Terminator::Return { value: Reg(0) },
        )],
    }
}

/// A mutation of the realization: both checked operations become wrapping and
/// no overflow is ever inspected.
///
/// This is the plan-level form of "model checked arithmetic as plain wrapping
/// arithmetic". It disagrees with the contract on every input above zero.
pub fn mutated_wrapping_realization() -> Plan {
    Plan {
        name: "mutant_wrapping_realization".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks: vec![Block::new(
            vec![
                Instruction::Input {
                    dst: Reg(0),
                    index: 0,
                },
                Instruction::Const {
                    dst: Reg(1),
                    value: Value::U8(255),
                },
                Instruction::Wrapping {
                    dst: Reg(2),
                    op: ArithOp::Add,
                    lhs: Reg(0),
                    rhs: Reg(1),
                },
                Instruction::Wrapping {
                    dst: Reg(3),
                    op: ArithOp::Sub,
                    lhs: Reg(2),
                    rhs: Reg(1),
                },
            ],
            Terminator::Return { value: Reg(3) },
        )],
    }
}

/// [`p_reference`] with the *first* overflow check removed.
///
/// This one is an equivalent mutant, and it is kept deliberately. Removing the
/// check on `x + 255` changes the plan's structure but not its meaning: for
/// every `x` above zero the wrapped sum is `x - 1`, and subtracting 255 from a
/// value below 255 underflows, so the second check raises exactly where the
/// first one would have. It is here so the suite is honest about what a
/// disagreement test does and does not show — a mutation that survives is not
/// automatically a gap in the tests.
pub fn equivalent_dropped_first_overflow_check() -> Plan {
    let mut plan = p_reference();
    plan.name = "equivalent_dropped_first_check".to_owned();
    plan.blocks[0].instructions[2] = Instruction::Wrapping {
        dst: Reg(2),
        op: ArithOp::Add,
        lhs: Reg(0),
        rhs: Reg(1),
    };
    // With no overflow flag from the add, the entry block simply falls through.
    plan.blocks[0].terminator = Terminator::Jump { target: BlockId(1) };
    plan
}

/// A mutation of [`p_specialized`]: the guard comparison becomes `>=`.
pub fn mutated_wrong_comparison() -> Plan {
    let mut plan = p_specialized();
    plan.name = "mutant_p_specialized_ge".to_owned();
    plan.blocks[0].instructions[2] = Instruction::Compare {
        dst: Reg(2),
        op: CompareOp::Ge,
        lhs: Reg(0),
        rhs: Reg(1),
    };
    plan
}

/// A mutation of [`p_specialized`]: the error arm returns a value instead of
/// raising, so the outcome *tag* is wrong while a payload still appears.
pub fn mutated_wrong_outcome_tag() -> Plan {
    let mut plan = p_specialized();
    plan.name = "mutant_p_specialized_returns_zero".to_owned();
    plan.blocks[1] = Block::new(
        vec![Instruction::Const {
            dst: Reg(4),
            value: Value::U8(0),
        }],
        Terminator::Return { value: Reg(4) },
    );
    plan
}

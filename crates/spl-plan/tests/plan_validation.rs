//! Structural validation of plans.
//!
//! Every rejection here is about form. None of them is about a plan computing
//! the wrong thing — the validator has no opinion on that, by design.

use spl_core::outcome::SemanticErrorKind;
use spl_core::prims::{ArithOp, CompareOp};
use spl_core::value::{Type, Value};
use spl_plan::demo;
use spl_plan::ir::{Block, BlockId, Instruction, Plan, Reg, Terminator, MAX_REGISTERS};
use spl_plan::validate::{validate, PlanError};

fn u8_plan(blocks: Vec<Block>) -> Plan {
    Plan {
        name: "fixture".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks,
    }
}

fn input0() -> Instruction {
    Instruction::Input {
        dst: Reg(0),
        index: 0,
    }
}

#[test]
fn the_three_demo_plans_all_validate() {
    for plan in [demo::p_reference(), demo::p_specialized(), demo::p_bad()] {
        let name = plan.name.clone();
        validate(plan).unwrap_or_else(|err| panic!("{name} must validate, got {err}"));
    }
}

#[test]
fn p_bad_is_structurally_valid() {
    // Stated on its own because the whole Stage-1 argument depends on it: the
    // semantically wrong realization is not detectable structurally.
    validate(demo::p_bad()).expect("p_bad is well formed and well typed");
}

#[test]
fn a_plan_needs_an_entry_block() {
    assert_eq!(validate(u8_plan(Vec::new())), Err(PlanError::NoBlocks));
}

#[test]
fn an_undefined_register_is_rejected() {
    let plan = u8_plan(vec![Block::new(
        vec![input0()],
        Terminator::Return { value: Reg(9) },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::RegisterUndefined { reg: Reg(9) })
    );
}

#[test]
fn a_register_defined_on_only_one_path_is_rejected() {
    let plan = u8_plan(vec![
        Block::new(
            vec![
                input0(),
                Instruction::Const {
                    dst: Reg(3),
                    value: Value::U8(0),
                },
                Instruction::Compare {
                    dst: Reg(2),
                    op: CompareOp::Gt,
                    lhs: Reg(0),
                    rhs: Reg(3),
                },
            ],
            Terminator::Branch {
                cond: Reg(2),
                if_true: BlockId(1),
                if_false: BlockId(2),
            },
        ),
        Block::new(
            vec![Instruction::Const {
                dst: Reg(1),
                value: Value::U8(5),
            }],
            Terminator::Jump { target: BlockId(3) },
        ),
        Block::new(Vec::new(), Terminator::Jump { target: BlockId(3) }),
        Block::new(Vec::new(), Terminator::Return { value: Reg(1) }),
    ]);
    assert_eq!(
        validate(plan),
        Err(PlanError::RegisterNotAvailable {
            reg: Reg(1),
            block: BlockId(3)
        })
    );
}

#[test]
fn redefining_a_register_is_rejected() {
    let plan = u8_plan(vec![Block::new(
        vec![
            input0(),
            Instruction::Const {
                dst: Reg(0),
                value: Value::U8(1),
            },
        ],
        Terminator::Return { value: Reg(0) },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::RegisterRedefined { reg: Reg(0) })
    );
}

#[test]
fn mismatched_operand_types_are_rejected() {
    let mut plan = u8_plan(vec![Block::new(
        vec![
            input0(),
            Instruction::Const {
                dst: Reg(1),
                value: Value::U32(1),
            },
            Instruction::Wrapping {
                dst: Reg(2),
                op: ArithOp::Add,
                lhs: Reg(0),
                rhs: Reg(1),
            },
        ],
        Terminator::Return { value: Reg(2) },
    )]);
    plan.params = vec![Type::U8];
    assert_eq!(
        validate(plan),
        Err(PlanError::OperandTypeMismatch {
            lhs: Type::U8,
            rhs: Type::U32
        })
    );
}

#[test]
fn branching_on_a_non_bool_register_is_rejected() {
    let plan = u8_plan(vec![
        Block::new(
            vec![input0()],
            Terminator::Branch {
                cond: Reg(0),
                if_true: BlockId(1),
                if_false: BlockId(1),
            },
        ),
        Block::new(Vec::new(), Terminator::Return { value: Reg(0) }),
    ]);
    assert_eq!(
        validate(plan),
        Err(PlanError::RegisterTypeMismatch {
            reg: Reg(0),
            expected: Type::Bool,
            found: Type::U8
        })
    );
}

#[test]
fn returning_the_wrong_type_is_rejected() {
    let plan = Plan {
        name: "fixture".to_owned(),
        params: vec![Type::U8],
        result: Type::U32,
        blocks: vec![Block::new(
            vec![input0()],
            Terminator::Return { value: Reg(0) },
        )],
    };
    assert_eq!(
        validate(plan),
        Err(PlanError::ResultTypeMismatch {
            declared: Type::U32,
            found: Type::U8
        })
    );
}

#[test]
fn an_out_of_range_block_target_is_rejected() {
    let plan = u8_plan(vec![Block::new(
        vec![input0()],
        Terminator::Jump { target: BlockId(7) },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::InvalidBlockTarget {
            target: BlockId(7),
            blocks: 1
        })
    );
}

#[test]
fn cyclic_control_flow_is_rejected() {
    let plan = u8_plan(vec![
        Block::new(vec![input0()], Terminator::Jump { target: BlockId(1) }),
        Block::new(Vec::new(), Terminator::Jump { target: BlockId(1) }),
    ]);
    assert_eq!(
        validate(plan),
        Err(PlanError::CyclicControlFlow { block: BlockId(1) })
    );
}

#[test]
fn a_longer_cycle_is_rejected_too() {
    let plan = u8_plan(vec![
        Block::new(vec![input0()], Terminator::Jump { target: BlockId(1) }),
        Block::new(Vec::new(), Terminator::Jump { target: BlockId(2) }),
        Block::new(Vec::new(), Terminator::Jump { target: BlockId(1) }),
    ]);
    assert_eq!(
        validate(plan),
        Err(PlanError::CyclicControlFlow { block: BlockId(1) })
    );
}

#[test]
fn an_unreachable_block_is_rejected() {
    let plan = u8_plan(vec![
        Block::new(vec![input0()], Terminator::Return { value: Reg(0) }),
        Block::new(
            Vec::new(),
            Terminator::Error {
                kind: SemanticErrorKind::Overflow,
            },
        ),
    ]);
    assert_eq!(
        validate(plan),
        Err(PlanError::UnreachableBlock { block: BlockId(1) })
    );
}

#[test]
fn arithmetic_on_bool_registers_is_rejected() {
    let plan = Plan {
        name: "fixture".to_owned(),
        params: vec![Type::Bool],
        result: Type::Bool,
        blocks: vec![Block::new(
            vec![
                input0(),
                Instruction::Const {
                    dst: Reg(1),
                    value: Value::Bool(true),
                },
                Instruction::Wrapping {
                    dst: Reg(2),
                    op: ArithOp::Add,
                    lhs: Reg(0),
                    rhs: Reg(1),
                },
            ],
            Terminator::Return { value: Reg(2) },
        )],
    };
    assert_eq!(
        validate(plan),
        Err(PlanError::UndefinedForType {
            op_name: "add",
            ty: Type::Bool
        })
    );
}

#[test]
fn ordering_bool_registers_is_rejected() {
    let plan = Plan {
        name: "fixture".to_owned(),
        params: vec![Type::Bool],
        result: Type::Bool,
        blocks: vec![Block::new(
            vec![
                input0(),
                Instruction::Const {
                    dst: Reg(1),
                    value: Value::Bool(true),
                },
                Instruction::Compare {
                    dst: Reg(2),
                    op: CompareOp::Lt,
                    lhs: Reg(0),
                    rhs: Reg(1),
                },
            ],
            Terminator::Return { value: Reg(2) },
        )],
    };
    assert_eq!(
        validate(plan),
        Err(PlanError::UndefinedForType {
            op_name: "lt",
            ty: Type::Bool
        })
    );
}

#[test]
fn an_unknown_input_index_is_rejected() {
    let plan = u8_plan(vec![Block::new(
        vec![Instruction::Input {
            dst: Reg(0),
            index: 4,
        }],
        Terminator::Return { value: Reg(0) },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::UnknownInput {
            index: 4,
            params: 1
        })
    );
}

#[test]
fn a_register_beyond_the_addressable_range_is_rejected() {
    let plan = u8_plan(vec![Block::new(
        vec![Instruction::Input {
            dst: Reg(MAX_REGISTERS),
            index: 0,
        }],
        Terminator::Return {
            value: Reg(MAX_REGISTERS),
        },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::RegisterOutOfRange {
            reg: Reg(MAX_REGISTERS),
            limit: MAX_REGISTERS
        })
    );
}

#[test]
fn the_overflow_flag_of_a_checked_instruction_is_a_bool() {
    // A plan is free to ignore the flag, but not to pretend it is a u8.
    let plan = u8_plan(vec![Block::new(
        vec![
            input0(),
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
        Terminator::Return { value: Reg(3) },
    )]);
    assert_eq!(
        validate(plan),
        Err(PlanError::ResultTypeMismatch {
            declared: Type::U8,
            found: Type::Bool
        })
    );
}

#[test]
fn blocks_stored_out_of_topological_order_still_validate() {
    // Block storage order is not program order; validation must not depend on it.
    let plan = u8_plan(vec![
        Block::new(vec![input0()], Terminator::Jump { target: BlockId(2) }),
        Block::new(Vec::new(), Terminator::Return { value: Reg(1) }),
        Block::new(
            vec![Instruction::Wrapping {
                dst: Reg(1),
                op: ArithOp::Add,
                lhs: Reg(0),
                rhs: Reg(0),
            }],
            Terminator::Jump { target: BlockId(1) },
        ),
    ]);
    validate(plan).expect("storage order must not matter");
}

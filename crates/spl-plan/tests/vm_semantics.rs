//! Plan VM behaviour, and the `S`/`Q` binding.

use spl_core::outcome::{ObservedOutcome, SemanticErrorKind, TrapKind};
use spl_core::prims::ArithOp;
use spl_core::value::{Type, Value};
use spl_plan::config::{ArithmeticMode, PlanVmConfiguration, SubstrateIdentity, PLAN_IR_VERSION};
use spl_plan::demo;
use spl_plan::ir::{Block, BlockId, Instruction, Plan, Reg, Terminator};
use spl_plan::validate::validate;
use spl_plan::vm::{PlanVm, VmError};

fn u8_plan(blocks: Vec<Block>) -> Plan {
    Plan {
        name: "fixture".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        blocks,
    }
}

#[test]
fn the_demo_runs_never_trap_get_stuck_or_diverge() {
    let vm = PlanVm::stage1();
    for plan in [demo::p_reference(), demo::p_specialized(), demo::p_bad()] {
        let name = plan.name.clone();
        let validated = validate(plan).unwrap();
        for x in 0u8..=255 {
            match vm.run(&validated, &[Value::U8(x)]).unwrap() {
                ObservedOutcome::Return(_) | ObservedOutcome::SemanticError(_) => {}
                other => panic!("{name} produced {other:?} at x = {x}"),
            }
        }
    }
}

#[test]
fn a_checked_instruction_writes_both_results() {
    // A plan that returns the truncated value and ignores the flag entirely.
    // This is legal Plan IR: the VM has no opinion about the flag.
    let plan = u8_plan(vec![Block::new(
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
        Terminator::Return { value: Reg(2) },
    )]);
    let validated = validate(plan).unwrap();
    let vm = PlanVm::stage1();
    // 1 + 255 wraps to 0 and sets the flag; the plan returns the wrapped value.
    assert_eq!(
        vm.run(&validated, &[Value::U8(1)]).unwrap(),
        ObservedOutcome::Return(Value::U8(0))
    );
}

#[test]
fn a_plan_can_branch_on_the_overflow_flag_in_either_direction() {
    // Same computation, branch arms swapped: the VM follows the plan, it does
    // not infer what an overflow "should" mean.
    let build = |if_true: u32, if_false: u32| {
        u8_plan(vec![
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
                    if_true: BlockId(if_true),
                    if_false: BlockId(if_false),
                },
            ),
            Block::new(
                Vec::new(),
                Terminator::Error {
                    kind: SemanticErrorKind::Overflow,
                },
            ),
            Block::new(Vec::new(), Terminator::Return { value: Reg(2) }),
        ])
    };
    let vm = PlanVm::stage1();
    let sane = validate(build(1, 2)).unwrap();
    let inverted = validate(build(2, 1)).unwrap();
    assert_eq!(
        vm.run(&sane, &[Value::U8(1)]).unwrap(),
        ObservedOutcome::SemanticError(SemanticErrorKind::Overflow)
    );
    assert_eq!(
        vm.run(&inverted, &[Value::U8(1)]).unwrap(),
        ObservedOutcome::Return(Value::U8(0))
    );
}

#[test]
fn a_configuration_without_the_needed_type_refuses_the_plan() {
    let validated = validate(demo::p_reference()).unwrap();
    let vm = PlanVm::new(
        SubstrateIdentity::stage1_prototype(),
        PlanVmConfiguration::new(
            [Type::Bool, Type::U32],
            [ArithmeticMode::Wrapping, ArithmeticMode::Checked],
            1000,
        ),
    );
    assert_eq!(
        vm.run(&validated, &[Value::U8(0)]),
        Err(VmError::UnsupportedType { ty: Type::U8 })
    );
}

#[test]
fn a_configuration_without_checked_arithmetic_refuses_the_reference_plan() {
    let validated = validate(demo::p_reference()).unwrap();
    let vm = PlanVm::new(
        SubstrateIdentity::stage1_prototype(),
        PlanVmConfiguration::new(
            [Type::Bool, Type::U8, Type::U32, Type::I32],
            [ArithmeticMode::Wrapping],
            1000,
        ),
    );
    assert_eq!(
        vm.run(&validated, &[Value::U8(0)]),
        Err(VmError::UnsupportedArithmeticMode {
            mode: ArithmeticMode::Checked
        })
    );
    // The specialization needs no arithmetic at all, so the same configuration
    // hosts it. `Q` restricts realizations, not the contract.
    let specialized = validate(demo::p_specialized()).unwrap();
    assert!(vm.run(&specialized, &[Value::U8(3)]).is_ok());
}

#[test]
fn a_configuration_for_another_plan_ir_revision_refuses_the_plan() {
    let validated = validate(demo::p_bad()).unwrap();
    let mut config = PlanVmConfiguration::stage1_full_core_a();
    config.plan_ir_version = PLAN_IR_VERSION + 1;
    let vm = PlanVm::new(SubstrateIdentity::stage1_prototype(), config);
    assert_eq!(
        vm.run(&validated, &[Value::U8(0)]),
        Err(VmError::PlanIrVersionMismatch {
            expected: PLAN_IR_VERSION,
            configured: PLAN_IR_VERSION + 1
        })
    );
}

#[test]
fn wrong_inputs_are_refused_rather_than_executed() {
    let validated = validate(demo::p_reference()).unwrap();
    let vm = PlanVm::stage1();
    assert_eq!(
        vm.run(&validated, &[]),
        Err(VmError::InputArityMismatch {
            expected: 1,
            found: 0
        })
    );
    assert_eq!(
        vm.run(&validated, &[Value::U32(0)]),
        Err(VmError::InputTypeMismatch {
            index: 0,
            expected: Type::U8,
            found: Type::U32
        })
    );
}

#[test]
fn the_step_limit_is_expressible_and_the_demo_never_reaches_it() {
    // A budget of one step cannot finish even the smallest plan, so `Trap` is
    // reachable and distinguishable from a semantic error.
    let validated = validate(demo::p_reference()).unwrap();
    let starved = PlanVm::new(
        SubstrateIdentity::stage1_prototype(),
        PlanVmConfiguration::new(
            [Type::Bool, Type::U8, Type::U32, Type::I32],
            [ArithmeticMode::Wrapping, ArithmeticMode::Checked],
            1,
        ),
    );
    assert_eq!(
        starved.run(&validated, &[Value::U8(0)]).unwrap(),
        ObservedOutcome::Trap(TrapKind::StepLimitExceeded)
    );

    // And a trap is not a semantic error: they are different observed outcomes,
    // as the frozen theory requires.
    assert_ne!(
        ObservedOutcome::Trap(TrapKind::StepLimitExceeded),
        ObservedOutcome::SemanticError(SemanticErrorKind::Overflow)
    );
}

#[test]
fn the_vm_records_the_substrate_it_runs_on() {
    let vm = PlanVm::stage1();
    assert_eq!(vm.substrate(), &SubstrateIdentity::stage1_prototype());
    assert_eq!(vm.config(), &PlanVmConfiguration::stage1_full_core_a());
}

#[test]
fn configuration_capability_lists_are_normalized() {
    // Two configurations describing the same capabilities must be the same
    // configuration, so that `Q` identity is not sensitive to argument order.
    let one = PlanVmConfiguration::new(
        [Type::U32, Type::Bool, Type::U32],
        [ArithmeticMode::Checked, ArithmeticMode::Wrapping],
        10,
    );
    let other = PlanVmConfiguration::new(
        [Type::Bool, Type::U32],
        [ArithmeticMode::Wrapping, ArithmeticMode::Checked],
        10,
    );
    assert_eq!(one, other);
}

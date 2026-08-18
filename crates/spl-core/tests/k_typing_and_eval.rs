//! Type checking and evaluation of `K`.

use spl_core::demo;
use spl_core::eval::{eval, KEvalError};
use spl_core::k::{InputIndex, KContract, KExpr, KTerm, KTypeError};
use spl_core::outcome::{SemanticErrorKind, SemanticOutcome};
use spl_core::prims::{ArithOp, CompareOp};
use spl_core::value::{Type, Value};

fn contract(params: Vec<Type>, result: Type, body: KTerm) -> KContract {
    KContract {
        name: "test".to_owned(),
        params,
        result,
        body,
    }
}

#[test]
fn the_demo_contract_is_well_typed() {
    demo::checked_roundtrip_contract()
        .type_check()
        .expect("the Stage-1 contract must type check");
}

#[test]
fn unknown_input_is_rejected() {
    let contract = contract(
        vec![Type::U8],
        Type::U8,
        KTerm::Return(KExpr::Input(InputIndex(3))),
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::UnknownInput {
            index: 3,
            params: 1
        })
    );
}

#[test]
fn mixed_operand_types_are_rejected() {
    let contract = contract(
        vec![Type::U8, Type::U32],
        Type::U8,
        KTerm::Return(KExpr::Wrapping {
            op: ArithOp::Add,
            lhs: Box::new(KExpr::Input(InputIndex(0))),
            rhs: Box::new(KExpr::Input(InputIndex(1))),
        }),
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::OperandTypeMismatch {
            lhs: Type::U8,
            rhs: Type::U32
        })
    );
}

#[test]
fn non_bool_condition_is_rejected() {
    let contract = contract(
        vec![Type::U8],
        Type::U8,
        KTerm::If {
            cond: KExpr::Input(InputIndex(0)),
            then_term: Box::new(KTerm::Return(KExpr::Const(Value::U8(0)))),
            else_term: Box::new(KTerm::Return(KExpr::Const(Value::U8(1)))),
        },
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::ConditionNotBool { found: Type::U8 })
    );
}

#[test]
fn disagreeing_branches_are_rejected() {
    let contract = contract(
        vec![Type::Bool],
        Type::U8,
        KTerm::Return(KExpr::If {
            cond: Box::new(KExpr::Input(InputIndex(0))),
            then_expr: Box::new(KExpr::Const(Value::U8(0))),
            else_expr: Box::new(KExpr::Const(Value::U32(0))),
        }),
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::BranchTypeMismatch {
            then_ty: Type::U8,
            else_ty: Type::U32
        })
    );
}

#[test]
fn wrong_result_type_is_rejected() {
    let contract = contract(vec![], Type::U8, KTerm::Return(KExpr::Const(Value::U32(0))));
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::ResultTypeMismatch {
            declared: Type::U8,
            found: Type::U32
        })
    );
}

#[test]
fn ordering_a_bool_is_rejected() {
    let contract = contract(
        vec![Type::Bool, Type::Bool],
        Type::Bool,
        KTerm::Return(KExpr::Compare {
            op: CompareOp::Lt,
            lhs: Box::new(KExpr::Input(InputIndex(0))),
            rhs: Box::new(KExpr::Input(InputIndex(1))),
        }),
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::UndefinedForType {
            op_name: "lt",
            ty: Type::Bool
        })
    );
}

#[test]
fn arithmetic_on_bool_is_rejected() {
    let contract = contract(
        vec![Type::Bool, Type::Bool],
        Type::Bool,
        KTerm::Return(KExpr::Checked {
            op: ArithOp::Add,
            lhs: Box::new(KExpr::Input(InputIndex(0))),
            rhs: Box::new(KExpr::Input(InputIndex(1))),
        }),
    );
    assert_eq!(
        contract.type_check(),
        Err(KTypeError::UndefinedForType {
            op_name: "add",
            ty: Type::Bool
        })
    );
}

#[test]
fn checked_overflow_propagates_out_of_a_nested_expression() {
    // The overflow happens in the inner operation and must surface as the whole
    // contract's outcome, not as a value the outer operation consumes.
    let contract = contract(
        vec![Type::U8],
        Type::U8,
        KTerm::Return(KExpr::Wrapping {
            op: ArithOp::Add,
            lhs: Box::new(KExpr::Checked {
                op: ArithOp::Add,
                lhs: Box::new(KExpr::Const(Value::U8(255))),
                rhs: Box::new(KExpr::Const(Value::U8(1))),
            }),
            rhs: Box::new(KExpr::Const(Value::U8(0))),
        }),
    );
    contract.type_check().unwrap();
    assert_eq!(
        eval(&contract, &[Value::U8(0)]),
        Ok(SemanticOutcome::SemanticError(SemanticErrorKind::Overflow))
    );
}

#[test]
fn wrapping_does_not_raise() {
    let contract = contract(
        vec![],
        Type::U8,
        KTerm::Return(KExpr::Wrapping {
            op: ArithOp::Add,
            lhs: Box::new(KExpr::Const(Value::U8(255))),
            rhs: Box::new(KExpr::Const(Value::U8(1))),
        }),
    );
    contract.type_check().unwrap();
    assert_eq!(
        eval(&contract, &[]),
        Ok(SemanticOutcome::Return(Value::U8(0)))
    );
}

#[test]
fn a_raise_short_circuits_the_other_operand() {
    // Left-to-right evaluation is a semantic choice and is pinned here.
    let contract = contract(
        vec![],
        Type::U8,
        KTerm::Return(KExpr::Wrapping {
            op: ArithOp::Add,
            lhs: Box::new(KExpr::Checked {
                op: ArithOp::Add,
                lhs: Box::new(KExpr::Const(Value::U8(255))),
                rhs: Box::new(KExpr::Const(Value::U8(1))),
            }),
            rhs: Box::new(KExpr::Checked {
                op: ArithOp::Mul,
                lhs: Box::new(KExpr::Const(Value::U8(16))),
                rhs: Box::new(KExpr::Const(Value::U8(16))),
            }),
        }),
    );
    contract.type_check().unwrap();
    assert_eq!(
        eval(&contract, &[]),
        Ok(SemanticOutcome::SemanticError(SemanticErrorKind::Overflow))
    );
}

#[test]
fn outcome_level_conditional_selects_an_outcome_not_a_value() {
    let contract = contract(
        vec![Type::U8],
        Type::U8,
        KTerm::If {
            cond: KExpr::Compare {
                op: CompareOp::Gt,
                lhs: Box::new(KExpr::Input(InputIndex(0))),
                rhs: Box::new(KExpr::Const(Value::U8(0))),
            },
            then_term: Box::new(KTerm::SemanticError(SemanticErrorKind::Overflow)),
            else_term: Box::new(KTerm::Return(KExpr::Const(Value::U8(0)))),
        },
    );
    contract.type_check().unwrap();
    assert_eq!(
        eval(&contract, &[Value::U8(7)]),
        Ok(SemanticOutcome::SemanticError(SemanticErrorKind::Overflow))
    );
    assert_eq!(
        eval(&contract, &[Value::U8(0)]),
        Ok(SemanticOutcome::Return(Value::U8(0)))
    );
}

#[test]
fn wrong_inputs_are_refused_rather_than_answered() {
    let contract = demo::checked_roundtrip_contract();
    assert_eq!(
        eval(&contract, &[]),
        Err(KEvalError::InputArityMismatch {
            expected: 1,
            found: 0
        })
    );
    assert_eq!(
        eval(&contract, &[Value::U32(0)]),
        Err(KEvalError::InputTypeMismatch {
            index: 0,
            expected: Type::U8,
            found: Type::U32
        })
    );
}

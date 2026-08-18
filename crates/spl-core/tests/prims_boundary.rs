//! Boundary conformance for the wide types.
//!
//! `u32` and `i32` cannot be enumerated, so the corpus is explicit: the extremes
//! of the range, their neighbours, zero, one, minus one for the signed type, and
//! the values around the point where multiplication starts to leave the range.
//! Every ordered pair from the corpus is checked for every operation and mode.

mod common;

use spl_core::prims::{self, ArithOp, CompareOp};
use spl_core::value::{Type, Value};

const ARITH_OPS: [ArithOp; 3] = [ArithOp::Add, ArithOp::Sub, ArithOp::Mul];

fn value_of(ty: Type, raw: i128) -> Value {
    match ty {
        Type::U32 => Value::U32(u32::try_from(raw).expect("corpus value is in range")),
        Type::I32 => Value::I32(i32::try_from(raw).expect("corpus value is in range")),
        Type::U8 => Value::U8(u8::try_from(raw).expect("corpus value is in range")),
        Type::Bool => panic!("Bool is not an arithmetic type"),
    }
}

fn check_type(ty: Type) {
    let corpus = common::boundary_corpus(ty);
    assert!(
        corpus.len() >= 7,
        "the corpus for {ty} must cover the documented boundaries"
    );
    for op in ARITH_OPS {
        for &a in &corpus {
            for &b in &corpus {
                let lhs = value_of(ty, a);
                let rhs = value_of(ty, b);

                let expected_wrapping = common::expected_wrapping(ty, op, a, b);
                let actual_wrapping = prims::wrapping(op, lhs, rhs).expect("arithmetic is defined");
                assert_eq!(
                    actual_wrapping,
                    value_of(ty, expected_wrapping),
                    "wrapping {} on {ty} {a}, {b}",
                    op.name()
                );

                let (expected_value, expected_overflow) = common::expected_checked(ty, op, a, b);
                let actual_checked = prims::checked(op, lhs, rhs).expect("arithmetic is defined");
                assert_eq!(
                    actual_checked.overflow,
                    expected_overflow,
                    "checked {} overflow flag on {ty} {a}, {b}",
                    op.name()
                );
                assert_eq!(
                    actual_checked.wrapped,
                    value_of(ty, expected_value),
                    "checked {} value on {ty} {a}, {b}",
                    op.name()
                );
            }
        }
    }
}

#[test]
fn u32_boundaries_are_correct() {
    check_type(Type::U32);
}

#[test]
fn i32_boundaries_are_correct() {
    check_type(Type::I32);
}

#[test]
fn corpus_actually_contains_the_extremes() {
    // Guards the guard: a corpus that silently lost its boundary values would
    // make the tests above pass while testing nothing interesting.
    let u32_corpus = common::boundary_corpus(Type::U32);
    assert!(u32_corpus.contains(&0));
    assert!(u32_corpus.contains(&i128::from(u32::MAX)));
    let i32_corpus = common::boundary_corpus(Type::I32);
    assert!(i32_corpus.contains(&i128::from(i32::MIN)));
    assert!(i32_corpus.contains(&-1));
    assert!(i32_corpus.contains(&i128::from(i32::MAX)));
}

#[test]
fn signed_comparison_is_signed_and_unsigned_comparison_is_unsigned() {
    // The same bit pattern orders differently in the two types. This is the
    // property a realization that confuses the two would break.
    assert_eq!(
        prims::compare(CompareOp::Lt, Value::I32(-1), Value::I32(1)),
        Ok(true)
    );
    assert_eq!(
        prims::compare(CompareOp::Lt, Value::U32(u32::MAX), Value::U32(1)),
        Ok(false)
    );
}

#[test]
fn signed_overflow_boundaries_behave_as_defined() {
    // Spot checks stated by hand, independent of both the implementation and
    // the i128 oracle, on the cases most often got wrong.
    let max = Value::I32(i32::MAX);
    let min = Value::I32(i32::MIN);
    let one = Value::I32(1);

    assert!(prims::checked(ArithOp::Add, max, one).unwrap().overflow);
    assert_eq!(
        prims::wrapping(ArithOp::Add, max, one),
        Ok(Value::I32(i32::MIN))
    );
    assert!(prims::checked(ArithOp::Sub, min, one).unwrap().overflow);
    assert_eq!(
        prims::wrapping(ArithOp::Sub, min, one),
        Ok(Value::I32(i32::MAX))
    );
    // -MIN is not representable.
    assert!(
        prims::checked(ArithOp::Mul, min, Value::I32(-1))
            .unwrap()
            .overflow
    );
    assert!(
        !prims::checked(ArithOp::Mul, max, Value::I32(-1))
            .unwrap()
            .overflow
    );
}

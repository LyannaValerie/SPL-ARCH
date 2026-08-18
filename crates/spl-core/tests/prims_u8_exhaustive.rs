//! Exhaustive conformance for the `u8` primitives: every one of the 65 536
//! operand pairs, for every arithmetic operation and mode, plus every
//! comparison. Expectations come from `common`, which shares no code with the
//! implementation.

mod common;

use spl_core::prims::{self, ArithOp, CompareOp};
use spl_core::value::{Type, Value};

const ARITH_OPS: [ArithOp; 3] = [ArithOp::Add, ArithOp::Sub, ArithOp::Mul];
const COMPARE_OPS: [CompareOp; 6] = [
    CompareOp::Eq,
    CompareOp::Ne,
    CompareOp::Lt,
    CompareOp::Le,
    CompareOp::Gt,
    CompareOp::Ge,
];

#[test]
fn u8_wrapping_is_exhaustively_correct() {
    for op in ARITH_OPS {
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                let expected =
                    common::expected_wrapping(Type::U8, op, i128::from(a), i128::from(b));
                let actual = prims::wrapping(op, Value::U8(a), Value::U8(b))
                    .expect("u8 arithmetic is defined");
                assert_eq!(
                    actual,
                    Value::U8(u8::try_from(expected).expect("reduced into u8 range")),
                    "wrapping {} on {a}, {b}",
                    op.name()
                );
            }
        }
    }
}

#[test]
fn u8_checked_is_exhaustively_correct() {
    for op in ARITH_OPS {
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                let (expected_value, expected_overflow) =
                    common::expected_checked(Type::U8, op, i128::from(a), i128::from(b));
                let actual = prims::checked(op, Value::U8(a), Value::U8(b))
                    .expect("u8 arithmetic is defined");
                assert_eq!(
                    actual.overflow,
                    expected_overflow,
                    "checked {} overflow flag on {a}, {b}",
                    op.name()
                );
                assert_eq!(
                    actual.wrapped,
                    Value::U8(u8::try_from(expected_value).expect("reduced into u8 range")),
                    "checked {} value on {a}, {b}",
                    op.name()
                );
            }
        }
    }
}

#[test]
fn u8_comparisons_are_exhaustively_correct() {
    for op in COMPARE_OPS {
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                // The expectation is stated over the mathematical integers, not
                // over `u8` comparison.
                let (x, y) = (i128::from(a), i128::from(b));
                let expected = match op {
                    CompareOp::Eq => x == y,
                    CompareOp::Ne => x != y,
                    CompareOp::Lt => x < y,
                    CompareOp::Le => x <= y,
                    CompareOp::Gt => x > y,
                    CompareOp::Ge => x >= y,
                };
                let actual = prims::compare(op, Value::U8(a), Value::U8(b))
                    .expect("u8 comparison is defined");
                assert_eq!(actual, expected, "compare {} on {a}, {b}", op.name());
            }
        }
    }
}

#[test]
fn checked_and_wrapping_agree_exactly_when_there_is_no_overflow() {
    // Not a tautology: it pins the relation between the two modes, which is
    // what makes "model checked as wrapping" a detectable mistake rather than
    // an invisible one.
    for op in ARITH_OPS {
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                let wrapped = prims::wrapping(op, Value::U8(a), Value::U8(b)).unwrap();
                let checked = prims::checked(op, Value::U8(a), Value::U8(b)).unwrap();
                assert_eq!(checked.wrapped, wrapped);
                let (_, overflow) =
                    common::expected_checked(Type::U8, op, i128::from(a), i128::from(b));
                assert_eq!(checked.overflow, overflow);
            }
        }
    }
}

#[test]
fn arithmetic_is_rejected_on_bool() {
    for op in ARITH_OPS {
        assert!(prims::wrapping(op, Value::Bool(true), Value::Bool(false)).is_err());
        assert!(prims::checked(op, Value::Bool(true), Value::Bool(false)).is_err());
    }
}

#[test]
fn mixed_operand_types_are_rejected() {
    assert!(prims::wrapping(ArithOp::Add, Value::U8(1), Value::U32(1)).is_err());
    assert!(prims::checked(ArithOp::Add, Value::U8(1), Value::I32(1)).is_err());
    assert!(prims::compare(CompareOp::Eq, Value::U32(1), Value::I32(1)).is_err());
}

#[test]
fn bool_admits_equality_only() {
    assert_eq!(
        prims::compare(CompareOp::Eq, Value::Bool(true), Value::Bool(true)),
        Ok(true)
    );
    assert_eq!(
        prims::compare(CompareOp::Ne, Value::Bool(true), Value::Bool(false)),
        Ok(true)
    );
    for op in [CompareOp::Lt, CompareOp::Le, CompareOp::Gt, CompareOp::Ge] {
        assert!(prims::compare(op, Value::Bool(false), Value::Bool(true)).is_err());
    }
}

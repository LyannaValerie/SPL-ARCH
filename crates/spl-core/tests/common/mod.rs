//! An oracle for Core A arithmetic that shares no code with the implementation.
//!
//! The Stage-1 plan forbids using one implementation under test as the oracle
//! for another. So this module does not call `spl_core::prims` and does not use
//! `wrapping_*` or `overflowing_*` on the narrow types either. It computes the
//! exact mathematical result in `i128` and then applies the definition of the
//! two arithmetic modes directly:
//!
//! * a type of width `w` has a range; *checked* overflows exactly when the
//!   exact result falls outside that range;
//! * *wrapping* reduces the exact result modulo `2^w`, interpreting the
//!   residue as signed or unsigned according to the type.

#![allow(dead_code)] // each integration test binary uses a different subset

use spl_core::prims::ArithOp;
use spl_core::value::Type;

/// Width in bits and signedness of a Core A integer type.
pub fn shape(ty: Type) -> (u32, bool) {
    match ty {
        Type::U8 => (8, false),
        Type::U32 => (32, false),
        Type::I32 => (32, true),
        Type::Bool => panic!("Bool is not an arithmetic type"),
    }
}

pub fn range(ty: Type) -> (i128, i128) {
    let (width, signed) = shape(ty);
    if signed {
        (-(1i128 << (width - 1)), (1i128 << (width - 1)) - 1)
    } else {
        (0, (1i128 << width) - 1)
    }
}

/// The exact mathematical result, with no truncation anywhere.
pub fn exact(op: ArithOp, a: i128, b: i128) -> i128 {
    match op {
        ArithOp::Add => a + b,
        ArithOp::Sub => a - b,
        ArithOp::Mul => a * b,
    }
}

/// Reduce an exact result into the type's range, by the definition of wrapping.
pub fn reduce(ty: Type, exact: i128) -> i128 {
    let (width, signed) = shape(ty);
    let modulus = 1i128 << width;
    let residue = exact.rem_euclid(modulus);
    if signed && residue >= modulus / 2 {
        residue - modulus
    } else {
        residue
    }
}

/// What wrapping arithmetic must produce.
pub fn expected_wrapping(ty: Type, op: ArithOp, a: i128, b: i128) -> i128 {
    reduce(ty, exact(op, a, b))
}

/// What checked arithmetic must produce: the wrapped value, and whether the
/// exact result left the range.
pub fn expected_checked(ty: Type, op: ArithOp, a: i128, b: i128) -> (i128, bool) {
    let exact = exact(op, a, b);
    let (low, high) = range(ty);
    (reduce(ty, exact), exact < low || exact > high)
}

/// The boundary corpus for the wide types.
pub fn boundary_corpus(ty: Type) -> Vec<i128> {
    let (low, high) = range(ty);
    let (width, signed) = shape(ty);
    let mut values = vec![low, low + 1, 0, 1, high - 1, high];
    if signed {
        values.push(-1);
    }
    // Values around the point where multiplication starts to leave the range.
    let half = 1i128 << (width / 2);
    values.extend([half - 1, half, half + 1]);
    if signed {
        values.extend([-half - 1, -half, -half + 1]);
    }
    values.sort_unstable();
    values.dedup();
    values.retain(|value| *value >= low && *value <= high);
    values
}

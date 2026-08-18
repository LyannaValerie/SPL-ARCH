//! Core A primitive semantics.
//!
//! This module is the single definition of what a Core A arithmetic or
//! comparison primitive *means*. Both the `K` evaluator and, in `spl-plan`, the
//! Plan VM consume it.
//!
//! That sharing is deliberate and has a cost. The plan calls it common-mode
//! semantic risk: a bug here is a bug in the contract and in every realization
//! at once, and no amount of K/P structural independence would catch it. The
//! alternative — two independent primitive implementations — would only move
//! the problem, because the conformance suite would then be comparing one
//! implementation under test against another, which the Stage-1 plan forbids as
//! an oracle. The mitigation used here is an oracle that shares no code with
//! this module: the conformance tests compute expectations in `i128` and reduce
//! into range explicitly.
//!
//! Nothing here relies on the host language's implicit overflow behaviour.
//! Every operation names the width and the mode it intends.

use crate::value::{Type, Value};

/// The Core A arithmetic operators. Each exists in a wrapping and a checked
/// flavour; the flavour is chosen by the caller, never inferred.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
}

impl ArithOp {
    pub fn name(self) -> &'static str {
        match self {
            ArithOp::Add => "add",
            ArithOp::Sub => "sub",
            ArithOp::Mul => "mul",
        }
    }
}

/// The Core A comparison operators.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CompareOp {
    pub fn name(self) -> &'static str {
        match self {
            CompareOp::Eq => "eq",
            CompareOp::Ne => "ne",
            CompareOp::Lt => "lt",
            CompareOp::Le => "le",
            CompareOp::Gt => "gt",
            CompareOp::Ge => "ge",
        }
    }

    /// Whether the operator needs an ordered type.
    pub fn needs_order(self) -> bool {
        !matches!(self, CompareOp::Eq | CompareOp::Ne)
    }
}

/// The result of a checked arithmetic primitive, before any decision is taken
/// about what it means.
///
/// `K` turns `overflow` into `SemanticError(Overflow)` by propagation; the Plan
/// IR exposes it as a separate boolean register the plan must branch on.
/// Keeping the primitive at this raw level is what lets both consume one
/// definition without either becoming a lowering of the other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CheckedResult {
    /// The width-truncated result, meaningful only when `overflow` is false.
    pub wrapped: Value,
    /// Whether the exact mathematical result is outside the type's range.
    pub overflow: bool,
}

/// A primitive was applied to operands it is not defined for.
///
/// This is a *type* error, not a semantic error: it means the caller handed the
/// primitive layer something a type-checked contract or a validated plan could
/// never have produced.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PrimError {
    /// The two operands have different types.
    OperandTypeMismatch { lhs: Type, rhs: Type },
    /// The operation is not defined for this type.
    UndefinedForType { op_name: &'static str, ty: Type },
}

impl core::fmt::Display for PrimError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PrimError::OperandTypeMismatch { lhs, rhs } => {
                write!(f, "operand type mismatch: {lhs} and {rhs}")
            }
            PrimError::UndefinedForType { op_name, ty } => {
                write!(f, "operation `{op_name}` is not defined for {ty}")
            }
        }
    }
}

fn wrapping_u8(op: ArithOp, a: u8, b: u8) -> u8 {
    match op {
        ArithOp::Add => a.wrapping_add(b),
        ArithOp::Sub => a.wrapping_sub(b),
        ArithOp::Mul => a.wrapping_mul(b),
    }
}

fn wrapping_u32(op: ArithOp, a: u32, b: u32) -> u32 {
    match op {
        ArithOp::Add => a.wrapping_add(b),
        ArithOp::Sub => a.wrapping_sub(b),
        ArithOp::Mul => a.wrapping_mul(b),
    }
}

fn wrapping_i32(op: ArithOp, a: i32, b: i32) -> i32 {
    match op {
        ArithOp::Add => a.wrapping_add(b),
        ArithOp::Sub => a.wrapping_sub(b),
        ArithOp::Mul => a.wrapping_mul(b),
    }
}

fn overflowing_u8(op: ArithOp, a: u8, b: u8) -> (u8, bool) {
    match op {
        ArithOp::Add => a.overflowing_add(b),
        ArithOp::Sub => a.overflowing_sub(b),
        ArithOp::Mul => a.overflowing_mul(b),
    }
}

fn overflowing_u32(op: ArithOp, a: u32, b: u32) -> (u32, bool) {
    match op {
        ArithOp::Add => a.overflowing_add(b),
        ArithOp::Sub => a.overflowing_sub(b),
        ArithOp::Mul => a.overflowing_mul(b),
    }
}

fn overflowing_i32(op: ArithOp, a: i32, b: i32) -> (i32, bool) {
    match op {
        ArithOp::Add => a.overflowing_add(b),
        ArithOp::Sub => a.overflowing_sub(b),
        ArithOp::Mul => a.overflowing_mul(b),
    }
}

/// Wrapping arithmetic: the exact result reduced into the type's range.
///
/// Wrapping never signals overflow. A realization that models a checked
/// operation with this primitive is semantically wrong, which is what the
/// mutation checks in the test suite exercise.
pub fn wrapping(op: ArithOp, lhs: Value, rhs: Value) -> Result<Value, PrimError> {
    match (lhs, rhs) {
        (Value::U8(a), Value::U8(b)) => Ok(Value::U8(wrapping_u8(op, a, b))),
        (Value::U32(a), Value::U32(b)) => Ok(Value::U32(wrapping_u32(op, a, b))),
        (Value::I32(a), Value::I32(b)) => Ok(Value::I32(wrapping_i32(op, a, b))),
        (Value::Bool(_), Value::Bool(_)) => Err(PrimError::UndefinedForType {
            op_name: op.name(),
            ty: Type::Bool,
        }),
        _ => Err(PrimError::OperandTypeMismatch {
            lhs: lhs.ty(),
            rhs: rhs.ty(),
        }),
    }
}

/// Checked arithmetic: the wrapped result together with whether the exact
/// mathematical result left the type's range.
pub fn checked(op: ArithOp, lhs: Value, rhs: Value) -> Result<CheckedResult, PrimError> {
    let (wrapped, overflow) = match (lhs, rhs) {
        (Value::U8(a), Value::U8(b)) => {
            let (v, o) = overflowing_u8(op, a, b);
            (Value::U8(v), o)
        }
        (Value::U32(a), Value::U32(b)) => {
            let (v, o) = overflowing_u32(op, a, b);
            (Value::U32(v), o)
        }
        (Value::I32(a), Value::I32(b)) => {
            let (v, o) = overflowing_i32(op, a, b);
            (Value::I32(v), o)
        }
        (Value::Bool(_), Value::Bool(_)) => {
            return Err(PrimError::UndefinedForType {
                op_name: op.name(),
                ty: Type::Bool,
            })
        }
        _ => {
            return Err(PrimError::OperandTypeMismatch {
                lhs: lhs.ty(),
                rhs: rhs.ty(),
            })
        }
    };
    Ok(CheckedResult { wrapped, overflow })
}

/// Comparison. Unsigned types compare unsigned, `i32` compares signed, and
/// `Bool` admits equality only.
pub fn compare(op: CompareOp, lhs: Value, rhs: Value) -> Result<bool, PrimError> {
    if lhs.ty() != rhs.ty() {
        return Err(PrimError::OperandTypeMismatch {
            lhs: lhs.ty(),
            rhs: rhs.ty(),
        });
    }
    match (lhs, rhs) {
        (Value::Bool(a), Value::Bool(b)) => match op {
            CompareOp::Eq => Ok(a == b),
            CompareOp::Ne => Ok(a != b),
            _ => Err(PrimError::UndefinedForType {
                op_name: op.name(),
                ty: Type::Bool,
            }),
        },
        (Value::U8(a), Value::U8(b)) => Ok(apply_order(op, a, b)),
        (Value::U32(a), Value::U32(b)) => Ok(apply_order(op, a, b)),
        (Value::I32(a), Value::I32(b)) => Ok(apply_order(op, a, b)),
        _ => unreachable!("operand types were compared for equality above"),
    }
}

fn apply_order<T: Ord>(op: CompareOp, a: T, b: T) -> bool {
    match op {
        CompareOp::Eq => a == b,
        CompareOp::Ne => a != b,
        CompareOp::Lt => a < b,
        CompareOp::Le => a <= b,
        CompareOp::Gt => a > b,
        CompareOp::Ge => a >= b,
    }
}

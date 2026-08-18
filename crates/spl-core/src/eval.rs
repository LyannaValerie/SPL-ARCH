//! The `K` evaluator.
//!
//! This is a direct recursive interpreter over the semantic tree. It is the
//! only executable meaning of a contract in Stage 1, and it is deliberately not
//! reusable as an execution engine for anything else: it consumes [`KExpr`] and
//! [`KTerm`] and nothing else. The Plan VM in `spl-plan` shares no code with it.
//!
//! A checked overflow is raised where it happens and propagates outward to the
//! outcome. There is no overflow flag and no branch on one: expressing overflow
//! detection operationally is a realization's job, not the contract's.

use crate::k::{InputIndex, KContract, KExpr, KTerm};
use crate::outcome::{SemanticErrorKind, SemanticOutcome};
use crate::prims::{self, PrimError};
use crate::value::{Type, Value};

/// Why the evaluator refused to run.
///
/// These are all caller errors or contract errors, never semantic results. A
/// `SemanticError(Overflow)` is a perfectly good outcome and is *not* here.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KEvalError {
    /// The number of supplied arguments does not match the signature.
    InputArityMismatch { expected: usize, found: usize },
    /// A supplied argument has the wrong type.
    InputTypeMismatch {
        index: usize,
        expected: Type,
        found: Type,
    },
    /// The contract is not well typed. Call `type_check` first.
    ///
    /// The evaluator re-detects this defensively rather than panicking, so that
    /// a caller who skipped type checking gets a refusal instead of a result
    /// that looks semantic but is not.
    IllTyped(PrimError),
    /// An input index in the body has no corresponding parameter.
    UnknownInput { index: u32 },
}

impl core::fmt::Display for KEvalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KEvalError::InputArityMismatch { expected, found } => {
                write!(f, "contract expects {expected} input(s), got {found}")
            }
            KEvalError::InputTypeMismatch {
                index,
                expected,
                found,
            } => write!(f, "input {index} expects {expected}, got {found}"),
            KEvalError::IllTyped(err) => write!(f, "contract is ill typed: {err}"),
            KEvalError::UnknownInput { index } => write!(f, "input {index} is not declared"),
        }
    }
}

/// Either a value, or the semantic error raised while producing it.
type Raised<T> = Result<Result<T, SemanticErrorKind>, KEvalError>;

/// Evaluate a contract on concrete inputs.
///
/// Returns the contract's `SemanticOutcome`. The outer `Result` carries only
/// refusals to run; a semantic error is inside the outcome.
pub fn eval(contract: &KContract, inputs: &[Value]) -> Result<SemanticOutcome, KEvalError> {
    if inputs.len() != contract.params.len() {
        return Err(KEvalError::InputArityMismatch {
            expected: contract.params.len(),
            found: inputs.len(),
        });
    }
    for (index, (declared, supplied)) in contract.params.iter().zip(inputs).enumerate() {
        if *declared != supplied.ty() {
            return Err(KEvalError::InputTypeMismatch {
                index,
                expected: *declared,
                found: supplied.ty(),
            });
        }
    }
    match eval_term(&contract.body, inputs)? {
        Ok(outcome) => Ok(outcome),
        Err(kind) => Ok(SemanticOutcome::SemanticError(kind)),
    }
}

fn eval_term(term: &KTerm, inputs: &[Value]) -> Raised<SemanticOutcome> {
    match term {
        KTerm::Return(expr) => match eval_expr(expr, inputs)? {
            Ok(value) => Ok(Ok(SemanticOutcome::Return(value))),
            Err(kind) => Ok(Err(kind)),
        },
        KTerm::SemanticError(kind) => Ok(Ok(SemanticOutcome::SemanticError(*kind))),
        KTerm::If {
            cond,
            then_term,
            else_term,
        } => {
            let taken = match eval_expr(cond, inputs)? {
                Ok(Value::Bool(taken)) => taken,
                Ok(other) => {
                    return Err(KEvalError::IllTyped(PrimError::UndefinedForType {
                        op_name: "if",
                        ty: other.ty(),
                    }))
                }
                Err(kind) => return Ok(Err(kind)),
            };
            eval_term(if taken { then_term } else { else_term }, inputs)
        }
    }
}

fn eval_expr(expr: &KExpr, inputs: &[Value]) -> Raised<Value> {
    match expr {
        KExpr::Input(InputIndex(index)) => inputs
            .get(*index as usize)
            .copied()
            .map(Ok)
            .ok_or(KEvalError::UnknownInput { index: *index }),
        KExpr::Const(value) => Ok(Ok(*value)),
        KExpr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            let taken = match eval_expr(cond, inputs)? {
                Ok(Value::Bool(taken)) => taken,
                Ok(other) => {
                    return Err(KEvalError::IllTyped(PrimError::UndefinedForType {
                        op_name: "if",
                        ty: other.ty(),
                    }))
                }
                Err(kind) => return Ok(Err(kind)),
            };
            eval_expr(if taken { then_expr } else { else_expr }, inputs)
        }
        KExpr::Compare { op, lhs, rhs } => {
            let (lhs, rhs) = match eval_operands(lhs, rhs, inputs)? {
                Ok(pair) => pair,
                Err(kind) => return Ok(Err(kind)),
            };
            prims::compare(*op, lhs, rhs)
                .map(|b| Ok(Value::Bool(b)))
                .map_err(KEvalError::IllTyped)
        }
        KExpr::Wrapping { op, lhs, rhs } => {
            let (lhs, rhs) = match eval_operands(lhs, rhs, inputs)? {
                Ok(pair) => pair,
                Err(kind) => return Ok(Err(kind)),
            };
            prims::wrapping(*op, lhs, rhs)
                .map(Ok)
                .map_err(KEvalError::IllTyped)
        }
        KExpr::Checked { op, lhs, rhs } => {
            let (lhs, rhs) = match eval_operands(lhs, rhs, inputs)? {
                Ok(pair) => pair,
                Err(kind) => return Ok(Err(kind)),
            };
            let checked = prims::checked(*op, lhs, rhs).map_err(KEvalError::IllTyped)?;
            // This is the whole of checked semantics in K: the overflow becomes
            // a raised semantic error at the point of the operation.
            if checked.overflow {
                Ok(Err(SemanticErrorKind::Overflow))
            } else {
                Ok(Ok(checked.wrapped))
            }
        }
    }
}

fn eval_operands(lhs: &KExpr, rhs: &KExpr, inputs: &[Value]) -> Raised<(Value, Value)> {
    // Left-to-right: if the left operand raises, the right one is not evaluated.
    // Core A is effect-free, so this is observable only through which semantic
    // error surfaces first, but it is a semantic choice and is fixed here.
    let lhs = match eval_expr(lhs, inputs)? {
        Ok(value) => value,
        Err(kind) => return Ok(Err(kind)),
    };
    let rhs = match eval_expr(rhs, inputs)? {
        Ok(value) => value,
        Err(kind) => return Ok(Err(kind)),
    };
    Ok(Ok((lhs, rhs)))
}

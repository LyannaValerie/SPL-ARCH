//! `K` — the semantic contract representation.
//!
//! `K` is a typed semantic tree. It says what the computation *means*; it says
//! nothing about registers, blocks, branches, evaluation order, or how an
//! overflow is detected on any machine. Those are realization concerns and live
//! in the Plan IR of `spl-plan`.
//!
//! The tree is split in two levels, because Core A distinguishes values from
//! outcomes:
//!
//! * [`KExpr`] denotes a value. Evaluating it may raise a semantic error, which
//!   propagates outward: that is how `K` expresses checked arithmetic. There is
//!   no overflow flag anywhere in `K`.
//! * [`KTerm`] denotes a `SemanticOutcome`. It either returns an expression's
//!   value or raises a semantic error directly.

use crate::outcome::SemanticErrorKind;
use crate::prims::{ArithOp, CompareOp};
use crate::value::{Type, Value};

/// Index of a contract input, into the declared parameter list.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct InputIndex(pub u32);

/// A Core A value expression.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KExpr {
    /// The value of a declared input.
    Input(InputIndex),
    /// A literal.
    Const(Value),
    /// Value-level conditional. Both arms must have the same type.
    If {
        cond: Box<KExpr>,
        then_expr: Box<KExpr>,
        else_expr: Box<KExpr>,
    },
    /// Comparison, yielding `Bool`.
    Compare {
        op: CompareOp,
        lhs: Box<KExpr>,
        rhs: Box<KExpr>,
    },
    /// Wrapping arithmetic. Cannot raise.
    Wrapping {
        op: ArithOp,
        lhs: Box<KExpr>,
        rhs: Box<KExpr>,
    },
    /// Checked arithmetic. Raises `SemanticError(Overflow)` when the exact
    /// result leaves the type's range, and the raise propagates to the outcome.
    Checked {
        op: ArithOp,
        lhs: Box<KExpr>,
        rhs: Box<KExpr>,
    },
}

/// A Core A outcome-level term.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KTerm {
    /// `Return(e)`, where evaluating `e` may itself raise.
    Return(KExpr),
    /// `SemanticError(kind)`, raised unconditionally.
    SemanticError(SemanticErrorKind),
    /// Outcome-level conditional.
    If {
        cond: KExpr,
        then_term: Box<KTerm>,
        else_term: Box<KTerm>,
    },
}

/// A complete semantic contract: the signature plus the body.
///
/// `name` is documentation for humans. It is part of the artifact bytes, so two
/// contracts that differ only in their name are different artifacts with
/// different content identities — identity is over exact content, never over
/// semantic equivalence.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KContract {
    pub name: String,
    pub params: Vec<Type>,
    pub result: Type,
    pub body: KTerm,
}

/// Why a contract is not well typed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KTypeError {
    /// An input index has no corresponding declared parameter.
    UnknownInput { index: u32, params: usize },
    /// A condition is not `Bool`.
    ConditionNotBool { found: Type },
    /// Two branches of a conditional disagree.
    BranchTypeMismatch { then_ty: Type, else_ty: Type },
    /// An operator's operands disagree.
    OperandTypeMismatch { lhs: Type, rhs: Type },
    /// An operator is not defined for the operand type.
    UndefinedForType { op_name: &'static str, ty: Type },
    /// The returned expression does not have the declared result type.
    ResultTypeMismatch { declared: Type, found: Type },
}

impl core::fmt::Display for KTypeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KTypeError::UnknownInput { index, params } => {
                write!(
                    f,
                    "input {index} out of range; the contract declares {params} parameter(s)"
                )
            }
            KTypeError::ConditionNotBool { found } => {
                write!(f, "condition must be Bool, found {found}")
            }
            KTypeError::BranchTypeMismatch { then_ty, else_ty } => {
                write!(f, "conditional branches disagree: {then_ty} and {else_ty}")
            }
            KTypeError::OperandTypeMismatch { lhs, rhs } => {
                write!(f, "operand type mismatch: {lhs} and {rhs}")
            }
            KTypeError::UndefinedForType { op_name, ty } => {
                write!(f, "operation `{op_name}` is not defined for {ty}")
            }
            KTypeError::ResultTypeMismatch { declared, found } => {
                write!(f, "contract declares result {declared} but returns {found}")
            }
        }
    }
}

impl KContract {
    /// Reject ill-typed contracts explicitly.
    ///
    /// Type checking is total and syntax-directed: it never evaluates anything,
    /// so a contract can be rejected without running it.
    pub fn type_check(&self) -> Result<(), KTypeError> {
        self.check_term(&self.body)
    }

    fn check_term(&self, term: &KTerm) -> Result<(), KTypeError> {
        match term {
            KTerm::Return(expr) => {
                let found = self.check_expr(expr)?;
                if found == self.result {
                    Ok(())
                } else {
                    Err(KTypeError::ResultTypeMismatch {
                        declared: self.result,
                        found,
                    })
                }
            }
            KTerm::SemanticError(_) => Ok(()),
            KTerm::If {
                cond,
                then_term,
                else_term,
            } => {
                let cond_ty = self.check_expr(cond)?;
                if cond_ty != Type::Bool {
                    return Err(KTypeError::ConditionNotBool { found: cond_ty });
                }
                self.check_term(then_term)?;
                self.check_term(else_term)
            }
        }
    }

    fn check_expr(&self, expr: &KExpr) -> Result<Type, KTypeError> {
        match expr {
            KExpr::Input(InputIndex(index)) => {
                self.params
                    .get(*index as usize)
                    .copied()
                    .ok_or(KTypeError::UnknownInput {
                        index: *index,
                        params: self.params.len(),
                    })
            }
            KExpr::Const(value) => Ok(value.ty()),
            KExpr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_ty = self.check_expr(cond)?;
                if cond_ty != Type::Bool {
                    return Err(KTypeError::ConditionNotBool { found: cond_ty });
                }
                let then_ty = self.check_expr(then_expr)?;
                let else_ty = self.check_expr(else_expr)?;
                if then_ty == else_ty {
                    Ok(then_ty)
                } else {
                    Err(KTypeError::BranchTypeMismatch { then_ty, else_ty })
                }
            }
            KExpr::Compare { op, lhs, rhs } => {
                let ty = self.check_binary(lhs, rhs)?;
                if op.needs_order() && !ty.is_ordered() {
                    return Err(KTypeError::UndefinedForType {
                        op_name: op.name(),
                        ty,
                    });
                }
                Ok(Type::Bool)
            }
            KExpr::Wrapping { op, lhs, rhs } | KExpr::Checked { op, lhs, rhs } => {
                let ty = self.check_binary(lhs, rhs)?;
                if !ty.is_arithmetic() {
                    return Err(KTypeError::UndefinedForType {
                        op_name: op.name(),
                        ty,
                    });
                }
                Ok(ty)
            }
        }
    }

    fn check_binary(&self, lhs: &KExpr, rhs: &KExpr) -> Result<Type, KTypeError> {
        let lhs_ty = self.check_expr(lhs)?;
        let rhs_ty = self.check_expr(rhs)?;
        if lhs_ty == rhs_ty {
            Ok(lhs_ty)
        } else {
            Err(KTypeError::OperandTypeMismatch {
                lhs: lhs_ty,
                rhs: rhs_ty,
            })
        }
    }
}

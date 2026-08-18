//! The Stage-1 experimental contract.
//!
//! One `u8` contract, small enough to check exhaustively over all 256 inputs:
//!
//! ```text
//! K(x):
//!     t = checked_add(x, 255)      raises SemanticError(Overflow) on overflow
//!     checked_sub(t, 255)          raises SemanticError(Overflow) on overflow
//! ```
//!
//! Its meaning, stated independently of this code, is
//!
//! ```text
//! x == 0  ->  Return(0)
//! x >  0  ->  SemanticError(Overflow)
//! ```
//!
//! because `x + 255` leaves `u8` for every `x` above zero. That statement is
//! the oracle the tests use; they do not obtain it by running `K`.
//!
//! Note what the contract does *not* contain: there is no overflow flag and no
//! branch on one. `K` raises where the overflow happens and the raise
//! propagates. Turning that into a test-and-branch is a realization's job.

use crate::k::{InputIndex, KContract, KExpr, KTerm};
use crate::prims::ArithOp;
use crate::value::{Type, Value};

/// The name carried inside the contract artifact.
pub const CHECKED_ROUNDTRIP_NAME: &str = "checked_roundtrip_u8";

/// `K(x) = checked_sub(checked_add(x, 255), 255)` over `u8`.
pub fn checked_roundtrip_contract() -> KContract {
    KContract {
        name: CHECKED_ROUNDTRIP_NAME.to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        body: KTerm::Return(KExpr::Checked {
            op: ArithOp::Sub,
            lhs: Box::new(KExpr::Checked {
                op: ArithOp::Add,
                lhs: Box::new(KExpr::Input(InputIndex(0))),
                rhs: Box::new(KExpr::Const(Value::U8(255))),
            }),
            rhs: Box::new(KExpr::Const(Value::U8(255))),
        }),
    }
}

/// A deliberately mutated contract: the checked operations become wrapping.
///
/// Used by the mutation sanity checks to confirm the suite notices a semantic
/// difference that a careless implementation would produce.
pub fn mutated_wrapping_contract() -> KContract {
    KContract {
        name: "mutant_wrapping_roundtrip_u8".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        body: KTerm::Return(KExpr::Wrapping {
            op: ArithOp::Sub,
            lhs: Box::new(KExpr::Wrapping {
                op: ArithOp::Add,
                lhs: Box::new(KExpr::Input(InputIndex(0))),
                rhs: Box::new(KExpr::Const(Value::U8(255))),
            }),
            rhs: Box::new(KExpr::Const(Value::U8(255))),
        }),
    }
}

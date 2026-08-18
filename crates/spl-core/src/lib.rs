//! `spl-core` — the SPL-0 semantic core.
//!
//! This crate holds the semantic authority side of the Stage-1 slice. So far:
//!
//! * the Core A value and outcome domains, and `embed`;
//! * the Core A primitive semantics;
//! * `K`, the semantic contract representation, its type checker and its
//!   evaluator.
//!
//! It holds no execution plan, no register machine and no interpreter for
//! anything other than `K`.
//!
//! Stage 1 has no admission, no verification condition, no evidence and no
//! trust decision of any kind. Nothing in this crate accepts or rejects a
//! realization; it only says what the contract means.

#![forbid(unsafe_code)]

pub mod demo;
pub mod eval;
pub mod k;
pub mod outcome;
pub mod prims;
pub mod value;

pub use eval::eval;
pub use outcome::{embed, project, ObservedOutcome, SemanticErrorKind, SemanticOutcome, TrapKind};
pub use value::{Type, Value};

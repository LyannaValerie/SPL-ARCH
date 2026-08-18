//! `spl-core` — the SPL-0 semantic core.
//!
//! This crate holds the semantic authority side of the Stage-1 slice. So far:
//!
//! * the Core A value and outcome domains, and `embed`;
//! * the Core A primitive semantics.
//!
//! Stage 1 has no admission, no verification condition, no evidence and no
//! trust decision of any kind. Nothing in this crate accepts or rejects a
//! realization; it only says what the contract means.

#![forbid(unsafe_code)]

pub mod outcome;
pub mod prims;
pub mod value;

pub use outcome::{embed, project, ObservedOutcome, SemanticErrorKind, SemanticOutcome, TrapKind};
pub use value::{Type, Value};

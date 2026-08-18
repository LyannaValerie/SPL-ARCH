//! `spl-plan` — the SPL-0 realization side of the Stage-1 slice.
//!
//! This crate holds the Plan IR, its structural validator, the Plan VM, and the
//! `S`/`Q` identities the VM runs under. It is where an *execution plan* lives.
//!
//! It does not contain a semantic contract, a contract evaluator, or any notion
//! of a plan being correct. It depends on `spl-core` for the Core A value and
//! outcome domains, the primitive semantics and the artifact encoding — shared
//! vocabulary — and for nothing else. In particular the Plan VM never lowers a
//! plan to `K`, never evaluates `K`, and never compares a plan against a
//! blessed lowering of one.
//!
//! Stage 1 has no admission, no obligation, no evidence and no guard. A plan
//! that runs here has been *executed*, never *authorized*.

#![forbid(unsafe_code)]

pub mod artifact;
pub mod config;
pub mod demo;
pub mod ir;
pub mod validate;
pub mod vm;

pub use config::{ArithmeticMode, PlanVmConfiguration, SubstrateIdentity};
pub use ir::{Block, BlockId, Instruction, Plan, Reg, Terminator};
pub use validate::{validate, PlanError, ValidatedPlan};
pub use vm::{PlanVm, VmError};

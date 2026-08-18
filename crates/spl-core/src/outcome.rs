//! Outcome domains.
//!
//! `SemanticOutcome` is what the semantic contract `K` produces. `ObservedOutcome`
//! is what an execution produces. The frozen theory relates them through `embed`,
//! whose image excludes `Trap`, `Stuck` and `Diverge`.

use crate::value::Value;

/// Semantic errors of Core A.
///
/// Core A raises exactly one semantic error. Adding a variant is a change to the
/// semantic contract language, not an implementation detail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticErrorKind {
    Overflow,
}

/// Machine-level faults. Deliberately *not* semantic errors.
///
/// The frozen theory requires `Trap(t) != SemanticError(e)`, so these are a
/// separate domain and no conversion between them exists in this crate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TrapKind {
    /// The execution substrate refused to continue because a resource bound was
    /// reached. Core A programs are loop-free, so a Stage-1 Plan VM run should
    /// never produce this; it exists so that the bound is expressible rather
    /// than silently assumed.
    StepLimitExceeded,
}

/// `SemanticOutcome<Y,E>` of the frozen theory, instantiated for Core A.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SemanticOutcome {
    Return(Value),
    SemanticError(SemanticErrorKind),
}

/// `ObservedOutcome<Y,E,T>` of the frozen theory, instantiated for Core A.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObservedOutcome {
    Return(Value),
    SemanticError(SemanticErrorKind),
    Trap(TrapKind),
    /// Execution reached a state from which it cannot proceed.
    Stuck,
    /// Execution does not terminate.
    Diverge,
}

/// `embed : SemanticOutcome -> ObservedOutcome`.
pub fn embed(outcome: SemanticOutcome) -> ObservedOutcome {
    match outcome {
        SemanticOutcome::Return(value) => ObservedOutcome::Return(value),
        SemanticOutcome::SemanticError(kind) => ObservedOutcome::SemanticError(kind),
    }
}

/// The partial inverse of [`embed`].
///
/// Returns `None` exactly on the observed outcomes that are outside the image of
/// `embed`, i.e. the ones no semantic contract can ever authorize.
pub fn project(observed: ObservedOutcome) -> Option<SemanticOutcome> {
    match observed {
        ObservedOutcome::Return(value) => Some(SemanticOutcome::Return(value)),
        ObservedOutcome::SemanticError(kind) => Some(SemanticOutcome::SemanticError(kind)),
        ObservedOutcome::Trap(_) | ObservedOutcome::Stuck | ObservedOutcome::Diverge => None,
    }
}

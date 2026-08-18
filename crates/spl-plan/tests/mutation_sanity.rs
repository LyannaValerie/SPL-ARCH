//! Mutation sanity checks.
//!
//! A test suite that agrees with everything proves nothing. These deliberately
//! break the contract or a realization in the three ways most likely to happen
//! by accident — checked becomes wrapping, a comparison becomes the neighbouring
//! one, an error arm returns a value — and assert that the Stage-1 comparison
//! notices.
//!
//! This is not a mutation-testing framework and does not claim coverage. It is
//! a handful of named mutants with a stated expected effect.

use spl_core::outcome::{embed, ObservedOutcome, SemanticErrorKind, SemanticOutcome};
use spl_core::value::Value;
use spl_core::{demo as k_demo, eval};
use spl_plan::demo as p_demo;
use spl_plan::ir::Plan;
use spl_plan::validate::validate;
use spl_plan::vm::PlanVm;

/// The independent expectation, restated here so that this file does not depend
/// on the slice test.
fn oracle(x: u8) -> SemanticOutcome {
    if x == 0 {
        SemanticOutcome::Return(Value::U8(0))
    } else {
        SemanticOutcome::SemanticError(SemanticErrorKind::Overflow)
    }
}

/// Every input at which a plan disagrees with the contract.
fn disagreements(plan: Plan) -> Vec<u8> {
    let contract = k_demo::checked_roundtrip_contract();
    let validated = validate(plan).expect("the mutants are still well formed");
    let vm = PlanVm::stage1();
    (0u8..=255)
        .filter(|x| {
            let observed = vm
                .run(&validated, &[Value::U8(*x)])
                .expect("the mutant runs");
            let expected = eval(&contract, &[Value::U8(*x)]).unwrap();
            observed != embed(expected)
        })
        .collect()
}

#[test]
fn the_unmutated_realizations_disagree_nowhere() {
    // The control. If this ever fails, the mutants below prove nothing.
    assert!(disagreements(p_demo::p_reference()).is_empty());
    assert!(disagreements(p_demo::p_specialized()).is_empty());
}

#[test]
fn mutating_the_contract_from_checked_to_wrapping_changes_its_meaning() {
    let mutant = k_demo::mutated_wrapping_contract();
    mutant
        .type_check()
        .expect("the mutant is still a well-typed contract");
    let mut disagreed = Vec::new();
    for x in 0u8..=255 {
        if eval(&mutant, &[Value::U8(x)]).unwrap() != oracle(x) {
            disagreed.push(x);
        }
    }
    assert_eq!(
        disagreed.len(),
        255,
        "wrapping never raises, so the mutant should disagree everywhere except x = 0"
    );
    // And concretely: it returns the input instead of raising.
    assert_eq!(
        eval(&mutant, &[Value::U8(7)]).unwrap(),
        SemanticOutcome::Return(Value::U8(7))
    );
}

#[test]
fn mutating_a_plan_from_checked_to_wrapping_is_detected() {
    let disagreed = disagreements(p_demo::mutated_wrapping_realization());
    assert_eq!(
        disagreed.len(),
        255,
        "wrapping never raises, so every x > 0 must disagree"
    );
    assert!(disagreed.contains(&1), "x = 1 must expose the mutation");
}

#[test]
fn an_equivalent_mutant_survives_and_that_is_the_correct_result() {
    // Removing the first overflow check changes the plan but not its meaning:
    // the second check raises on exactly the same inputs. The suite reports no
    // disagreement, which is right. Recorded so that "the mutation survived"
    // is not silently read as "the tests are weak".
    let disagreed = disagreements(p_demo::equivalent_dropped_first_overflow_check());
    assert!(
        disagreed.is_empty(),
        "this mutation is behaviour-preserving for this contract, \
         yet it disagreed at {disagreed:?}"
    );
}

#[test]
fn mutating_a_guard_comparison_is_detected() {
    // `x > 0` becomes `x >= 0`, which is true for every u8, so the
    // specialization raises even at x = 0.
    let disagreed = disagreements(p_demo::mutated_wrong_comparison());
    assert_eq!(
        disagreed,
        vec![0],
        "the widened guard must be wrong exactly at x = 0"
    );
}

#[test]
fn mutating_an_outcome_tag_is_detected_even_when_a_payload_coincides() {
    // The error arm returns `Return(0)` instead of raising. The payload is the
    // same value the contract returns at x = 0, so only the *tag* distinguishes
    // the outcomes. A comparison over payloads alone would miss this.
    let mutant = p_demo::mutated_wrong_outcome_tag();
    let disagreed = disagreements(mutant.clone());
    assert_eq!(disagreed.len(), 255, "every x > 0 must disagree");

    let validated = validate(mutant).unwrap();
    let observed = PlanVm::stage1().run(&validated, &[Value::U8(9)]).unwrap();
    assert_eq!(observed, ObservedOutcome::Return(Value::U8(0)));
    assert_eq!(
        eval(&k_demo::checked_roundtrip_contract(), &[Value::U8(9)]).unwrap(),
        SemanticOutcome::SemanticError(SemanticErrorKind::Overflow)
    );
    // Same payload, different tag, therefore different outcome.
    assert_ne!(
        observed,
        embed(SemanticOutcome::SemanticError(SemanticErrorKind::Overflow))
    );
}

#[test]
fn every_mutant_is_still_structurally_valid() {
    // The mutations must fail semantically, not structurally. A mutant the
    // validator rejects would be testing the validator, not the comparison.
    for plan in [
        p_demo::mutated_wrapping_realization(),
        p_demo::equivalent_dropped_first_overflow_check(),
        p_demo::mutated_wrong_comparison(),
        p_demo::mutated_wrong_outcome_tag(),
    ] {
        let name = plan.name.clone();
        validate(plan).unwrap_or_else(|err| panic!("{name} should still validate, got {err}"));
    }
}

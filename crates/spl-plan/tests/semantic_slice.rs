//! The Stage-1 semantic vertical slice.
//!
//! One contract, three realizations, all 256 `u8` inputs, and an oracle written
//! out by hand rather than obtained by running either side.
//!
//! The relation being checked is the frozen one, for `Gv = true`:
//!
//! ```text
//! forall x:  Run(P, x) == embed(K(x))
//! ```
//!
//! Nothing here admits anything. Stage 1 has no admission, no obligation and no
//! evidence; the disagreement of `p_bad` is *observed*, not acted on.

use spl_core::artifact::{Artifact, KArtifact};
use spl_core::outcome::{embed, project, ObservedOutcome, SemanticErrorKind, SemanticOutcome};
use spl_core::value::Value;
use spl_core::{demo as k_demo, eval};
use spl_plan::artifact::PlanArtifact;
use spl_plan::demo as p_demo;
use spl_plan::ir::Plan;
use spl_plan::validate::validate;
use spl_plan::vm::PlanVm;

/// The independent expectation for the Stage-1 contract.
///
/// This is the meaning stated in prose in the contract's documentation, written
/// out directly. It does not call `K`, it does not call a plan, and it does not
/// use checked arithmetic to decide whether checked arithmetic overflows:
/// `x + 255` leaves `u8` for exactly the inputs above zero.
fn oracle(x: u8) -> SemanticOutcome {
    if x == 0 {
        SemanticOutcome::Return(Value::U8(0))
    } else {
        SemanticOutcome::SemanticError(SemanticErrorKind::Overflow)
    }
}

fn run_plan(plan: Plan, x: u8) -> ObservedOutcome {
    let validated = validate(plan).expect("the demo plans are well formed");
    PlanVm::stage1()
        .run(&validated, &[Value::U8(x)])
        .expect("the Stage-1 configuration hosts the demo plans")
}

#[test]
fn the_contract_means_what_the_oracle_says_for_all_256_inputs() {
    let contract = k_demo::checked_roundtrip_contract();
    contract.type_check().expect("the contract is well typed");
    for x in 0u8..=255 {
        assert_eq!(
            eval(&contract, &[Value::U8(x)]).expect("inputs match the signature"),
            oracle(x),
            "K disagrees with the independent expectation at x = {x}"
        );
    }
}

#[test]
fn p_reference_agrees_with_the_contract_for_all_256_inputs() {
    let contract = k_demo::checked_roundtrip_contract();
    let validated = validate(p_demo::p_reference()).expect("p_reference is well formed");
    let vm = PlanVm::stage1();
    for x in 0u8..=255 {
        let observed = vm
            .run(&validated, &[Value::U8(x)])
            .expect("the configuration hosts the plan");
        let expected = eval(&contract, &[Value::U8(x)]).unwrap();
        // Both directions of the frozen relation: the execution equals the
        // embedded contract outcome, and projecting the execution recovers it.
        assert_eq!(observed, embed(expected), "at x = {x}");
        assert_eq!(project(observed), Some(expected), "at x = {x}");
    }
}

#[test]
fn p_specialized_agrees_with_the_contract_for_all_256_inputs() {
    let contract = k_demo::checked_roundtrip_contract();
    let validated = validate(p_demo::p_specialized()).expect("p_specialized is well formed");
    let vm = PlanVm::stage1();
    for x in 0u8..=255 {
        let observed = vm
            .run(&validated, &[Value::U8(x)])
            .expect("the configuration hosts the plan");
        let expected = eval(&contract, &[Value::U8(x)]).unwrap();
        assert_eq!(observed, embed(expected), "at x = {x}");
        assert_eq!(project(observed), Some(expected), "at x = {x}");
    }
}

#[test]
fn p_bad_is_well_typed_executable_and_wrong() {
    let contract = k_demo::checked_roundtrip_contract();
    let validated = validate(p_demo::p_bad()).expect("p_bad is structurally valid");
    let vm = PlanVm::stage1();

    let mut disagreements = Vec::new();
    for x in 0u8..=255 {
        let observed = vm
            .run(&validated, &[Value::U8(x)])
            .expect("p_bad executes normally");
        // It runs. It does not trap, get stuck or diverge: it is a perfectly
        // ordinary execution that returns the wrong thing.
        assert!(
            project(observed).is_some(),
            "p_bad must execute normally at x = {x}, observed {observed:?}"
        );
        let expected = eval(&contract, &[Value::U8(x)]).unwrap();
        if observed != embed(expected) {
            disagreements.push(x);
        }
    }

    assert!(
        !disagreements.is_empty(),
        "p_bad must disagree with the contract somewhere"
    );
    assert_eq!(
        disagreements.first().copied(),
        Some(1),
        "the first disagreement should be at x = 1"
    );
    assert_eq!(disagreements.len(), 255, "p_bad should agree only at x = 0");

    // The shape of the disagreement: the contract raises, the plan returns.
    let observed = vm.run(&validated, &[Value::U8(1)]).unwrap();
    assert_eq!(observed, ObservedOutcome::Return(Value::U8(1)));
    assert_eq!(
        eval(&contract, &[Value::U8(1)]).unwrap(),
        SemanticOutcome::SemanticError(SemanticErrorKind::Overflow)
    );
}

#[test]
fn the_two_valid_plans_are_structurally_different() {
    let reference = p_demo::p_reference();
    let specialized = p_demo::p_specialized();

    assert_ne!(reference.blocks.len(), specialized.blocks.len());

    // The reference performs checked arithmetic and never compares; the
    // specialization compares and performs no arithmetic at all. Instruction
    // *counts* would be a poor test — these two happen to have the same number
    // — so the kinds are what is compared.
    let reference_has_checked = reference
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .any(|i| matches!(i, spl_plan::ir::Instruction::Checked { .. }));
    let specialized_has_checked = specialized
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .any(|i| matches!(i, spl_plan::ir::Instruction::Checked { .. }));
    let specialized_has_compare = specialized
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .any(|i| matches!(i, spl_plan::ir::Instruction::Compare { .. }));
    let reference_has_compare = reference
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .any(|i| matches!(i, spl_plan::ir::Instruction::Compare { .. }));
    assert!(reference_has_checked);
    assert!(!reference_has_compare);
    assert!(!specialized_has_checked);
    assert!(specialized_has_compare);

    // They also use different numbers of registers, so no register-by-register
    // correspondence between them exists.
    let reference_registers = validate(reference.clone()).unwrap().register_types().len();
    let specialized_registers = validate(specialized.clone())
        .unwrap()
        .register_types()
        .len();
    assert_ne!(reference_registers, specialized_registers);

    // And they are different artifacts.
    let reference_id = PlanArtifact::new(reference).content_id().unwrap();
    let specialized_id = PlanArtifact::new(specialized).content_id().unwrap();
    assert_ne!(reference_id, specialized_id);
}

#[test]
fn the_contract_and_its_realizations_are_separate_artifacts() {
    let contract_bytes = KArtifact::new(k_demo::checked_roundtrip_contract())
        .to_canonical_bytes()
        .unwrap();
    for plan in [
        p_demo::p_reference(),
        p_demo::p_specialized(),
        p_demo::p_bad(),
    ] {
        let plan_bytes = PlanArtifact::new(plan).to_canonical_bytes().unwrap();
        assert_ne!(contract_bytes, plan_bytes);
    }
}

#[test]
fn plans_survive_a_round_trip_through_their_artifact_form() {
    // The plan that runs must be the plan that was encoded, which is the
    // Stage-1 shadow of the later "bytes identified == bytes executed" rule.
    let contract = k_demo::checked_roundtrip_contract();
    for plan in [p_demo::p_reference(), p_demo::p_specialized()] {
        let artifact = PlanArtifact::new(plan);
        let bytes = artifact.to_canonical_bytes().unwrap();
        let decoded = PlanArtifact::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(decoded, artifact);
        for x in [0u8, 1, 128, 255] {
            let observed = run_plan(decoded.plan.clone(), x);
            let expected = eval(&contract, &[Value::U8(x)]).unwrap();
            assert_eq!(observed, embed(expected), "at x = {x}");
        }
    }
}

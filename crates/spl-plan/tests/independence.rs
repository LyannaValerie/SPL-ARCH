//! Anti-tautology checks.
//!
//! Stage 1 is only interesting if `K` and `P` are genuinely two things. These
//! checks make that inspectable rather than asserted: the artifacts differ, the
//! artifact schemas refuse each other, and the Plan VM's own source contains no
//! reference to the `K` representation or its evaluator.

use spl_core::artifact::{Artifact, ArtifactError, KArtifact};
use spl_core::demo as k_demo;
use spl_plan::artifact::{PlanArtifact, QArtifact};
use spl_plan::config::PlanVmConfiguration;
use spl_plan::demo as p_demo;

#[test]
fn the_contract_and_the_plan_serialize_to_different_bytes() {
    let contract = KArtifact::new(k_demo::checked_roundtrip_contract())
        .to_canonical_bytes()
        .unwrap();
    let plan = PlanArtifact::new(p_demo::p_reference())
        .to_canonical_bytes()
        .unwrap();
    assert_ne!(contract, plan);
}

#[test]
fn contract_bytes_do_not_decode_as_a_plan_and_the_reverse() {
    let contract_bytes = KArtifact::new(k_demo::checked_roundtrip_contract())
        .to_canonical_bytes()
        .unwrap();
    let plan_bytes = PlanArtifact::new(p_demo::p_reference())
        .to_canonical_bytes()
        .unwrap();

    match PlanArtifact::from_canonical_bytes(&contract_bytes) {
        Err(ArtifactError::SchemaTagMismatch { .. }) => {}
        other => panic!("a contract must not decode as a plan: {other:?}"),
    }
    match KArtifact::from_canonical_bytes(&plan_bytes) {
        Err(ArtifactError::SchemaTagMismatch { .. }) => {}
        other => panic!("a plan must not decode as a contract: {other:?}"),
    }
}

#[test]
fn the_three_artifact_kinds_have_distinct_schema_tags() {
    let tags = [
        KArtifact::SCHEMA_TAG,
        PlanArtifact::SCHEMA_TAG,
        QArtifact::SCHEMA_TAG,
    ];
    let mut sorted = tags.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), tags.len(), "schema tags must be distinct");
}

#[test]
fn q_artifacts_round_trip_and_identify_their_content() {
    let artifact = QArtifact::new(PlanVmConfiguration::stage1_full_core_a());
    let bytes = artifact.to_canonical_bytes().unwrap();
    assert_eq!(QArtifact::from_canonical_bytes(&bytes).unwrap(), artifact);

    let mut other = artifact.clone();
    other.config.step_limit += 1;
    assert_ne!(artifact.content_id().unwrap(), other.content_id().unwrap());
}

#[test]
fn a_q_artifact_with_unnormalized_capability_lists_is_rejected() {
    // Same capabilities written in a different order must not be a second
    // identity for one configuration.
    use spl_core::canonical;
    let body = canonical::array(vec![
        canonical::uint(1),
        canonical::uint(1),
        // types listed out of order
        canonical::array(vec![canonical::uint(1), canonical::uint(0)]),
        canonical::array(vec![canonical::uint(0)]),
        canonical::uint(10),
    ]);
    let envelope = canonical::array(vec![
        canonical::uint(QArtifact::SCHEMA_TAG),
        canonical::uint(QArtifact::SCHEMA_VERSION),
        body,
    ]);
    let bytes = canonical::encode(&envelope).unwrap();
    match QArtifact::from_canonical_bytes(&bytes) {
        Err(ArtifactError::Structure(detail)) => assert!(detail.contains("sorted")),
        other => panic!("expected a normalization rejection, got {other:?}"),
    }
}

/// The Plan VM must not be a `K` interpreter in disguise.
///
/// This reads `spl-plan`'s own sources, strips line comments so that prose
/// about `K` does not count, and asserts that no code in the crate names the
/// `K` representation, the `K` evaluator or the `K` demo. It is a coarse check
/// and it is not a substitute for reading the code — but it does fail loudly if
/// someone later reaches for `eval` to make a plan "work".
#[test]
fn the_plan_crate_does_not_reference_the_k_representation() {
    const FORBIDDEN: [&str; 6] = [
        "KExpr",
        "KTerm",
        "KContract",
        "spl_core::k",
        "spl_core::eval",
        "spl_core::demo",
    ];

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked = 0usize;
    let mut findings = Vec::new();

    let mut stack = vec![src.clone()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("the crate has a src directory") {
            let path = entry.expect("readable directory entry").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable source file");
            let code: String = text
                .lines()
                .map(|line| match line.find("//") {
                    Some(index) => &line[..index],
                    None => line,
                })
                .collect::<Vec<_>>()
                .join("\n");
            for needle in FORBIDDEN {
                if code.contains(needle) {
                    findings.push(format!("{} names `{needle}`", path.display()));
                }
            }
            checked += 1;
        }
    }

    assert!(checked > 0, "no sources were scanned; the check is vacuous");
    assert!(
        findings.is_empty(),
        "the plan crate must not reference K: {findings:?}"
    );
}

/// The `K` crate must not depend on the plan crate either.
///
/// The dependency direction is already one-way in `Cargo.toml`; this states the
/// consequence that matters: the semantic authority does not know how anything
/// is realized.
#[test]
fn the_core_crate_does_not_reference_the_plan_representation() {
    const FORBIDDEN: [&str; 4] = ["spl_plan", "PlanVm", "ValidatedPlan", "Terminator"];

    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("spl-core")
        .join("src");
    let mut checked = 0usize;
    let mut findings = Vec::new();

    for entry in std::fs::read_dir(&src).expect("the core crate has a src directory") {
        let path = entry.expect("readable directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("readable source file");
        let code: String = text
            .lines()
            .map(|line| match line.find("//") {
                Some(index) => &line[..index],
                None => line,
            })
            .collect::<Vec<_>>()
            .join("\n");
        for needle in FORBIDDEN {
            if code.contains(needle) {
                findings.push(format!("{} names `{needle}`", path.display()));
            }
        }
        checked += 1;
    }

    assert!(checked > 0, "no sources were scanned; the check is vacuous");
    assert!(
        findings.is_empty(),
        "the core crate must not reference the plan representation: {findings:?}"
    );
}

/// Neither Stage-1 crate contains unsafe code.
///
/// Both crates carry `#![forbid(unsafe_code)]`, so this is a check that the
/// attribute is still there rather than a second mechanism.
#[test]
fn both_stage1_crates_forbid_unsafe_code() {
    for crate_name in ["spl-core", "spl-plan"] {
        let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(crate_name)
            .join("src")
            .join("lib.rs");
        let text = std::fs::read_to_string(&lib).expect("readable lib.rs");
        assert!(
            text.contains("#![forbid(unsafe_code)]"),
            "{crate_name} must forbid unsafe code"
        );
    }
}

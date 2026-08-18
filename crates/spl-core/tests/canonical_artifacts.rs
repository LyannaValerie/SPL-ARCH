//! The canonical artifact encoding, the canonical decode rule, and identity.

use spl_core::artifact::{Artifact, KArtifact};
use spl_core::canonical::{self, CanonicalError, MAX_ARTIFACT_BYTES};
use spl_core::demo;
use spl_core::id::ContentId;
use spl_core::k::{KContract, KExpr, KTerm};
use spl_core::value::{Type, Value};

fn demo_artifact() -> KArtifact {
    KArtifact::new(demo::checked_roundtrip_contract())
}

#[test]
fn encoding_is_deterministic() {
    let first = demo_artifact().to_canonical_bytes().unwrap();
    for _ in 0..64 {
        assert_eq!(demo_artifact().to_canonical_bytes().unwrap(), first);
    }
}

#[test]
fn round_trip_preserves_the_contract_and_the_bytes() {
    let artifact = demo_artifact();
    let bytes = artifact.to_canonical_bytes().unwrap();
    let decoded = KArtifact::from_canonical_bytes(&bytes).unwrap();
    assert_eq!(decoded, artifact);
    assert_eq!(decoded.to_canonical_bytes().unwrap(), bytes);
}

#[test]
fn identity_is_stable_and_textual_form_is_documented() {
    let id = demo_artifact().content_id().unwrap();
    assert_eq!(id, demo_artifact().content_id().unwrap());
    let text = id.to_text();
    assert!(
        text.starts_with("sha256:"),
        "unexpected identity form: {text}"
    );
    assert_eq!(text.len(), "sha256:".len() + 64);
    assert!(text["sha256:".len()..]
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

#[test]
fn identity_is_over_bytes_and_changes_with_content() {
    let artifact = demo_artifact();
    let mut renamed = artifact.clone();
    renamed.contract.name.push('!');
    assert_ne!(
        artifact.content_id().unwrap(),
        renamed.content_id().unwrap()
    );

    let mut retyped = artifact.clone();
    retyped.contract.body = KTerm::Return(KExpr::Const(Value::U8(0)));
    assert_ne!(
        artifact.content_id().unwrap(),
        retyped.content_id().unwrap()
    );
}

#[test]
fn semantically_equal_but_differently_written_contracts_may_differ_in_identity() {
    // Identity names exact content, never a semantic equivalence class. Both of
    // these contracts return 0 for every input; their identities differ, and
    // that is correct behaviour, not a defect.
    let direct = KArtifact::new(KContract {
        name: "zero".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        body: KTerm::Return(KExpr::Const(Value::U8(0))),
    });
    let roundabout = KArtifact::new(KContract {
        name: "zero".to_owned(),
        params: vec![Type::U8],
        result: Type::U8,
        body: KTerm::Return(KExpr::Wrapping {
            op: spl_core::prims::ArithOp::Mul,
            lhs: Box::new(KExpr::Input(spl_core::k::InputIndex(0))),
            rhs: Box::new(KExpr::Const(Value::U8(0))),
        }),
    });
    assert_ne!(
        direct.content_id().unwrap(),
        roundabout.content_id().unwrap()
    );
}

#[test]
fn identical_bytes_give_identical_identity() {
    let bytes = demo_artifact().to_canonical_bytes().unwrap();
    assert_eq!(ContentId::of_bytes(&bytes), ContentId::of_bytes(&bytes));
    let copy = bytes.clone();
    assert_eq!(ContentId::of_bytes(&bytes), ContentId::of_bytes(&copy));
}

#[test]
fn the_encoder_emits_shortest_form_integers() {
    canonical::assert_shortest_form_integers().expect("shortest-form integer heads");
}

#[test]
fn a_non_shortest_integer_head_is_rejected() {
    // 5 encoded as a one-byte-argument uint instead of the shortest form.
    let non_canonical = [0x81u8, 0x18, 0x05];
    match canonical::decode(&non_canonical) {
        Err(CanonicalError::NotCanonical { .. }) => {}
        other => panic!("expected a canonicality rejection, got {other:?}"),
    }
    // The shortest form of the same value is accepted.
    canonical::decode(&[0x81, 0x05]).expect("shortest form is canonical");
}

#[test]
fn an_indefinite_length_array_is_rejected() {
    let indefinite = [0x9fu8, 0x01, 0x02, 0xff];
    match canonical::decode(&indefinite) {
        Err(CanonicalError::NotCanonical { .. }) => {}
        other => panic!("expected a canonicality rejection, got {other:?}"),
    }
}

#[test]
fn trailing_bytes_after_a_complete_item_are_rejected() {
    let mut bytes = demo_artifact().to_canonical_bytes().unwrap();
    bytes.push(0x00);
    match KArtifact::from_canonical_bytes(&bytes) {
        Err(spl_core::artifact::ArtifactError::Canonical(CanonicalError::NotCanonical {
            ..
        })) => {}
        other => panic!("expected a canonicality rejection, got {other:?}"),
    }
}

#[test]
fn maps_floats_tags_and_null_are_outside_the_profile() {
    // { } , 0.0 , tag(0, 0) , null
    for bytes in [
        vec![0xa0u8],
        vec![0xf9, 0x00, 0x00],
        vec![0xc0, 0x00],
        vec![0xf6],
    ] {
        match canonical::decode(&bytes) {
            Err(CanonicalError::ForbiddenItem { .. }) => {}
            other => panic!("expected a profile rejection for {bytes:02x?}, got {other:?}"),
        }
    }
}

#[test]
fn oversized_input_is_refused_before_parsing() {
    let bytes = vec![0u8; MAX_ARTIFACT_BYTES + 1];
    match canonical::decode(&bytes) {
        Err(CanonicalError::TooLarge { .. }) => {}
        other => panic!("expected a size rejection, got {other:?}"),
    }
}

#[test]
fn nesting_is_bounded() {
    // Deeply nested definite-length arrays. The decoder must refuse rather than
    // exhaust the host stack; this pins the behaviour that makes the size bound
    // above sufficient.
    let mut bytes = vec![0x81u8; 10_000];
    bytes.push(0x00);
    assert!(
        canonical::decode(&bytes).is_err(),
        "deep nesting must be rejected"
    );
    // A shallow nest of the same shape is fine, so the rejection above is about
    // depth and not about the shape being unsupported.
    canonical::decode(&[0x81, 0x81, 0x81, 0x00]).expect("shallow nesting is fine");
}

#[test]
fn a_foreign_schema_tag_is_rejected() {
    // A well-formed canonical envelope that claims to be a plan artifact.
    let foreign = canonical::array(vec![
        canonical::uint(spl_core::artifact::SCHEMA_TAG_PLAN),
        canonical::uint(1),
        canonical::array(vec![]),
    ]);
    let bytes = canonical::encode(&foreign).unwrap();
    match KArtifact::from_canonical_bytes(&bytes) {
        Err(spl_core::artifact::ArtifactError::SchemaTagMismatch { .. }) => {}
        other => panic!("expected a schema tag rejection, got {other:?}"),
    }
}

#[test]
fn an_unknown_schema_version_is_rejected() {
    let future = canonical::array(vec![
        canonical::uint(spl_core::artifact::SCHEMA_TAG_K),
        canonical::uint(99),
        canonical::array(vec![]),
    ]);
    let bytes = canonical::encode(&future).unwrap();
    match KArtifact::from_canonical_bytes(&bytes) {
        Err(spl_core::artifact::ArtifactError::SchemaVersionMismatch { .. }) => {}
        other => panic!("expected a schema version rejection, got {other:?}"),
    }
}

#[test]
fn out_of_range_payloads_are_rejected_rather_than_truncated() {
    // A u8 literal whose payload is 300.
    let body = canonical::array(vec![
        canonical::text("bad"),
        canonical::array(vec![]),
        canonical::uint(1),
        canonical::array(vec![
            canonical::uint(0),
            canonical::array(vec![
                canonical::uint(1),
                canonical::array(vec![canonical::uint(1), canonical::uint(300)]),
            ]),
        ]),
    ]);
    let envelope = canonical::array(vec![
        canonical::uint(spl_core::artifact::SCHEMA_TAG_K),
        canonical::uint(1),
        body,
    ]);
    let bytes = canonical::encode(&envelope).unwrap();
    match KArtifact::from_canonical_bytes(&bytes) {
        Err(spl_core::artifact::ArtifactError::Structure(detail)) => {
            assert!(detail.contains("300"), "unexpected detail: {detail}");
        }
        other => panic!("expected a structural rejection, got {other:?}"),
    }
}

#[test]
fn an_unknown_node_tag_is_rejected() {
    let body = canonical::array(vec![
        canonical::text("bad"),
        canonical::array(vec![]),
        canonical::uint(1),
        canonical::array(vec![canonical::uint(77), canonical::uint(0)]),
    ]);
    let envelope = canonical::array(vec![
        canonical::uint(spl_core::artifact::SCHEMA_TAG_K),
        canonical::uint(1),
        body,
    ]);
    let bytes = canonical::encode(&envelope).unwrap();
    match KArtifact::from_canonical_bytes(&bytes) {
        Err(spl_core::artifact::ArtifactError::UnknownTag { position, tag }) => {
            assert_eq!(position, "term");
            assert_eq!(tag, 77);
        }
        other => panic!("expected an unknown tag rejection, got {other:?}"),
    }
}

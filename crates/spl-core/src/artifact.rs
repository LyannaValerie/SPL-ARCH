//! Artifacts: versioned, canonically encoded, content-identified.
//!
//! An artifact is the standalone form of an object. The binding invariant the
//! plan states is
//!
//! ```text
//! bytes identified == bytes semantically decoded
//! ```
//!
//! and it is realized here by two rules together: every artifact is wrapped in
//! a `[schema_tag, schema_version, body]` envelope so that a `K` artifact can
//! never be mistaken for a `P` artifact even if their bodies coincided, and
//! decoding goes through [`crate::canonical::decode`], which rejects bytes that
//! are decodable but not canonical.

use crate::canonical::{self, CanonicalError};
use crate::id::ContentId;
use crate::k::{InputIndex, KContract, KExpr, KTerm};
use crate::outcome::SemanticErrorKind;
use crate::prims::{ArithOp, CompareOp};
use crate::value::{Type, Value};
use ciborium::value::Value as CborValue;

/// Schema tag of a `K` semantic-contract artifact.
pub const SCHEMA_TAG_K: u64 = 1;
/// Schema tag of a `P` execution-plan artifact. Defined here so that the tag
/// space stays visible in one place; the encoding itself lives in `spl-plan`.
pub const SCHEMA_TAG_PLAN: u64 = 2;
/// Schema tag of a `Q` execution-configuration artifact.
pub const SCHEMA_TAG_Q: u64 = 3;

/// Why bytes are not the artifact they claim to be.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArtifactError {
    /// The bytes are not a canonical encoding at all.
    Canonical(CanonicalError),
    /// The envelope names a different kind of artifact.
    SchemaTagMismatch { expected: u64, found: u64 },
    /// The envelope names a schema version this build does not implement.
    SchemaVersionMismatch { expected: u64, found: u64 },
    /// The body does not have the shape this schema requires.
    Structure(String),
    /// A discriminant is outside the set this schema defines.
    UnknownTag { position: &'static str, tag: u64 },
}

impl From<CanonicalError> for ArtifactError {
    fn from(err: CanonicalError) -> Self {
        ArtifactError::Canonical(err)
    }
}

impl core::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArtifactError::Canonical(err) => write!(f, "{err}"),
            ArtifactError::SchemaTagMismatch { expected, found } => {
                write!(f, "expected schema tag {expected}, found {found}")
            }
            ArtifactError::SchemaVersionMismatch { expected, found } => {
                write!(f, "expected schema version {expected}, found {found}")
            }
            ArtifactError::Structure(detail) => write!(f, "malformed artifact body: {detail}"),
            ArtifactError::UnknownTag { position, tag } => {
                write!(f, "unknown {position} tag {tag}")
            }
        }
    }
}

fn structure(detail: impl Into<String>) -> ArtifactError {
    ArtifactError::Structure(detail.into())
}

/// Read a body item as an array of exactly `n` items.
pub fn expect_array<'a>(
    value: &'a CborValue,
    n: usize,
    what: &str,
) -> Result<&'a [CborValue], ArtifactError> {
    let items =
        canonical::as_array(value).ok_or_else(|| structure(format!("{what} must be an array")))?;
    if items.len() != n {
        return Err(structure(format!(
            "{what} must have {n} field(s), found {}",
            items.len()
        )));
    }
    Ok(items)
}

/// Read a body item as an array of any length.
pub fn expect_list<'a>(value: &'a CborValue, what: &str) -> Result<&'a [CborValue], ArtifactError> {
    canonical::as_array(value).ok_or_else(|| structure(format!("{what} must be an array")))
}

/// Read a body item as an unsigned integer.
pub fn expect_uint(value: &CborValue, what: &str) -> Result<u64, ArtifactError> {
    canonical::as_uint(value)
        .ok_or_else(|| structure(format!("{what} must be an unsigned integer")))
}

/// Read a body item as text.
pub fn expect_text(value: &CborValue, what: &str) -> Result<String, ArtifactError> {
    canonical::as_text(value)
        .map(str::to_owned)
        .ok_or_else(|| structure(format!("{what} must be text")))
}

/// The common artifact behaviour: envelope, canonical bytes, identity.
pub trait Artifact: Sized {
    /// Which kind of artifact this is.
    const SCHEMA_TAG: u64;
    /// Which revision of that kind this build implements.
    const SCHEMA_VERSION: u64;

    /// Encode the artifact-specific body.
    fn to_body(&self) -> CborValue;

    /// Strictly decode the artifact-specific body.
    fn from_body(body: &CborValue) -> Result<Self, ArtifactError>;

    /// The exact canonical bytes of this artifact.
    fn to_canonical_bytes(&self) -> Result<Vec<u8>, ArtifactError> {
        let envelope = canonical::array(vec![
            canonical::uint(Self::SCHEMA_TAG),
            canonical::uint(Self::SCHEMA_VERSION),
            self.to_body(),
        ]);
        Ok(canonical::encode(&envelope)?)
    }

    /// Decode, rejecting non-canonical bytes and foreign envelopes.
    fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, ArtifactError> {
        let value = canonical::decode(bytes)?;
        let items = expect_array(&value, 3, "artifact envelope")?;
        let tag = expect_uint(&items[0], "schema tag")?;
        if tag != Self::SCHEMA_TAG {
            return Err(ArtifactError::SchemaTagMismatch {
                expected: Self::SCHEMA_TAG,
                found: tag,
            });
        }
        let version = expect_uint(&items[1], "schema version")?;
        if version != Self::SCHEMA_VERSION {
            return Err(ArtifactError::SchemaVersionMismatch {
                expected: Self::SCHEMA_VERSION,
                found: version,
            });
        }
        Self::from_body(&items[2])
    }

    /// The identity of this artifact's exact canonical bytes.
    fn content_id(&self) -> Result<ContentId, ArtifactError> {
        Ok(ContentId::of_bytes(&self.to_canonical_bytes()?))
    }
}

// ---------------------------------------------------------------------------
// Shared encodings for the Core A vocabulary.
//
// `spl-plan` reuses these. Sharing the *encoding of a u8 literal* is sharing a
// primitive type definition, which the plan allows; it is not sharing a
// representation of K, and the Plan IR encoding below is built independently
// from these leaves.
// ---------------------------------------------------------------------------

/// Encode a Core A type as its discriminant.
pub fn encode_type(ty: Type) -> CborValue {
    canonical::uint(match ty {
        Type::Bool => 0,
        Type::U8 => 1,
        Type::U32 => 2,
        Type::I32 => 3,
    })
}

/// Decode a Core A type discriminant.
pub fn decode_type(value: &CborValue) -> Result<Type, ArtifactError> {
    match expect_uint(value, "type")? {
        0 => Ok(Type::Bool),
        1 => Ok(Type::U8),
        2 => Ok(Type::U32),
        3 => Ok(Type::I32),
        tag => Err(ArtifactError::UnknownTag {
            position: "type",
            tag,
        }),
    }
}

/// Encode a Core A value as `[type, payload]`.
pub fn encode_value(value: Value) -> CborValue {
    let payload = match value {
        Value::Bool(v) => canonical::uint(u64::from(v)),
        Value::U8(v) => canonical::uint(u64::from(v)),
        Value::U32(v) => canonical::uint(u64::from(v)),
        Value::I32(v) => canonical::int(i64::from(v)),
    };
    canonical::array(vec![encode_type(value.ty()), payload])
}

/// Strictly decode a Core A value.
///
/// Payloads are range-checked against the declared type, so `[u8, 300]` is
/// rejected rather than truncated.
pub fn decode_value(value: &CborValue) -> Result<Value, ArtifactError> {
    let items = expect_array(value, 2, "value")?;
    let ty = decode_type(&items[0])?;
    match ty {
        Type::Bool => match expect_uint(&items[1], "Bool payload")? {
            0 => Ok(Value::Bool(false)),
            1 => Ok(Value::Bool(true)),
            other => Err(structure(format!(
                "Bool payload must be 0 or 1, found {other}"
            ))),
        },
        Type::U8 => {
            let raw = expect_uint(&items[1], "u8 payload")?;
            u8::try_from(raw)
                .map(Value::U8)
                .map_err(|_| structure(format!("u8 payload {raw} is out of range")))
        }
        Type::U32 => {
            let raw = expect_uint(&items[1], "u32 payload")?;
            u32::try_from(raw)
                .map(Value::U32)
                .map_err(|_| structure(format!("u32 payload {raw} is out of range")))
        }
        Type::I32 => {
            let raw = canonical::as_int(&items[1])
                .ok_or_else(|| structure("i32 payload must be an integer"))?;
            i32::try_from(raw)
                .map(Value::I32)
                .map_err(|_| structure(format!("i32 payload {raw} is out of range")))
        }
    }
}

/// Encode an arithmetic operator discriminant.
pub fn encode_arith_op(op: ArithOp) -> CborValue {
    canonical::uint(match op {
        ArithOp::Add => 0,
        ArithOp::Sub => 1,
        ArithOp::Mul => 2,
    })
}

/// Decode an arithmetic operator discriminant.
pub fn decode_arith_op(value: &CborValue) -> Result<ArithOp, ArtifactError> {
    match expect_uint(value, "arithmetic operator")? {
        0 => Ok(ArithOp::Add),
        1 => Ok(ArithOp::Sub),
        2 => Ok(ArithOp::Mul),
        tag => Err(ArtifactError::UnknownTag {
            position: "arithmetic operator",
            tag,
        }),
    }
}

/// Encode a comparison operator discriminant.
pub fn encode_compare_op(op: CompareOp) -> CborValue {
    canonical::uint(match op {
        CompareOp::Eq => 0,
        CompareOp::Ne => 1,
        CompareOp::Lt => 2,
        CompareOp::Le => 3,
        CompareOp::Gt => 4,
        CompareOp::Ge => 5,
    })
}

/// Decode a comparison operator discriminant.
pub fn decode_compare_op(value: &CborValue) -> Result<CompareOp, ArtifactError> {
    match expect_uint(value, "comparison operator")? {
        0 => Ok(CompareOp::Eq),
        1 => Ok(CompareOp::Ne),
        2 => Ok(CompareOp::Lt),
        3 => Ok(CompareOp::Le),
        4 => Ok(CompareOp::Gt),
        5 => Ok(CompareOp::Ge),
        tag => Err(ArtifactError::UnknownTag {
            position: "comparison operator",
            tag,
        }),
    }
}

/// Encode a semantic-error discriminant.
pub fn encode_semantic_error(kind: SemanticErrorKind) -> CborValue {
    canonical::uint(match kind {
        SemanticErrorKind::Overflow => 0,
    })
}

/// Decode a semantic-error discriminant.
pub fn decode_semantic_error(value: &CborValue) -> Result<SemanticErrorKind, ArtifactError> {
    match expect_uint(value, "semantic error")? {
        0 => Ok(SemanticErrorKind::Overflow),
        tag => Err(ArtifactError::UnknownTag {
            position: "semantic error",
            tag,
        }),
    }
}

// ---------------------------------------------------------------------------
// The K artifact.
// ---------------------------------------------------------------------------

/// The standalone artifact form of a semantic contract.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct KArtifact {
    pub contract: KContract,
}

impl KArtifact {
    pub fn new(contract: KContract) -> Self {
        KArtifact { contract }
    }
}

fn encode_expr(expr: &KExpr) -> CborValue {
    match expr {
        KExpr::Input(InputIndex(index)) => {
            canonical::array(vec![canonical::uint(0), canonical::uint(u64::from(*index))])
        }
        KExpr::Const(value) => canonical::array(vec![canonical::uint(1), encode_value(*value)]),
        KExpr::If {
            cond,
            then_expr,
            else_expr,
        } => canonical::array(vec![
            canonical::uint(2),
            encode_expr(cond),
            encode_expr(then_expr),
            encode_expr(else_expr),
        ]),
        KExpr::Compare { op, lhs, rhs } => canonical::array(vec![
            canonical::uint(3),
            encode_compare_op(*op),
            encode_expr(lhs),
            encode_expr(rhs),
        ]),
        KExpr::Wrapping { op, lhs, rhs } => canonical::array(vec![
            canonical::uint(4),
            encode_arith_op(*op),
            encode_expr(lhs),
            encode_expr(rhs),
        ]),
        KExpr::Checked { op, lhs, rhs } => canonical::array(vec![
            canonical::uint(5),
            encode_arith_op(*op),
            encode_expr(lhs),
            encode_expr(rhs),
        ]),
    }
}

fn decode_expr(value: &CborValue) -> Result<KExpr, ArtifactError> {
    let items = expect_list(value, "expression")?;
    let tag = items
        .first()
        .ok_or_else(|| structure("expression must not be empty"))
        .and_then(|item| expect_uint(item, "expression tag"))?;
    match (tag, items.len()) {
        (0, 2) => {
            let raw = expect_uint(&items[1], "input index")?;
            let index = u32::try_from(raw)
                .map_err(|_| structure(format!("input index {raw} is out of range")))?;
            Ok(KExpr::Input(InputIndex(index)))
        }
        (1, 2) => Ok(KExpr::Const(decode_value(&items[1])?)),
        (2, 4) => Ok(KExpr::If {
            cond: Box::new(decode_expr(&items[1])?),
            then_expr: Box::new(decode_expr(&items[2])?),
            else_expr: Box::new(decode_expr(&items[3])?),
        }),
        (3, 4) => Ok(KExpr::Compare {
            op: decode_compare_op(&items[1])?,
            lhs: Box::new(decode_expr(&items[2])?),
            rhs: Box::new(decode_expr(&items[3])?),
        }),
        (4, 4) => Ok(KExpr::Wrapping {
            op: decode_arith_op(&items[1])?,
            lhs: Box::new(decode_expr(&items[2])?),
            rhs: Box::new(decode_expr(&items[3])?),
        }),
        (5, 4) => Ok(KExpr::Checked {
            op: decode_arith_op(&items[1])?,
            lhs: Box::new(decode_expr(&items[2])?),
            rhs: Box::new(decode_expr(&items[3])?),
        }),
        (0..=5, len) => Err(structure(format!(
            "expression tag {tag} does not take {len} field(s)"
        ))),
        _ => Err(ArtifactError::UnknownTag {
            position: "expression",
            tag,
        }),
    }
}

fn encode_term(term: &KTerm) -> CborValue {
    match term {
        KTerm::Return(expr) => canonical::array(vec![canonical::uint(0), encode_expr(expr)]),
        KTerm::SemanticError(kind) => {
            canonical::array(vec![canonical::uint(1), encode_semantic_error(*kind)])
        }
        KTerm::If {
            cond,
            then_term,
            else_term,
        } => canonical::array(vec![
            canonical::uint(2),
            encode_expr(cond),
            encode_term(then_term),
            encode_term(else_term),
        ]),
    }
}

fn decode_term(value: &CborValue) -> Result<KTerm, ArtifactError> {
    let items = expect_list(value, "term")?;
    let tag = items
        .first()
        .ok_or_else(|| structure("term must not be empty"))
        .and_then(|item| expect_uint(item, "term tag"))?;
    match (tag, items.len()) {
        (0, 2) => Ok(KTerm::Return(decode_expr(&items[1])?)),
        (1, 2) => Ok(KTerm::SemanticError(decode_semantic_error(&items[1])?)),
        (2, 4) => Ok(KTerm::If {
            cond: decode_expr(&items[1])?,
            then_term: Box::new(decode_term(&items[2])?),
            else_term: Box::new(decode_term(&items[3])?),
        }),
        (0..=2, len) => Err(structure(format!(
            "term tag {tag} does not take {len} field(s)"
        ))),
        _ => Err(ArtifactError::UnknownTag {
            position: "term",
            tag,
        }),
    }
}

impl Artifact for KArtifact {
    const SCHEMA_TAG: u64 = SCHEMA_TAG_K;
    const SCHEMA_VERSION: u64 = 1;

    fn to_body(&self) -> CborValue {
        canonical::array(vec![
            canonical::text(&self.contract.name),
            canonical::array(
                self.contract
                    .params
                    .iter()
                    .copied()
                    .map(encode_type)
                    .collect(),
            ),
            encode_type(self.contract.result),
            encode_term(&self.contract.body),
        ])
    }

    fn from_body(body: &CborValue) -> Result<Self, ArtifactError> {
        let items = expect_array(body, 4, "K contract")?;
        let name = expect_text(&items[0], "contract name")?;
        let params = expect_list(&items[1], "parameter list")?
            .iter()
            .map(decode_type)
            .collect::<Result<Vec<_>, _>>()?;
        let result = decode_type(&items[2])?;
        let body = decode_term(&items[3])?;
        Ok(KArtifact {
            contract: KContract {
                name,
                params,
                result,
                body,
            },
        })
    }
}

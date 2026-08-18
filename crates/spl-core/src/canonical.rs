//! The SPL-0 canonical artifact encoding.
//!
//! # Why this module exists
//!
//! The implementation plan names deterministic CBOR as the leading candidate
//! and deliberately does not freeze a Rust library. Before choosing one, the
//! two candidates were checked against their own documentation and then
//! measured. The finding that shaped this module:
//!
//! * Neither `ciborium` 0.2.2 nor `minicbor` 2.3.0 documents a canonical or
//!   deterministic encoding guarantee.
//! * Both *encoders* do emit shortest-form integer heads and definite-length
//!   arrays.
//! * Both *decoders* accept non-canonical input: a non-shortest integer head
//!   (`0x18 0x05` for 5) decodes happily, `ciborium` accepts indefinite-length
//!   arrays, and both accept trailing bytes after a complete item.
//!
//! So `serde + CBOR` is not canonical, exactly as the plan warned. The library
//! is used for the wire format; canonicality is enforced here.
//!
//! # The SPL-0 canonical profile
//!
//! 1. Every artifact is a definite-length CBOR **array**. Positional fields
//!    only. Maps are forbidden outright, which removes the key-ordering
//!    question rather than answering it. (`ciborium` does ship a
//!    `CanonicalValue` ordering helper for maps; the profile does not need it.)
//! 2. The only permitted item kinds are: unsigned integer, negative integer,
//!    byte string, text string, boolean, array. Floats, tags, `null`,
//!    `undefined`, maps and indefinite-length items are rejected on both encode
//!    and decode.
//! 3. Integers are shortest-form, which the encoder produces and
//!    [`assert_shortest_form_integers`] pins with a test.
//! 4. Untrusted input is bounded: at most [`MAX_ARTIFACT_BYTES`] bytes, and
//!    nesting is bounded by the decoder (see the `nesting_is_bounded` test).
//!
//! # The canonical decode rule
//!
//! Profile conformance is not established by trusting the library. It is
//! established by round-tripping:
//!
//! ```text
//! decode(bytes) -> value
//! encode(value) -> canonical_bytes
//! bytes == canonical_bytes   required
//! ```
//!
//! Anything decodable but not canonical is rejected as an artifact. This single
//! rule subsumes all three permissive behaviours measured above: a non-shortest
//! integer, an indefinite-length array and a trailing byte each re-encode to
//! something different from the input.

use ciborium::value::{Integer, Value as CborValue};

/// The largest artifact this layer will parse.
///
/// Stage-1 artifacts are small. The bound exists so that parsing untrusted
/// bytes has an explicit budget rather than an implicit one.
pub const MAX_ARTIFACT_BYTES: usize = 65_536;

/// Why bytes are not a canonical SPL-0 artifact encoding.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CanonicalError {
    /// The input exceeds [`MAX_ARTIFACT_BYTES`].
    TooLarge { len: usize, limit: usize },
    /// The bytes are not decodable CBOR at all, or exceed the decoder's own
    /// structural limits.
    Malformed(String),
    /// The value contains an item kind the profile forbids.
    ForbiddenItem { kind: &'static str },
    /// The bytes decode, but re-encoding the decoded value does not reproduce
    /// them. The input is CBOR, but not *canonical* CBOR under this profile.
    NotCanonical {
        input_len: usize,
        canonical_len: usize,
    },
    /// The encoder failed. This is an internal fault, not an input problem.
    EncodeFailed(String),
}

impl core::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CanonicalError::TooLarge { len, limit } => {
                write!(f, "artifact is {len} bytes, limit is {limit}")
            }
            CanonicalError::Malformed(detail) => write!(f, "malformed CBOR: {detail}"),
            CanonicalError::ForbiddenItem { kind } => {
                write!(f, "the canonical profile forbids CBOR {kind} items")
            }
            CanonicalError::NotCanonical {
                input_len,
                canonical_len,
            } => write!(
                f,
                "decodable but not canonical: input is {input_len} bytes, \
                 the canonical encoding of the same value is {canonical_len}"
            ),
            CanonicalError::EncodeFailed(detail) => write!(f, "encoding failed: {detail}"),
        }
    }
}

/// Reject any item the profile does not permit.
fn check_profile(value: &CborValue) -> Result<(), CanonicalError> {
    match value {
        CborValue::Integer(_) | CborValue::Bytes(_) | CborValue::Text(_) | CborValue::Bool(_) => {
            Ok(())
        }
        CborValue::Array(items) => items.iter().try_for_each(check_profile),
        CborValue::Float(_) => Err(CanonicalError::ForbiddenItem { kind: "float" }),
        CborValue::Null => Err(CanonicalError::ForbiddenItem { kind: "null" }),
        CborValue::Tag(_, _) => Err(CanonicalError::ForbiddenItem { kind: "tag" }),
        CborValue::Map(_) => Err(CanonicalError::ForbiddenItem { kind: "map" }),
        // `ciborium::value::Value` is `#[non_exhaustive]`. Anything it grows
        // later is outside the profile until this module says otherwise.
        _ => Err(CanonicalError::ForbiddenItem {
            kind: "unsupported",
        }),
    }
}

/// Encode a profile-conforming value to its canonical bytes.
pub fn encode(value: &CborValue) -> Result<Vec<u8>, CanonicalError> {
    check_profile(value)?;
    let mut bytes = Vec::new();
    ciborium::into_writer(value, &mut bytes)
        .map_err(|err| CanonicalError::EncodeFailed(err.to_string()))?;
    Ok(bytes)
}

/// Decode bytes, enforcing the canonical decode rule.
///
/// Succeeds only for input that is byte-identical to the canonical encoding of
/// the value it denotes.
pub fn decode(bytes: &[u8]) -> Result<CborValue, CanonicalError> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(CanonicalError::TooLarge {
            len: bytes.len(),
            limit: MAX_ARTIFACT_BYTES,
        });
    }
    let value: CborValue =
        ciborium::from_reader(bytes).map_err(|err| CanonicalError::Malformed(err.to_string()))?;
    check_profile(&value)?;
    let canonical = encode(&value)?;
    if canonical.as_slice() != bytes {
        return Err(CanonicalError::NotCanonical {
            input_len: bytes.len(),
            canonical_len: canonical.len(),
        });
    }
    Ok(value)
}

/// Build a CBOR unsigned integer item.
pub fn uint(value: u64) -> CborValue {
    CborValue::Integer(Integer::from(value))
}

/// Build a CBOR integer item from a signed value.
pub fn int(value: i64) -> CborValue {
    CborValue::Integer(Integer::from(value))
}

/// Build a definite-length CBOR array item.
pub fn array(items: Vec<CborValue>) -> CborValue {
    CborValue::Array(items)
}

/// Build a CBOR text item.
pub fn text(value: &str) -> CborValue {
    CborValue::Text(value.to_owned())
}

/// Read a CBOR item as an unsigned integer within `u64`.
pub fn as_uint(value: &CborValue) -> Option<u64> {
    match value {
        CborValue::Integer(integer) => u64::try_from(i128::from(*integer)).ok(),
        _ => None,
    }
}

/// Read a CBOR item as a signed integer within `i64`.
pub fn as_int(value: &CborValue) -> Option<i64> {
    match value {
        CborValue::Integer(integer) => i64::try_from(i128::from(*integer)).ok(),
        _ => None,
    }
}

/// Read a CBOR item as an array.
pub fn as_array(value: &CborValue) -> Option<&[CborValue]> {
    match value {
        CborValue::Array(items) => Some(items),
        _ => None,
    }
}

/// Read a CBOR item as text.
pub fn as_text(value: &CborValue) -> Option<&str> {
    match value {
        CborValue::Text(text) => Some(text),
        _ => None,
    }
}

/// Read a CBOR item as a boolean.
pub fn as_bool(value: &CborValue) -> Option<bool> {
    match value {
        CborValue::Bool(value) => Some(*value),
        _ => None,
    }
}

/// Pin the encoder's shortest-form integer behaviour.
///
/// The canonical decode rule makes the round trip authoritative, so this is a
/// regression guard rather than the mechanism: if a future library version
/// stopped emitting shortest-form heads, every previously stored artifact would
/// start failing the round trip, and this function localizes that failure.
pub fn assert_shortest_form_integers() -> Result<(), CanonicalError> {
    let expected: [(u64, &[u8]); 6] = [
        (0, &[0x00]),
        (23, &[0x17]),
        (24, &[0x18, 0x18]),
        (255, &[0x18, 0xff]),
        (256, &[0x19, 0x01, 0x00]),
        (65_536, &[0x1a, 0x00, 0x01, 0x00, 0x00]),
    ];
    for (value, bytes) in expected {
        let encoded = encode(&uint(value))?;
        if encoded != bytes {
            return Err(CanonicalError::EncodeFailed(format!(
                "integer {value} encoded as {encoded:02x?}, expected {bytes:02x?}"
            )));
        }
    }
    Ok(())
}

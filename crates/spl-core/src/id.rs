//! Content identity for artifacts.
//!
//! An identity names *exact canonical bytes*. It is not a semantic equivalence
//! class: two contracts that mean the same thing but are represented
//! differently have different identities, and that is the intended behaviour.
//! The frozen theory says the same thing about `K_ref` — semantic equivalence of
//! two independently encoded contracts does not imply identical references.

use sha2::{Digest, Sha256};

/// The digest algorithm. Recorded in the textual form so that the identity
/// string stays unambiguous if the algorithm is ever changed.
pub const DIGEST_ALGORITHM: &str = "sha256";

/// A content identity: the SHA-256 digest of exact canonical artifact bytes.
///
/// Textual form: `sha256:` followed by 64 lowercase hex characters.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentId([u8; 32]);

impl ContentId {
    /// Compute the identity of these exact bytes.
    ///
    /// Takes bytes, never a value: computing an identity from a value would
    /// require re-encoding it, and then the identity would name the encoder's
    /// current behaviour rather than the bytes that were actually stored.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        ContentId(out)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The canonical textual form, `sha256:<64 lowercase hex>`.
    pub fn to_text(self) -> String {
        let mut text = String::with_capacity(DIGEST_ALGORITHM.len() + 1 + 64);
        text.push_str(DIGEST_ALGORITHM);
        text.push(':');
        for byte in self.0 {
            text.push_str(&format!("{byte:02x}"));
        }
        text
    }
}

impl core::fmt::Display for ContentId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.to_text())
    }
}

impl core::fmt::Debug for ContentId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ContentId({})", self.to_text())
    }
}

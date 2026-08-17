//! SHA-256 helpers used for commitments and the settlement hash chain.

use sha2::{Digest, Sha256};

use crate::encode::Encoder;
use crate::types::Hash;

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> Hash {
    let digest = Sha256::digest(data);
    Hash::from_bytes(digest.into())
}

/// Hash `length_prefixed(tag) || body`.
pub fn hash_tagged(tag: &[u8], write_body: impl FnOnce(&mut Encoder)) -> Hash {
    let mut encoder = Encoder::new();
    encoder.bytes(tag);
    write_body(&mut encoder);
    sha256(encoder.as_slice())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::{TAG_COMMIT, TAG_REVEAL};

    #[test]
    fn domain_separation_changes_digest() {
        let a = hash_tagged(TAG_COMMIT, |e| {
            e.u8(1);
        });
        let b = hash_tagged(TAG_REVEAL, |e| {
            e.u8(1);
        });
        assert_ne!(a, b);
    }

    #[test]
    fn same_tag_and_body_are_stable() {
        let a = hash_tagged(TAG_COMMIT, |e| {
            e.u64(42);
            e.bytes(b"salt");
        });
        let b = hash_tagged(TAG_COMMIT, |e| {
            e.u64(42);
            e.bytes(b"salt");
        });
        assert_eq!(a, b);
    }
}

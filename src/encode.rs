//! Deterministic length-prefixed encoding for anything that is hashed.
//!
//! Serde / JSON is forbidden on this path: the encoding must not depend on
//! rustc, field order of derive output, or floating point.

/// Domain tags. Changing a tag is a consensus break.
pub const TAG_COMMIT: &[u8] = b"futarch/commit/v1";
pub const TAG_REVEAL: &[u8] = b"futarch/reveal/v1";
pub const TAG_PROPOSAL: &[u8] = b"futarch/proposal/v1";
pub const TAG_GENESIS: &[u8] = b"futarch/genesis/v1";
pub const TAG_SETTLEMENT: &[u8] = b"futarch/settlement/v1";

/// Growing buffer written in big-endian, length-prefixed form.
#[derive(Clone, Debug, Default)]
pub struct Encoder {
    buf: Vec<u8>,
}

impl Encoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.buf.push(v);
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn bytes32(&mut self, v: &[u8; 32]) -> &mut Self {
        self.buf.extend_from_slice(v);
        self
    }

    /// `u32` big-endian length followed by the bytes.
    pub fn bytes(&mut self, v: &[u8]) -> &mut Self {
        self.u32(v.len() as u32);
        self.buf.extend_from_slice(v);
        self
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_prefix_is_big_endian() {
        let mut e = Encoder::new();
        e.bytes(b"ab");
        assert_eq!(e.as_slice(), &[0, 0, 0, 2, b'a', b'b']);
    }

    #[test]
    fn u64_is_big_endian() {
        let mut e = Encoder::new();
        e.u64(0x0102_0304_0506_0708);
        assert_eq!(
            e.as_slice(),
            &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }
}

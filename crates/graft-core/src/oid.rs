//! Git object identity.
//!
//! `git` produces these, `graph` lays them out, `ops` journals them and `ui`
//! displays them — four crates that may not depend on each other, which is why
//! the shape lives here (DESIGN.md §8.2).

use std::fmt;

use crate::error::Error;

/// A git object id: the raw hash bytes, exactly as the repository stores them.
///
/// Twenty bytes in a SHA-1 repository, thirty-two in a repository created with
/// `--object-format=sha256`. Both fit inline, so an `Oid` is `Copy` and a
/// layout pass can move millions of them without touching the allocator.
///
/// Hex is a *rendering* of an id, never its storage: [`Oid::from_hex`] and
/// [`Display`](fmt::Display) convert in each direction. Hex output is always
/// lowercase and always safe to draw — it is hex digits, not repository text —
/// so it needs no sanitising.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Oid {
    /// The hash, left-aligned; everything past `len` is zero so that derived
    /// equality and ordering only ever see real bytes.
    bytes: [u8; Oid::MAX_BYTES],
    /// How many of `bytes` are the hash: 20 or 32.
    len: u8,
}

impl Oid {
    /// The length of a SHA-1 id, git's default hash.
    pub const SHA1_BYTES: usize = 20;

    /// The length of a SHA-256 id, the largest hash git uses.
    pub const MAX_BYTES: usize = 32;

    /// Wraps raw hash bytes.
    ///
    /// # Errors
    /// [`Error::OidLength`] if `bytes` is not 20 or 32 bytes long.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        let len = hash_len(bytes.len()).ok_or(Error::OidLength { len: bytes.len() })?;

        let mut stored = [0u8; Self::MAX_BYTES];
        stored[..bytes.len()].copy_from_slice(bytes);
        Ok(Oid { bytes: stored, len })
    }

    /// Parses a full hex object id, in either case.
    ///
    /// Abbreviated ids are deliberately not accepted: resolving one needs the
    /// object database, which is the `git` crate's job, not a constructor's.
    ///
    /// # Errors
    /// [`Error::OidHex`] if `hex` is not exactly 40 or 64 hex digits.
    pub fn from_hex(hex: &str) -> Result<Self, Error> {
        let invalid = || Error::OidHex {
            text: hex.escape_debug().to_string(),
        };

        let digits = hex.as_bytes();
        let len = hash_len(digits.len() / 2)
            .filter(|_| digits.len() % 2 == 0)
            .ok_or_else(invalid)?;

        let mut bytes = [0u8; Self::MAX_BYTES];
        for (byte, pair) in bytes.iter_mut().zip(digits.chunks_exact(2)) {
            let high = nibble(pair[0]).ok_or_else(invalid)?;
            let low = nibble(pair[1]).ok_or_else(invalid)?;
            *byte = (high << 4) | low;
        }

        Ok(Oid { bytes, len })
    }

    /// The raw hash bytes: 20 or 32 of them.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..usize::from(self.len)]
    }
}

/// A byte count as a hash length, or `None` if git has no hash that size.
const fn hash_len(len: usize) -> Option<u8> {
    match len {
        Oid::SHA1_BYTES => Some(20),
        Oid::MAX_BYTES => Some(32),
        _ => None,
    }
}

/// One hex digit as its value, or `None` if it is not a hex digit.
const fn nibble(digit: u8) -> Option<u8> {
    match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        b'A'..=b'F' => Some(digit - b'A' + 10),
        _ => None,
    }
}

impl fmt::Display for Oid {
    /// The full id in lowercase hex.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.as_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({self})")
    }
}

#[cfg(test)]
mod tests;

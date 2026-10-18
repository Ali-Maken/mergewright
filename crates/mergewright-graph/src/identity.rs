//! A stable identity for a branch, independent of which lane it occupies.
//!
//! Lane colour comes from this, never from a column index (PROMPT.md §1.2,
//! DESIGN.md §3.1): a branch keeps its colour across a scroll or a refresh
//! even though its column can, and does, change as other branches open and
//! close around it.

use mergewright_core::oid::Oid;

/// FNV-1a's 64-bit offset basis.
const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a's 64-bit prime.
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// How many leading bytes of an [`Oid`] a short-SHA identity hashes over —
/// enough to distinguish tips in the same repository without hashing the
/// whole id.
const SHORT_SHA_BYTES: usize = 4;

/// A stable hash of a branch name, or of a commit id when no name survives.
///
/// Hashed as raw bytes, not a [`RefName`](mergewright_core::ref_name::RefName): a
/// branch name recovered from a merge message (DESIGN.md §3.4) is not
/// necessarily valid `check-ref-format` syntax, and an identity must still be
/// computable for it.
///
/// Hand-rolled FNV-1a rather than `std`'s hasher or a crate's: `std`'s
/// `DefaultHasher` is explicitly unstable across releases, which would
/// recolour every branch on a toolchain upgrade, and a stable hash of a
/// handful of bytes needs no dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BranchIdentity(u64);

impl BranchIdentity {
    /// The identity of a named branch.
    #[must_use]
    pub fn of_name(name: &[u8]) -> Self {
        BranchIdentity(fnv1a(name))
    }

    /// A fallback identity keyed by a commit id, for a lane whose branch name
    /// is not known.
    ///
    /// Not a substitute for [`of_name`](Self::of_name) on a branch that is
    /// merely unregistered: an ordinary open lane's colour comes from its
    /// branch name whenever that name is known, or the lane recolours every
    /// time the branch gains a commit and its tip id changes. This exists for
    /// the cases where no name survives at all — DESIGN.md §3.4's
    /// merge-message recovery failing, or a lane opened for a tip nobody
    /// named.
    #[must_use]
    pub fn of_short_sha(id: Oid) -> Self {
        let bytes = id.as_bytes();
        let end = SHORT_SHA_BYTES.min(bytes.len());
        BranchIdentity(fnv1a(&bytes[..end]))
    }

    /// Which of `slots` palette entries this identity draws in: the same
    /// answer for the same branch on every row, run and machine.
    ///
    /// FNV-1a alone mixes poorly for short names — one-letter branch names
    /// share their high bits, and their low bits cluster — so the hash goes
    /// through murmur3's 64-bit finaliser before its top 16 bits pick the
    /// slot. Stable: fixed constants, no `std` hasher.
    ///
    /// # Panics
    ///
    /// If `slots` is zero.
    #[must_use]
    pub fn slot(self, slots: usize) -> usize {
        let mut mixed = self.0;
        mixed ^= mixed >> 33;
        mixed = mixed.wrapping_mul(0xff51_afd7_ed55_8ccd);
        mixed ^= mixed >> 33;
        mixed = mixed.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
        mixed ^= mixed >> 33;
        let [.., high, top] = mixed.to_le_bytes();
        let folded = u16::from_le_bytes([high, top]);
        usize::from(folded) % slots
    }
}

/// A 64-bit FNV-1a hash.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = OFFSET_BASIS;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests;

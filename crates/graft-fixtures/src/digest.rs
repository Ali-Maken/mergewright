//! What the cached corpus is keyed by.
//!
//! "Rebuilt only when the generating script changes" (PROMPT.md §7.1) is
//! implemented by putting a digest of the generator *in the path*. A changed
//! generator names a directory that does not exist yet, so the rebuild is
//! automatic and an old corpus is never mistaken for a current one. There is
//! no staleness check to get wrong.
//!
//! `git --version` is in the digest too. Most objects come from the stream
//! and are ours, but the stash commits are git's own work, and a different
//! git could write them differently.

use crate::git::{self, GitError};

/// The sources that decide what the corpus contains. Any edit to one of them
/// changes the digest, and so the directory the corpus is built in.
const GENERATOR: &[&str] = &[
    include_str!("shapes.rs"),
    include_str!("stream.rs"),
    include_str!("shape.rs"),
    include_str!("git.rs"),
    include_str!("corpus.rs"),
    // Decides whether the non-UTF-8 shape is built or skipped here.
    include_str!("filesystem.rs"),
];

/// Bumped by hand when the *layout* of the corpus changes in a way the
/// sources above do not capture — a renamed marker file, say.
const LAYOUT_VERSION: &str = "1";

/// The cache key: 16 lowercase hex digits.
pub fn corpus_key() -> Result<String, GitError> {
    let mut hash = FNV_OFFSET;
    for source in GENERATOR {
        hash = fold(hash, source.as_bytes());
    }
    hash = fold(hash, LAYOUT_VERSION.as_bytes());
    hash = fold(hash, git::version()?.as_bytes());
    Ok(format!("{hash:016x}"))
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a, with a separator after each input so two sources cannot be
/// rearranged into the same digest.
///
/// Not a security hash and not asked to be one: it only has to change when
/// the input changes, and be the same on two runs of the same build.
fn fold(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes.iter().chain(std::iter::once(&SEPARATOR)) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// A byte no Rust source file contains, so it cannot occur inside an input.
const SEPARATOR: u8 = 0;

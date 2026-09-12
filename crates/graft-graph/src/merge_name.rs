//! Recovering the name of a merged branch from a merge commit's subject.
//!
//! A merge commit's second (and later) parents carry no branch name of their
//! own — only a tip commit does, and by the time history is merged that tip
//! may be gone. Git's own tools write a conventional subject line for an
//! ordinary merge, and DESIGN.md §3.4 has layout read it back out, so a
//! merged branch keeps its colour instead of falling back to a short SHA.
//! Not finding a match is not a failure: most of history has no merges to
//! recover a name from, and a merge made with `--no-edit` after a rebase of
//! the subject line, or a hand-written message, simply falls back.

/// Recovers a branch name from a merge commit's subject line, trying each of
/// git's conventional forms in turn.
///
/// Recognises:
/// - `Merge branch '<name>'`, with or without a trailing `into <target>`;
/// - `Merge remote-tracking branch '<name>'`, git's own form for a remote;
/// - `Merge pull request #<n> from <name>`, GitHub's form.
///
/// [`None`] when the subject matches none of these — the caller falls back
/// to a short SHA (DESIGN.md §3.4), not an error.
#[must_use]
pub fn recover_branch_name(subject: &[u8]) -> Option<Vec<u8>> {
    quoted_after(subject, b"Merge branch '")
        .or_else(|| quoted_after(subject, b"Merge remote-tracking branch '"))
        .or_else(|| pull_request_source(subject))
}

/// The text between `prefix` and the next `'`.
///
/// Covers both `Merge branch '<name>'` and `Merge branch '<name>' into
/// <target>` at once: whatever the closing quote is followed by, the name is
/// the same slice, so there is only one pattern to match here, not two.
fn quoted_after(subject: &[u8], prefix: &[u8]) -> Option<Vec<u8>> {
    let rest = subject.strip_prefix(prefix)?;
    let end = rest.iter().position(|&byte| byte == b'\'')?;
    Some(rest[..end].to_vec())
}

/// The `<name>` in `Merge pull request #<n> from <name>`.
fn pull_request_source(subject: &[u8]) -> Option<Vec<u8>> {
    let rest = subject.strip_prefix(b"Merge pull request #")?;
    let digits = rest.iter().position(|byte| !byte.is_ascii_digit())?;
    if digits == 0 {
        // `#` with no digits after it is not this pattern at all.
        return None;
    }
    let name = rest[digits..].strip_prefix(b" from ")?;
    if name.is_empty() {
        return None;
    }
    Some(name.to_vec())
}

#[cfg(test)]
mod tests;

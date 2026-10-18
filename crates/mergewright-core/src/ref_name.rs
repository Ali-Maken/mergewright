//! Ref identity: the full name of a branch, tag, remote-tracking branch or
//! `HEAD` itself.
//!
//! `git` reads these, `graph` groups rows by them and `ui` draws them as ref
//! badges, so the shape lives here (DESIGN.md §8.2).

use std::fmt;

use crate::error::{Error, RefNameRule};
use crate::on_disk::Generation;

/// A git ref's **full** name — `refs/heads/main`, `refs/tags/v1.0`,
/// `refs/remotes/origin/main`, or a root ref such as `HEAD`.
///
/// Stored as bytes, not a `String`. Git's own rules (see below) forbid control
/// characters and spaces but say nothing about encoding, so a ref name on disk
/// is a byte string that is *usually* UTF-8. Keeping the bytes means a repo
/// with a Latin-1 branch name still opens, rather than being silently mangled
/// at the boundary. Turning one into text is the display layer's job, and goes
/// through the sanitiser (`mergewright_term::sanitize_path_bytes`) like every other
/// piece of repository-controlled text.
///
/// The constructor enforces git's `check-ref-format` rules, so holding a
/// `RefName` is proof the name is one git would accept — in particular, proof
/// that it carries no control character. That is an invariant of the type, not
/// a substitute for sanitising: a name may still contain bidi overrides or
/// zero-width characters, which git permits and a terminal will happily act on.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RefName(Vec<u8>);

impl RefName {
    /// Validates a full ref name against git's `check-ref-format` rules.
    ///
    /// # Errors
    /// [`Error::RefName`], naming the rule that was broken.
    pub fn new(name: impl Into<Vec<u8>>) -> Result<Self, Error> {
        let name = name.into();
        match check(&name) {
            Ok(()) => Ok(RefName(name)),
            Err(reason) => Err(Error::RefName {
                name: String::from_utf8_lossy(&name).escape_debug().to_string(),
                reason,
            }),
        }
    }

    /// The full name, as git stores it.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The part a person recognises: `main` for `refs/heads/main`,
    /// `origin/main` for `refs/remotes/origin/main`, `v1.0` for
    /// `refs/tags/v1.0`.
    ///
    /// Any other name — `HEAD`, `refs/stash`, `refs/notes/commits` — is
    /// returned whole, because there is no prefix that can be dropped without
    /// making two different refs look the same.
    #[must_use]
    pub fn shorthand(&self) -> &[u8] {
        const PREFIXES: [&[u8]; 3] = [b"refs/heads/", b"refs/tags/", b"refs/remotes/"];

        for prefix in PREFIXES {
            if let Some(rest) = self.0.strip_prefix(prefix) {
                return rest;
            }
        }
        &self.0
    }

    /// Mergewright's own bookkeeping, not the user's history.
    ///
    /// Everything under any generation's [`Generation::ref_prefix`] —
    /// `refs/graft/`, `refs/tributary/` or `refs/mergewright/` (ADR-0057,
    /// Amendments) — is written by the app for itself: today the undo
    /// journal's backup chains,
    /// `<prefix>backup/<worktree>/<ulid>` (ADR-0009, ADR-0010), kept
    /// indefinitely so that a write can always be put back. The user never
    /// made them and must never be shown them — a backup is two commits
    /// above `HEAD`, so a day of staging would bury the branch the graph is
    /// about.
    ///
    /// This is about *drawing*, not about reading: undo and the journal
    /// still reach their refs, by exact name, through `RepoHandle::reference`.
    ///
    /// `refs/stash` is deliberately **not** internal: git made it, the user
    /// knows about it, and Mergewright draws stash entries on purpose.
    #[must_use]
    pub fn is_graft_internal(&self) -> bool {
        // Every generation, whichever is in force: a repository half-way
        // through a rename must never show any namespace in the graph. This
        // is `Generation::of_ref(..).is_some()` on bytes, so a name that is
        // not UTF-8 is judged by its prefix like any other.
        Generation::GENERATIONS
            .iter()
            .any(|generation| self.0.starts_with(generation.ref_prefix().as_bytes()))
    }
}

/// Git's `check-ref-format` rules for a full ref name, in git's own order.
///
/// The rules that only apply to *user input* (`--allow-onelevel`,
/// `--refspec-pattern`) are left out: every name reaching this constructor
/// came out of a repository or out of Mergewright itself.
fn check(name: &[u8]) -> Result<(), RefNameRule> {
    if name.is_empty() {
        return Err(RefNameRule::Empty);
    }
    if name.last() == Some(&b'.') {
        return Err(RefNameRule::TrailingDot);
    }
    if name == b"@" {
        return Err(RefNameRule::ForbiddenSequence);
    }

    for byte in name {
        match byte {
            0x00..=0x20 | 0x7f => return Err(RefNameRule::ControlOrSpace),
            b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\' => {
                return Err(RefNameRule::ForbiddenCharacter);
            }
            _ => {}
        }
    }

    for pair in name.windows(2) {
        if pair == b".." || pair == b"@{" {
            return Err(RefNameRule::ForbiddenSequence);
        }
    }

    // `split` on the separator yields an empty component for a leading,
    // trailing or doubled `/`, so those three cases need no separate check.
    for component in name.split(|byte| *byte == b'/') {
        if component.is_empty() || component.starts_with(b".") || component.ends_with(b".lock") {
            return Err(RefNameRule::BadComponent);
        }
    }

    Ok(())
}

impl fmt::Debug for RefName {
    /// Escaped, so that dumping a ref name into a panic message or a crash
    /// report cannot carry repository bytes to the terminal unchanged.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RefName(\"{}\")",
            String::from_utf8_lossy(&self.0).escape_debug()
        )
    }
}

#[cfg(test)]
mod tests;

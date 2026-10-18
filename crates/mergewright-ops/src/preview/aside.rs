//! The set-aside a preview counts: what is set aside and put back.

use crate::operation::{Operation, RepoFacts, SetAside};

/// The set-aside the preview counts: what is set aside **and put back**.
/// A put-in's and an absorb's staged edits go into commits, so only the
/// unstaged part comes back — the landing's "your unstaged edits" (review
/// m7-09..21c Minor 6); every other kind puts back all it set aside. The
/// set-aside itself stays `Some`, so the put-back is still forecast.
pub(super) fn shown_aside(op: &Operation, facts: &RepoFacts, aside: SetAside) -> SetAside {
    match op {
        Operation::Rewrite(params) if !params.made.is_empty() || params.index_base.is_some() => {
            SetAside {
                files: facts
                    .status
                    .entries
                    .iter()
                    .filter(|entry| entry.is_unstaged())
                    .count(),
            }
        }
        _ => aside,
    }
}

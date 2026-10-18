//! What a fetch moved, in words, and whether an entry wrote anything at all
//! (ADR-0062).
//!
//! A fetch's moved refs live outside the undo record
//! (`JournalEntry::fetched_refs`): `in_force` never reads them, so "not in
//! force" no longer means "nothing written". [`wrote_anything`] is the
//! question J's `nothing written` mark asks; `in_force` keeps meaning "`u`
//! has this to take back".

use std::collections::BTreeMap;

use mergewright_core::journal::{EntryStatus, JournalEntry, OperationKind, RefMove};
use mergewright_core::ref_name::RefName;

use super::took_effect;

/// How many refs [`moved_words`] names before `+N more`: a status line and a
/// J row have room for two.
const NAMED: usize = 2;

/// Whether the entry wrote anything to the repository: it took effect (what
/// `in_force` starts from), or a fetch moved refs, or it is a fetch or pull
/// that failed with no record of what its fetch moved — unknown is not
/// unchanged, the rule `in_force` states for a field never read after.
#[must_use]
pub fn wrote_anything(entry: &JournalEntry) -> bool {
    took_effect(entry)
        || entry
            .fetched_refs
            .as_ref()
            .is_some_and(|moved| !moved.is_empty())
        || (entry.fetched_refs.is_none()
            && matches!(entry.kind, OperationKind::Fetch | OperationKind::Pull)
            && matches!(entry.status, EntryStatus::Aborted { .. }))
}

/// The moved refs in a few words, `None` when nothing moved: up to two refs
/// by short name, each `moved`, `new` or `gone`, then `+N more` —
/// `origin/main moved, origin/feat new +3 more`, `tag v1.2 new`. In the
/// map's order (full ref name).
#[must_use]
pub fn moved_words(moved: &BTreeMap<String, RefMove>) -> Option<String> {
    if moved.is_empty() {
        return None;
    }
    let named: Vec<String> = moved
        .iter()
        .take(NAMED)
        .map(|(name, change)| format!("{} {}", label(name), verb(*change)))
        .collect();
    let mut words = named.join(", ");
    if moved.len() > NAMED {
        words.push_str(&format!(" +{} more", moved.len() - NAMED));
    }
    Some(words)
}

/// One line per moved ref for J's expanded row, at most `at_most` of them,
/// then `+N more`: `origin/main  a1b2c3d → e4f5a6b`, `origin/feat  new at
/// e4f5a6b`, `origin/old  gone, was a1b2c3d`, `tag v1.2  new at e4f5a6b`.
#[must_use]
pub fn moved_lines(moved: &BTreeMap<String, RefMove>, at_most: usize) -> Vec<String> {
    let mut lines: Vec<String> = moved
        .iter()
        .take(at_most)
        .map(|(name, change)| format!("{}  {}", label(name), values(*change)))
        .collect();
    if moved.len() > at_most {
        lines.push(format!("+{} more", moved.len() - at_most));
    }
    lines
}

/// `origin/main` for `refs/remotes/origin/main`, `tag v1.2` for
/// `refs/tags/v1.2`.
fn label(name: &str) -> String {
    // The name came from git's own `for-each-ref`, so it is one git
    // accepts; were it not, the full name is still the truth, only longer.
    let short = RefName::new(name).map_or_else(
        |_| name.to_owned(),
        |valid| String::from_utf8_lossy(valid.shorthand()).into_owned(),
    );
    if name.starts_with("refs/tags/") {
        format!("tag {short}")
    } else {
        short
    }
}

/// `moved`, `new` or `gone`. Both sides absent is no move at all, which
/// `apply`'s diff never records; it reads as `new` rather than panic.
fn verb(change: RefMove) -> &'static str {
    match (change.before, change.after) {
        (None, _) => "new",
        (Some(_), None) => "gone",
        (Some(_), Some(_)) => "moved",
    }
}

/// `a1b2c3d → e4f5a6b`, `new at e4f5a6b`, `gone, was a1b2c3d`.
fn values(change: RefMove) -> String {
    let short =
        |oid: mergewright_core::oid::Oid| oid.to_string().chars().take(7).collect::<String>();
    match (change.before, change.after) {
        (Some(before), Some(after)) => format!("{} → {}", short(before), short(after)),
        (None, Some(after)) => format!("new at {}", short(after)),
        (Some(before), None) => format!("gone, was {}", short(before)),
        (None, None) => "new".to_owned(),
    }
}

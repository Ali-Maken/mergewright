//! Steps onto Graft's own entries (ADR-0046 §2); [`super::steps`] makes the
//! steps.
//!
//! A step is Graft's when one of its records is one of Graft's moves: a
//! journal entry's `refs_before`/`refs_after` for that ref, old and new
//! alike; or, for a switch (whose entry records `HEAD`'s branches but no
//! commit), `checkout: moving from <before> to <after>` within
//! [`CHECKOUT_WINDOW_MS`] of the entry.

use graft_core::journal::{HeadRef, JournalEntry};

use super::OutsideRecord;
use super::steps::{Steps, branch_short, checkout_names};

/// How close (ms) a `checkout:` record must be to a Graft switch's entry to
/// be that switch's. A switch's entry records `HEAD`'s branch before and
/// after but no commit, so the record's (ref, old, new) has nothing to match
/// and time stands in; a minute covers a slow checkout with its set-aside,
/// and the same switch in the shell within a minute of Graft's is the only
/// thing it could mistake.
pub(super) const CHECKOUT_WINDOW_MS: i64 = 60_000;

/// Per step, the journal entry (an index into `entries`) it is one of
/// Graft's own moves for, or `None` for the shell's.
///
/// Each (entry, ref) takes at most one step, the nearest in time, so the
/// same move made again in the shell later is still drawn.
pub(super) fn match_entries(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    grouped: &Steps,
) -> Vec<Option<usize>> {
    let mut owner: Vec<Option<usize>> = vec![None; grouped.steps.len()];
    for (entry_index, entry) in entries.iter().enumerate().rev() {
        for (key, before) in &entry.refs_before {
            let Some(after) = entry.refs_after.get(key) else {
                continue;
            };
            if before == after {
                continue;
            }
            let hit = |record: usize| {
                let record_ref = &outside[record].record;
                record_ref.old == *before
                    && record_ref.new == *after
                    && moves_ref(record, key, entry, outside, &grouped.on)
            };
            claim(&mut owner, entry_index, entry, outside, grouped, hit);
        }
        if let Some((from, to)) = switch_names(entry) {
            let hit = |record: usize| {
                let record_ref = &outside[record].record;
                checkout_names(&record_ref.subject) == Some((from.as_str(), to.as_str()))
                    && (record_ref.at.saturating_mul(1000) - entry.at_unix_ms).abs()
                        <= CHECKOUT_WINDOW_MS
            };
            claim(&mut owner, entry_index, entry, outside, grouped, hit);
        }
    }
    owner
}

/// The unclaimed step nearest the entry in time with a record `hit` accepts
/// becomes the entry's.
fn claim(
    owner: &mut [Option<usize>],
    entry_index: usize,
    entry: &JournalEntry,
    outside: &[OutsideRecord],
    grouped: &Steps,
    hit: impl Fn(usize) -> bool,
) {
    let best = grouped
        .steps
        .iter()
        .enumerate()
        .filter(|(index, step)| owner[*index].is_none() && step.members.iter().any(|m| hit(*m)))
        .min_by_key(|(_, step)| {
            (outside[step.primary].record.at.saturating_mul(1000) - entry.at_unix_ms).abs()
        })
        .map(|(index, _)| index);
    if let Some(index) = best {
        owner[index] = Some(entry_index);
    }
}

/// Does the record at `record` move the ref `key` the entry recorded? Its
/// own log's ref, or `HEAD` while on that branch (by the logs, or by the
/// entry's own `HEAD`).
fn moves_ref(
    record: usize,
    key: &str,
    entry: &JournalEntry,
    outside: &[OutsideRecord],
    on: &[Option<String>],
) -> bool {
    let refname = &outside[record].record.refname;
    if refname == key {
        return true;
    }
    if refname != "HEAD" {
        return false;
    }
    let Some(short) = branch_short(key) else {
        return false;
    };
    let head_on = |head: &HeadRef| matches!(head, HeadRef::Branch(full) if full == key);
    on[record].as_deref() == Some(short)
        || head_on(&entry.head_before)
        || entry.head_after.as_ref().is_some_and(head_on)
}

/// A switch's `(from, to)` as git's `checkout:` subject names them: short
/// branch names, a detached `HEAD` as its full hex id.
fn switch_names(entry: &JournalEntry) -> Option<(String, String)> {
    let after = entry.head_after.as_ref()?;
    if *after == entry.head_before {
        return None;
    }
    Some((head_name(&entry.head_before), head_name(after)))
}

fn head_name(head: &HeadRef) -> String {
    match head {
        HeadRef::Branch(full) | HeadRef::Unborn(full) => {
            branch_short(full).unwrap_or(full).to_owned()
        }
        HeadRef::Detached(oid) => oid.to_string(),
    }
}

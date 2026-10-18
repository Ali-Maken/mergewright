//! A journal's ref names read under the generation in force: the journal's
//! bytes are never rewritten; every reader maps at read.

use std::borrow::Cow;

use mergewright_core::journal::JournalEntry;
use mergewright_core::on_disk::Generation;

/// `name` as the generation in force spells it: a ref the journal recorded
/// under any generation older than `in_force` is now under `in_force`'s
/// prefix. A newer generation's name, or anyone else's, reads as written.
/// The journal's bytes are never rewritten; every reader maps at read.
#[must_use]
pub fn map_ref(name: &str, in_force: Generation) -> Cow<'_, str> {
    let rest = Generation::of_ref(name)
        .filter(|written| *written < in_force)
        .and_then(|written| name.strip_prefix(written.ref_prefix()));
    match rest {
        Some(rest) => Cow::Owned(format!("{}{rest}", in_force.ref_prefix())),
        None => Cow::Borrowed(name),
    }
}

/// Every ref name an entry holds, through [`map_ref`]: the backup ref, the
/// touched refs, the before/after maps and a fetch's moved refs.
pub fn map_entry(entry: &mut JournalEntry, in_force: Generation) {
    if in_force.older().next().is_none() {
        // Nothing older to map from.
        return;
    }
    let map = |name: &String| map_ref(name, in_force).into_owned();
    if let Some(backup) = entry.backup_ref.as_mut() {
        *backup = map(backup);
    }
    for name in &mut entry.touches.refs {
        *name = map(name);
    }
    entry.refs_before = std::mem::take(&mut entry.refs_before)
        .into_iter()
        .map(|(name, oid)| (map(&name), oid))
        .collect();
    entry.refs_after = std::mem::take(&mut entry.refs_after)
        .into_iter()
        .map(|(name, oid)| (map(&name), oid))
        .collect();
    if let Some(fetched) = entry.fetched_refs.take() {
        entry.fetched_refs = Some(
            fetched
                .into_iter()
                .map(|(name, moved)| (map(&name), moved))
                .collect(),
        );
    }
}

/// [`map_entry`] over a whole journal.
pub fn map_entries(entries: &mut [JournalEntry], in_force: Generation) {
    for entry in entries {
        map_entry(entry, in_force);
    }
}

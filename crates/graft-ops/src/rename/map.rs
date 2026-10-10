//! A journal's ref names read under the names in force: the journal's
//! bytes are never rewritten; every reader maps at read.

use std::borrow::Cow;

use graft_core::journal::JournalEntry;
use graft_core::on_disk::OnDiskNames;

/// `name` as the names in force spell it: under the new names, a ref the
/// journal recorded under the old prefix is now under the new one. The
/// journal's bytes are never rewritten; every reader maps at read.
#[must_use]
pub fn map_ref(name: &str, names: OnDiskNames) -> Cow<'_, str> {
    let (old, new) = (OnDiskNames::Old.ref_prefix(), OnDiskNames::New.ref_prefix());
    match (names, name.strip_prefix(old)) {
        (OnDiskNames::New, Some(rest)) => Cow::Owned(format!("{new}{rest}")),
        _ => Cow::Borrowed(name),
    }
}

/// Every ref name an entry holds, through [`map_ref`]: the backup ref, the
/// touched refs, the before/after maps and a fetch's moved refs.
pub fn map_entry(entry: &mut JournalEntry, names: OnDiskNames) {
    if names == OnDiskNames::Old {
        return;
    }
    let map = |name: &String| map_ref(name, names).into_owned();
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
pub fn map_entries(entries: &mut [JournalEntry], names: OnDiskNames) {
    for entry in entries {
        map_entry(entry, names);
    }
}

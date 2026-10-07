//! A journal entry's summary words: what it did, how it ended, and why it failed.

use graft_ui::keymap::Keymap;

use super::{EntryStatus, JournalEntry, OperationKind};

/// The reason an entry closed by his own `--abort` carries
/// (`graft_ops::apply::resume`, `went_through`).
pub(super) const ABORTED: &str = "aborted";

/// What such a row says instead of `failed: aborted` (M6/68).
pub(super) const ABORTED_BY_YOU: &str = "aborted by you · nothing changed";

/// What a live entry's row says instead of a verdict: the operation has not
/// finished happening, and the two keys that end it either way — the
/// banner's own `continue_abort` words (m8/10f), the dash dropped with them
/// when both are unbound (W3).
pub(super) fn in_progress(keys: &Keymap) -> String {
    match crate::update::banner::continue_abort(keys) {
        Some(keys) => format!("in progress \u{2014} {keys}"),
        None => "in progress".to_owned(),
    }
}

/// What the entry moved, and whether a backup was kept — the two facts that
/// decide whether `u` can put it back.
///
/// `live` is git standing part-way through this very entry
/// ([`crate::update::AppState::live_entries`]); it outranks every verdict,
/// because there is not one yet.
pub(super) fn summary(entry: &JournalEntry, live: bool, keys: &Keymap) -> String {
    let mut parts: Vec<String> = Vec::new();
    // A push or a fetch is journaled but `u` passes over it (design note,
    // choice 1): the row says what moved and that it stays moved, instead of
    // the ref count and backup that mean "undoable" everywhere else. The
    // verdict comes first and the tracking ref is not repeated (the
    // description names it): at 80 columns the summary has some 40 cells,
    // and it is the verdict that must survive (m4 acceptance review M2).
    match entry.kind {
        OperationKind::Push => {
            let tracking = crate::update::remote::pushed_to(entry);
            if let Some(name) = entry.touches.refs.first()
                && let Some(Some(after)) = entry.refs_after.get(name)
            {
                let before = entry
                    .refs_before
                    .get(name)
                    .copied()
                    .flatten()
                    .map_or_else(|| "new".to_owned(), short);
                parts.push(format!("{before} → {}", short(*after)));
            }
            let remote = tracking.split('/').next().unwrap_or("the remote");
            parts.insert(0, format!("not undoable \u{2014} {remote} has it"));
            return with_failure(entry, live, &parts, keys);
        }
        OperationKind::Fetch => {
            parts.push("remote-tracking refs".to_owned());
            parts.push("not undoable".to_owned());
            return with_failure(entry, live, &parts, keys);
        }
        _ => {}
    }
    match entry.touches.refs.len() {
        0 => {}
        1 => parts.push("1 ref".to_owned()),
        many => parts.push(format!("{many} refs")),
    }
    if entry.touches.index {
        parts.push("the index".to_owned());
    }
    if entry.touches.worktree {
        parts.push("the working tree".to_owned());
    }
    // Interrupted first, and *instead of* `failed:` — recovery closes a torn
    // entry as `Aborted { reason: "interrupted" }`, so the two arms overlap
    // and "failed: interrupted" is the wrong word for a process that died
    // (m3/06 review, Major 2).
    if live {
        parts.insert(0, in_progress(keys));
    } else if entry.is_interrupted() {
        parts.insert(0, "interrupted".to_owned());
    } else if let EntryStatus::Aborted { reason } = &entry.status {
        // He stopped it himself (`--abort`): not a failure, and git put
        // everything back (M6/68).
        if reason == ABORTED {
            return ABORTED_BY_YOU.to_owned();
        }
        // The row carries the failure in short form; the whole reason is
        // under `Space`, which is where a multi-line stderr belongs.
        parts.insert(0, format!("failed: {}", first_line(reason)));
    }
    if entry.backup_ref.is_some() {
        parts.push("backup saved".to_owned());
    }
    parts.join(" · ")
}

/// A push or fetch row that failed says so first, and nothing else: an
/// aborted push sent nothing, so "origin has it" would be false.
pub(super) fn with_failure(
    entry: &JournalEntry,
    live: bool,
    parts: &[String],
    keys: &Keymap,
) -> String {
    if live {
        return in_progress(keys);
    }
    if entry.is_interrupted() {
        return "interrupted".to_owned();
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => ABORTED_BY_YOU.to_owned(),
        EntryStatus::Aborted { reason } => format!("failed: {}", first_line(reason)),
        EntryStatus::Pending | EntryStatus::Committed => parts.join(" · "),
    }
}

/// Seven hex digits, as every row writes a commit.
pub(super) fn short(oid: graft_core::oid::Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

/// The whole reason an `Aborted` entry gives, for the expanded row.
///
/// A live entry has none: nothing failed, and the operation has not finished
/// happening. Saying "a previous run stopped between this entry's two
/// journal lines" about a merge running right now is the very lie this
/// ticket exists to stop.
pub(super) fn failure(entry: &JournalEntry, live: bool) -> Option<String> {
    if live {
        return None;
    }
    if entry.is_interrupted() {
        return Some(
            "interrupted: a previous run stopped between this entry's two journal lines".to_owned(),
        );
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => Some(ABORTED_BY_YOU.to_owned()),
        EntryStatus::Aborted { reason } => Some(format!("failed: {reason}")),
        // Unreachable: `is_interrupted` has taken every `Pending` entry
        // above. Kept as the honest arm of the match rather than a
        // catch-all that would silently call a torn entry a clean one.
        EntryStatus::Pending | EntryStatus::Committed => None,
    }
}

/// git's first stderr line, which is the one that says what went wrong.
pub(super) fn first_line(reason: &str) -> &str {
    reason.lines().next().unwrap_or(reason)
}

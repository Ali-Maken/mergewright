//! A journal entry's summary words: what it did, how it ended, and why it failed.

use graft_ui::keymap::Keymap;

use super::{EntryStatus, JournalEntry, OperationKind};

/// The reason an entry closed by his own `--abort` carries
/// (`graft_ops::apply::resume`, `went_through`).
pub(super) const ABORTED: &str = "aborted";

/// What such a row says instead of `failed: aborted` (M6/68).
pub(super) const ABORTED_BY_YOU: &str = "aborted by you · nothing changed";

/// The reason a write he stopped himself carries (`Esc` on a running write,
/// or `q`'s "stop it now"): `graft_ops`' `RemoteFailure::Cancelled` and a
/// stopped restore both record exactly this.
const CANCELLED: &str = "cancelled";

/// What such a row says instead of `failed: cancelled`: his choice is not a
/// failure (m8/59 m3). Whether anything was written is the row's mark.
const CANCELLED_BY_YOU: &str = "cancelled by you";

/// The verdict for an `Aborted` entry other than his `--abort`.
fn stopped_words(reason: &str) -> String {
    if reason == CANCELLED {
        CANCELLED_BY_YOU.to_owned()
    } else {
        format!("failed: {}", first_line(reason))
    }
}

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

/// What a row says for an entry an operation is still writing (58r).
const RUNNING: &str = "running";

/// Whether an entry has finished yet, as the last journal read found it.
/// An open entry (`Live`, `Running`) outranks every verdict, because there
/// is not one yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Progress {
    /// Finished, or interrupted: the entry's own status is the verdict.
    Settled,
    /// git is standing part-way through this very entry
    /// ([`crate::update::AppState::live_entries`]).
    Live,
    /// An operation was writing the journal when it was read, so this
    /// `Pending` entry was held, not judged
    /// ([`crate::jobs::undo::JournalRead::held`]): its own, running.
    Running,
}

impl Progress {
    /// The words that stand in for the verdict, or `None` when there is one.
    fn words(self, keys: &Keymap) -> Option<String> {
        match self {
            Progress::Settled => None,
            Progress::Live => Some(in_progress(keys)),
            Progress::Running => Some(RUNNING.to_owned()),
        }
    }
}

/// What the entry moved, and whether a backup was kept — the two facts that
/// decide whether `u` can put it back.
pub(super) fn summary(entry: &JournalEntry, progress: Progress, keys: &Keymap) -> String {
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
            return with_failure(entry, progress, &parts, keys);
        }
        OperationKind::Fetch => return fetch_summary(entry, progress, keys),
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
    if let Some(words) = progress.words(keys) {
        parts.insert(0, words);
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
        parts.insert(0, stopped_words(reason));
    }
    if entry.backup_ref.is_some() {
        parts.push("backup saved".to_owned());
    }
    // A pull's fetch moved its tracking refs whatever became of the merge
    // (ADR-0062): last, after what `u` would take back.
    if let Some(moved) = moves(entry) {
        parts.push(moved);
    }
    parts.join(" · ")
}

/// A fetch row: what it moved, then `not undoable` (ADR-0062). The refs by
/// name; `nothing new` when it was read and nothing moved; today's
/// `remote-tracking refs` where nothing was recorded (a line from before the
/// ADR) or a failed fetch moved nothing — that row stays exactly as it was.
fn fetch_summary(entry: &JournalEntry, progress: Progress, keys: &Keymap) -> String {
    let moved = moves(entry);
    let clause = match (&moved, &entry.fetched_refs, &entry.status) {
        (Some(moved), ..) => moved.clone(),
        (None, Some(_), EntryStatus::Committed) => "nothing new".to_owned(),
        (None, ..) => "remote-tracking refs".to_owned(),
    };
    let parts = [clause, "not undoable".to_owned()];
    let row = with_failure(entry, progress, &parts, keys);
    // `with_failure` keeps a failed row's failure alone. What the fetch
    // moved before it stopped is the one fact that has to outlive it: the
    // row no longer says `nothing written`, so it must say what was.
    match moved {
        Some(_) if failed(entry, progress) => format!("{row} · {}", parts.join(" · ")),
        _ => row,
    }
}

/// What the entry's fetch moved, in `graft_ops`' words (`origin/main moved,
/// origin/feat new +3 more`), `before it stopped` after them when it failed;
/// `None` when it moved nothing or nothing was recorded.
fn moves(entry: &JournalEntry) -> Option<String> {
    let words = graft_ops::journal::moved_words(entry.fetched_refs.as_ref()?)?;
    Some(match entry.status {
        EntryStatus::Aborted { .. } => format!("{words} before it stopped"),
        EntryStatus::Pending | EntryStatus::Committed => words,
    })
}

/// The row reads `failed: …` ([`with_failure`]'s failure arm): `Aborted`
/// for a reason other than his own `--abort`, and neither open nor
/// interrupted.
fn failed(entry: &JournalEntry, progress: Progress) -> bool {
    progress == Progress::Settled
        && !entry.is_interrupted()
        && matches!(&entry.status, EntryStatus::Aborted { reason } if reason != ABORTED)
}

/// A push or fetch row that failed says so first, and nothing else: an
/// aborted push sent nothing, so "origin has it" would be false.
pub(super) fn with_failure(
    entry: &JournalEntry,
    progress: Progress,
    parts: &[String],
    keys: &Keymap,
) -> String {
    if let Some(words) = progress.words(keys) {
        return words;
    }
    if entry.is_interrupted() {
        return "interrupted".to_owned();
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => ABORTED_BY_YOU.to_owned(),
        EntryStatus::Aborted { reason } => stopped_words(reason),
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
/// An open entry has none: nothing failed, and the operation has not
/// finished happening. Saying "a previous run stopped between this entry's
/// two journal lines" about a merge standing right now, or a push still
/// running, is the very lie m5/09 and 58r exist to stop.
pub(super) fn failure(entry: &JournalEntry, progress: Progress) -> Option<String> {
    if progress != Progress::Settled {
        return None;
    }
    if entry.is_interrupted() {
        return Some(
            "interrupted: a previous run stopped between this entry's two journal lines".to_owned(),
        );
    }
    match &entry.status {
        EntryStatus::Aborted { reason } if reason == ABORTED => Some(ABORTED_BY_YOU.to_owned()),
        EntryStatus::Aborted { reason } if reason == CANCELLED => Some(CANCELLED_BY_YOU.to_owned()),
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

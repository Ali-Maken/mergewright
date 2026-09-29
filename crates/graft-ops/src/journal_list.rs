//! The journal and git's reflog as one list (ADR-0046): what Graft did
//! (`●`), what the shell did that no journal entry explains (`$`), and the
//! commits and edits nothing holds any more (`✗`), each under the event
//! after which nothing held it.
//!
//! The list arrives in three stages so `J` never waits (note J1): the app
//! calls [`build`] with the journal alone, again with [`read_outside`]'s
//! records, and again with [`find_lost`]'s answer. `build` is pure; the two
//! reads are the only parts that touch the repository. [`recover_for`] says
//! what `⏎` on a row puts back.
//!
//! How a reflog record is matched to Graft's own entry, paired with its twin
//! and folded into a rebase is `steps`' business; the plain words are
//! `words`'; the put-back is `recover`'s.

mod edits;
mod recover;
mod rows;
mod steps;
mod words;

use std::path::PathBuf;

use graft_core::journal::{EntryId, JournalEntry};
use graft_core::oid::Oid;
use graft_git::history_reads::{LostRun, Unreached, unreached};
use graft_git::reflog::{self, ReflogRecord};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;

use crate::facts::FactsError;

pub use edits::{LOST_EDITS_CAP, lost_edits};
pub use recover::{Recover, recover_for, recover_scope};
pub use words::words;

/// How many reflog records the list reads, newest first, by default.
pub const RECORDS_CAP: usize = 2_000;
/// How many lost-commit candidates the scan checks, newest first.
pub const LOST_CAP: usize = 500;
/// A `HEAD` record and a branch record with the same old and new values
/// written this close together (seconds) are one step. git writes both in
/// one process, in the same second in every oracle run (m7/22 NOTES); the
/// second of slack covers the clock ticking over between the two writes.
pub const PAIR_WINDOW_SECS: i64 = 2;
/// How far back along the first parent a reset's "N commits back" (and a
/// fast-forward's "N commits") is counted before the words give the target
/// instead.
pub const BACK_LIMIT: usize = 100;

/// One reflog record, with the distance its words need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutsideRecord {
    /// The record as git wrote it.
    pub record: ReflogRecord,
    /// The first-parent distance between the old and new values, when the
    /// words use it and one is within [`BACK_LIMIT`] of the other: for a
    /// `reset: moving to …` the new value's distance **back** from the old;
    /// for a fast-forward the old value's distance back from the new (how
    /// many commits it moved forward). `None` otherwise.
    pub back: Option<usize>,
}

/// A step the shell took that no journal entry explains: one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutsideStep {
    /// The record the row stands for: the branch's record when a `HEAD`
    /// record paired with it, the rebase's finish when records folded.
    pub record: ReflogRecord,
    /// What happened, in plain words (`reset main 1 commit back`).
    pub words: String,
    /// How many further records this row stands for: a shell rebase's
    /// `HEAD` records folded into its finish. Zero for most rows.
    pub folded: usize,
}

/// What held a lost commit before the event it sits under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HeldBy {
    /// A branch was on it.
    Branch {
        /// The branch, short name.
        name: String,
        /// The branch does not exist now.
        gone: bool,
    },
    /// Only `HEAD` was, as far as git's logs say.
    Head,
}

/// A run of commits nothing holds, as the list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LostCommit {
    /// The run: its tip and how many lost ancestors lie under it.
    pub run: LostRun,
    /// The tip's subject; `None` when its commit could not be read (the
    /// screen then names it by its sha alone, m7/30o).
    pub subject: Option<String>,
    /// What held the tip before its event.
    pub held_by: HeldBy,
}

/// An edit an undo or redo replaced, kept only in that entry's backup.
/// Found by recover-op's `lost_edits`; the list places it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LostEdit {
    /// The undo or redo.
    pub entry: EntryId,
    /// Its backup's working-tree tree.
    pub backup_tree: Oid,
    /// The files whose edits it holds.
    pub paths: Vec<PathBuf>,
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListRow {
    /// A journal entry.
    Graft(EntryId),
    /// A step the shell took.
    Outside(OutsideStep),
    /// A lost commit, under the row `event` (an index into `rows`).
    Lost {
        /// The row it sits under.
        event: usize,
        /// The commit.
        commit: LostCommit,
    },
    /// A lost edit, under its undo's row `event`.
    LostEdit {
        /// The row it sits under.
        event: usize,
        /// The edit.
        edit: LostEdit,
    },
}

/// Where the lost-commit scan is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LostScan {
    /// Still looking.
    Running,
    /// Every candidate was checked.
    Done,
    /// Only the newest `checked` candidates were.
    Capped {
        /// How many.
        checked: usize,
    },
    /// The scan failed, and why.
    Failed(String),
}

/// The list, newest first; lost rows sit right after their event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalList {
    /// The rows.
    pub rows: Vec<ListRow>,
    /// The scan's state, for the title.
    pub scan: LostScan,
}

/// The newest `cap` reflog records of `HEAD` and every local branch, each
/// with the distance its words need. Reads through `gix`, no spawn.
///
/// # Errors
/// [`FactsError::Reflog`] when a log cannot be read; [`FactsError::Stopped`]
/// when `stop` asks.
pub fn read_outside(
    repo: &RepoHandle,
    cap: usize,
    stop: &dyn Stop,
) -> Result<Vec<OutsideRecord>, FactsError> {
    let records = reflog::records(repo, cap)?;
    let mut outside = Vec::with_capacity(records.len());
    for record in records {
        if stop.should_stop() {
            return Err(FactsError::Stopped);
        }
        let back = back_of(repo, &record)?;
        outside.push(OutsideRecord { record, back });
    }
    Ok(outside)
}

/// The distance a record's words need, if they need one.
fn back_of(repo: &RepoHandle, record: &ReflogRecord) -> Result<Option<usize>, FactsError> {
    let (Some(old), Some(new)) = (record.old, record.new) else {
        return Ok(None);
    };
    let (from, to) = if words::is_reset(&record.subject) {
        (old, new)
    } else if words::is_fast_forward(&record.subject) {
        (new, old)
    } else {
        return Ok(None);
    };
    Ok(reflog::first_parent_distance(repo, from, to, BACK_LIMIT)?)
}

/// Which of the events' old values nothing holds now.
///
/// The candidates are every outside record's old value and every journal
/// entry's `refs_before` values, newest first by time, each once; the first
/// `cap` are checked (`graft_git::history_reads::unreached`, which never
/// counts Graft's own `refs/graft/**` as holding anything).
///
/// # Errors
/// [`FactsError::History`] when git's walk fails; [`FactsError::Stopped`]
/// when `stop` asks.
pub fn find_lost(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    cap: usize,
    stop: &dyn Stop,
) -> Result<Unreached, FactsError> {
    let candidates = candidates(entries, outside);
    unreached(repo, &candidates, cap, stop).map_err(FactsError::of_history)
}

/// [`find_lost`]'s candidates, newest first, each once.
fn candidates(entries: &[JournalEntry], outside: &[OutsideRecord]) -> Vec<Oid> {
    let mut dated: Vec<(i64, Oid)> = Vec::new();
    for record in outside {
        if let Some(old) = record.record.old {
            dated.push((record.record.at.saturating_mul(1000), old));
        }
    }
    for entry in entries.iter().rev() {
        for (_, before) in branch_or_head(&entry.refs_before) {
            dated.push((entry.at_unix_ms, before));
        }
    }
    // Stable: records keep their log order, entries newest first, on a tie.
    dated.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    let mut seen = std::collections::HashSet::new();
    dated
        .into_iter()
        .filter_map(|(_, oid)| seen.insert(oid).then_some(oid))
        .collect()
}

/// An entry's recorded values of `HEAD` and the local branches: the refs
/// whose logs the list reads. `refs/stash` is left out — a dropped stash is
/// the journal's to bring back with `u`, and "was HEAD" would be wrong
/// words for it — and so are tags.
fn branch_or_head(
    refs: &std::collections::BTreeMap<String, Option<Oid>>,
) -> impl Iterator<Item = (&str, Oid)> {
    refs.iter().filter_map(|(key, value)| {
        let wanted = key == "HEAD" || key.starts_with("refs/heads/");
        match value {
            Some(oid) if wanted => Some((key.as_str(), *oid)),
            _ => None,
        }
    })
}

/// The list from what is known so far. Pure.
///
/// - `outside` empty and `lost` `None`: stage 1, the journal alone.
/// - `lost` `None`: stage 2, the shell's steps too.
/// - Both: stage 3, with the lost commits under their events.
///
/// A record that is one of Graft's own entries is that entry's row, not a
/// `$` twin (ADR-0046 §2). `subject_of` names a lost tip; `branch_exists`
/// says whether `refs/heads/<name>` exists now (the `gone` of
/// [`HeldBy::Branch`]). Rows of an earlier stage keep their relative order.
pub fn build(
    entries: &[JournalEntry],
    outside: &[OutsideRecord],
    lost: Option<&Unreached>,
    subject_of: &dyn Fn(Oid) -> Option<String>,
    branch_exists: &dyn Fn(&str) -> bool,
    edits: &[LostEdit],
    scan: LostScan,
) -> JournalList {
    let steps = steps::steps(outside);
    let owner = steps::match_entries(entries, outside, &steps);
    let events = rows::events(entries, outside, &steps, &owner);
    let rows = rows::assemble(
        entries,
        outside,
        &steps,
        &events,
        &rows::Lost {
            found: lost,
            subject_of,
            branch_exists,
            edits,
        },
    );
    JournalList { rows, scan }
}

#[cfg(test)]
mod tests;

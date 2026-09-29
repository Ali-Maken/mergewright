//! The reading half of J's list (m7-power-tools/24): the shell's steps
//! (stage 2), what nothing holds any more (stage 3), and what `⏎` on a row
//! puts back. `update::journal` builds the list from the answers; nothing
//! here decides anything (`graft_ops::journal_list` does).

use std::sync::Arc;

use graft_core::oid::Oid;
use graft_git::history_reads::Unreached;
use graft_git::version::system_git;
use graft_ops::journal_list::{
    JournalList, LOST_CAP, LostEdit, LostScan, OutsideRecord, RECORDS_CAP, Recover,
};

use super::Cancel;
use crate::repos::RepoStore;
use crate::session::Clock;

/// Stage 3's answer: the lost runs, the lost edits, how far the scan got,
/// and each lost tip's subject — read here, because a commit no branch
/// holds is on no graph row to take it from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LostFound {
    /// `find_lost`'s runs.
    pub unreached: Unreached,
    /// `lost_edits`' answer.
    pub edits: Vec<LostEdit>,
    /// Where the scan stopped: all of it, or the newest [`LOST_CAP`].
    pub scan: LostScan,
    /// Each run's tip and its subject; a tip whose commit could not be read
    /// is left out, and the list says so in its row.
    pub subjects: Vec<(Oid, String)>,
}

impl LostFound {
    /// The subject read for `tip`, if it was.
    #[must_use]
    pub(crate) fn subject_of(&self, tip: Oid) -> Option<String> {
        self.subjects
            .iter()
            .find_map(|(at, subject)| (*at == tip).then(|| subject.clone()))
    }
}

/// Stage 2: the newest [`RECORDS_CAP`] reflog records of `HEAD` and every
/// local branch.
#[expect(
    dead_code,
    reason = "the seam 24-a stopped at: its Command is not in the enum yet (ticket 24 NOTES)"
)]
pub(crate) fn read_outside(
    repos: &RepoStore,
    cancel: &Cancel,
) -> Result<Arc<Vec<OutsideRecord>>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    graft_ops::journal_list::read_outside(&repo.handle(), RECORDS_CAP, cancel)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// Stage 3: which of the events' old values nothing holds, the edits undos
/// replaced, and the lost tips' subjects.
#[expect(
    dead_code,
    reason = "the seam 24-a stopped at: its Command is not in the enum yet (ticket 24 NOTES)"
)]
pub(crate) fn find_lost(
    repos: &RepoStore,
    clock: Clock,
    outside: &[OutsideRecord],
    cancel: &Cancel,
) -> Result<Arc<LostFound>, String> {
    let entries = super::undo::journal_entries(repos, clock, cancel)?;
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let unreached =
        graft_ops::journal_list::find_lost(&handle, &entries, outside, LOST_CAP, cancel)
            .map_err(|error| error.to_string())?;
    let edits = graft_ops::journal_list::lost_edits(&handle, &entries, cancel)
        .map_err(|error| error.to_string())?;
    let mut subjects = Vec::with_capacity(unreached.lost.len());
    for run in &unreached.lost {
        // A tip whose commit cannot be read keeps its row: the list words
        // it `(its subject could not be read)` (`journal_list::build`).
        if let Ok(commit) = graft_git::history::read_commit(&handle, run.tip) {
            subjects.push((
                run.tip,
                String::from_utf8_lossy(commit.subject()).into_owned(),
            ));
        }
    }
    let scan = if unreached.capped {
        LostScan::Capped {
            checked: unreached.checked,
        }
    } else {
        LostScan::Done
    };
    Ok(Arc::new(LostFound {
        unreached,
        edits,
        scan,
        subjects,
    }))
}

/// What `⏎` on `list.rows[row]` puts back: the refs `recover_scope` names
/// read fresh, then `recover_for`. `Err` is the words for J's status row:
/// `recover_for`'s refusal, or why the reads it needs failed — `Refusal` has
/// no variant for a failed read, and the words are all the screen uses.
#[expect(
    dead_code,
    reason = "the seam 24-a stopped at: its Command is not in the enum yet (ticket 24 NOTES)"
)]
pub(crate) fn recover_for(
    repos: &RepoStore,
    clock: Clock,
    list: &JournalList,
    row: usize,
    cancel: &Cancel,
) -> Result<Recover, String> {
    let unread = |why: String| format!("couldn't read what to put back: {why}");
    let entries = super::undo::journal_entries(repos, clock, cancel).map_err(unread)?;
    let repo = repos.shared().map_err(|error| unread(error.to_string()))?;
    let git = system_git()
        .map_err(|error| unread(error.to_string()))?
        .version();
    let scope = graft_ops::journal_list::recover_scope(list, row);
    let facts = graft_ops::facts::read_facts(&repo.handle(), git, cancel, &scope)
        .map_err(|error| unread(error.to_string()))?;
    graft_ops::journal_list::recover_for(list, row, &entries, &facts)
        .map_err(|refusal| refusal.explain())
}

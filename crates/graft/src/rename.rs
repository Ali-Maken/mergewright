//! Where the app meets the one-time move to the new on-disk names
//! (m8-polish-and-ship/26 Part B, `graft_ops::rename`): every repository is
//! settled the moment it is opened, before any job takes a handle from it,
//! and what the move could not finish is said on the first frame's status
//! row — only then.

use std::path::Path;

use graft_git::repo::{OpenError, SharedRepo};
use graft_ops::rename::{OldNamesHold, SettleOutcome, settle_names};

/// What the settle left behind for the life of the store: the words for the
/// status row, and the shared lock a run on the old names holds until it
/// ends.
#[derive(Debug)]
pub(crate) struct Settlement {
    pub(crate) outcome: SettleOutcome,
    _hold: Option<OldNamesHold>,
}

/// Open the repository at `path` and settle its names. When the settle
/// changed the names in force, the repository is opened again: a
/// `SharedRepo` reads the marker once, at open, and every handle carries
/// what it read.
///
/// # Errors
/// As [`SharedRepo::open`], either time.
pub(crate) fn open_settled(path: &Path) -> Result<(SharedRepo, Settlement), OpenError> {
    let repo = SharedRepo::open(path)?;
    let settled = settle_names(&repo.handle());
    let repo = if settled.names == repo.on_disk_names() {
        repo
    } else {
        SharedRepo::open(path)?
    };
    Ok((
        repo,
        Settlement {
            outcome: settled.outcome,
            _hold: settled.hold,
        },
    ))
}

/// The status row's words for what the settle did, `None` when there is
/// nothing to say — a move that finished, or nothing to move, draws nothing
/// (CLAUDE.md's status rule).
#[must_use]
pub fn words(outcome: &SettleOutcome) -> Option<String> {
    match outcome {
        SettleOutcome::KeptOld(why) => {
            Some(format!("rename: still using the old names \u{b7} {why}"))
        }
        SettleOutcome::LeftOld(why) => Some(format!("rename: old names left behind \u{b7} {why}")),
        SettleOutcome::Migrated {
            not_journaled: Some(why),
            ..
        } => Some(format!("rename: done \u{b7} not in the journal: {why}")),
        SettleOutcome::Done
        | SettleOutcome::Migrated {
            not_journaled: None,
            ..
        }
        | SettleOutcome::CleanedUp { .. } => None,
    }
}

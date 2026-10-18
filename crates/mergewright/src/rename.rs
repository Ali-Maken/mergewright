//! Where the app meets the one-time move to the new on-disk names
//! (m8-polish-and-ship/26 Part B, `mergewright_ops::rename`): every repository is
//! settled the moment it is opened, before any job takes a handle from it,
//! and what the move could not finish is said on the first frame's status
//! row — only then.

use std::path::{Path, PathBuf};

use mergewright_git::repo::{OpenError, SharedRepo};
use mergewright_ops::rename::{OldNamesHold, SettleOutcome, settle_names};

/// What the settle left behind for the life of the store: the words for the
/// status row, and the shared lock a run on the old names holds until it
/// ends.
#[derive(Debug)]
pub(crate) struct Settlement {
    pub(crate) outcome: SettleOutcome,
    /// The repository root a kept-aside path is said relative to.
    pub(crate) root: PathBuf,
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
    let handle = repo.handle();
    let settled = settle_names(&handle);
    let root = handle
        .workdir()
        .unwrap_or_else(|| handle.common_dir())
        .to_path_buf();
    let repo = if settled.names == repo.on_disk_names() {
        repo
    } else {
        SharedRepo::open(path)?
    };
    Ok((
        repo,
        Settlement {
            outcome: settled.outcome,
            root,
            _hold: settled.hold,
        },
    ))
}

/// The status row's words for what the settle did, `None` when there is
/// nothing to say — a move that finished, or nothing to move, draws nothing
/// (CLAUDE.md's status rule). A kept-aside path is said relative to `root`,
/// the repository's.
#[must_use]
pub fn words(outcome: &SettleOutcome, root: &Path) -> Option<String> {
    match outcome {
        SettleOutcome::KeptOld(why) => {
            Some(format!("rename: still using the old names \u{b7} {why}"))
        }
        SettleOutcome::LeftOld(why) => Some(format!("rename: old names left behind \u{b7} {why}")),
        SettleOutcome::OlderWrote { kept_aside, .. } => {
            Some(older_wrote(kept_aside.as_deref(), root))
        }
        SettleOutcome::Migrated {
            not_journaled,
            kept_aside,
            ..
        } => {
            let older = kept_aside
                .as_deref()
                .map(|kept| older_wrote(Some(kept), root));
            let journal = not_journaled
                .as_ref()
                .map(|why| format!("not in the journal: {why}"));
            match (older, journal) {
                (None, None) => None,
                (Some(older), None) => Some(older),
                (None, Some(journal)) => Some(format!("rename: done \u{b7} {journal}")),
                (Some(older), Some(journal)) => Some(format!("{older} \u{b7} {journal}")),
            }
        }
        SettleOutcome::CleanedUp {
            kept_aside: Some(kept),
            ..
        } => Some(older_wrote(Some(kept), root)),
        SettleOutcome::Done
        | SettleOutcome::CleanedUp {
            kept_aside: None, ..
        } => None,
    }
}

/// `rename: an older version wrote under the old names · close it`, and
/// where what it wrote was kept, when it was.
fn older_wrote(kept_aside: Option<&Path>, root: &Path) -> String {
    let head = "rename: an older version wrote under the old names \u{b7} close it";
    match kept_aside {
        Some(kept) => {
            let shown = kept.strip_prefix(root).unwrap_or(kept);
            format!("{head}; kept in {}", shown.display())
        }
        None => head.to_owned(),
    }
}

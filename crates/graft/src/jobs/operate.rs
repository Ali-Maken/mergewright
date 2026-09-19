//! [`Command::Operate`](crate::update::Command::Operate)'s job: the one place
//! `app` opens the undo journal, builds a [`graft_ops::apply::Context`] and
//! calls [`graft_ops::apply::apply`]. The seam rule (ARCHITECTURE.md,
//! `crates/graft/tests/it/seam_rule.rs`): `ops` decides what a write does,
//! `app` only wires the key to the call.

use graft_git::version::system_git;
use graft_ops::apply::{self, Applied, ApplyError, Context};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::operation::Operation;

use super::Cancel;
use crate::repos::RepoStore;

/// What running one operation came back with.
///
/// Not a plain `Result`: [`ApplyError::AfterState`] means the write *did*
/// happen — journaled, on disk — and only part of its after-state could not
/// be read back. Collapsing that into an ordinary `Err(String)` would lose
/// the [`Applied`] value the caller needs to refresh with, which is exactly
/// the "operation ran, caller still refreshes" case
/// `ApplyError::AfterState`'s own doc comment calls out. `update::operate`
/// matches on this instead of a `Result` so that case cannot be dropped on
/// the floor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operated {
    /// Ran and journaled cleanly.
    Done(Applied),
    /// Ran and journaled, but part of the after-state could not be read or
    /// kept reachable. The write happened: refresh, and show `error`.
    Incomplete {
        /// What ran.
        applied: Applied,
        /// What could not be read, as one line.
        error: String,
    },
    /// Nothing was written, or nothing could be recorded — the one case
    /// with no [`Applied`] to refresh from.
    Failed(String),
}

/// Runs one operation against `repos`'s repository, journaled.
///
/// Opens the system `git`, the journal file and a fresh [`Context`] every
/// call — one context per operation, since a context carries the one clock
/// reading and random half that name its entry.
pub(super) fn run_operation(repos: &RepoStore, op: &Operation, cancel: &Cancel) -> Operated {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return Operated::Failed(error.to_string()),
    };
    let handle = repo.handle();
    let git = match system_git() {
        Ok(found) => found.version(),
        Err(error) => return Operated::Failed(error.to_string()),
    };
    let path = match journal_path(&handle) {
        Ok(path) => path,
        Err(error) => return Operated::Failed(error.to_string()),
    };
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal,
        Err(error) => return Operated::Failed(error.to_string()),
    };
    let now_unix_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis()),
    )
    .unwrap_or(i64::MAX);
    let mut random = [0u8; 10];
    if let Err(error) = getrandom::fill(&mut random) {
        return Operated::Failed(format!("could not read random bytes: {error}"));
    }
    let mut ctx = match Context::open(&handle, git, &mut journal, now_unix_ms, random, cancel) {
        Ok(ctx) => ctx,
        Err(error) => return Operated::Failed(error.to_string()),
    };
    match apply::apply(&mut ctx, op) {
        Ok(applied) => Operated::Done(applied),
        Err(ApplyError::AfterState { applied, unread }) => Operated::Incomplete {
            applied: *applied,
            error: unread
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        },
        Err(error) => Operated::Failed(error.to_string()),
    }
}

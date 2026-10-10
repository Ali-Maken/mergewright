//! The refs a fetch moved (ADR-0062): every ref under `refs/` listed before
//! git's fetch runs — a fetch's, or a pull's fetch line — and again after it
//! exits, success, failure or stop. The ones whose value differs go on the
//! entry's outcome line as `fetched_refs`, outside the undo record.
//!
//! A fetch cannot name its refs before it runs (`--all`, `--prune`, tag
//! auto-follow and refspecs decide them), so the whole store is listed: one
//! `for-each-ref` each side. Like `begin`'s before-reads and `finish`'s
//! after-reads they are reads of state, not steps of the operation, so they
//! are not on the Command Log.

use std::collections::BTreeMap;

use graft_core::journal::{JournalEntry, RefMove};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;
use crate::journal::moved_words;

use super::run::PlanRun;
use super::{AfterState, ApplyError};

/// Every ref under `refs/` by full name, the app's own refs aside (either
/// prefix, `RefName::is_graft_internal`).
pub(crate) type RefStore = BTreeMap<String, Oid>;

/// What a fetch moved, by full ref name.
pub(crate) type Moved = BTreeMap<String, RefMove>;

/// List every ref under `refs/` — one `for-each-ref`.
///
/// A symbolic ref (`refs/remotes/origin/HEAD`) is left out: it names
/// another ref, which is listed in its own right, and listing its target's
/// value under its name too would report one move twice.
pub(crate) fn ref_store(repo: &RepoHandle, stop: &dyn Stop) -> Result<RefStore, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Word("--format=%(objectname)%00%(refname)%00%(symref)".to_owned()),
            GitArg::Word("refs/".to_owned()),
        ])
        .run(stop)?;
    // Lossy on purpose: these names are words for J and the status line,
    // never a ref anything writes to, and a fetch must not be refused over
    // one branch name somewhere in the store that is not UTF-8.
    let text = String::from_utf8_lossy(&output.stdout);
    let mut store = RefStore::new();
    for line in text.lines() {
        let mut fields = line.split('\0');
        let (Some(hex), Some(name), Some(symref)) = (fields.next(), fields.next(), fields.next())
        else {
            return Err(RunError::Unparsable(format!("for-each-ref line {line:?}")));
        };
        if !symref.is_empty() {
            continue;
        }
        // The app's own refs (backups), under either prefix: never a
        // fetch's doing. One rule, graft-core's (ADR-0057, Amendment).
        let internal = RefName::new(name)
            .map_err(|why| RunError::Unparsable(format!("for-each-ref ref name: {why}")))?
            .is_graft_internal();
        if internal {
            continue;
        }
        let oid = Oid::from_hex(hex)
            .map_err(|_| RunError::Unparsable(format!("for-each-ref object id {hex:?}")))?;
        store.insert(name.to_owned(), oid);
    }
    Ok(store)
}

/// The refs whose value differs between two listings: moved, new
/// (`before: None`) or gone (`after: None`). Pure.
pub(crate) fn moved(before: &RefStore, after: &RefStore) -> Moved {
    let mut moved = Moved::new();
    for (name, was) in before {
        let now = after.get(name).copied();
        if now != Some(*was) {
            moved.insert(
                name.clone(),
                RefMove {
                    before: Some(*was),
                    after: now,
                },
            );
        }
    }
    for (name, now) in after {
        if !before.contains_key(name) {
            moved.insert(
                name.clone(),
                RefMove {
                    before: None,
                    after: Some(*now),
                },
            );
        }
    }
    moved
}

/// The listing before the fetch runs, with the operation's stop. Nothing
/// has run and no entry is open: a failure here is a refusal of facts.
pub(super) fn before(repo: &RepoHandle, stop: &dyn Stop) -> Result<RefStore, ApplyError> {
    ref_store(repo, stop).map_err(|error| match error {
        RunError::Stopped => ApplyError::Stopped,
        other => ApplyError::Facts(FactsError::FetchedRefs(other)),
    })
}

/// What the fetch moved, listed once git exited. Never stopped: the fetch
/// has run, and a stop that cancelled it must not also cancel the record of
/// what it left (`record::read_after`'s rule).
///
/// When the listing fails, its error goes where its words are seen — on
/// git's own remote failure (`remote::explain` adds `which refs it moved
/// could not be read`), else as an unread after-state — and the entry
/// records `None`.
pub(super) fn after(repo: &RepoHandle, before: &RefStore, run: &mut PlanRun) -> Option<Moved> {
    match ref_store(repo, &()) {
        Ok(now) => Some(moved(before, &now)),
        Err(error) => {
            let error = match run.failure.as_mut() {
                Some(failed) => failed.carry_moves_unread(error),
                None => Some(error),
            };
            run.cleanup
                .extend(error.map(|error| AfterState::Refs(FactsError::FetchedRefs(error))));
            None
        }
    }
}

/// What a failed or stopped fetch or pull says after its failure:
/// `origin/main moved before it stopped` when its fetch moved refs, `which
/// refs it moved could not be read: <error>` when they could not be listed,
/// nothing otherwise.
pub(super) fn stopped_words(
    entry: &JournalEntry,
    moves_unread: Option<&RunError>,
) -> Option<String> {
    if let Some(error) = moves_unread {
        return Some(format!("which refs it moved could not be read: {error}"));
    }
    let moved = moved_words(entry.fetched_refs.as_ref()?)?;
    Some(format!("{moved} before it stopped"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; 20]).unwrap()
    }

    #[test]
    fn moved_names_each_moved_new_and_gone_ref_once() {
        let before = RefStore::from([
            ("refs/remotes/origin/main".to_owned(), oid(1)),
            ("refs/remotes/origin/old".to_owned(), oid(2)),
            ("refs/remotes/origin/same".to_owned(), oid(3)),
        ]);
        let after = RefStore::from([
            ("refs/remotes/origin/main".to_owned(), oid(4)),
            ("refs/remotes/origin/new".to_owned(), oid(5)),
            ("refs/remotes/origin/same".to_owned(), oid(3)),
        ]);
        let change = |before, after| RefMove { before, after };
        assert_eq!(
            moved(&before, &after),
            Moved::from([
                (
                    "refs/remotes/origin/main".to_owned(),
                    change(Some(oid(1)), Some(oid(4)))
                ),
                (
                    "refs/remotes/origin/new".to_owned(),
                    change(None, Some(oid(5)))
                ),
                (
                    "refs/remotes/origin/old".to_owned(),
                    change(Some(oid(2)), None)
                ),
            ])
        );
        assert_eq!(moved(&before, &before), Moved::new());
    }
}

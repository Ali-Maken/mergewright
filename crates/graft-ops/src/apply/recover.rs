//! What a relaunch makes of an entry a previous run left `Pending`: closed as
//! `interrupted` with the repository as it stands, unless git's own state
//! says its operation is still under way.

use graft_core::journal::EntryId;
use graft_core::oid::Oid;
use graft_git::state;

use crate::facts::{self, FactsError};
use crate::snapshot;
use crate::undo::RepoStateNow;

use super::record::anchors;
use super::resume::live_entry;
use super::{ApplyError, Context};

/// Close every entry left `Pending` by a previous run as `interrupted`,
/// recording the repository as it stands now (DESIGN.md §6.2 step 3).
///
/// Run this at launch, before the user can touch anything: the state read
/// here is recorded as the interrupted operation's after-state, so the later
/// it runs, the more of the user's own work it attributes to that operation.
/// When the repository cannot be read at all, the entry is closed with no
/// after-state — `u` then offers to restore the whole tree, and says so.
///
/// Returns what it closed **and** what it could not read, so the caller can
/// surface both: an entry closed with no after-state is one whose `u` will
/// offer to overwrite the whole working tree, and the reason for that belongs
/// on screen, not on the floor (m3/03 review, M1).
pub fn recover_interrupted(ctx: &mut Context<'_>) -> Result<Recovered, ApplyError> {
    // A conflict that survived a relaunch: git's own state says the
    // operation is still live, so its entry stays `Pending` and the banner
    // takes over (m5/07, hard choice 2). Only the newest matching entry is
    // live — an older one of the same kind was left behind by something
    // else and closes as interrupted like any other.
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let live_id = match live {
        Some(kind) => live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?
            .map(|entry| entry.id.clone()),
        None => None,
    };
    let open: Vec<(EntryId, Vec<String>, Option<String>)> = ctx
        .journal
        .unresolved()
        .into_iter()
        .filter(|entry| Some(&entry.id) != live_id.as_ref())
        .map(|entry| {
            (
                entry.id.clone(),
                entry.refs_before.keys().cloned().collect(),
                entry.backup_ref.clone(),
            )
        })
        .collect();
    let mut recovered = Recovered {
        live: live_id.into_iter().collect(),
        ..Recovered::default()
    };
    for (id, refs, backup_ref) in open {
        let read = facts::read_state_now(ctx.repo, ctx.scratch(), &refs, ctx.stop);
        // The repository being unreadable must not leave the entry `Pending`
        // for ever, which blocks `u` outright. `None` is the honest record
        // and the widest restore, and the user is asked before it runs
        // (`Drift::unknown`) — and told why, here.
        let now = match read {
            Ok(now) => match seal_recovered(ctx, &id, backup_ref.as_deref(), &now) {
                Ok(()) => Some(now),
                // The trees were read but nothing keeps them alive, so
                // recording them would name objects `git gc` will prune.
                Err(why) => {
                    recovered.unreadable.push((id.clone(), why));
                    None
                }
            },
            Err(FactsError::Stopped) => return Err(ApplyError::Stopped),
            Err(unreadable) => {
                recovered.unreadable.push((id.clone(), unreadable));
                None
            }
        };
        ctx.journal
            .mark_interrupted(&id, now.as_ref())
            .map_err(|error| ApplyError::Journal { error, after: None })?;
        recovered.closed.push(id);
    }
    Ok(recovered)
}

/// What [`recover_interrupted`] did at launch.
#[derive(Debug, Default)]
pub struct Recovered {
    /// The entries closed as `interrupted`, in journal order.
    pub closed: Vec<EntryId>,
    /// The ones whose after-state could not be read or kept reachable, and
    /// why. Each is closed with no after-state, so its `u` restores the whole
    /// tree after asking.
    pub unreadable: Vec<(EntryId, FactsError)>,
    /// The entry left `Pending` on purpose: a history operation still
    /// stopped on a conflict by git's own account (`MERGE_HEAD`,
    /// `rebase-merge/` …). The banner offers `resume`; nothing was closed
    /// (m5/07). At most one.
    pub live: Vec<EntryId>,
}

/// Hang a recovered entry's after-trees off its backup ref before they are
/// recorded, so `git gc` cannot prune what the entry is about to name.
///
/// Nothing to do when the entry took no backup: such an entry records no tree
/// (`mark_interrupted` copies its before-fields, which are `None`), and the
/// refs it names are live refs.
fn seal_recovered(
    ctx: &Context<'_>,
    id: &EntryId,
    backup_ref: Option<&str>,
    now: &RepoStateNow,
) -> Result<(), FactsError> {
    let Some(name) = backup_ref else {
        return Ok(());
    };
    let at = facts::read_refs(ctx.repo, std::slice::from_ref(&name.to_owned()))?
        .get(name)
        .copied()
        .flatten();
    let mut parents: Vec<Oid> = at.into_iter().collect();
    for anchor in anchors(Some(&now.head), &now.refs) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    let label = format!("interrupted, recovered: {id}");
    let commit = snapshot::commit_trees(
        ctx.repo,
        Some(now.index),
        Some(now.worktree_tree),
        id,
        &label,
        &parents,
        &(),
    )?;
    snapshot::move_backup_ref(ctx.repo, name, commit, &())?;
    Ok(())
}

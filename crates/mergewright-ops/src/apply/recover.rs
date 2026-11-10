//! What a relaunch makes of an entry a previous run left `Pending`: closed as
//! `interrupted` with the repository as it stands, unless git's own state
//! says its operation is still under way.

use std::path::{Path, PathBuf};

use mergewright_core::journal::{EntryId, MergeState};
use mergewright_core::oid::Oid;
use mergewright_git::state;

use crate::facts::{self, FactsError};
use crate::rewrite::saved;
use crate::snapshot;
use crate::undo::RepoStateNow;

use super::record::anchors;
use super::resume::live_entry;
use super::run::SCRATCH_INDEX;
use super::{ApplyError, Context, EditsAt, set_aside};

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
///
/// Closes nothing while any other `Journal` — another job, another window —
/// holds the writer's lock ([`crate::journal::Journal::alone`]): its
/// `Pending` entry is running, not interrupted, and a dead run's cannot be
/// told apart from it. That read still says what git has live, and lists
/// every other `Pending` entry as [`Recovered::held`] (58r).
///
/// `ctx.journal` must not hold the writer's lock itself: that is
/// [`crate::journal::JournalError::StillWriting`].
pub fn recover_interrupted(ctx: &mut Context<'_>) -> Result<Recovered, ApplyError> {
    // Only an entry whose writer is gone is interrupted. A `Pending` entry
    // whose writer is alive — an operation running in another job or
    // another window, git perhaps already done and its outcome line about
    // to land — is not, and closing it here gives it two outcomes, after
    // which every read refuses the whole journal (m8-polish-and-ship/58).
    // While any writer holds the journal, nothing is closed: the entries a
    // dead run left wait for the read that follows that writer's operation.
    // Held alone to the end, so no operation starts under the state read
    // here as the interrupted entries' after-state.
    let alone = ctx.journal.alone().map_err(ApplyError::journal)?;
    // What is live is git's own state, a read that needs no lock: a busy
    // read works it out too, or a merge standing on a conflict would be
    // reported as nothing live — and drawn as interrupted — whenever some
    // other job happened to be writing (m8-polish-and-ship/58r).
    let live = find_live_entry(ctx)?;
    let Some(_alone) = alone else {
        return Ok(hold_the_rest(ctx, live));
    };
    let open: Vec<Open> = ctx
        .journal
        .unresolved()
        .into_iter()
        .filter(|entry| Some(&entry.id) != live.as_ref())
        .map(|entry| Open {
            id: entry.id.clone(),
            refs: entry.refs_before.keys().cloned().collect(),
            backup_ref: entry.backup_ref.clone(),
            set_aside: set_aside::stash_of(entry),
        })
        .collect();
    let mut recovered = Recovered {
        live: live.into_iter().collect(),
        set_aside: edits_left(ctx, &open),
        scratch_left: sweep_scratch_indexes(ctx.scratch()),
        ..Recovered::default()
    };
    for Open {
        id,
        refs,
        backup_ref,
        ..
    } in open
    {
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
            .map_err(ApplyError::journal)?;
        // A rewrite's saved plan (ADR-0047) goes with its entry. One left
        // behind is harmless to git, but it is reported, not dropped.
        if let Err(error) = saved::remove(&saved::dir(ctx.repo, &id)) {
            recovered.plans_left.push((id.clone(), error));
        }
        recovered.closed.push(id);
    }
    Ok(recovered)
}

/// Remove every commit's index copy a dead run left in the names dir
/// (`run::ScratchIndex`; review ease-pass-04, Minor 4): index-sized, and
/// read by nothing. Called only while recovery holds the journal alone, and
/// a run holds the writer's lock for as long as its copy exists, so no copy
/// here is a live run's. The old single name, `commit-index`, goes too.
fn sweep_scratch_indexes(names_dir: &Path) -> Vec<(PathBuf, std::io::Error)> {
    let prefix = format!("{SCRATCH_INDEX}-");
    let entries = match std::fs::read_dir(names_dir) {
        Ok(entries) => entries,
        // No names dir: nothing was ever copied into it.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(why) => return vec![(names_dir.to_path_buf(), why)],
    };
    let mut left = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(why) => {
                left.push((names_dir.to_path_buf(), why));
                continue;
            }
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name != SCRATCH_INDEX && !name.starts_with(&prefix) {
            continue;
        }
        let path = entry.path();
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            // Gone already: what the sweep wanted.
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
            Err(why) => left.push((path, why)),
        }
    }
    left
}

/// The entry git's own state says is still under way, if any.
///
/// A conflict that survived a relaunch: git's own state says the operation
/// is still live, so its entry stays `Pending` and the banner takes over
/// (m5/07, hard choice 2). Only the newest matching entry is live — an older
/// one of the same kind was left behind by something else and closes as
/// interrupted like any other.
fn find_live_entry(ctx: &Context<'_>) -> Result<Option<EntryId>, ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    Ok(match live {
        Some(kind) => live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?
            .map(|entry| entry.id.clone()),
        None => None,
    })
}

/// The answer of a read that found a writer holding the journal: what git
/// has live, and every other `Pending` entry held, not closed (58r).
///
/// Said, not left for the caller to infer from an empty `closed`: each entry
/// here is `Pending` exactly as a dead run's is, and only this answer knows
/// it was not for recovery to judge.
fn hold_the_rest(ctx: &Context<'_>, live: Option<EntryId>) -> Recovered {
    let held = ctx
        .journal
        .unresolved()
        .into_iter()
        .filter(|entry| Some(&entry.id) != live.as_ref())
        .map(|entry| entry.id.clone())
        .collect();
    Recovered {
        live: live.into_iter().collect(),
        held,
        ..Recovered::default()
    }
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
    /// The `Pending` entries this read could not decide, because another
    /// `Journal` held the writer's lock (m8-polish-and-ship/58r): one of them
    /// is that writer's own, running now — and nothing tells which, so none
    /// of them is closed, and none is interrupted *yet*. A dead run's entry
    /// among them is closed by the first read after the writers let go.
    /// Empty whenever the read held the journal alone. Never holds a `live`
    /// entry.
    pub held: Vec<EntryId>,
    /// Closed entries whose rewrite plan (`mergewright/rewrite/<id>/` under the
    /// git dir) could not be removed, and why (rewrite-engine-b).
    pub plans_left: Vec<(EntryId, std::io::Error)>,
    /// Commits' index copies a dead run left in the names dir that could not
    /// be removed, and why (review ease-pass-04, Minor 4). Swept only while
    /// the read held the journal alone.
    pub scratch_left: Vec<(PathBuf, std::io::Error)>,
    /// Interrupted set-aside entries whose stash is still listed, and where
    /// their edits were when recovery read the repository (ADR-0045 §5;
    /// 30c). Only this launch knows it: the entries are closed by then, so
    /// a later launch finds nothing here.
    pub set_aside: Vec<(EntryId, EditsAt)>,
}

/// An entry recovery closes, as it reads it from the journal.
struct Open {
    id: EntryId,
    refs: Vec<String>,
    backup_ref: Option<String>,
    /// `S`, when the entry set edits aside.
    set_aside: Option<Oid>,
}

/// Where each interrupted set-aside's edits are, read once for all of them:
/// the stash list, and the status judged as the put-back judges it (every
/// tracked file equal to `HEAD`, or not).
///
/// A read that fails leaves the list empty, on purpose: the relaunch notice
/// then keeps its older words (` · edits in stash@{n}`, from the refs it
/// reads), which are true in both states. Refusing the launch over a read
/// that only sharpens a notice would cost more than it tells.
fn edits_left(ctx: &Context<'_>, open: &[Open]) -> Vec<(EntryId, EditsAt)> {
    if open.iter().all(|entry| entry.set_aside.is_none()) {
        return Vec::new();
    }
    let Ok(stashes) = mergewright_git::stash::commits(ctx.repo, &()) else {
        return Vec::new();
    };
    let Ok(status) = mergewright_git::status::read(ctx.repo, &()) else {
        return Vec::new();
    };
    let changed = set_aside::tracked_changed(&status);
    open.iter()
        .filter_map(|entry| {
            let at = left_at(entry.set_aside?, &stashes, changed)?;
            Some((entry.id.clone(), at))
        })
        .collect()
}

/// Where an interrupted set-aside's edits are. `tracked_changed`: the
/// put-back's own judge, false when every tracked file equals `HEAD`.
/// `None` when `stash` is not listed: nothing to say.
///
/// Listed with the tree at `HEAD`: the put-back never landed (the reset ran,
/// nothing came back), so the stash is the only place they are. Listed with
/// the tree changed: the put-back landed and the drop never ran, so the
/// stash is a copy.
pub(crate) fn left_at(
    stash: Oid,
    stashes: &[(Oid, usize)],
    tracked_changed: bool,
) -> Option<EditsAt> {
    let n = stashes
        .iter()
        .find_map(|(commit, at)| (*commit == stash).then_some(*at))?;
    Some(if tracked_changed {
        EditsAt::Back { copy: Some(n) }
    } else {
        EditsAt::Stash { n }
    })
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
    let merging = now.merge.iter().flat_map(MergeState::heads);
    for anchor in anchors(Some(&now.head), &now.refs)
        .into_iter()
        .chain(merging)
    {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "a failing test should panic")]

    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_hex(&format!("{byte:02x}").repeat(20)).unwrap()
    }

    /// K3 (30c): where the edits are is read from the tree, not guessed
    /// from the list.
    #[test]
    fn left_at_reads_the_tree_not_the_list() {
        let listed = [(oid(1), 0), (oid(2), 1)];
        assert_eq!(left_at(oid(3), &listed, false), None, "not listed");
        assert_eq!(left_at(oid(3), &listed, true), None, "not listed");
        assert_eq!(
            left_at(oid(2), &listed, false),
            Some(EditsAt::Stash { n: 1 }),
            "listed, tree at HEAD"
        );
        assert_eq!(
            left_at(oid(2), &listed, true),
            Some(EditsAt::Back { copy: Some(1) }),
            "listed, tree changed"
        );
    }
}

//! The two halves of the recording protocol (DESIGN.md §6.2) around the
//! run: [`begin`] takes the before-state and the backup and writes the
//! `Pending` line; [`finish`] reads the after-state, seals it behind the
//! backup ref and writes the outcome line.

use std::collections::BTreeMap;

use graft_core::journal::{
    EntryId, EntryStatus, HeadRef, JournalEntry, MergeState, OperationKind, PutBack, PutBackWhy,
    Touches,
};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::branch_config;
use crate::facts::{self, FactsError};
use crate::journal::JournalError;
use crate::keys::NoKeys;
use crate::merge_state::read_merge_state;
use crate::operation::Operation;
use crate::snapshot::{self, SnapshotError, SnapshotScope};

use super::error::joined;
use super::{AfterState, Context};

/// An entry whose `Pending` line is written and whose backup, if any, is
/// taken. The before-commit is kept so the after-snapshot can hang off it.
pub(crate) struct Pending {
    pub(crate) entry: JournalEntry,
    pub(crate) before_commit: Option<Oid>,
}

/// Why an entry could not be opened. Both `apply` and `restore` map it into
/// their own error.
#[derive(Debug, thiserror::Error)]
pub(crate) enum PrepareError {
    #[error("{0}")]
    Facts(FactsError),
    #[error("{0}")]
    Snapshot(SnapshotError),
    #[error("{0}")]
    Journal(JournalError),
    #[error("stopped")]
    Stopped,
}

/// What an entry is opened with, besides its id: everything its `Pending`
/// line says that the repository cannot be asked for.
pub(crate) struct Opening {
    pub(crate) kind: OperationKind,
    pub(crate) description: String,
    pub(crate) touches: Touches,
    /// Commits the inverse needs alive.
    pub(crate) pins: Vec<Oid>,
    /// For `Undo`/`Redo`: the entry reversed.
    pub(crate) reverses: Option<EntryId>,
    /// The stash a pop or apply puts back (ADR-0045 §1).
    pub(crate) put_back: Option<PutBack>,
}

/// Read the before-state, take the backup, write the `Pending` line.
///
/// A backup is taken when the work touches the index or the working tree
/// (ADR-0009) **or** when it has pins: a pin is a commit the inverse needs
/// alive, and the backup commit's parents are the only thing keeping it that
/// way once `git gc` runs. `snapshot::scope` stays the one knob hard choice 3
/// names; pins are an executor concern and sit beside it.
pub(crate) fn begin(
    ctx: &mut Context<'_>,
    id: EntryId,
    opening: Opening,
) -> Result<Pending, PrepareError> {
    if ctx.stop.should_stop() {
        return Err(PrepareError::Stopped);
    }
    let Opening {
        kind,
        description,
        touches,
        pins,
        reverses,
        put_back,
    } = opening;
    let head_before = facts::head_ref(ctx.repo).map_err(PrepareError::Facts)?;
    let refs_before = facts::read_refs(ctx.repo, &touches.refs).map_err(PrepareError::Facts)?;
    let merge_before = merge_before(ctx)?;
    let config_before = config_before(ctx, &touches.refs)?;

    let mut backup = None;
    if snapshot::scope(&touches) == SnapshotScope::Full || !pins.is_empty() {
        // The snapshot's parents are the pins **and** every commit the
        // entry's before-fields name: the pre-amend commit, the branch tip a
        // commit is about to move off. Both are otherwise reachable only
        // through a reflog, which `gc.reflogExpireUnreachable` drops after 30
        // days, and after that undoing an amend would lose the commit
        // outright (m3/03 review, C1). Every name here is a branch or
        // `refs/stash`, so every value is a commit git will take as a parent.
        let parents = with_anchors(
            pins.clone(),
            Some(&head_before),
            &refs_before,
            merge_before.as_ref(),
        );
        let taken = take_backup(ctx, &id, kind, &description, &parents, &head_before)?;
        backup = Some(taken);
    }

    let entry = JournalEntry {
        backup_ref: backup.map(|_| backup_name(ctx, &id)),
        id,
        at_unix_ms: ctx.now_unix_ms,
        kind,
        description,
        status: EntryStatus::Pending,
        touches,
        head_before,
        head_after: None,
        refs_before,
        refs_after: BTreeMap::default(),
        index_before: backup.map(|taken| taken.index),
        index_after: None,
        worktree_before: backup.map(|taken| taken.worktree_tree),
        worktree_after: None,
        worktree_raw_before: backup.and_then(|taken| taken.worktree_raw),
        pins,
        commands: Vec::new(),
        reverses,
        merge_before,
        merge_after: None,
        config_before,
        config_after: BTreeMap::default(),
        put_back,
        fetched_refs: None, // outcome line only, read around git's run (ADR-0062)
    };
    ctx.journal
        .append_pending(&entry)
        .map_err(PrepareError::Journal)?;
    Ok(Pending {
        entry,
        before_commit: backup.map(|taken| taken.commit),
    })
}

/// Git's merge files, read on every entry whatever it touches: they are
/// state the inverse puts back where they changed (m6-conflicts/20).
fn merge_before(ctx: &Context<'_>) -> Result<Option<MergeState>, PrepareError> {
    read_merge_state(ctx.repo).map_err(|why| PrepareError::Facts(FactsError::MergeState(why)))
}

/// The config of each branch the entry touches, so the inverse can put back
/// what the operation writes there — a tracking checkout's upstream
/// (review m7-16b..21b, Major 1).
fn config_before(
    ctx: &Context<'_>,
    refs: &[String],
) -> Result<BTreeMap<String, Vec<String>>, PrepareError> {
    branch_config::read(ctx.repo, refs, ctx.stop).map_err(|why| match why {
        RunError::Stopped => PrepareError::Stopped,
        other => PrepareError::Facts(FactsError::BranchConfig(other)),
    })
}

/// The before-snapshot under this worktree's backup namespace, labelled with
/// the entry's kind and description, measured against `head_before`'s commit.
fn take_backup(
    ctx: &Context<'_>,
    id: &EntryId,
    kind: OperationKind,
    description: &str,
    parents: &[Oid],
    head_before: &HeadRef,
) -> Result<snapshot::Snapshot, PrepareError> {
    let label = format!("{}: {description}", kind_name(kind));
    let head = head_commit(ctx, head_before)?;
    snapshot::take(
        ctx.repo,
        ctx.scratch(),
        id,
        &ctx.worktree,
        &label,
        parents,
        head,
        ctx.stop,
    )
    .map_err(PrepareError::Snapshot)
}

/// The commit `HEAD` names, which the snapshot measures "uncommitted"
/// against (ADR-0052); `None` on an unborn branch. Read in-process, no spawn.
///
/// A branch that reads as absent here — deleted between the two reads — is
/// taken as unborn: every file then counts as uncommitted, so the snapshot
/// keeps more raw, never less.
fn head_commit(ctx: &Context<'_>, head: &HeadRef) -> Result<Option<Oid>, PrepareError> {
    match head {
        HeadRef::Detached(commit) => Ok(Some(*commit)),
        HeadRef::Unborn(_) => Ok(None),
        HeadRef::Branch(name) => {
            let mut read = facts::read_refs(ctx.repo, std::slice::from_ref(name))
                .map_err(PrepareError::Facts)?;
            Ok(read.remove(name).flatten())
        }
    }
}

/// Read the after-state, keep it reachable, write the outcome line.
///
/// Every after-field the repository will give up is recorded, on abort as
/// much as on success: `refs_after` for every key of `refs_before`,
/// `head_after`, and each tree the entry touches. The outcome line is written
/// either way, since an entry with no outcome reads as an operation still in
/// flight, which this one is not — but a field that could not be read is
/// **returned**, never dropped. A missing tree costs the user a wider `u`
/// behind a confirm box; missing refs cost them an `u` that refuses
/// (`generic_steps` reads them as `Incomplete`), and either way they are told.
///
/// Then the after-state is sealed: an after-commit over whatever trees were
/// readable, parented on the before-commit and on every commit the
/// after-fields name, with the backup ref moved to it. **Whenever there is a
/// backup ref**, not only for worktree operations — an index-only operation's
/// `index_after` is a loose tree no commit holds, and `git gc` prunes it two
/// weeks later, long after the entry promised the operation could be undone
/// (m3/03 review, C1).
pub(crate) fn finish(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    status: EntryStatus,
    commands: Vec<String>,
) -> Result<Vec<AfterState>, JournalError> {
    let entry = &mut pending.entry;
    entry.commands = commands;
    let mut unread = read_after(ctx, entry);
    if let Some(before) = pending.before_commit {
        unread.extend(seal_after(ctx, entry, before));
    }

    entry.status = match status {
        EntryStatus::Aborted { reason } if !unread.is_empty() => EntryStatus::Aborted {
            // A journal record names no key: keyless words (m8/10e).
            reason: format!("{reason}; {}", joined(&unread, &NoKeys)),
        },
        other => other,
    };
    ctx.journal.append_outcome(entry)?;
    Ok(unread)
}

/// Read every after-field the repository will give up, and say which it
/// would not.
///
/// The operation has already run, so the reads never stop: a stop that
/// cancelled it mid-plan (Esc during a slow pop) must not also cancel the
/// record of what it left behind, which would record a perfectly readable
/// repository as unknown and force the widest restore (m3/03 review, M1).
fn read_after(ctx: &Context<'_>, entry: &mut JournalEntry) -> Vec<AfterState> {
    let never = &();
    let mut unread = Vec::new();
    match facts::head_ref(ctx.repo) {
        Ok(head) => entry.head_after = Some(head),
        Err(why) => unread.push(AfterState::Head(why)),
    }
    match facts::read_refs(ctx.repo, &entry.touches.refs) {
        Ok(refs) => entry.refs_after = refs,
        Err(why) => unread.push(AfterState::Refs(why)),
    }
    if entry.touches.index {
        match snapshot::index_state(ctx.repo, ctx.scratch(), never) {
            // An unmerged index is a state, not an absence: a conflicted pop
            // records its stage tree here and is undoable from it
            // (m6-conflicts/08).
            Ok(index) => entry.index_after = Some(index),
            Err(why) => unread.push(AfterState::Index(why)),
        }
    } else {
        entry.index_after = entry.index_before;
    }
    if entry.touches.worktree {
        match snapshot::worktree_tree(ctx.repo, ctx.scratch(), never) {
            Ok(tree) => entry.worktree_after = Some(tree),
            Err(why) => unread.push(AfterState::Worktree(why)),
        }
    } else {
        entry.worktree_after = entry.worktree_before;
    }
    match read_merge_state(ctx.repo) {
        Ok(merge) => entry.merge_after = merge,
        Err(why) => unread.push(AfterState::MergeState(why)),
    }
    match branch_config::read(ctx.repo, &entry.touches.refs, never) {
        Ok(config) => entry.config_after = config,
        Err(why) => unread.push(AfterState::BranchConfig(FactsError::BranchConfig(why))),
    }
    unread
}

/// Hang the after-state off the entry's backup ref.
///
/// The commit's parents are the before-commit — which carries the
/// before-trees and the pins — and every commit the after-fields name, so one
/// ref keeps everything the entry says reachable. A failure drops the
/// freshly-read trees from the entry: a tree nothing reaches is a tree
/// `git gc` prunes, and an entry naming one is a promise that expires. The
/// before-fields stay, because the ref still holds the before-commit.
fn seal_after(ctx: &Context<'_>, entry: &mut JournalEntry, before: Oid) -> Option<AfterState> {
    let name = entry.backup_ref.clone()?;
    // The ref's **current** target, not the pre-operation commit: a stopped
    // operation's folded steps chain commits onto it
    // ([`snapshot::chain_backup`]), and parenting the pre-operation commit
    // would leave every one of them unreachable the moment the ref moved
    // here.
    //
    // A ref that cannot be read is reported and nothing is sealed: parenting
    // `before` on a guess would drop those folded steps (m7-power-tools/01a,
    // gap :601). An **absent** ref is not a failed read: there is no chain
    // to keep, and `before` — which `begin` wrote — is the whole backup.
    let tip = match facts::read_refs(ctx.repo, std::slice::from_ref(&name)) {
        Ok(refs) => refs.get(&name).copied().flatten().unwrap_or(before),
        Err(why) => {
            drop_after_trees(entry);
            return Some(AfterState::BackupUnread(why));
        }
    };
    let parents = with_anchors(
        vec![tip],
        entry.head_after.as_ref(),
        &entry.refs_after,
        entry.merge_after.as_ref(),
    );
    let label = format!("{} after: {}", kind_name(entry.kind), entry.description);
    let sealed = snapshot::commit_trees(
        ctx.repo,
        entry.index_after,
        entry.worktree_after,
        &entry.id,
        &label,
        &parents,
        &(),
    )
    .and_then(|commit| snapshot::move_backup_ref(ctx.repo, &name, commit, &()));
    match sealed {
        Ok(()) => None,
        Err(why) => {
            drop_after_trees(entry);
            Some(AfterState::Unsealed(why))
        }
    }
}

/// Forget the after-trees of an entry whose after-state was not sealed: a
/// tree nothing reaches is one `git gc` prunes, and an entry naming it is a
/// promise that expires. The before-fields stay; the ref still holds them.
fn drop_after_trees(entry: &mut JournalEntry) {
    if entry.touches.index {
        entry.index_after = None;
    }
    if entry.touches.worktree {
        entry.worktree_after = None;
    }
}

/// A backup commit's parents: `first` (the pins, or the ref's current tip),
/// then every [`anchors`] commit and every commit `MERGE_HEAD` names (all
/// of an octopus merge's, m6-conflicts/38), each once.
///
/// `MERGE_HEAD`'s commit is anchored because a `git pull <url> <branch>`
/// leaves one that only `FETCH_HEAD` reaches, and `u` of the finish has to
/// write it back after `gc` (m6-conflicts/20).
fn with_anchors(
    first: Vec<Oid>,
    head: Option<&HeadRef>,
    refs: &BTreeMap<String, Option<Oid>>,
    merge: Option<&MergeState>,
) -> Vec<Oid> {
    let mut parents = first;
    let merging = merge.into_iter().flat_map(MergeState::heads);
    for anchor in anchors(head, refs).into_iter().chain(merging) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    parents
}

/// Every commit an entry's `HEAD`/refs fields name.
///
/// These are ids the journal records, so keeping them reachable is the backup
/// ref's job: once the operation (or its undo) moves the ref off them, git's
/// reflog is all that holds them, and git expires that.
pub(super) fn anchors(head: Option<&HeadRef>, refs: &BTreeMap<String, Option<Oid>>) -> Vec<Oid> {
    let mut found = Vec::new();
    if let Some(HeadRef::Detached(id)) = head {
        found.push(*id);
    }
    for id in refs.values().flatten() {
        if !found.contains(id) {
            found.push(*id);
        }
    }
    found
}

/// The commits an operation is about to make unreachable, and which its
/// inverse therefore needs pinned (rule 2 of the ops-model contract).
pub(super) fn pins_for(op: &Operation) -> Vec<Oid> {
    match op {
        Operation::StashPop { commit, .. } | Operation::StashDrop { commit, .. } => vec![*commit],
        // Finishing a clash drops the stash: `u` stores it back (ADR-0045 §3).
        Operation::DropPutBack { stash, .. } => vec![*stash],
        // The stash `stash branch` drops: undo stores it back (m5/07).
        Operation::BranchFromStash(params) => vec![params.commit],
        // The commit objects written from the index: nothing but the entry
        // holds them until git has melded them in (ADR-0047).
        // Absorb's index base too: the put-back reads it when the entry
        // closes, perhaps after a relaunch (06b).
        Operation::Rewrite(params) => params
            .made
            .iter()
            .copied()
            .chain(params.index_base)
            .collect(),
        // A branch put back moves off a tip nothing else may hold.
        Operation::Recover(params) => crate::operation::recover_pins(params),
        Operation::Merge(_)
        | Operation::Rebase(_)
        | Operation::CherryPick(_)
        | Operation::Revert(_)
        | Operation::Reset(_)
        | Operation::Checkout(_)
        | Operation::BranchCreate(_)
        | Operation::BranchDelete(_)
        | Operation::TagCreate(_)
        | Operation::TagDelete(_)
        | Operation::Stage(_)
        | Operation::Unstage(_)
        | Operation::Discard(_)
        | Operation::Commit(_)
        | Operation::Amend(_)
        | Operation::StashPush(_)
        | Operation::StashApply { .. }
        | Operation::Fetch(_)
        | Operation::Pull(_)
        | Operation::Push(_)
        // Nothing becomes unreachable: the stages the resolution drops live
        // in the index, not in a commit anything could collect.
        | Operation::ResolveWholeFile(_)
        | Operation::RestartConflict(_)
        // `MERGE_HEAD`'s commit is anchored by the backup from the entry's
        // `merge_before` (`begin`), not pinned.
        | Operation::FinishMerge(_)
        | Operation::AbortMerge(_) => Vec::new(),
    }
}

/// The stash an operation puts back into the working tree, written on its
/// `Pending` line (ADR-0045 §1): a pop's or apply's own stash commit. `None`
/// for every other operation.
pub(super) fn put_back_for(op: &Operation) -> Option<PutBack> {
    let (stash, why) = match op {
        Operation::StashPop { commit, .. } => (*commit, PutBackWhy::Pop),
        Operation::StashApply { commit, .. } => (*commit, PutBackWhy::Apply),
        _ => return None,
    };
    Some(PutBack { stash, why })
}

/// The backup ref this context's entry `id` is kept under, in the
/// repository's names in force — the same name `snapshot::take` writes.
fn backup_name(ctx: &Context<'_>, id: &EntryId) -> String {
    snapshot::backup_ref_name(ctx.repo.on_disk_names(), &ctx.worktree, id)
}

/// The kind as it reads in a backup commit's message — and, in the app,
/// what a set-aside's `u` takes back with the edits (graft
/// `update::clash::undone_with_edits`, m8-polish-and-ship/19d-a2).
#[must_use]
pub fn kind_name(kind: OperationKind) -> &'static str {
    match kind {
        OperationKind::Stage => "stage",
        OperationKind::Unstage => "unstage",
        OperationKind::Discard => "discard",
        OperationKind::Commit => "commit",
        OperationKind::Amend => "amend",
        OperationKind::StashPush => "stash push",
        OperationKind::StashApply => "stash apply",
        OperationKind::StashPop => "stash pop",
        OperationKind::StashDrop => "stash drop",
        OperationKind::Fetch => "fetch",
        OperationKind::Pull => "pull",
        OperationKind::Push => "push",
        OperationKind::Merge => "merge",
        OperationKind::Rebase => "rebase",
        OperationKind::Rewrite => "rewrite",
        OperationKind::CherryPick => "cherry-pick",
        OperationKind::Revert => "revert",
        OperationKind::Reset => "reset",
        OperationKind::Checkout => "checkout",
        OperationKind::BranchCreate => "branch",
        OperationKind::BranchDelete => "delete branch",
        OperationKind::TagCreate => "tag",
        OperationKind::TagDelete => "delete tag",
        OperationKind::BranchFromStash => "branch from stash",
        OperationKind::ResolveConflict => "resolve conflict",
        OperationKind::RestartConflict => "restart conflict",
        OperationKind::FinishMerge => "finish merge",
        OperationKind::AbortMerge => "abort merge",
        OperationKind::Recover => "recover",
        OperationKind::RevertTo => "revert to",
        OperationKind::Undo => "undo",
        OperationKind::Redo => "redo",
        OperationKind::Migrate => "rename",
    }
}

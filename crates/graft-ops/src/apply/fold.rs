//! The steps a user takes *while* a merge, rebase, cherry-pick or revert is
//! stopped: whether one folds into the operation's entry, and running it
//! when it does.
//!
//! M5 hard choice 2 and DESIGN.md §6.4: a stopped operation is **one entry**,
//! `Pending` from the stop to the finish. Every step inside the span folds
//! into that entry and writes no line of its own, so `u` after the finish
//! takes the whole span back. An operation begun in the shell has no such
//! entry: there, a step that can write the working tree is its own entry
//! instead ([`folded_step`]).

use std::collections::BTreeMap;

use graft_core::journal::{EntryStatus, JournalEntry};

use crate::operation::{Invocation, Operation, RepoFacts};
use crate::snapshot;

use super::resume::live_entry;
use super::run::{PlanRun, run_plan};
use super::{Applied, ApplyError, Context, Refresh, RestartPrep, as_apply_error, run_restart_prep};

/// Run `op` folded into the operation git already has in progress, or answer
/// `None`: it is not a folded step, and the caller journals it as an entry of
/// its own.
///
/// Folding is for a span Graft opened: the step's line joins that entry and a
/// chained backup keeps what it overwrites. An operation **begun in the
/// shell** — a `git pull` that conflicted — has no entry of Graft's, so there
/// is no span to join and no backup ref to chain onto. A step there that can
/// write the working tree (a whole-file take, a restart) would run with no
/// backup at all, and a hand edit it overwrote would be gone. So it is its
/// own entry, with its own backup, exactly as it is when a conflicted stash
/// pop leaves nothing live (m6-conflicts/03 scope 2: "folds into the live
/// entry when one is live; with no live entry it is its own entry") — which
/// is what closes the m6 g04/g08 review's M1.
///
/// A step that writes only the index still folds into nothing there, as it
/// did before: `git add` and `restore --staged` move no byte on disk.
pub(super) fn folded_step(
    ctx: &mut Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
    prep: &RestartPrep,
) -> Result<Option<Applied>, ApplyError> {
    let Some(live) = facts.in_progress else {
        return Ok(None);
    };
    let span = live_entry(ctx.repo, ctx.journal.entries(), live, ctx.stop)?.cloned();
    if span.is_none() && op.touches(facts).worktree {
        return Ok(None);
    }
    inside_live_operation(ctx, op, facts, span.as_ref(), prep).map(Some)
}

/// Run a step that belongs to an operation git already has in progress.
///
/// `check` admits `Stage`, `Unstage` and the two conflict operations while a
/// merge, rebase, cherry-pick or revert is open — the resolver's steps. Such a
/// step is **part of that operation, not an operation of its own**: DESIGN
/// §6.4 ("a mid-flight operation … is not a journal entry yet … the undo
/// journal picks up once there's a finished operation to reverse") and M5
/// hard choice 2 (one entry, `Pending` from the stop to the finish). So it
/// runs, its command line joins the live entry, and it writes **no journal
/// line of its own**. `u` after the finish takes the whole span back.
///
/// Journaling it as an ordinary entry is what killed `u` outright (m5/07
/// review, C1): `write-tree` has no tree for an unmerged index, so the
/// entry's `index_before` was `None` and its inverse `NoBackup` — and undo
/// is linear, so the merge behind it became unreachable. The rule is "an
/// operation is in progress", not "the index cannot be snapshotted": the
/// last file a user resolves leaves the index clean, and that step must fold
/// into the span like every one before it.
///
/// **Writing no line is not the same as keeping no backup.** This used to say
/// nothing was at risk here, on the grounds that every admitted step wrote
/// only the index. That stopped being true when the resolver landed: a
/// whole-file take is `checkout --ours|--theirs` + `add`, or `rm --quiet`,
/// and a restart is `checkout --merge` — all of them write the working tree
/// (`touches.worktree`), and all of them are admitted here. A hand edit made
/// in `$EDITOR` and then resolved the other way was overwritten with no
/// confirmation, no line and no snapshot, and the span's own entry predates
/// the edit (m6 g03 review, M2).
///
/// So the span's entry stays one entry, and every step whose `touches` names
/// the index or the working tree takes a **chained backup commit** first
/// ([`snapshot::chain_backup`], Decision 2 of m6-conflicts/08): the
/// repository as it stands, hung off the entry's own backup ref, no journal
/// line. It is what `d` reads back after `⏎`, and it is the recovery point
/// for that hand edit.
///
/// An operation begun in the shell has no entry to chain onto, so
/// [`folded_step`] never sends a step that can write the working tree here:
/// that one is its own entry (m6 g04/g08 review, M1). What does arrive with
/// `span` `None` writes the index alone.
fn inside_live_operation(
    ctx: &mut Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
    span: Option<&JournalEntry>,
    prep: &RestartPrep,
) -> Result<Applied, ApplyError> {
    // Before anything runs: the snapshot that keeps what the step is about
    // to overwrite.
    let touches = op.touches(facts);
    let step_backup = match span {
        Some(entry) if touches.index || touches.worktree => {
            snapshot::chain_backup(ctx, entry, snapshot::LinkKind::Step).map_err(as_apply_error)?
        }
        // A step that writes neither, or an index-only step inside an
        // operation begun in the shell: `git add` and `restore --staged`
        // leave every byte on disk where it was, and git's own state is the
        // whole record of an operation Graft did not begin.
        _ => None,
    };

    // A restart of a path `⏎` already staged needs its stages back in the
    // index before `checkout --merge` has anything to merge (scope 5(c)).
    // After the chained backup, so what it is about to overwrite is kept.
    // `prepare_restart` decided this before a byte was written; this is the
    // write half.
    let mut commands = run_restart_prep(ctx, prep)?;

    let plan = op.plan(facts);
    commands.extend(plan.iter().map(Invocation::command_line));
    let PlanRun { failure, .. } = run_plan(ctx, &plan, op.remote_names().as_ref());
    fold_commands(ctx, span, commands.clone());

    let mut record = JournalEntry {
        id: ctx.next_id(),
        at_unix_ms: ctx.now_unix_ms,
        kind: op.kind(),
        description: op.describe(),
        status: EntryStatus::Committed,
        touches,
        head_before: facts.head.clone(),
        head_after: Some(facts.head.clone()),
        refs_before: BTreeMap::default(),
        refs_after: BTreeMap::default(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: Vec::new(),
        commands,
        reverses: None,
    };
    match failure {
        None => Ok(Applied {
            entry: record,
            refresh: Refresh::WorkingTree,
            step_backup,
        }),
        Some(failed) => {
            record.status = EntryStatus::Aborted {
                reason: failed.reason(),
            };
            Err(failed.into_error(record))
        }
    }
}

/// Record a folded step on the live entry: its command line after the ones
/// already held, so the Command Log shows the span in the order it happened.
///
/// A `Pending` entry's commands live in memory until its outcome line is
/// written, so a caller that opens a fresh journal per job sees only the
/// step it just ran (the same limitation the stop itself has — the plan's
/// own lines do not reach disk either until `resume` closes the entry).
///
/// `None` is an operation begun outside Graft: there is no entry of Graft's
/// to record the step on. Nothing is dropped that was ever ours — git's own
/// state is the whole record of that operation, and the step is reported to
/// the caller on the returned `Applied`.
fn fold_commands(ctx: &mut Context<'_>, span: Option<&JournalEntry>, commands: Vec<String>) {
    let Some(entry) = span else {
        return;
    };
    let mut held = entry.commands.clone();
    held.extend(commands);
    ctx.journal.hold_commands(&entry.id, held);
}

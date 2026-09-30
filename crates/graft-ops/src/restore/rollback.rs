//! Putting back what a cancelled restore had already done.
//!
//! The one rule the whole module exists for: a cancelled undo is
//! all-or-nothing. Every reversal is built from the *entry's own*
//! before-fields and its own backup ref — both read by `begin` before the
//! first step — and every ref goes back by compare-and-swap against the value
//! this plan's own step wrote, so a refusal means something outside Graft
//! moved it and is reported rather than clobbered. Where the repository will
//! not say what happened, nothing is written at all and the entry says so.

use std::collections::BTreeMap;

use graft_core::journal::{HeadRef, IndexState, MergeState};
use graft_core::oid::Oid;
use graft_git::run::Stop;

use crate::apply::Context;
use crate::facts;
use crate::undo::Step;

use super::RestoreError;
use super::steps::run_step;

/// The restore's own before-state: where a roll-back puts things back to.
///
/// Every field is the undo entry's, written by `apply::begin` before the
/// first step ran, so it is the pre-undo repository and not the pre-operation
/// one.
pub(super) struct Before {
    pub(super) refs: BTreeMap<String, Option<Oid>>,
    pub(super) head: HeadRef,
    pub(super) index: Option<IndexState>,
    pub(super) worktree: Option<Oid>,
    /// The same before-snapshot's `raw/` (ADR-0052): his bytes where git's
    /// checkout of `worktree` would not give them back. `None`: none kept.
    pub(super) worktree_raw: Option<Oid>,
    /// git's merge state when the restore began. `None` is a recorded state
    /// here ("not merging"), not a gap, so it never reads as `NoBefore`.
    pub(super) merge: Option<MergeState>,
    /// `branch.<name>.*` of every branch the restore touches, when it began.
    /// A key not in the map was not set: a recorded state, not a gap.
    pub(super) config: BTreeMap<String, Vec<String>>,
}

/// What a cancelled restore left behind.
pub(super) enum Cancelled {
    /// Every step that ran was put back; the repository is where it started.
    Clean(usize),
    /// The roll-back could not finish. `applied` names what is still in
    /// force, newest first.
    Partway {
        applied: Vec<String>,
        why: RestoreError,
    },
}

/// How far a roll-back got.
enum RolledBack {
    /// All of them, reversed newest first.
    All(usize),
    /// It refused at a step; `still` is that step and everything older.
    Partway {
        still: Vec<String>,
        why: RestoreError,
    },
}

/// Whether a step the runner killed git inside of actually landed.
enum Settled {
    /// It did, so it has to be reversed.
    Applied,
    /// It did not, so there is nothing to reverse.
    NotApplied,
    /// The repository would not say.
    Unknown(RestoreError),
}

/// Put back what the plan had already done, and say whether it managed to.
///
/// The roll-back is not itself cancellable — it runs against `&()` — because
/// a cancel of the cancel would strand the repository half-way through an
/// undo, which is the one state this exists to prevent.
pub(super) fn roll_back_after_stop<'p>(
    ctx: &mut Context<'_>,
    before: &Before,
    applied: &mut Vec<&'p Step>,
    stopped_in: Option<&'p Step>,
    commands: &mut Vec<String>,
) -> Cancelled {
    uncancellable(ctx, |ctx| {
        if let Some(step) = stopped_in {
            match settled(ctx, step, before) {
                Settled::Applied => applied.push(step),
                Settled::NotApplied => {}
                // Nothing is rolled back here, deliberately — not even the older
                // steps this does know about.
                //
                // Two ways to get here, and the argument differs. `Facts`: the
                // repository would not answer, so every aim is a guess and a
                // roll-back is a write. `RefMoved`: the read succeeded and said
                // something outside this plan moved the ref — the same case
                // `roll_back` stops at on its first refusal, so stopping here is
                // the same rule applied one step earlier (ticket scope 2).
                //
                // The cost is real and is the user's: the older steps are
                // known-applied and their reversals are compare-and-swapped or
                // idempotent, so leaving them in force is caution, not safety.
                // The entry says so, `drift` explains it, and `U` restores the
                // backup. Reversing the known ones is the open question here.
                Settled::Unknown(why) => {
                    let mut still = vec![step_name(step)];
                    still.extend(newest_first(applied));
                    return Cancelled::Partway {
                        applied: still,
                        why,
                    };
                }
            }
        }

        match roll_back(ctx, before, applied, commands) {
            RolledBack::All(count) => Cancelled::Clean(count),
            RolledBack::Partway { still, why } => Cancelled::Partway {
                applied: still,
                why,
            },
        }
    })
}

/// Put back what the plan had already done when a step **failed** (gap
/// :496, m7-power-tools/01a): the same all-or-nothing rule as a cancel, so a
/// failure never leaves half a restore behind in silence.
///
/// The failed step itself is reversed too, except a ref's. `git update-ref`
/// is atomic — a non-zero exit wrote nothing — and its reversal is a
/// compare-and-swap against the value it would have written, which would
/// refuse and read as drift that is not there. Every other step may have
/// written part of itself (a working-tree restore is several commands), and
/// its reversal is idempotent by construction (see [`settled`]), so putting
/// it back costs nothing when it wrote nothing.
///
/// Not cancellable, for the reason [`roll_back_after_stop`] gives.
pub(super) fn roll_back_after_failure<'p>(
    ctx: &mut Context<'_>,
    before: &Before,
    applied: &mut Vec<&'p Step>,
    failed: &'p Step,
    commands: &mut Vec<String>,
) -> Cancelled {
    if !matches!(failed, Step::UpdateRef { .. }) {
        applied.push(failed);
    }
    uncancellable(ctx, |ctx| match roll_back(ctx, before, applied, commands) {
        RolledBack::All(count) => Cancelled::Clean(count),
        RolledBack::Partway { still, why } => Cancelled::Partway {
            applied: still,
            why,
        },
    })
}

/// Run `work` against a context that cannot be stopped, and give the caller's
/// stop back whatever `work` does.
///
/// A cancel of the cancel would strand the repository half-way through an
/// undo, which is the one state this module exists to prevent — and the
/// hand-back has to happen on *every* path out, including the early ones. A
/// closure is what makes that structural rather than something to remember.
fn uncancellable<T>(ctx: &mut Context<'_>, work: impl FnOnce(&mut Context<'_>) -> T) -> T {
    let never: &dyn Stop = &();
    let asked = std::mem::replace(&mut ctx.stop, never);
    let done = work(ctx);
    ctx.stop = asked;
    done
}

/// Reverse `applied`, newest first, stopping at the first step that refuses.
///
/// Newest first is what makes the compare-and-swaps meaningful: each ref is
/// swapped against the value this plan's own step wrote, so a refusal means
/// something outside Graft moved it in between — which is reported, never
/// clobbered.
fn roll_back(
    ctx: &mut Context<'_>,
    before: &Before,
    applied: &[&Step],
    commands: &mut Vec<String>,
) -> RolledBack {
    let mut done = 0;
    for (at, step) in applied.iter().enumerate().rev() {
        let reverse = match reverse_of(step, before) {
            Ok(reverse) => reverse,
            Err(why) => {
                return RolledBack::Partway {
                    still: newest_first(&applied[..=at]),
                    why,
                };
            }
        };
        match run_step(ctx, &reverse, commands) {
            Ok(()) => done += 1,
            Err(why) => {
                return RolledBack::Partway {
                    still: newest_first(&applied[..=at]),
                    why,
                };
            }
        }
    }
    RolledBack::All(done)
}

/// The step that puts `step` back, from the entry's own before-state.
fn reverse_of(step: &Step, before: &Before) -> Result<Step, RestoreError> {
    Ok(match step {
        Step::UpdateRef { name, to, .. } => {
            let back = *before
                .refs
                .get(name)
                .ok_or_else(|| RestoreError::NoBefore {
                    field: name.clone(),
                })?;
            Step::UpdateRef {
                name: name.clone(),
                // Swap against what this step wrote, not against what the
                // plan expected: the plan's `expect` is the operation's
                // after-value, which is where the ref is going, not where it
                // is.
                expect: *to,
                to: back,
            }
        }
        Step::SetHead(_) => Step::SetHead(before.head.clone()),
        // The same paths the forward step could have written — a roll-back
        // is never wider than what it undoes — read out of the entry's own
        // backup tree instead of the operation's.
        // `NoBefore` can no longer arise here from an unmerged index: one
        // has a state of its own now (m6-conflicts/08). It stays for an
        // entry that recorded nothing at all, which is what a restore of an
        // operation that never touched the index would be.
        Step::RestoreIndex { paths, .. } => Step::RestoreIndex {
            index: before.index.ok_or_else(|| RestoreError::NoBefore {
                field: "the index".to_owned(),
            })?,
            paths: paths.clone(),
        },
        Step::RestoreWorktree { paths, .. } => Step::RestoreWorktree {
            tree: before.worktree.ok_or_else(|| RestoreError::NoBefore {
                field: "the working tree".to_owned(),
            })?,
            paths: paths.clone(),
            // The restore's own `raw/`, beside its own tree: never the
            // forward step's, which belongs to the operation's snapshot.
            raw: before.worktree_raw,
        },
        Step::StashStore { commit } => Step::StashDrop { commit: *commit },
        Step::StashDrop { commit } => Step::StashStore { commit: *commit },
        // Idempotent like the restores: writing the state the restore found
        // is right whether or not the forward step landed.
        Step::SetMergeState(_) => Step::SetMergeState(before.merge.clone()),
        // The same: the values the restore found. A key the entry's
        // before-read did not find was unset, so the reverse unsets it.
        Step::SetBranchConfig { key, .. } => Step::SetBranchConfig {
            key: key.clone(),
            to: before.config.get(key).cloned().unwrap_or_default(),
        },
    })
}

/// Read the repository to settle whether a killed step landed.
///
/// Only a [`Step::UpdateRef`] needs asking. Every other reversal is
/// idempotent by construction — a restore from the entry's own backup writes
/// the same bytes whether or not the step ran, `steps::stash_store` scans the
/// reflog first and `steps::stash_drop` is a no-op when the reflog lacks the
/// commit — so reversing one that never ran costs nothing. A ref is
/// different: its reversal is a compare-and-swap against what the step wrote,
/// and running that on a step that never ran would refuse and be reported as
/// drift that is not there.
fn settled(ctx: &mut Context<'_>, step: &Step, before: &Before) -> Settled {
    let Step::UpdateRef { name, to, .. } = step else {
        return Settled::Applied;
    };
    // `read_refs` reads the ref store directly, so there is no command line
    // to record for it.
    let now = match facts::read_refs(ctx.repo, std::slice::from_ref(name)) {
        Ok(found) => found.get(name).copied().flatten(),
        Err(why) => return Settled::Unknown(RestoreError::from_facts(why)),
    };
    // A name the entry's before-state does not record at all is the same
    // nothing-to-put-it-back-to that `reverse_of` calls `NoBefore`, and it
    // must read the same way here: treating the absence as "it did not
    // exist" would settle a killed delete-step `NotApplied` on no evidence.
    // Impossible by construction — `begin` reads every ref the plan touches —
    // which is why it is reported rather than handled.
    let Some(found_at) = before.refs.get(name) else {
        return Settled::Unknown(RestoreError::NoBefore {
            field: name.clone(),
        });
    };
    if now == *to {
        Settled::Applied
    } else if now == *found_at {
        Settled::NotApplied
    } else {
        // Neither where the step would have put it nor where the restore
        // found it: something outside this plan moved it.
        Settled::Unknown(RestoreError::RefMoved { name: name.clone() })
    }
}

/// What a step touches, in the words the status line uses.
fn step_name(step: &Step) -> String {
    match step {
        Step::UpdateRef { name, .. } => name.clone(),
        Step::SetHead(_) => "HEAD".to_owned(),
        Step::RestoreIndex { .. } => "the index".to_owned(),
        Step::RestoreWorktree { .. } => "the working tree".to_owned(),
        Step::StashStore { .. } | Step::StashDrop { .. } => "the stash".to_owned(),
        Step::SetMergeState(_) => "the merge in progress".to_owned(),
        Step::SetBranchConfig { key, .. } => key.clone(),
    }
}

fn newest_first(steps: &[&Step]) -> Vec<String> {
    steps.iter().rev().map(|step| step_name(step)).collect()
}

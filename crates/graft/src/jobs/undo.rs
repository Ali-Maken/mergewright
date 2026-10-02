//! `u`, `U` and the journal read: the other place `app` opens the undo
//! journal and hands it to `ops` (`jobs::operate` is the first).
//!
//! The seam rule (ARCHITECTURE.md, `crates/graft/tests/it/seam_rule.rs`):
//! `ops` plans the inverse, checks §6.7's rails, journals and restores;
//! `app` only wires the key to the call and carries the answer back.

use std::sync::Arc;

use graft_core::journal::{EntryId, JournalEntry};
use graft_git::version::system_git;
use graft_ops::apply::{Applied, Context, Refresh};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::restore::RestoreError;
use graft_ops::undo::{self, Drift, NotInvertible, Replaced, UndoError};

use super::Cancel;
use crate::repos::RepoStore;
use crate::session::Clock;

/// What one undo or redo came back with.
///
/// Mirrors [`crate::jobs::operate::Operated`], and for the same reason: a
/// plain `Result<Applied, _>` cannot carry [`RestoreError::AfterState`],
/// where the restore *did* run and only part of its after-state could not be
/// read back. Two of the answers here are not failures at all — "nothing to
/// undo" is a fact about the journal, and drift is a question for the user —
/// so neither is worded as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undone {
    /// Ran and journaled cleanly.
    Done(Applied),
    /// Ran and journaled, but part of the after-state could not be read or
    /// kept reachable. It happened: refresh, and show `error`.
    Incomplete {
        /// What ran.
        applied: Applied,
        /// What could not be read, as one line.
        error: String,
    },
    /// There was nothing to undo or redo, or the entry cannot be inverted.
    /// An ordinary answer; the reason is already a sentence.
    Nothing {
        /// Why, as a sentence (`nothing to undo`).
        reason: String,
        /// `u` found nothing to undo while a redo waits: the root
        /// operation's description, so the line can name the key that
        /// goes the other way (`U redoes checkout fix`, m6-conflicts/77).
        /// `None` for `U`, and for any other reason.
        redo: Option<String>,
    },
    /// Ops' gate refused: git is part-way through an operation and the
    /// entry is not a step inside that same merge
    /// (`NotInvertible::OperationLive`, m6-conflicts/57). Nothing was
    /// written. Its own shape rather than [`Undone::Nothing`], because it is
    /// a refusal the app words (`update::undo::live_refusal`), not an
    /// acknowledgement.
    OperationLive {
        /// What git has in progress.
        kind: graft_git::state::InProgress,
    },
    /// The repository moved since the operation, so nothing was touched.
    /// The user decides whether to go ahead (§6.7's rail).
    Drift {
        /// The entry the undo was aimed at, in the journal's own words.
        description: String,
        /// What differs from what the entry recorded.
        drift: Drift,
        /// Whether this was `U` rather than `u`, so the confirm box's
        /// answer re-runs the right one.
        redo: bool,
    },
    /// `Esc` stopped it, and `graft_ops` put back the steps that had
    /// already run (m5-history-and-ghost-preview/06).
    ///
    /// Its own shape rather than a [`Undone::Failed`] because a cancel is
    /// not a failure — the user asked for it. `clean` decides which of the
    /// two lines the status bar gets, and it is the whole difference
    /// between "nothing changed" and "some of it is still in force".
    Cancelled {
        /// The roll-back finished: the repository stands where it did
        /// before the undo began.
        clean: bool,
        /// `graft_ops::restore`'s own sentence — `undo cancelled · nothing
        /// changed`, or `undo cancelled part-way · … · u shows the backup`.
        why: String,
        /// The journal gained an `Aborted` entry either way, and a
        /// part-way cancel moved refs — so unlike [`Undone::Failed`] there
        /// is never a case with nothing to re-read.
        refresh: Refresh,
    },
    /// Nothing was put back, and this is why.
    Failed {
        /// What went wrong, as one line.
        error: String,
        /// What the caller must re-read anyway, or `None` where nothing can
        /// have been written. An undo restores refs before files, so a
        /// failure part-way through is `Everything`, never a narrower guess.
        refresh: Option<Refresh>,
    },
}

/// What the journal's time machine came back with (DESIGN §6.6,
/// m5-history-and-ghost-preview/08).
///
/// Apart from [`Undone`] because a jump is several restores, not one: what
/// the user needs to read is how many of them ran, and `Undone`'s shapes
/// carry one [`Applied`] or none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevertedTo {
    /// Every step ran and journaled. One `Undo` entry per step, as §6.6
    /// asks — never one summary entry.
    Done {
        /// The entry jumped back past, in the journal's own words.
        description: String,
        /// How many steps the plan had.
        total: usize,
        /// What ran, oldest step first.
        steps: Vec<Applied>,
        /// What the steps replaced between them (m6-conflicts/77): the
        /// union of each step's `Applied::replaced`.
        replaced: Replaced,
        /// How many `U`s bring it back: the steps from the first one that
        /// replaced anything to the last, since `U` walks the jump back one
        /// step at a time, newest first. Meaningless when `replaced` is
        /// empty.
        presses: usize,
    },
    /// The repository moved since one of those entries, so nothing ran.
    /// §6.7's rail: the user decides whether to go ahead.
    ///
    /// Only ever the **first** step's drift: a later step's is met part-way
    /// (`RevertedTo::Failed`), so the box names what step 0 replaces.
    Drift {
        /// The entry the jump was aimed at.
        description: String,
        /// How many steps the plan has: the `U`s the box names.
        total: usize,
        /// What differs from what the entry recorded.
        drift: Drift,
        /// That entry's id, carried so the confirm box's `[ Revert ]` can
        /// re-run **this** jump with `force`. Reconstructing it from the
        /// journal afterwards would be asking a second time and hoping for
        /// the same answer.
        target: graft_core::journal::EntryId,
    },
    /// The jump never started: no plan could be made, or the facts it
    /// needed could not be read.
    ///
    /// Apart from [`RevertedTo::Failed`] because the two read differently
    /// to the user. `stopped after 0 of 0 steps` says something ran and
    /// gave up; nothing ran (review 08 Minor 4).
    Refused {
        /// The entry the jump was aimed at.
        description: String,
        /// Why, as one line.
        error: String,
    },
    /// The jump stopped, and this is how far it got.
    ///
    /// `done` steps really did run — they are journaled and each is
    /// undoable on its own. Nothing is rolled back: that would be a second
    /// write nobody asked for (`as-built.md` §5).
    Failed {
        /// The entry the jump was aimed at.
        description: String,
        /// Steps that ran before it stopped.
        done: usize,
        /// Steps the plan had.
        total: usize,
        /// Why the next one did not, as one line.
        error: String,
    },
}

/// The journal as the browser shows it, plus what reading it had to close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRead {
    /// Every entry, oldest first — the order the file has them in.
    pub entries: Vec<JournalEntry>,
    /// Interrupted entries whose after-state could not be read, and why.
    /// Their `u` restores the whole working tree after asking, and the
    /// reason for that belongs on screen rather than on the floor
    /// (m3/03 review, M1).
    pub unreadable: Vec<String>,
    /// The entries recovery left `Pending` **on purpose**: git is still
    /// part-way through that very operation, so the entry is not torn, it is
    /// open (`graft_ops::apply::Recovered::live`, m5/09 seam A).
    ///
    /// A live entry and an interrupted one are the same bytes on disk — a
    /// `Pending` line with no outcome — and the difference is decided here,
    /// at read time, against git's own state. So nothing is migrated, and a
    /// journal written by an older Graft reads correctly.
    pub live: Vec<EntryId>,
    /// Where each interrupted set-aside's edits were when **this** read
    /// closed it (`graft_ops::apply::Recovered::set_aside`, 30c). Only the
    /// read that closed the entry knows; every later read finds nothing, so
    /// the app keeps what the first one said (`AppState::interrupted_edits`).
    pub interrupted_edits: Vec<(EntryId, graft_ops::EditsAt)>,
}

/// What a failed restore leaves stale on screen.
///
/// `None` only before `ops` began: planning, or a rail that stopped it. Once
/// [`graft_ops::restore::execute`] has been entered the answer is always
/// `Everything`, because `execute` writes the entry's `Pending` line first
/// and then runs its steps in order, breaking at the first failure and
/// journalling `Aborted`. So even a failure that touched no file has changed
/// the journal — and with two `UpdateRef` steps, a `RefMoved` on the second
/// leaves the first ref already moved and the graph showing a tip that is no
/// longer there.
///
/// This used to answer `None` for `RefMoved`, `Facts` and `Snapshot` on the
/// grounds that "the compare-and-swap refused before any file was touched".
/// True of the first step only, and never true of the journal
/// (m3/06 review, Major 3). The honest narrower answer needs `ops` to say how
/// many steps ran; until it does, a re-read of everything is the answer that
/// cannot be wrong, and it is what `Ctrl+R` does anyway.
fn refresh_after_failure(error: &UndoError) -> Option<Refresh> {
    match error {
        // Planning failed, or the rails stopped it: `execute` was never
        // entered, so not even a journal line exists.
        UndoError::Nothing(_) | UndoError::Drift { .. } | UndoError::Facts(_) => None,
        UndoError::Restore(_) => Some(Refresh::Everything),
        // The time machine (`undo::revert_to`) stopped part-way. Every step
        // in `done` is a full `restore::execute` that ran, journaled its own
        // `Undo` entry and moved refs — so once one has run there is always
        // something stale on screen. With none run, the step that failed
        // decides, exactly as it would on its own.
        UndoError::PartWay { done, error } => {
            if done.is_empty() {
                refresh_after_failure(error)
            } else {
                Some(Refresh::Everything)
            }
        }
    }
}

/// `run_undo`'s answer for one `Err` from `ops`, with the `AfterState` case
/// lifted back out into [`Undone::Incomplete`] rather than collapsed into a
/// string that drops the [`Applied`] the caller refreshes from.
///
/// `redo_waiting` is asked only when `u` found nothing to undo: what `U`
/// would redo, so the answer can name it rather than end on a dead end.
fn undone_from(
    error: UndoError,
    redo: bool,
    description: impl FnOnce() -> String,
    redo_waiting: impl FnOnce() -> Option<String>,
) -> Undone {
    match error {
        UndoError::Nothing(NotInvertible::OperationLive { kind }) => Undone::OperationLive { kind },
        UndoError::Nothing(NotInvertible::NothingToUndo) if !redo => Undone::Nothing {
            reason: error_line(&error),
            redo: redo_waiting(),
        },
        UndoError::Nothing(_) => Undone::Nothing {
            reason: error_line(&error),
            redo: None,
        },
        UndoError::Drift { drift, .. } => Undone::Drift {
            description: description(),
            drift: *drift,
            redo,
        },
        // `Esc`. `restore::execute` rolled back what it had applied (clean)
        // or said what it could not (part-way); both are answers, not
        // failures, and both leave an `Aborted` entry in the journal, so
        // the list on screen is stale either way.
        UndoError::Restore(cancel @ RestoreError::Cancelled { .. }) => Undone::Cancelled {
            clean: true,
            why: cancel.to_string(),
            refresh: Refresh::Everything,
        },
        UndoError::Restore(cancel @ RestoreError::CancelledPartway { .. }) => Undone::Cancelled {
            clean: false,
            why: cancel.to_string(),
            refresh: Refresh::Everything,
        },
        UndoError::Restore(RestoreError::AfterState { applied, unread }) => Undone::Incomplete {
            applied: *applied,
            error: unread
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        },
        // A `revert_to` that stopped part-way. Its `done` list is not carried
        // into `Undone`, and that is a decision rather than an oversight:
        // the message already says how many steps ran ("2 of 3 steps done,
        // then: …"), and `refresh_after_failure` answers `Everything`, which
        // is wider than any of those `Applied`s would have asked for — so
        // nothing on screen is left stale by dropping them. Nothing reaches
        // this arm today: `run_undo` only ever calls `undo`/`redo`, and the
        // time machine gets its own job and its own `Product::RevertedTo`
        // (ticket m5/08 item 7), which is where "stopped after 2 of 3
        // steps" is meant to be worded.
        error @ UndoError::PartWay { .. } => Undone::Failed {
            refresh: refresh_after_failure(&error),
            error: error_line(&error),
        },
        other => Undone::Failed {
            refresh: refresh_after_failure(&other),
            error: error_line(&other),
        },
    }
}

/// One line for the status line, in `ops`' own words.
fn error_line(error: &UndoError) -> String {
    error.explain()
}

/// A failure before anything could run: nothing on disk to re-read.
fn nothing_ran(error: &dyn std::fmt::Display) -> Undone {
    Undone::Failed {
        error: error.to_string(),
        refresh: None,
    }
}

/// Everything an operation on this repository needs, opened fresh.
///
/// The journal is opened here and handed out by reference, because
/// [`Context`] borrows it for as long as the operation runs.
fn with_context<T>(
    repos: &RepoStore,
    clock: Clock,
    cancel: &Cancel,
    then: impl FnOnce(&mut Context<'_>) -> T,
    failed: impl FnOnce(&dyn std::fmt::Display) -> T,
) -> T {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return failed(&error),
    };
    let handle = repo.handle();
    let git = match system_git() {
        Ok(found) => found.version(),
        Err(error) => return failed(&error),
    };
    let path = match journal_path(&handle) {
        Ok(path) => path,
        Err(error) => return failed(&error),
    };
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal,
        Err(error) => return failed(&error),
    };
    // Through the `Clock`, for the reason `jobs::operate::in_context`
    // gives: an instant that can reach a frame is never read from the wall
    // clock directly (`crate::session::Clock`).
    let now_unix_ms = clock.now_millis();
    let mut random = [0u8; 10];
    if let Err(error) = getrandom::fill(&mut random) {
        return failed(&format!("could not read random bytes: {error}"));
    }
    let mut ctx = match Context::open(&handle, git, &mut journal, now_unix_ms, random, cancel) {
        Ok(ctx) => ctx,
        Err(error) => return failed(&error),
    };
    then(&mut ctx)
}

/// Take back the newest operation still in force, or put back the newest
/// undo. `force` goes past §6.7's drift rail — the user has been shown what
/// moved and said go ahead.
pub(super) fn run_undo(
    repos: &RepoStore,
    redo: bool,
    force: bool,
    clock: Clock,
    cancel: &Cancel,
) -> Undone {
    with_context(
        repos,
        clock,
        cancel,
        |ctx| {
            // The entry the inverse is aimed at, read before running: a
            // drift refusal has to name it, and afterwards the journal has
            // a newer entry of its own on top.
            let aimed_at = aimed_at(ctx, redo);
            let ran = if redo {
                undo::redo(ctx, force)
            } else {
                undo::undo(ctx, force)
            };
            match ran {
                Ok(applied) => Undone::Done(applied),
                Err(error) => undone_from(error, redo, || aimed_at, || redo_waiting(ctx)),
            }
        },
        nothing_ran,
    )
}

/// The journal's time machine: undo every entry back to and including
/// `target`, one step at a time (DESIGN §6.6, ADR-0031).
///
/// Reached only from a confirmed Ghost Preview, never straight from a key —
/// the picture is drawn from the very `InversePlan`s this runs, so the two
/// cannot disagree about what is about to happen.
///
/// `total` is read from the plan **before** anything runs, because
/// `UndoError::PartWay` carries only what it managed (`done`) and "2 of 3"
/// is the sentence the user needs. Reading it afterwards would be reading
/// it from a journal the run has already changed.
pub(super) fn run_revert_to(
    repos: &RepoStore,
    target: &graft_core::journal::EntryId,
    force: bool,
    clock: Clock,
    cancel: &Cancel,
) -> RevertedTo {
    with_context(
        repos,
        clock,
        cancel,
        |ctx| {
            let aimed_at = ctx
                .journal
                .entries()
                .iter()
                .find(|entry| entry.id == *target)
                .map_or_else(
                    || "that point".to_owned(),
                    |entry| entry.description.clone(),
                );
            // How many steps the jump is, from the same planner
            // `revert_to` will use. A plan that cannot be made at all is
            // the run's own answer a moment later, so the count falls back
            // to zero rather than being reported twice.
            let total =
                undo::plan_revert_to(ctx.journal.entries(), target).map_or(0, |plans| plans.len());
            match undo::revert_to(ctx, target, force) {
                Ok(steps) => {
                    let (replaced, presses) = replaced_by_jump(&steps);
                    RevertedTo::Done {
                        description: aimed_at,
                        total,
                        steps,
                        replaced,
                        presses,
                    }
                }
                // `revert_to` wraps every step's error in `PartWay`, the
                // first step's too — so drift before anything ran arrives
                // as `PartWay { done: [], Drift }`, and matching the bare
                // `Drift` alone left this box unreachable: the jump ended
                // "stopped after 0 of 2 steps … u again to go ahead", naming
                // a key that does not re-run it (m6-conflicts/77).
                Err(UndoError::PartWay { done, error }) => match (done.len(), *error) {
                    (0, UndoError::Drift { drift, .. }) => RevertedTo::Drift {
                        description: aimed_at,
                        total,
                        drift: *drift,
                        target: target.clone(),
                    },
                    (done, error) => RevertedTo::Failed {
                        description: aimed_at,
                        done,
                        total,
                        error: error_line(&error),
                    },
                },
                // No bare `Drift` arm: `revert_to` answers only `Nothing`
                // before its loop and wraps every step's error in `PartWay`
                // (`graft_ops::undo::execute::revert_to`), so one would never
                // run (m6-conflicts/80, review m9).
                Err(other) => RevertedTo::Refused {
                    description: aimed_at,
                    error: error_line(&other),
                },
            }
        },
        |error| RevertedTo::Refused {
            description: "that point".to_owned(),
            error: error.to_string(),
        },
    )
}

/// What a finished jump replaced, and how many `U`s bring it back.
///
/// `revert_to` journals one `Undo` entry per step, newest operation first,
/// and `U` redoes the newest `Undo` entry — the jump's **last** step — so it
/// walks back from the end. The edit step `k` replaced comes back with the
/// `U` that redoes step `k`: `steps.len() - k` presses. The earliest step
/// that replaced anything sets the count, since reaching it brings every
/// later one back on the way.
fn replaced_by_jump(steps: &[Applied]) -> (Replaced, usize) {
    let mut replaced = Replaced::default();
    for step in steps {
        replaced.extend(step.replaced.clone());
    }
    let presses = steps
        .iter()
        .position(|step| !step.replaced.is_empty())
        .map_or(0, |first| steps.len() - first);
    (replaced, presses)
}

/// What `U` would redo now, named by its root operation (`checkout fix`,
/// not the `Undo` entry's own `undo: checkout fix`), or `None` when
/// nothing waits. Read from the journal `u` just consulted: a `Nothing`
/// answer wrote nothing, so it is still the journal as it stands.
fn redo_waiting(ctx: &Context<'_>) -> Option<String> {
    let entries = ctx.journal.entries();
    let waiting = graft_ops::journal::next_redo(entries)?;
    Some(
        graft_ops::journal::root_operation(entries, waiting).map_or_else(
            || waiting.description.clone(),
            |root| root.description.clone(),
        ),
    )
}

/// What `u` (or `U`) is about to take back, in the journal's own words.
fn aimed_at(ctx: &Context<'_>, redo: bool) -> String {
    let entries = ctx.journal.entries();
    let entry = if redo {
        graft_ops::journal::next_redo(entries)
    } else {
        graft_ops::journal::next_undo(entries)
    };
    entry.map_or_else(|| "operation".to_owned(), |entry| entry.description.clone())
}

/// Read the journal for the browser, closing whatever a previous run left
/// open first (DESIGN.md §6.2 step 3).
///
/// Recovery is `ops`' [`graft_ops::apply::recover_interrupted`], not a bare
/// `mark_interrupted`: it also seals the recovered entry's trees behind its
/// backup ref, without which `git gc` prunes exactly what that entry's `u`
/// would restore (group 03's review fix).
/// The journal's entries, for a caller that only needs to read them.
///
/// Ghost Preview's time machine plans from the same entries `revert_to`
/// will run against, which is what keeps the picture and the act one act
/// (design note hard choice 3).
pub(super) fn journal_entries(
    repos: &RepoStore,
    clock: Clock,
    cancel: &Cancel,
) -> Result<Vec<JournalEntry>, String> {
    with_context(
        repos,
        clock,
        cancel,
        |ctx| Ok(ctx.journal.entries().to_vec()),
        |error| Err(error.to_string()),
    )
}

pub(super) fn read_journal(
    repos: &RepoStore,
    clock: Clock,
    cancel: &Cancel,
) -> Result<Arc<JournalRead>, String> {
    with_context(
        repos,
        clock,
        cancel,
        |ctx| {
            let recovered =
                graft_ops::apply::recover_interrupted(ctx).map_err(|error| error.to_string())?;
            let entries = ctx.journal.entries();
            let described = |id: &graft_core::journal::EntryId| {
                entries
                    .iter()
                    .find(|entry| &entry.id == id)
                    .map_or_else(|| id.as_str().to_owned(), |entry| entry.description.clone())
            };
            // `recovered.closed` is deliberately not carried: the notice is
            // derived from the entries themselves, so that it survives the
            // next read, which closes nothing (m3/06 review, Major 1).
            Ok(Arc::new(JournalRead {
                unreadable: recovered
                    .unreadable
                    .iter()
                    .map(|(id, why)| format!("{}: {why}", described(id)))
                    .collect(),
                // Carried, unlike `recovered.closed`: a live entry is a fact
                // about git's state *now*, which every read re-establishes,
                // so it cannot go stale the way "what this read closed" did.
                live: recovered.live.clone(),
                interrupted_edits: recovered.set_aside.clone(),
                entries: entries.to_vec(),
            }))
        },
        |error| Err(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A restore that stopped at a compare-and-swap still wrote a `Pending`
    /// line and journalled its `Aborted` outcome, and with more than one ref
    /// step it may have moved a ref before refusing. Answering "nothing to
    /// re-read" left the browser and possibly the graph stale
    /// (m3/06 review, Major 3).
    #[test]
    fn a_restore_that_refused_still_asks_for_a_refresh() {
        let refused = UndoError::Restore(RestoreError::RefMoved {
            name: "refs/heads/main".to_owned(),
        });
        assert_eq!(
            refresh_after_failure(&refused),
            Some(Refresh::Everything),
            "the journal changed even where no file did"
        );
    }

    /// Ops' undo gate (m6-conflicts/57) reaches the app as its own shape,
    /// carrying the kind, so `update::undo` words it as a refusal rather
    /// than as the plain acknowledgement every other `Nothing` is.
    #[test]
    fn operation_live_is_its_own_answer() {
        let kind = graft_git::state::InProgress::Merge;
        let refused = UndoError::Nothing(NotInvertible::OperationLive { kind });
        assert_eq!(
            undone_from(refused, false, String::new, || None),
            Undone::OperationLive { kind }
        );
    }

    /// The other half of the same rule: `ops` never began, so there is
    /// nothing new on disk to read.
    #[test]
    fn a_failure_before_ops_began_asks_for_nothing() {
        assert_eq!(
            refresh_after_failure(&UndoError::Nothing(
                graft_ops::undo::NotInvertible::NothingToUndo
            )),
            None
        );
    }

    /// m5/07 seam. A time machine (`undo::revert_to`) that stopped part-way
    /// has already run whole restores: refs moved, the journal grew an
    /// `Undo` entry per step. The screen is stale however the *next* step
    /// failed, so the scope is the wide one.
    #[test]
    fn a_time_machine_that_stopped_part_way_re_reads_everything() {
        let applied = graft_ops::apply::Applied {
            came_back: Vec::new(),
            entry: entry_for("undo: commit \"fix\""),
            refresh: Refresh::WorkingTree,
            step_backup: None,
            replaced: graft_ops::undo::Replaced::default(),
        };
        let stopped = UndoError::PartWay {
            done: vec![applied],
            // A rail the user has not answered yet: on its own it would
            // have asked for no refresh at all.
            error: Box::new(UndoError::Nothing(
                graft_ops::undo::NotInvertible::NothingToUndo,
            )),
        };
        assert_eq!(
            refresh_after_failure(&stopped),
            Some(Refresh::Everything),
            "a step that ran moved refs, whatever stopped the next one"
        );
    }

    /// And with nothing done, the step that failed answers for itself: a
    /// blanket `Everything` here would have the browser re-read over a
    /// refusal that wrote nothing.
    #[test]
    fn a_time_machine_that_stopped_before_its_first_step_asks_for_nothing() {
        let stopped = UndoError::PartWay {
            done: Vec::new(),
            error: Box::new(UndoError::Nothing(
                graft_ops::undo::NotInvertible::NothingToUndo,
            )),
        };
        assert_eq!(refresh_after_failure(&stopped), None);
    }

    /// The message a part-way revert carries reaches the status line whole —
    /// how far it got and what stopped it — rather than being replaced by a
    /// generic failure line.
    #[test]
    fn a_part_way_revert_says_how_far_it_got() {
        let stopped = UndoError::PartWay {
            done: vec![graft_ops::apply::Applied {
                came_back: Vec::new(),
                entry: entry_for("undo: commit \"fix\""),
                refresh: Refresh::WorkingTree,
                step_backup: None,
                replaced: graft_ops::undo::Replaced::default(),
            }],
            error: Box::new(UndoError::Nothing(
                graft_ops::undo::NotInvertible::NothingToUndo,
            )),
        };
        let Undone::Failed { error, refresh } = undone_from(stopped, false, String::new, || None)
        else {
            panic!("a part-way revert is not a drift and not a clean run")
        };
        assert!(error.contains("1 of 2 steps done"), "{error}");
        assert!(
            error.contains("nothing"),
            "what stopped it survives too: {error}"
        );
        assert_eq!(refresh, Some(Refresh::Everything));
    }

    /// A plausible journal entry; only `description` is read here.
    fn entry_for(description: &str) -> JournalEntry {
        use graft_core::journal::{EntryId, EntryStatus, HeadRef, OperationKind, Touches};
        JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 0,
            kind: OperationKind::Undo,
            description: description.to_owned(),
            status: EntryStatus::Committed,
            touches: Touches::default(),
            head_before: HeadRef::Unborn("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            worktree_raw_before: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            config_before: std::collections::BTreeMap::new(),
            config_after: std::collections::BTreeMap::new(),
            put_back: None,
        }
    }

    /// `KNOWN_GAPS` :495, the planner's ruling (m7/01b): `Esc` before any
    /// step ran is a clean cancel — `undo cancelled · nothing changed`,
    /// which `update::undo` shows as `Status::done` — never a failure line.
    /// The stop is raised before the job starts, so it lands on the very
    /// first poll, ahead of the rails' reads.
    #[test]
    fn a_stop_before_any_step_reads_as_cancelled() {
        let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
        let repo = scratch.path();
        std::fs::write(repo.join("new.txt"), "new\n").expect("the scratch repo is writable");
        let store = RepoStore::new(repo.to_path_buf(), None);
        let stage =
            graft_ops::operation::Operation::Stage(graft_ops::operation::Target::Paths(vec![
                std::path::PathBuf::from("new.txt"),
            ]));
        with_context(
            &store,
            Clock::Fixed(1_700_000_000),
            &Cancel::new(),
            |ctx| graft_ops::apply::apply(ctx, &stage).map(|_| ()),
            |why| panic!("no context: {why}"),
        )
        .unwrap_or_else(|why| panic!("the stage runs: {}", why.explain()));
        let staged = || {
            std::process::Command::new("git")
                .current_dir(repo)
                .args(["diff", "--cached", "--name-only"])
                .output()
                .expect("git runs")
                .stdout
        };
        let before = staged();

        let cancel = Cancel::new();
        cancel.cancel();
        let undone = run_undo(&store, false, false, Clock::Fixed(1_700_000_001), &cancel);
        let Undone::Cancelled { clean, why, .. } = &undone else {
            panic!("a stop before any step is a cancel, got {undone:?}");
        };
        assert!(*clean, "{why}");
        assert_eq!(why, "undo cancelled · nothing changed");
        assert_eq!(staged(), before, "nothing changed");
    }
}

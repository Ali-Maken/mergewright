//! `u`, `Ctrl+Z` and `U`: the keys the milestone's promise rests on
//! (m3-working-tree-undo-and-stash/06, ADR-0002).
//!
//! Pure, like every other `update` module: the key becomes a
//! [`Command::Undo`] and the job does the rest. What lives here is the part
//! that decides — when a key is a no-op, what the drift guard asks, and what
//! the status line says afterwards.
//!
//! The drift rail (DESIGN.md §6.7) is the one place undo stops to ask. `ops`
//! compares the three things the entry recorded — refs, the index and the
//! working tree — against what is there now, and refuses if any moved. That
//! refusal reaches here as [`Undone::Drift`] and becomes the confirm box; a
//! second, deliberate `Enter` on `[ Undo ]` re-runs the same key with
//! `force`.

use graft_core::job::JobId;
use graft_ops::apply::Refresh;
use graft_term::sanitize_line;

use super::confirm::{ConfirmThen, PendingConfirm};
use super::events::Command;
use super::state::{AppState, Status};
use crate::jobs::undo::{RevertedTo, Undone};

/// `u`/`Ctrl+Z` (`redo` false) and `U` (`redo` true).
///
/// Nothing while a write — including another undo — is already running:
/// one at a time, and the status line is already saying what that one is
/// (`update::operate::operation_in_flight`).
pub fn undo_key(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    launch(state, redo, false, next_id)
}

/// The same key again, past the drift guard: the user has read what moved
/// and chosen `[ Undo ]`.
pub(super) fn forced(
    state: &mut AppState,
    redo: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    launch(state, redo, true, next_id)
}

fn launch(
    state: &mut AppState,
    redo: bool,
    force: bool,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if super::operate::operation_in_flight(state) {
        return Vec::new();
    }
    let id = next_id();
    state.operating = Some(id);
    let words = if redo { "redo" } else { "undo" };
    // `Esc` stops it. `graft_ops::restore::execute` now checks its stop
    // token between steps and rolls back the ones that already ran
    // (m5-history-and-ghost-preview/06), so a cancelled undo leaves the
    // repository where it started rather than half-applied — which is what
    // `UNDO_CANNOT_STOP` used to be the honest answer to.
    state.status = Status::Busy(format!("{words}{}", super::remote::ESC_HINT));
    super::remote::launched(
        state,
        id,
        words.to_owned(),
        None,
        super::remote::Stoppable::Yes,
        None,
    );
    vec![if redo {
        Command::Redo(id, force)
    } else {
        Command::Undo(id, force)
    }]
}

/// An undo or redo job answered. Stale answers — a job that is not the one
/// `AppState::operating` waits for — are dropped, exactly as
/// `update::operate::operated` drops them.
pub fn undone(
    state: &mut AppState,
    id: JobId,
    outcome: Undone,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    match outcome {
        Undone::Done(applied) => {
            // The entry's own description already reads `undo: stage
            // src/a.rs` (`InversePlan::description`), so it is shown as it
            // stands rather than prefixed into `undid: undo: …`.
            // `u` passes over a push (design note, choice 1), so a commit
            // taken back here may still be on the remote: say so, or the
            // graph's `origin/main` badge reads as a bug.
            let still_there = super::remote::pushed_since(state, &applied.entry);
            state.status = Status::Done(format!("{}{still_there}", applied.entry.description));
            refresh(state, applied.refresh, next_id)
        }
        Undone::Incomplete { applied, error } => {
            state.status = Status::sticky(format!(
                "{} \u{2014} could not confirm it fully: {error}",
                applied.entry.description
            ));
            refresh(state, applied.refresh, next_id)
        }
        // Not a failure: the journal simply has nothing in force to take
        // back. Cleared by the next key, like any other acknowledgement.
        Undone::Nothing(why) => {
            state.status = Status::Done(why);
            Vec::new()
        }
        // `Esc`, answered. A clean cancel is an acknowledgement — `Done`,
        // cleared by the next key, because nothing is wrong (CLAUDE.md: a
        // status field must never read as an error when nothing is). A
        // part-way cancel left something in force and is `Sticky` until
        // it is read.
        Undone::Cancelled {
            clean,
            why,
            refresh: what,
        } => {
            state.status = if clean {
                Status::Done(why)
            } else {
                Status::sticky(why)
            };
            refresh(state, what, next_id)
        }
        Undone::Drift {
            description,
            drift,
            redo,
        } => {
            state.confirm = Some(drift_box(&description, &drift, redo));
            Vec::new()
        }
        Undone::Failed {
            error,
            refresh: what,
        } => {
            // Sticky, not `Failed`: an undo that ran part-way asks for a
            // refresh of its own, and that refresh finishing would wipe the
            // reason off the screen before it could be read (m3/05 review,
            // Major 7).
            state.status = Status::sticky(error);
            match what {
                Some(what) => refresh(state, what, next_id),
                None => Vec::new(),
            }
        }
    }
}

/// The time machine again, past the drift guard: the user has read what
/// moved and chosen `[ Revert ]`.
///
/// Unstoppable for the same reason the first attempt was: a jump is several
/// restores, and `Esc` between two of them would leave the repository
/// somewhere nobody asked for.
pub(super) fn revert_to_forced(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    entry: graft_core::journal::EntryId,
    next_id: &mut impl FnMut() -> JobId,
) {
    if super::operate::operation_in_flight(state) {
        return;
    }
    let id = next_id();
    state.operating = Some(id);
    let description = format!("revert to before \u{201c}{}\u{201d}", entry.as_str());
    state.status = Status::Busy(description.clone());
    super::remote::launched(
        state,
        id,
        description,
        None,
        super::remote::Stoppable::No(super::preview::HISTORY_CANNOT_STOP),
        None,
    );
    commands.push(Command::RevertTo(id, entry, true));
}

/// The journal's time machine answered (DESIGN §6.6,
/// m5-history-and-ghost-preview/08).
///
/// A jump is several restores, and the three answers say so plainly: how
/// many steps it took, how far it got when it stopped, or what moved
/// underneath before it started. **`Everything` is re-read whenever a step
/// ran** — a jump moves refs, `HEAD`, the index and the working tree, and no
/// narrower scope covers all four.
pub(super) fn reverted_to(
    state: &mut AppState,
    id: JobId,
    outcome: RevertedTo,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    match outcome {
        RevertedTo::Done {
            description,
            total,
            steps,
        } => {
            state.status = Status::Done(format!(
                "reverted to before \u{201c}{description}\u{201d} ({total} {})",
                if total == 1 { "step" } else { "steps" }
            ));
            // `steps` is what ran, kept for the record rather than for the
            // refresh: `Everything` is wider than any of their own scopes.
            debug_assert_eq!(steps.len(), total, "every planned step journaled");
            refresh(state, Refresh::Everything, next_id)
        }
        RevertedTo::Drift {
            description,
            drift,
            target,
        } => {
            state.confirm = Some(revert_drift_box(&description, &drift, target));
            Vec::new()
        }
        RevertedTo::Failed {
            description: _,
            done,
            total,
            error,
        } => {
            // Sticky, and worded as a state: some steps really did run, and
            // each of them is its own journal entry the browser can show.
            state.status = Status::Sticky {
                reason: format!(
                    "revert stopped after {done} of {total} {}: {error}",
                    if total == 1 { "step" } else { "steps" }
                ),
                keys: Some("J journal".to_owned()),
            };
            if done > 0 {
                refresh(state, Refresh::Everything, next_id)
            } else {
                Vec::new()
            }
        }
    }
}

/// The time machine's own drift box. Same rail, same words, but `[ Revert ]`
/// re-runs the jump rather than one `u`.
fn revert_drift_box(
    description: &str,
    drift: &graft_ops::undo::Drift,
    target: graft_core::journal::EntryId,
) -> PendingConfirm {
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Revert to before \u{201c}{description}\u{201d}?")),
            {
                let mut message = wrapped(&format!("Since then, {}.", drift.explain()));
                message.push("A backup is taken first, so this is itself undoable.".to_owned());
                message.iter().map(|line| sanitize_line(line)).collect()
            },
            sanitize_line("revert restores the recorded state, step by step"),
        )
        .with_labels("Stop", "Revert"),
        then: ConfirmThen::RevertTo { entry: target },
    }
}

/// The drift guard's box: what the operation was, what has moved since, and
/// the safe answer under the cursor.
///
/// The consequence line is `drift.explain()` verbatim — `ops` names which of
/// the three comparisons failed, and naming them again here would let the two
/// wordings drift apart.
fn drift_box(description: &str, drift: &graft_ops::undo::Drift, redo: bool) -> PendingConfirm {
    let verb = if redo { "Redo" } else { "Undo" };
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("{verb} \u{201c}{description}\u{201d}?")),
            {
                let mut message =
                    wrapped(&format!("Since this {description}, {}.", drift.explain()));
                message.push("A backup is taken first, so this is itself undoable.".to_owned());
                message.iter().map(|line| sanitize_line(line)).collect()
            },
            // No single git command to show: a restore is a plan of several,
            // and the journal browser's `Space` is where they are listed.
            sanitize_line(&format!(
                "{} restores the recorded state",
                verb.to_lowercase()
            )),
        )
        .with_labels("Stop", verb),
        then: ConfirmThen::Undo { redo },
    }
}

/// The widest line the confirm box draws without clipping: it is 64
/// columns at most (`graft_ui::confirm::confirm_area`), and a `…` where the
/// drift's own words should be is the one thing this box must not do.
const CONFIRM_COLUMNS: usize = 62;

/// `text` broken at spaces into lines of at most [`CONFIRM_COLUMNS`]. A
/// single word longer than that is left whole — the box clips it, and
/// cutting a path in half would be worse than a clipped one.
fn wrapped(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.chars().count() + 1 + word.chars().count() <= CONFIRM_COLUMNS => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

/// The footer's interrupted-entry line, until `J` has been pressed once.
///
/// Graft was killed — or the machine was — with an operation half-run. The
/// launch read closes such an entry and seals its backup, so `u` can still
/// take it back; the user is told it is there rather than left to find it.
/// An entry whose after-state could not be read says so too: its `u`
/// restores the whole working tree, which is not a thing to discover by
/// pressing the key.
///
/// Read off the journal itself, not off what a read happened to close. The
/// journal is re-read after every write and on `Ctrl+R`, and the second read
/// closes nothing — deriving the notice from "what this read recovered" made
/// it vanish at the first staged file, which is the one moment the user most
/// needs to know an operation did not finish (m3/06 review, Major 1).
/// An entry a later entry has taken back is not announced.
#[must_use]
pub fn interrupted_notice(state: &AppState) -> Option<String> {
    if state.journal.notice_seen {
        return None;
    }
    let super::detail::Loading::Ready(read) = &state.journal.entries else {
        return None;
    };
    // Announced unless a later entry has taken it back. Deliberately *not*
    // `in_force`: that answers "can `u` reach it", which is false for an
    // interrupted entry with no backup — and that is the one the user most
    // needs to be told about, not the one to keep quiet about.
    let undone: std::collections::HashSet<&graft_core::journal::EntryId> = read
        .entries
        .iter()
        .filter_map(|entry| entry.reverses.as_ref())
        .collect();
    // A live entry is `Pending` with no outcome, exactly like a torn one, so
    // `is_interrupted` says yes to both. Only git's own state tells them
    // apart, and the last journal read asked it: an operation standing right
    // there, with a banner two rows above saying so, was never interrupted
    // (m5/09 seam A).
    let mut interrupted = read
        .entries
        .iter()
        .filter(|entry| {
            entry.is_interrupted()
                && !undone.contains(&entry.id)
                && !state.live_entries.contains(&entry.id)
        })
        .map(|entry| entry.description.as_str());
    let first = interrupted.next()?;
    let more = interrupted.count();
    let rest = if more == 0 {
        String::new()
    } else {
        format!(" (and {more} more)")
    };
    let widest = if state.journal.unreadable.is_empty() {
        String::new()
    } else {
        " \u{b7} its state could not be read, so u restores the whole tree".to_owned()
    };
    Some(format!(
        "an operation was interrupted: {first}{rest}{widest} \u{b7} J to inspect"
    ))
}

/// Re-read what the restore changed — `update::operate`'s own two scopes,
/// so an undo and the operation it reverses cannot refresh differently.
fn refresh(
    state: &mut AppState,
    what: Refresh,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    // `refresh_scope` re-reads the journal itself: an undo journals an
    // entry of its own, exactly as the operation it reverses did.
    super::operate::refresh_scope(state, what, next_id)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::event::{KeyCode, KeyPress};
    use graft_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };
    use graft_ops::apply::Applied;

    use super::*;
    use crate::jobs::undo::JournalRead;
    use crate::update::detail::Loading;
    use crate::update::journal::JournalState;

    fn ids() -> impl FnMut() -> JobId {
        let mut next = 0;
        move || {
            next += 1;
            JobId::new(next)
        }
    }

    /// An `Applied` naming `description`, with `refresh` as its scope: the
    /// two fields `undone` reads.
    fn applied(description: &str, refresh: Refresh) -> Applied {
        Applied {
            entry: JournalEntry {
                id: EntryId::new(1, [0; 10]),
                at_unix_ms: 1,
                kind: OperationKind::Undo,
                description: description.to_owned(),
                status: EntryStatus::Committed,
                touches: Touches::default(),
                head_before: HeadRef::Branch("refs/heads/main".to_owned()),
                head_after: None,
                refs_before: std::collections::BTreeMap::new(),
                refs_after: std::collections::BTreeMap::new(),
                index_before: None,
                index_after: None,
                worktree_before: None,
                worktree_after: None,
                backup_ref: None,
                pins: Vec::new(),
                commands: Vec::new(),
                reverses: None,
            },
            refresh,
        }
    }

    fn drift() -> graft_ops::undo::Drift {
        graft_ops::undo::Drift {
            index: true,
            ..graft_ops::undo::Drift::default()
        }
    }

    #[test]
    fn u_asks_for_an_undo_and_says_so() {
        let mut state = AppState::default();
        let commands = undo_key(&mut state, false, &mut ids());
        assert_eq!(commands, vec![Command::Undo(JobId::new(1), false)]);
        assert_eq!(state.operating, Some(JobId::new(1)));
        // The busy line offers the cancel, because the cancel is now real
        // (m5-history-and-ghost-preview/06): `restore::execute` rolls back
        // the steps it had applied. `update::remote::tests::
        // esc_cancels_an_undo_in_flight` is the other half.
        assert_eq!(
            state.status,
            Status::Busy(format!("undo{}", super::super::remote::ESC_HINT))
        );
    }

    #[test]
    fn capital_u_asks_for_a_redo() {
        let mut state = AppState::default();
        let commands = undo_key(&mut state, true, &mut ids());
        assert_eq!(commands, vec![Command::Redo(JobId::new(1), false)]);
    }

    #[test]
    fn u_while_a_write_is_running_asks_for_nothing() {
        let mut state = AppState {
            operating: Some(JobId::new(4)),
            ..AppState::default()
        };
        assert!(undo_key(&mut state, false, &mut ids()).is_empty());
        // The running write keeps the slot: a second `u` must not steal it.
        assert_eq!(state.operating, Some(JobId::new(4)));
    }

    #[test]
    fn a_finished_undo_says_what_it_undid_and_re_reads_the_working_tree() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: stage src/a.rs", Refresh::WorkingTree)),
            &mut ids(),
        );
        assert_eq!(
            state.status,
            Status::Done("undo: stage src/a.rs".to_owned())
        );
        assert!(state.operating.is_none());
        assert!(commands.contains(&Command::ReadWorkingTree), "{commands:?}");
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::ReadJournal(_))),
            "the journal has a new entry of its own: {commands:?}"
        );
    }

    #[test]
    fn an_undo_that_moved_refs_re_reads_everything() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: commit \"fix\"", Refresh::Everything)),
            &mut ids(),
        );
        assert!(commands.contains(&Command::OpenRepository), "{commands:?}");
        assert!(commands.contains(&Command::ReadRefs), "{commands:?}");
    }

    #[test]
    fn an_answer_for_another_job_is_dropped() {
        let mut state = AppState {
            operating: Some(JobId::new(9)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Done(applied("undo: stage a", Refresh::Everything)),
            &mut ids(),
        );
        assert!(commands.is_empty());
        assert_eq!(state.operating, Some(JobId::new(9)));
        assert_eq!(state.status, Status::Ready);
    }

    #[test]
    fn nothing_to_undo_is_an_answer_not_a_failure() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Nothing("nothing to undo".to_owned()),
            &mut ids(),
        );
        assert_eq!(state.status, Status::Done("nothing to undo".to_owned()));
        assert!(commands.is_empty());
    }

    #[test]
    fn a_failed_undo_stays_on_screen_past_its_own_refresh() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Failed {
                error: "refs/heads/main is not where it was".to_owned(),
                refresh: None,
            },
            &mut ids(),
        );
        assert_eq!(
            state.status,
            Status::sticky("refs/heads/main is not where it was".to_owned())
        );
        assert!(commands.is_empty(), "nothing ran: {commands:?}");
    }

    #[test]
    fn drift_opens_a_box_that_quotes_what_moved_and_answers_nothing_yet() {
        let mut state = AppState {
            operating: Some(JobId::new(2)),
            ..AppState::default()
        };
        let commands = undone(
            &mut state,
            JobId::new(2),
            Undone::Drift {
                description: "stage src/a.rs".to_owned(),
                drift: drift(),
                redo: false,
            },
            &mut ids(),
        );
        assert!(commands.is_empty(), "nothing runs until the user says so");
        let pending = state.confirm.as_ref().expect("the drift box opened");
        assert_eq!(pending.view.selected(), 0, "the safe choice first");
        assert_eq!(pending.view.choices[0].as_str(), "Stop");
        assert_eq!(pending.view.choices[1].as_str(), "Undo");
        assert!(
            pending
                .view
                .message
                .iter()
                .any(|line| line.as_str().contains(&drift().explain())),
            "the box says what ops said: {:?}",
            pending.view.message
        );
        assert_eq!(pending.then, ConfirmThen::Undo { redo: false });
    }

    #[test]
    fn a_long_drift_explanation_is_wrapped_rather_than_clipped() {
        // The box is 64 columns at most, and a `…` where the drift's own
        // words should be is the one thing it must not do.
        let drift = graft_ops::undo::Drift {
            index: true,
            worktree: true,
            unknown: vec!["HEAD", "the working tree"],
            ..drift()
        };
        let mut state = AppState::default();
        let id = JobId::new(1);
        state.operating = Some(id);
        undone(
            &mut state,
            id,
            Undone::Drift {
                description: "commit \"Add the OAuth provider and its tests\"".to_owned(),
                drift,
                redo: false,
            },
            &mut ids(),
        );
        let view = &state.confirm.as_ref().expect("the guard opened").view;
        assert!(view.message.len() > 2, "{:?}", view.message);
        for line in &view.message {
            assert!(line.chars().count() <= 62, "{line}");
        }
        let joined = view
            .message
            .iter()
            .map(graft_term::SanitizedText::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.contains("the working tree"), "{joined}");
    }

    #[test]
    fn enter_on_stop_changes_nothing_and_enter_on_undo_forces() {
        let box_open = |cursor_on_undo: bool| {
            let mut view = drift_box("stage src/a.rs", &drift(), false);
            if cursor_on_undo {
                view.view.toggle();
            }
            AppState {
                confirm: Some(view),
                ..AppState::default()
            }
        };

        let mut stopped = box_open(false);
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut stopped,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert!(commands.is_empty(), "Stop does nothing: {commands:?}");
        assert!(stopped.operating.is_none());

        let mut forced = box_open(true);
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut forced,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert_eq!(commands, vec![Command::Undo(JobId::new(1), true)]);
    }

    #[test]
    fn a_redo_drift_box_re_runs_the_redo() {
        let mut state = AppState {
            confirm: Some(drift_box("undo: commit \"fix\"", &drift(), true)),
            ..AppState::default()
        };
        state
            .confirm
            .as_mut()
            .expect("the box is open")
            .view
            .toggle();
        let mut commands = Vec::new();
        assert!(super::super::confirm::confirm_key(
            &mut state,
            &mut commands,
            KeyPress::plain(KeyCode::Enter),
            &mut ids(),
        ));
        assert_eq!(commands, vec![Command::Redo(JobId::new(1), true)]);
    }

    /// A journal holding one entry per description, each one torn the way
    /// recovery leaves it: `Aborted { reason: "interrupted" }`, which is what
    /// the view is actually handed (`Pending` never survives a read).
    fn read(interrupted: &[&str], unreadable: &[&str]) -> JournalState {
        let entries = interrupted
            .iter()
            .enumerate()
            .map(|(index, description)| torn_entry(index, description))
            .collect();
        JournalState {
            entries: Loading::Ready(Arc::new(JournalRead {
                entries,
                unreadable: Vec::new(),
                live: Vec::new(),
            })),
            unreadable: unreadable.iter().map(|&s| s.to_owned()).collect(),
            ..JournalState::default()
        }
    }

    /// One entry a previous run left open and recovery then closed.
    fn torn_entry(index: usize, description: &str) -> graft_core::journal::JournalEntry {
        use graft_core::journal::{EntryId, EntryStatus, HeadRef, OperationKind, Touches};
        let oid = |byte: u8| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
        let number = i64::try_from(index).expect("a handful of entries") + 1;
        graft_core::journal::JournalEntry {
            id: EntryId::new(number, [0; 10]),
            at_unix_ms: number,
            kind: OperationKind::Commit,
            description: description.to_owned(),
            status: EntryStatus::Aborted {
                reason: graft_core::journal::INTERRUPTED.to_owned(),
            },
            touches: Touches {
                refs: Vec::new(),
                index: false,
                worktree: true,
            },
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
            head_after: None,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            // A backup of the tree it changed, so `in_force` counts it: an
            // entry `u` can no longer reach is not one to announce.
            worktree_before: Some(oid(3)),
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
        }
    }

    #[test]
    fn the_notice_names_the_interrupted_entry_until_it_has_been_seen() {
        let mut state = AppState {
            journal: read(&["commit \"fix\""], &[]),
            ..AppState::default()
        };
        let notice = interrupted_notice(&state).expect("there is one to report");
        assert!(notice.contains("commit \"fix\""), "{notice}");
        assert!(notice.contains("J to inspect"), "{notice}");

        state.journal.notice_seen = true;
        assert_eq!(interrupted_notice(&state), None);
    }

    #[test]
    fn a_clean_journal_has_no_notice() {
        let state = AppState {
            journal: read(&[], &[]),
            ..AppState::default()
        };
        assert_eq!(interrupted_notice(&state), None);
    }

    #[test]
    fn an_unreadable_after_state_says_undo_would_restore_everything() {
        let state = AppState {
            journal: read(&["stash pop"], &["stash pop: could not read HEAD"]),
            ..AppState::default()
        };
        let notice = interrupted_notice(&state).expect("there is one to report");
        assert!(notice.contains("restores the whole tree"), "{notice}");
    }
}

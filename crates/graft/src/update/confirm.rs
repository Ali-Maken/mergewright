//! The confirmation box: opened by `d` (discard) in the changes pane
//! (`update::operate::request_discard`), answered here — its own keys first,
//! nothing else reaches the pane while it is open (`context_menu.rs`'s own
//! pattern, m3-working-tree-undo-and-stash/04).

use std::path::PathBuf;

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;

use graft_ops::operation::Operation;
use graft_ui::confirm::ConfirmView;

use super::events::Command;
use super::state::{AppState, Status};

/// A confirm box waiting for an answer: what it shows, and what `Confirm`
/// does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingConfirm {
    /// What the box draws.
    pub view: ConfirmView,
    /// What `Confirm` dispatches.
    pub then: ConfirmThen,
}

/// What a confirmed box goes on to do.
///
/// Two kinds, because the undo drift guard is not an [`Operation`]: `ops`
/// plans an undo from the journal, not from a key, so there is nothing for
/// `app` to hand over but "the same key again, forcing"
/// (m3-working-tree-undo-and-stash/06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConfirmThen {
    /// Run this write (`d` discard, stash drop).
    Operate(Operation),
    /// Re-run `u` (or `U`) past the drift rail the user has just read.
    Undo {
        /// `true` for `U`.
        redo: bool,
    },
    /// Re-run the journal's time machine past the same rail
    /// (m5-history-and-ghost-preview/08). Its own variant rather than a
    /// flag on [`ConfirmThen::Undo`] because it is a different call with a
    /// different answer: `revert_to` takes an entry id and comes back as
    /// [`crate::jobs::undo::RevertedTo`].
    RevertTo {
        /// The entry to jump back past.
        entry: graft_core::journal::EntryId,
        /// The entry's own words, carried through so the busy line reads
        /// `revert to before “commit …”` rather than the raw ULID
        /// (review 08 Minor 2).
        description: String,
    },
    /// Carry the live operation forward, skip its step, or abort it — `s`
    /// and `a` on the operation banner, which both box before they run
    /// (m5-history-and-ghost-preview/09). `c` does not: continuing is the
    /// operation doing what the user already asked for.
    Resume(graft_ops::apply::Resume),
    /// `u` on work a push has already sent (ADR-0038 §4): the one box with
    /// three real answers, so the index the cursor is on decides which.
    /// `[ Undo locally ]` runs the ordinary undo; `[ Re-push old tip ]`
    /// opens the lease box, which is the confirm that actually touches the
    /// remote — this one never does.
    UndoAfterPush {
        /// The push entry that sent it. Remembered in
        /// [`crate::update::remote::RemoteState::undo_after_push_asked`] once
        /// answered, so the same push explains itself once.
        push: graft_core::journal::EntryId,
        /// What `[ Re-push old tip ]` would push: the old tip, under a
        /// lease. Boxed because it is three `String`s wide and every other
        /// variant here is a word or two. `None` when the box could not
        /// build one — the remote could not be read, or has not answered
        /// yet — and then the box has no third choice to reach it by.
        repush: Option<Box<graft_ops::operation::PushParams>>,
    },
    /// `d` in the resolver: start this file over, behind a confirm box
    /// (m6-conflicts/05, design note Proposed change 2(c)). Its own variant
    /// rather than a plain `Operate` because the resolver has to reopen the
    /// path once the write lands, which `ConfirmThen::Operate` has no way
    /// to ask for.
    RestartConflict {
        /// The file to put back the way git left it.
        path: PathBuf,
    },
}

/// The box's own keys, while it is open: `Left`/`Right` move between
/// `Cancel` and the acting choices and **clamp** at either end, `Tab`
/// cycles, `Enter` acts on whichever is under the cursor, `Esc` closes it
/// without acting. `Ctrl+C` passes through (returns `false`) so the app
/// quits. Every other key is swallowed: the box stays open.
/// Modal: this returns `true` for every other key while a box is open, so
/// the caller sends it nothing else.
pub fn confirm_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(pending) = next.confirm.as_mut() else {
        return false;
    };
    // `Ctrl+C` quits from anywhere, a box included (keymap §2.1), exactly
    // as `preview_key` lets it through.
    if key.code == KeyCode::Char('c') && key.modifiers == Modifiers::CONTROL {
        return false;
    }
    match key.code {
        // The arrows clamp; only `Tab` wraps. The safe choice is index 0
        // and the cursor rests there (ADR-0038 §1), so an `←` that wrapped
        // would put the acting choice one key-repeat away from where the
        // cursor sits — the exact thing the uniform default is for.
        KeyCode::Left => pending.view.left(),
        KeyCode::Right => pending.view.right(),
        KeyCode::Tab | KeyCode::BackTab => pending.view.toggle(),
        KeyCode::Enter => choose(next, commands, next_id),
        KeyCode::Escape => next.confirm = None,
        // Swallowed: the box stays open, cursor and status untouched
        // (usability walk M1). A stray key — `y` from habit, a `j` meant for
        // the graph — must never cancel the box without a word, nor reach
        // what is behind it.
        _ => {}
    }
    true
}

/// `Enter`: close the box, and act if the cursor is off `Cancel`.
fn choose(next: &mut AppState, commands: &mut Vec<Command>, next_id: &mut impl FnMut() -> JobId) {
    // `confirm_key` calls this only with a box open; nothing to answer
    // otherwise.
    let Some(pending) = next.confirm.take() else {
        return;
    };
    // Index 0 is always the safe choice and always where the
    // cursor starts (ADR-0038 §1), so anything else is the box's
    // acting answer. Most boxes have exactly two choices; the
    // undo-after-push box has three, and is handed the index so it
    // can tell `[ Undo locally ]` from `[ Re-push old tip ]`.
    let chosen = pending.view.selected();
    let then = pending.then;
    if chosen != 0 {
        match then {
            ConfirmThen::Operate(op) => {
                super::operate::launch(next, commands, op, next_id);
            }
            ConfirmThen::Undo { redo } => {
                commands.extend(super::undo::forced(next, redo, next_id));
            }
            ConfirmThen::RevertTo { entry, description } => {
                super::undo::revert_to_forced(next, commands, entry, &description, next_id);
            }
            ConfirmThen::Resume(what) => {
                super::banner::resume(next, commands, what, next_id);
            }
            ConfirmThen::UndoAfterPush { push, repush } => {
                super::undo::after_push_chosen(
                    next,
                    commands,
                    chosen,
                    push,
                    repush.map(|params| *params),
                    next_id,
                );
            }
            // A write that began while the box was up owns the
            // index: a second one would run beside it, and the
            // answer of whichever lost the race would be dropped
            // (m6-05 review, Major 3). `restart_box` refuses the
            // same before the box opens.
            ConfirmThen::RestartConflict { .. } if next.operating.is_some() => {
                next.status = Status::sticky("a write is already running");
            }
            ConfirmThen::RestartConflict { path } => {
                super::operate::launch(
                    next,
                    commands,
                    Operation::RestartConflict(graft_ops::operation::RestartParams { path }),
                    next_id,
                );
                // The same "which write is this?" tag `⏎`'s `Stage`
                // carries as `staging`, but on its own field: a
                // restart reopens the file fresh rather than
                // advancing to the next one, and the two must never
                // be confused for the same write landing twice.
                if let Some(open) = next.resolver.as_mut() {
                    open.restarting = next.operating;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use graft_ops::operation::Target;
    use graft_term::sanitize_line;

    use super::*;
    use crate::update::state::AppState;

    fn pending() -> PendingConfirm {
        PendingConfirm {
            view: ConfirmView::new(
                sanitize_line("Discard changes to a.rs?"),
                vec![sanitize_line("consequence")],
                sanitize_line("git restore --worktree -- a.rs"),
            ),
            then: ConfirmThen::Operate(Operation::Discard(Target::Paths(vec!["a.rs".into()]))),
        }
    }

    fn key(code: KeyCode) -> KeyPress {
        KeyPress {
            code,
            modifiers: Modifiers::NONE,
        }
    }

    /// ADR-0038 §1: `Cancel` is where the cursor rests, and walking left
    /// off it must not arrive at the acting choice. `←` on index 0 stays on
    /// index 0 — on the three-choice box too, where `toggle`'s wrap used to
    /// move the cursor *right*.
    #[test]
    fn left_on_cancel_stays_on_cancel() {
        for choices in [
            vec![sanitize_line("Cancel"), sanitize_line("Discard")],
            vec![
                sanitize_line("Cancel"),
                sanitize_line("Undo locally"),
                sanitize_line("Re-push old tip"),
            ],
        ] {
            let mut view = pending();
            view.view = view.view.clone().with_choices(choices);
            let mut state = AppState {
                confirm: Some(view),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Left),
                &mut || { graft_core::job::JobId::new(1) }
            ));
            let after = state.confirm.expect("the box stays open").view;
            assert_eq!(after.selected(), 0, "← walked off Cancel");
            assert!(commands.is_empty(), "moving the cursor ran something");
        }
    }

    /// `→` moves onto the acting choice and clamps at the last one: the
    /// user reaches it on purpose, and never past it back onto `Cancel`.
    #[test]
    fn right_moves_onto_the_acting_choice_and_clamps() {
        let mut view = pending();
        view.view = view.view.clone().with_choices(vec![
            sanitize_line("Cancel"),
            sanitize_line("Undo locally"),
            sanitize_line("Re-push old tip"),
        ]);
        let mut state = AppState {
            confirm: Some(view),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        for _ in 0..4 {
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Right),
                &mut || { graft_core::job::JobId::new(1) }
            ));
        }
        assert_eq!(state.confirm.expect("still open").view.selected(), 2);
    }

    #[test]
    fn absent_box_answers_nothing() {
        let mut state = AppState::default();
        let mut commands = Vec::new();
        assert!(!confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(1)
        ));
    }

    #[test]
    fn tab_toggles_the_cursor() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Tab),
            &mut || { graft_core::job::JobId::new(1) }
        ));
        assert_eq!(state.confirm.unwrap().view.selected(), 1);
    }

    #[test]
    fn enter_on_cancel_closes_without_dispatching() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(1)
        ));
        assert!(state.confirm.is_none());
        assert!(commands.is_empty());
        assert!(state.operating.is_none());
    }

    #[test]
    fn enter_on_confirm_dispatches_the_operation() {
        let mut view_pending = pending();
        view_pending.view.toggle();
        let mut state = AppState {
            confirm: Some(view_pending),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Enter),
            &mut || graft_core::job::JobId::new(7)
        ));
        assert!(state.confirm.is_none());
        assert_eq!(state.operating, Some(graft_core::job::JobId::new(7)));
        assert_eq!(commands.len(), 1);
        let Command::Operate(id, op) = &commands[0] else {
            panic!("expected Operate");
        };
        assert_eq!(*id, graft_core::job::JobId::new(7));
        assert_eq!(**op, Operation::Discard(Target::Paths(vec!["a.rs".into()])));
    }

    #[test]
    fn escape_closes_without_dispatching() {
        let mut state = AppState {
            confirm: Some(pending()),
            ..AppState::default()
        };
        let mut commands = Vec::new();
        assert!(confirm_key(
            &mut state,
            &mut commands,
            key(KeyCode::Escape),
            &mut || graft_core::job::JobId::new(1)
        ));
        assert!(state.confirm.is_none());
        assert!(commands.is_empty());
    }

    fn oid() -> graft_core::oid::Oid {
        graft_core::oid::Oid::from_bytes(&[7; 20]).expect("20 bytes is a SHA-1")
    }

    fn push_params() -> graft_ops::operation::PushParams {
        graft_ops::operation::PushParams {
            branch: "refs/heads/main".into(),
            remote: "origin".into(),
            remote_branch: "refs/heads/main".into(),
            set_upstream: false,
            lease: None,
            source_commit: None,
        }
    }

    fn entry() -> graft_core::journal::EntryId {
        graft_core::journal::EntryId::new(1, [0; 10])
    }

    fn with_then(then: ConfirmThen) -> PendingConfirm {
        PendingConfirm { then, ..pending() }
    }

    /// One row per `confirm = Some` site outside tests (ticket
    /// m6-conflicts/47's grep). The box's keys read only `then`, and only on
    /// `Enter`, so each row carries the `then` its site builds.
    fn every_box() -> Vec<(&'static str, PendingConfirm)> {
        let repush = graft_ops::operation::PushParams {
            lease: Some(graft_ops::operation::Lease { expected: oid() }),
            source_commit: Some(oid()),
            ..push_params()
        };
        vec![
            (
                "operate.rs:515 discard a patch",
                with_then(ConfirmThen::Operate(Operation::Discard(Target::Patch {
                    path: "a.rs".into(),
                    patch: b"@@".to_vec(),
                    side: graft_ops::operation::PatchSide::Worktree,
                }))),
            ),
            ("operate.rs:657 discard paths", pending()),
            (
                "stash.rs:482 stash drop",
                with_then(ConfirmThen::Operate(Operation::StashDrop {
                    index: 0,
                    commit: oid(),
                })),
            ),
        ]
        .into_iter()
        .chain(later_boxes(repush))
        .collect()
    }

    /// The undo, resume, restart and push rows of [`every_box`].
    fn later_boxes(
        repush: graft_ops::operation::PushParams,
    ) -> Vec<(&'static str, PendingConfirm)> {
        vec![
            (
                "banner.rs:491 resume skip",
                with_then(ConfirmThen::Resume(graft_ops::apply::Resume::Skip)),
            ),
            (
                "banner.rs:507 resume abort",
                with_then(ConfirmThen::Resume(graft_ops::apply::Resume::Abort)),
            ),
            (
                "undo.rs:39 undo after push",
                with_then(ConfirmThen::UndoAfterPush {
                    push: entry(),
                    repush: Some(Box::new(repush.clone())),
                }),
            ),
            (
                "undo.rs:300 undo drift guard",
                with_then(ConfirmThen::Undo { redo: false }),
            ),
            (
                "undo.rs:392 revert-to drift guard",
                with_then(ConfirmThen::RevertTo {
                    entry: entry(),
                    description: "commit x".into(),
                }),
            ),
            (
                "resolver/restart.rs:58 restart conflict",
                with_then(ConfirmThen::RestartConflict {
                    path: "a.rs".into(),
                }),
            ),
            (
                "remote/push_box.rs:24 push",
                with_then(ConfirmThen::Operate(Operation::Push(push_params()))),
            ),
            (
                "remote/after_push.rs:247 re-push lease",
                with_then(ConfirmThen::Operate(Operation::Push(repush))),
            ),
        ]
    }

    /// Usability walk M1: a key the box does not bind is swallowed. The box
    /// stays, the cursor stays on `Cancel`, the status line is untouched.
    #[test]
    fn a_key_the_box_does_not_bind_leaves_it_open() {
        let mut state = AppState {
            confirm: Some(with_then(ConfirmThen::Operate(Operation::Push(
                push_params(),
            )))),
            status: Status::sticky("before"),
            ..AppState::default()
        };
        for code in [
            KeyCode::Char('y'),
            KeyCode::Char('q'),
            KeyCode::Char('j'),
            KeyCode::Char('P'),
            KeyCode::Char(' '),
        ] {
            let before = state.status.clone();
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(code),
                &mut || { graft_core::job::JobId::new(1) }
            ));
            let open = state
                .confirm
                .as_ref()
                .unwrap_or_else(|| panic!("{code:?} closed the box"));
            assert_eq!(open.view.selected(), 0, "{code:?} moved the cursor");
            assert_eq!(state.status, before, "{code:?} changed the status");
            assert!(commands.is_empty(), "{code:?} ran something");
        }
    }

    #[test]
    fn a_key_the_box_does_not_bind_leaves_it_open_in_every_box() {
        for (site, open) in every_box() {
            let mut state = AppState {
                confirm: Some(open.clone()),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Char('y')),
                &mut || graft_core::job::JobId::new(1)
            ));
            assert_eq!(state.confirm, Some(open), "{site}: `y` changed the box");
            assert!(commands.is_empty(), "{site}: `y` ran something");
        }
    }

    #[test]
    fn esc_still_closes_every_box_without_acting() {
        for (site, open) in every_box() {
            let mut state = AppState {
                confirm: Some(open),
                ..AppState::default()
            };
            let mut commands = Vec::new();
            assert!(confirm_key(
                &mut state,
                &mut commands,
                key(KeyCode::Escape),
                &mut || graft_core::job::JobId::new(1)
            ));
            assert!(state.confirm.is_none(), "{site}: Esc left the box open");
            assert!(commands.is_empty(), "{site}: Esc ran something");
            assert!(state.operating.is_none(), "{site}: Esc started a write");
        }
    }

    /// Keymap §2.1: every modal excepts `Ctrl+C`, which quits (:164). Driven
    /// through `update`, so any handler between the box and `graph_key`
    /// that claimed it would show here.
    #[test]
    fn ctrl_c_with_a_box_open_quits() {
        for (site, open) in every_box() {
            let state = AppState {
                confirm: Some(open),
                ..AppState::default()
            };
            let event = crate::update::events::AppEvent::Core(graft_core::event::Event::Key(
                KeyPress::control(KeyCode::Char('c')),
            ));
            let (next, commands) = crate::update::update(&state, event);
            assert_eq!(
                next.quit,
                Some(crate::update::state::Quit::Requested),
                "{site}: Ctrl+C did not quit"
            );
            assert!(
                !commands.iter().any(|c| matches!(c, Command::Operate(..))),
                "{site}: Ctrl+C ran a write"
            );
        }
    }
}

//! The confirmation box: opened by `d` (discard) in the changes pane
//! (`update::operate::request_discard`), answered here — its own keys first,
//! nothing else reaches the pane while it is open (`context_menu.rs`'s own
//! pattern, m3-working-tree-undo-and-stash/04).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;

use graft_ops::operation::Operation;
use graft_ui::confirm::ConfirmView;

use super::events::Command;
use super::state::AppState;

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
    },
    /// Carry the live operation forward, skip its step, or abort it — `s`
    /// and `a` on the operation banner, which both box before they run
    /// (m5-history-and-ghost-preview/09). `c` does not: continuing is the
    /// operation doing what the user already asked for.
    Resume(graft_ops::apply::Resume),
}

/// The box's own keys, while it is open: `Left`/`Right` move between
/// `Cancel` and the acting choices and **clamp** at either end, `Tab`
/// cycles, `Enter` acts on whichever is under the cursor, everything else —
/// `Esc` included — closes it without acting.
/// Modal, like the context menu: this returns `true` whenever a box was
/// open, so the caller sends it nothing else.
pub fn confirm_key(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(pending) = next.confirm.as_mut() else {
        return false;
    };
    match key.code {
        // The arrows clamp; only `Tab` wraps. The safe choice is index 0
        // and the cursor rests there (ADR-0038 §1), so an `←` that wrapped
        // would put the acting choice one key-repeat away from where the
        // cursor sits — the exact thing the uniform default is for.
        KeyCode::Left => pending.view.left(),
        KeyCode::Right => pending.view.right(),
        KeyCode::Tab | KeyCode::BackTab => pending.view.toggle(),
        KeyCode::Enter => {
            // Index 0 is always the safe choice and always where the
            // cursor starts (ADR-0038 §1), so anything else is the box's
            // acting answer. Every box `graft` opens today has exactly two.
            let acting = pending.view.selected() != 0;
            let then = pending.then.clone();
            next.confirm = None;
            if acting {
                match then {
                    ConfirmThen::Operate(op) => {
                        super::operate::launch(next, commands, op, next_id);
                    }
                    ConfirmThen::Undo { redo } => {
                        commands.extend(super::undo::forced(next, redo, next_id));
                    }
                    ConfirmThen::RevertTo { entry } => {
                        super::undo::revert_to_forced(next, commands, entry, next_id);
                    }
                    ConfirmThen::Resume(what) => {
                        super::banner::resume(next, commands, what, next_id);
                    }
                }
            }
        }
        _ => next.confirm = None,
    }
    true
}

#[cfg(test)]
mod tests {
    use graft_core::event::Modifiers;
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
}

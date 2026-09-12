//! The middle of the loop: `Event ──▶ update(state) ──▶ (new state, [Command])`
//! (ARCHITECTURE.md).
//!
//! [`update`] is the only place [`AppState`] changes, and it is pure: given the
//! same state and the same event it returns the same answer, every time,
//! forever. It reads no clock, opens no file, spawns no thread and draws
//! nothing. Anything that has to touch the world leaves here as a [`Command`]
//! for the job runner (ticket 10) to execute.
//!
//! That is what makes the application testable without a terminal: every test
//! below is a plain function call over plain data.

use graft_core::event::{Event, KeyCode, KeyPress};

/// Everything Graft knows, as one value.
///
/// Minimal for now, on purpose: this ticket builds the loop, not the
/// application. Screens, the repository and the selection arrive with the
/// tickets that need them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppState {
    /// Set once the user has asked to leave. The event loop reads it after
    /// every event and returns, which drops the `TerminalSession` and hands
    /// the terminal back.
    pub should_quit: bool,
    /// What the status line has to say.
    pub status: Status,
    /// The terminal's last reported size, in character cells.
    pub size: Size,
}

/// What the status line has to say.
///
/// Modelled as a state rather than as a string with an `error:` prefix: a
/// field that always looks like an error teaches the user to ignore it, which
/// is how a real failure gets missed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Nothing to report.
    #[default]
    Ready,
    /// A background job failed, and this is what it said. Shown until
    /// something replaces it — a failure is never dropped on the floor.
    Failed(String),
}

/// A terminal size in character cells.
///
/// Either number can be zero. A pty that has not been sized yet reports `0×0`,
/// and that is what silently broke the graph spike's layout (ADR-0012), so the
/// size is carried as-is and every consumer is expected to cope.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Size {
    /// Width in character cells.
    pub columns: u16,
    /// Height in character cells.
    pub rows: u16,
}

/// Work for the job runner: something [`update`] wants done that it cannot do
/// itself, because doing it would mean touching the world.
///
/// Empty, and that is deliberate. An uninhabited enum makes "there is nothing
/// to run yet" a fact the compiler enforces rather than a promise in a comment:
/// no `Command` value can exist, so the loop cannot silently drop one. The
/// moment ticket 10 adds the first variant, every place that has to handle a
/// command stops compiling until it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {}

/// Answer one event: the new state, and the work it wants done.
///
/// Takes the old state by reference and returns a new one instead of mutating
/// in place, so a caller can always compare before with after — which is what
/// undo, and the tests below, rely on.
#[must_use]
pub fn update(state: &AppState, event: Event) -> (AppState, Vec<Command>) {
    let mut next = state.clone();

    match event {
        Event::Key(key) => {
            if is_quit(key) {
                next.should_quit = true;
            }
        }
        Event::Resize { columns, rows } => {
            next.size = Size { columns, rows };
        }
        Event::JobFailed(id, error) => {
            // Surfaced, not swallowed: the user is told which job failed and
            // why, even though this ticket has no job that can fail yet.
            next.status = Status::Failed(format!("{id} failed: {error}"));
        }
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Mouse(_) | Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
        Event::JobFinished(_) => {}
    }

    (next, Vec::new())
}

/// The two keys that mean "leave" (DESIGN.md §2.2).
///
/// `Ctrl+C` is here as well as `q` because a terminal application that ignores
/// it feels broken, and because raw mode means the terminal no longer turns it
/// into a signal on Graft's behalf.
fn is_quit(key: KeyPress) -> bool {
    let plain_q = key.code == KeyCode::Char('q') && !key.modifiers.control && !key.modifiers.alt;
    let control_c = key.code == KeyCode::Char('c') && key.modifiers.control;
    plain_q || control_c
}

#[cfg(test)]
mod tests {
    use graft_core::error::Error;
    use graft_core::event::{Modifiers, MouseInput, MouseKind};
    use graft_core::job::JobId;

    use super::*;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyPress::plain(code))
    }

    #[test]
    fn q_quits() {
        let (next, commands) = update(&AppState::default(), key(KeyCode::Char('q')));

        assert!(next.should_quit);
        assert!(commands.is_empty());
    }

    #[test]
    fn control_c_quits() {
        let event = Event::Key(KeyPress::control(KeyCode::Char('c')));

        let (next, _) = update(&AppState::default(), event);

        assert!(next.should_quit);
    }

    #[test]
    fn control_q_is_not_quit() {
        // Only the bare `q` is bound. A modified one belongs to whatever
        // binding claims it later, and must not fall through to quitting.
        let event = Event::Key(KeyPress::control(KeyCode::Char('q')));

        let (next, _) = update(&AppState::default(), event);

        assert!(!next.should_quit);
    }

    #[test]
    fn a_bare_c_is_not_quit() {
        let (next, _) = update(&AppState::default(), key(KeyCode::Char('c')));

        assert!(!next.should_quit);
    }

    #[test]
    fn unbound_events_change_nothing() {
        let before = AppState::default();
        let unbound = [
            key(KeyCode::Char('z')),
            key(KeyCode::Enter),
            key(KeyCode::Function(7)),
            Event::Mouse(MouseInput {
                kind: MouseKind::Moved,
                column: 3,
                row: 4,
                modifiers: Modifiers::NONE,
            }),
            Event::Paste("hello".to_owned()),
            Event::FocusGained,
            Event::FocusLost,
            Event::JobFinished(JobId::new(1)),
        ];

        for event in unbound {
            let (next, commands) = update(&before, event.clone());

            assert_eq!(next, before, "{event:?} changed the state");
            assert!(commands.is_empty(), "{event:?} asked for work");
        }
    }

    #[test]
    fn resize_is_remembered_including_a_degenerate_one() {
        let (next, _) = update(
            &AppState::default(),
            Event::Resize {
                columns: 0,
                rows: 0,
            },
        );

        assert_eq!(
            next.size,
            Size {
                columns: 0,
                rows: 0
            }
        );

        let (next, _) = update(
            &next,
            Event::Resize {
                columns: 120,
                rows: 40,
            },
        );

        assert_eq!(
            next.size,
            Size {
                columns: 120,
                rows: 40
            }
        );
    }

    #[test]
    fn a_failed_job_reaches_the_status_line() {
        let error = Error::OidLength { len: 3 };
        let expected = format!("job 9 failed: {error}");

        let (next, _) = update(
            &AppState::default(),
            Event::JobFailed(JobId::new(9), error.clone()),
        );

        assert_eq!(next.status, Status::Failed(expected));
    }

    #[test]
    fn the_same_event_always_gives_the_same_answer() {
        // Purity's observable half: no clock, no counter, no hidden state, so
        // repeating a call repeats the answer.
        let state = AppState {
            should_quit: false,
            status: Status::Failed("earlier".to_owned()),
            size: Size {
                columns: 80,
                rows: 24,
            },
        };

        let first = update(&state, key(KeyCode::Char('q')));
        let second = update(&state, key(KeyCode::Char('q')));

        assert_eq!(first, second);
        assert!(!state.should_quit, "the input state was mutated");
    }

    /// Purity's other half, which no runtime test can observe: that `update`
    /// cannot reach the world at all.
    ///
    /// This module is small and self-contained precisely so this check is
    /// possible — it reads its own source and fails if anything that talks to
    /// the outside (a file, a clock, a thread, a terminal) has been imported
    /// or called above the tests. A reviewer would have to notice; the test
    /// cannot help noticing.
    #[test]
    fn update_cannot_reach_the_world() {
        let source = include_str!("update.rs");
        let (code, _tests) = source
            .split_once("#[cfg(test)]")
            .expect("this module has a test section");

        for forbidden in [
            "std::io",
            "std::fs",
            "std::time",
            "std::thread",
            "std::process",
            "std::net",
            "std::env",
            "SystemTime",
            "Instant",
            "print!",
            "println!",
            "eprint!",
            "eprintln!",
            "crossterm",
            "ratatui",
        ] {
            assert!(
                !code.contains(forbidden),
                "`update` must stay pure, but its module mentions `{forbidden}`"
            );
        }
    }
}

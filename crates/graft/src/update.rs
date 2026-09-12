//! The middle of the loop: `Event ──▶ update(state) ──▶ (new state, [Command])`
//! (ARCHITECTURE.md).
//!
//! [`update`] is the only place [`AppState`] changes, and it is pure: given the
//! same state and the same event it returns the same answer, every time,
//! forever. It reads no clock, opens no file, spawns no thread and draws
//! nothing. Anything that has to touch the world leaves here as a [`Command`]
//! for the job runner (`crate::jobs`) to execute.
//!
//! That is what makes the application testable without a terminal: every test
//! below is a plain function call over plain data.

use std::path::PathBuf;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;

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
    /// The repository Graft was pointed at. Empty in a default state, which is
    /// only ever a starting point: `event_loop::run` fills it in before the
    /// first frame.
    pub repo_path: PathBuf,
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
/// Plain data, with no closure and no handle in it, so that `update` stays a
/// function whose answer can be compared with `assert_eq!`. Turning a command
/// into the code that runs it is the job runner's business (`crate::jobs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the repository at this path and check it is one.
    ///
    /// At M0 that is all there is to read from a repository, so this is also
    /// what `Ctrl+R` (manual refresh, DESIGN.md §2.1) re-runs. The tickets that
    /// add the rev-walk and the status read extend the work behind it.
    OpenRepository(PathBuf),
    /// Stop a job whose result is no longer wanted. Cooperative: a job that has
    /// not started is dropped, and a running one stops at its next check.
    CancelJob(JobId),
}

/// Answer one event: the new state, and the work it wants done.
///
/// Takes the old state by reference and returns a new one instead of mutating
/// in place, so a caller can always compare before with after — which is what
/// undo, and the tests below, rely on.
#[must_use]
pub fn update(state: &AppState, event: Event) -> (AppState, Vec<Command>) {
    let mut next = state.clone();

    let mut commands = Vec::new();

    match event {
        Event::Key(key) => {
            if is_quit(key) {
                next.should_quit = true;
            } else if is_refresh(key) {
                commands.push(Command::OpenRepository(next.repo_path.clone()));
            }
        }
        Event::Resize { columns, rows } => {
            next.size = Size { columns, rows };
        }
        Event::JobFailed(id, failure) => {
            // Surfaced, not swallowed: the user is told which job failed and
            // why. A panicking job says so in the same place — it is still a
            // failure, and the crash report's path rides along with it.
            next.status = Status::Failed(format!("{id} failed: {failure}"));
        }
        Event::JobFinished(_) => {
            // The last thing Graft was asked to do worked, so a failure still
            // on the status line is stale. Nothing keyed by job id is kept yet,
            // which is also what makes a result for a job whose screen has gone
            // harmless: there is nothing for it to overwrite.
            next.status = Status::Ready;
        }
        // Cancellation is not a failure and not news: the application asked for
        // it. The status line keeps whatever it was already saying.
        Event::JobCancelled(_) => {}
        // Nothing is bound to these yet. They reach `update` so that the
        // tickets that need them have somewhere obvious to add the handling.
        Event::Mouse(_) | Event::Paste(_) | Event::FocusGained | Event::FocusLost => {}
    }

    (next, commands)
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

/// Manual refresh (DESIGN.md §2.1): re-read the repository now, rather than
/// waiting for whatever would have noticed on its own.
fn is_refresh(key: KeyPress) -> bool {
    key.code == KeyCode::Char('r') && key.modifiers.control
}

#[cfg(test)]
mod tests {
    use graft_core::event::{Modifiers, MouseInput, MouseKind};
    use graft_core::job::{JobFailure, JobId};

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
            Event::JobCancelled(JobId::new(1)),
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
        let failure = JobFailure::Failed("not a git repository".to_owned());

        let (next, commands) = update(
            &AppState::default(),
            Event::JobFailed(JobId::new(9), failure),
        );

        assert_eq!(
            next.status,
            Status::Failed("job 9 failed: not a git repository".to_owned())
        );
        assert!(commands.is_empty());
    }

    #[test]
    fn a_panicking_job_says_so_and_names_its_report() {
        let failure = JobFailure::Panicked {
            message: "lane 3 vanished".to_owned(),
            report: Some("/state/graft/crash-1.log".to_owned()),
        };

        let (next, _) = update(
            &AppState::default(),
            Event::JobFailed(JobId::new(2), failure),
        );

        assert_eq!(
            next.status,
            Status::Failed(
                "job 2 failed: panicked: lane 3 vanished — report: /state/graft/crash-1.log"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_job_that_worked_clears_a_stale_failure() {
        let failed = AppState {
            status: Status::Failed("job 1 failed: not a git repository".to_owned()),
            ..AppState::default()
        };

        let (next, _) = update(&failed, Event::JobFinished(JobId::new(2)));

        assert_eq!(next.status, Status::Ready);
    }

    #[test]
    fn a_cancelled_job_leaves_the_status_line_alone() {
        // Cancellation is something Graft asked for. Reporting it would put an
        // error in front of the user when nothing is wrong (CLAUDE.md).
        let failed = AppState {
            status: Status::Failed("job 1 failed: not a git repository".to_owned()),
            ..AppState::default()
        };

        let (next, _) = update(&failed, Event::JobCancelled(JobId::new(1)));

        assert_eq!(next, failed);
    }

    #[test]
    fn control_r_asks_for_the_repository_to_be_re_read() {
        let state = AppState {
            repo_path: PathBuf::from("/code/api"),
            ..AppState::default()
        };

        let (next, commands) = update(&state, Event::Key(KeyPress::control(KeyCode::Char('r'))));

        assert_eq!(
            commands,
            vec![Command::OpenRepository(PathBuf::from("/code/api"))]
        );
        // Asking for work is all it does: the state is the job's to change,
        // once the job comes back.
        assert_eq!(next, state);
    }

    #[test]
    fn a_bare_r_asks_for_nothing() {
        // `r` is rebase on the history screen (DESIGN.md §2.3), not refresh.
        let (_, commands) = update(&AppState::default(), key(KeyCode::Char('r')));

        assert!(commands.is_empty());
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
            repo_path: PathBuf::from("/code/api"),
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

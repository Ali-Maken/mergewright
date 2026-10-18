//! Runs Mergewright to one finished frame with no terminal at all, for a stdout
//! [`super::run`] found is not a tty (ticket m1-the-graph/19).

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use mergewright_term::detect::{self, TtyHost};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::opening_repository;
use super::{ChannelWakes, Exit, Wake, Wakes};
use crate::history::Phase;
use crate::jobs::{Dispatch, JobRunner};
use crate::repos::RepoStore;
use crate::session::Session;
use crate::update::{AppState, Quit, Size, update};
use crate::view::view;

/// Runs Mergewright to one finished frame with no terminal at all, for a stdout
/// that [`super::run`] found is not a tty (ticket m1-the-graph/19).
///
/// The same `update`/`Command`/job machinery as the interactive loop — jobs
/// are dispatched and awaited over the same channel `pump` uses — but there
/// is nothing to draw between wakes: nobody is watching a screen that
/// blinks, so the loop simply runs the walk to its end and prints one plain
/// frame, in the tier `Session::settle` chose (ASCII, since detection never
/// probes a terminal that cannot be probeable — `mergewright_term::detect`).
pub(super) fn run_headless(
    repo_path: &Path,
    config: &crate::config::Loaded,
    repos: Arc<RepoStore>,
) -> io::Result<Exit> {
    // Never probed (stdout is not a terminal), so no mux is started to read
    // a stdin that may belong to the user's shell.
    let capabilities = detect::detect(&detect::Environment::from_process(), &mut TtyHost::unread());
    let session = Session::settle(config, capabilities);

    let (mut wakes, sender) = ChannelWakes::new();
    let jobs = JobRunner::with_default_size(sender, repos)?;

    // No real terminal to size against; wide and tall enough that the
    // fixtures this ships with are not truncated mid-row.
    let size = Size {
        columns: 200,
        rows: 200,
    };
    let mut state = opening_repository(&jobs, repo_path, size);

    while !headless_settled(&state) {
        let Some(wake) = wakes.next_wake() else {
            break;
        };
        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Exit::Signal(signal)),
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            // Headless runs start no redetector and no fence listener, so
            // nothing sends these; and plain text has no cell size or theme
            // to change, and is never paced, if one did.
            Wake::Terminal(_) | Wake::Fence => continue,
        };

        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        if let Some(Quit::Fatal(reason)) = &state.quit {
            return Ok(Exit::Fatal(reason.clone()));
        }
    }

    print_headless_frame(&session, &state, size)?;
    Ok(Exit::Quit)
}

/// Whether the headless loop has seen enough to print: history exists and
/// its walk has left `Streaming`, one way or another.
///
/// `None` (before `Task::ReadHistory`'s `Started` event has been handled) is
/// not settled — counting it as settled printed an "opening…" frame with no
/// rows before the repository ever opened (M1 review, M4). This cannot spin
/// forever: a repository that fails to open sets `Quit::Fatal` before this is
/// checked again, and `run_headless`'s wake loop itself ends (breaking out
/// rather than looping) once its wake source is exhausted.
pub(super) fn headless_settled(state: &AppState) -> bool {
    state
        .history
        .as_ref()
        .is_some_and(|history| !matches!(history.phase, Phase::Streaming))
}

/// Renders one frame into a backend nothing writes escape bytes to, and
/// prints it as plain text: exactly the picture a real terminal would show,
/// with none of the bytes that only make sense to one.
fn print_headless_frame(session: &Session, state: &AppState, size: Size) -> io::Result<()> {
    let mut terminal = Terminal::new(TestBackend::new(size.columns, size.rows))
        .unwrap_or_else(|error: std::convert::Infallible| match error {});
    terminal
        .draw(|frame| view(session, state, frame))
        .unwrap_or_else(|error: std::convert::Infallible| match error {});

    let buffer = terminal.backend().buffer();
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(text.as_bytes())?;
    stdout.flush()
}

//! The commands the frame loop carries out itself rather than dispatching:
//! they need the terminal, which no worker thread may touch. The six
//! handovers give it to another program and take it back
//! (`super::super::outside`); the clipboard is one OSC 52 write.

use std::collections::VecDeque;
use std::io::{self, Write};

use graft_core::job::JobId;
use graft_term::Transport;
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::super::outside::{Handover, Outside};
use super::super::redetect;
use crate::jobs::Dispatch;
use crate::session::Session;
use crate::update::{AppEvent, AppState, Command, Product, update};

/// Do what one frame's commands ask for, and return the state they left.
///
/// Most go straight to the job runner and are forgotten — `dispatch` starts
/// the work and returns, which is the line that keeps the keyboard responsive
/// during a two-minute fetch (ARCHITECTURE.md). The few the loop carries out
/// itself are output, not work: they need the terminal, which no worker
/// thread may touch. Those answer straight back into `update`, and what it
/// answers with joins the same queue.
pub(in crate::event_loop) fn carry_out<B, D>(
    terminal: &mut Terminal<B>,
    session: &Session,
    jobs: &D,
    mut state: AppState,
    outside: Outside<'_>,
    commands: Vec<Command>,
) -> io::Result<(AppState, Carried)>
where
    B: Backend<Error = io::Error> + Write,
    D: Dispatch,
{
    let mut queue: VecDeque<Command> = commands.into();
    let mut carried = Carried::Continue;
    while let Some(command) = queue.pop_front() {
        match command {
            // The clipboard request goes to the terminal now, and a failed
            // write ends the loop like any other.
            Command::CopyToClipboard(text) => write_clipboard(terminal, session, &text)?,
            Command::SaveLayout(split) => {
                if let Some(common_dir) = outside.common_dir {
                    // Best-effort (`crate::layout`'s module doc): a missing or
                    // unwritable `graft` directory is the expected case here,
                    // not a failure worth surfacing.
                    drop(crate::layout::save_split(common_dir, &split));
                }
            }
            Command::FirstLaunchDone(config_dir) => {
                // The shell's switch first: ended once per process, so a
                // later place arrives without the line; already ended, there
                // is nothing to write. With no switch lent, the folder named.
                let to_write = match outside.first_launch {
                    Some(line) => line.end(),
                    None => Some(config_dir),
                };
                if let Some(config_dir) = to_write
                    && let Err(error) = crate::first_launch::first_launch_done(&config_dir)
                {
                    state.status = crate::update::Status::Failed(error.words());
                }
            }
            // These six restore the terminal, hand it over and take it
            // back (`super::outside`).
            command if hands_over(&command) => {
                carried = Carried::EndsBatch;
                let answer = handed_over(outside.handover.as_ref(), command);
                // Whatever ran out there drew over every cell, and the frame
                // loop's own idea of the screen is now wrong.
                terminal.clear()?;
                let (next, more) = update(&state, answer);
                state = next;
                queue.extend(more);
            }
            command => jobs.dispatch(command),
        }
    }
    if state.quit.is_some() {
        carried = Carried::EndsBatch;
    }
    Ok((state, carried))
}

/// The six commands that restore the terminal, hand it over and take it
/// back (`super::outside`).
pub(super) fn hands_over(command: &Command) -> bool {
    matches!(
        command,
        Command::EditCommitMessage { .. }
            | Command::OpenExternalEditor { .. }
            | Command::OpenMergetool { .. }
            | Command::OpenConflictEditor { .. }
            | Command::ContinueInGit { .. }
            | Command::Suspend
    )
}

/// Whether the turn may take another queued wake after this one's commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::event_loop) enum Carried {
    /// Nothing stops the turn here.
    Continue,
    /// A command handed the terminal over, or the loop is ending: the turn
    /// ends and what is queued stays queued.
    EndsBatch,
}

/// Carry out one of the six commands that need the terminal itself, and
/// say what happened as the event `update` answers next.
///
/// Without a [`Handover`] — the headless path, and every test over a
/// `TestBackend` — the key is refused in words rather than ignored: a `Ctrl+E`
/// that does nothing at all reads as a hang.
fn handed_over(outside: Option<&Handover<'_>>, command: Command) -> AppEvent {
    let Some(outside) = outside else {
        let why = "there is no terminal to hand over".to_owned();
        return match command {
            Command::EditCommitMessage { id, .. } => {
                AppEvent::Produced(id, Product::ExternalEditor(Err(why)))
            }
            // Refused in words, with the path, like every other mergetool
            // failure: the conflict is still there, and the banner's line
            // has to say so rather than look as though the tool ran.
            Command::OpenMergetool { path } => AppEvent::Produced(
                JobId::new(0),
                Product::Mergetool {
                    path,
                    result: Err(why),
                },
            ),
            Command::ContinueInGit { id, .. } => {
                AppEvent::Produced(id, Product::ContinuedInGit(Err(why)))
            }
            _ => AppEvent::Produced(JobId::new(0), Product::HandedOver(Err(why))),
        };
    };
    match command {
        Command::EditCommitMessage { id, message } => {
            AppEvent::Produced(id, Product::ExternalEditor(outside.edit_message(&message)))
        }
        Command::OpenExternalEditor { path, line } => AppEvent::Produced(
            JobId::new(0),
            Product::HandedOver(outside.edit_file(&path, line)),
        ),
        Command::OpenConflictEditor { path } => AppEvent::Produced(
            JobId::new(0),
            Product::HandedOver(outside.edit_conflict(&path)),
        ),
        Command::OpenMergetool { path } => {
            let result = outside.mergetool(&path);
            AppEvent::Produced(JobId::new(0), Product::Mergetool { path, result })
        }
        Command::ContinueInGit { id, kind } => AppEvent::Produced(
            id,
            Product::ContinuedInGit(outside.continue_in_git(kind).map(|status| status.code())),
        ),
        Command::Suspend => {
            AppEvent::Produced(JobId::new(0), Product::HandedOver(outside.suspend()))
        }
        // `frames` matches exactly these six before calling.
        other => unreachable!("not a terminal handover: {other:?}"),
    }
}

/// Ask the terminal to put `text` on its clipboard (OSC 52), wrapped for
/// tmux the way the image commands are, as one whole write.
fn write_clipboard<B>(terminal: &mut Terminal<B>, session: &Session, text: &str) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
{
    let mut bytes = Vec::new();
    Transport::for_tmux(&session.capabilities.tmux)
        .write(&graft_term::clipboard::osc52_copy(text), &mut bytes);
    let _whole = redetect::hold_output();
    let backend = terminal.backend_mut();
    backend
        .write_all(&bytes)
        .and_then(|()| Write::flush(backend))
}

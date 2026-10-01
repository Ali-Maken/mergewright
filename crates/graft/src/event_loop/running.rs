//! The loop's body: draw, wait, update, repeat. Split out from
//! [`super::startup::run`] so it can be driven over any backend, any source
//! of wakes and any job runner — which is what lets the frame bytes be
//! asserted in a test.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_graph::raster::cache::Flush;
use graft_term::Transport;
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::graphics::{Graphics, GraphicsReplies, NoReplies};
use super::outside::{Handover, Outside};
use super::redetect;
use super::{Exit, Phase, Wake, Wakes};
use crate::jobs::Dispatch;
use crate::session::{Session, TerminalChange};
use crate::update::{AppEvent, AppState, Command, Product, Quit, update};
use crate::view::{draws_pixels, view_with_graphics};

/// Begin a synchronized update: the terminal buffers what follows and paints
/// it in one go (DEC 2026, DESIGN.md §4.1).
const BEGIN_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026h";

/// End a synchronized update, which is what makes the frame appear.
pub(super) const END_SYNCHRONIZED_UPDATE: &[u8] = b"\x1b[?2026l";

/// The loop itself: draw, wait, update, repeat.
///
/// Split out from [`super::run`] so it can be driven over any backend, any
/// source of wakes and any job runner — which is what lets the frame bytes be
/// asserted in a test.
pub fn pump<B, W, D>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: AppState,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
{
    pump_with(
        terminal,
        wakes,
        jobs,
        session,
        state,
        &mut Graphics::new(NoReplies),
        Outside::default(),
    )
}

/// [`pump`] at the text tiers, handing back the state the loop ended on too:
/// what a test reads to see where a turn's keys left the cursor.
#[cfg(test)]
pub(super) fn pump_keeping<B, W, D>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    mut state: AppState,
) -> (io::Result<Exit>, AppState)
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
{
    let exit = frames(
        terminal,
        wakes,
        jobs,
        session,
        &mut state,
        &mut Graphics::new(NoReplies),
        Outside::default(),
    );
    (exit, state)
}

/// [`pump`], with the pixel tier's images and the replies about them; every
/// upload is freed however the loop ends.
pub(super) fn pump_with<B, W, D, R>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: AppState,
    graphics: &mut Graphics<R>,
    outside: Outside<'_>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    let mut state = state;
    let exit = frames(
        terminal, wakes, jobs, session, &mut state, graphics, outside,
    );
    let released = release(terminal, graphics);
    // When the loop itself failed, its error is the one to report; a free
    // that fails too failed writing to the same broken terminal.
    exit.and_then(|exit| released.map(|()| exit))
}

/// The most update time a turn spends on queued wakes before its frame shows
/// them — job answers and keys alike. Half a 60 Hz frame: a burst longer than
/// this still draws as it goes, so the screen never sits stale behind a long
/// queue.
const BATCH_BUDGET: Duration = Duration::from_millis(8);

/// The loop's body: draw, wait, update, repeat.
fn frames<B, W, D, R>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: &mut AppState,
    graphics: &mut Graphics<R>,
    outside: Outside<'_>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    // Draw before waiting: the first frame must be on screen while Graft sits
    // idle waiting for the user's first key.
    draw(terminal, session, state, graphics)?;
    // When the first update not yet drawn began: job answers queued back to
    // back share one frame, up to `BATCH_BUDGET` of updates.
    let mut undrawn_since: Option<Instant> = None;

    loop {
        // Idle: nothing is waiting, so get the next page ready.
        prefetch(session, state, graphics, wakes);
        let Some(wake) = wakes.next_wake() else {
            // Nothing can wake the loop again. Leave as if the user had quit
            // rather than spinning on a dead channel.
            return Ok(Exit::Quit);
        };
        // Replies to the frames already drawn: an `ENOENT` here makes the
        // next frame transmit that strip again.
        let replied = graphics.drain()?;
        wakes.mark(Phase::Drained);

        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Exit::Signal(signal)),
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            Wake::Terminal(change) => {
                let changed = session.apply(change);
                if changed {
                    graphics.flush(match change {
                        TerminalChange::CellSize(_) => Flush::CellSize,
                        TerminalChange::Theme(_) => Flush::Theme,
                    });
                }
                if changed || replied {
                    draw(terminal, session, state, graphics)?;
                    undrawn_since = None;
                }
                continue;
            }
        };

        let began = *undrawn_since.get_or_insert_with(Instant::now);
        let (next, commands) = update(state, event);
        *state = next;
        wakes.mark(Phase::Updated);

        let (next, carried) = carry_out(
            terminal,
            session,
            jobs,
            std::mem::take(state),
            outside,
            commands,
        )?;
        *state = next;
        wakes.mark(Phase::CarriedOut);

        match &state.quit {
            Some(Quit::Requested) => return Ok(Exit::Quit),
            Some(Quit::HandedBack(key)) => return Ok(Exit::HandedBack(*key)),
            // Reported by the caller, once the terminal is the user's again.
            Some(Quit::Fatal(reason)) => return Ok(Exit::Fatal(reason.clone())),
            None => {}
        }

        // One turn (m7-power-tools/43): while the next wake joins the frame
        // — a job's answer, a key, a wheel scroll — apply it before drawing,
        // so a backlog of held-key repeats is shown by one frame, not one
        // frame each. A handover ends the turn (what follows waits for the
        // terminal to be ours again), and so does `BATCH_BUDGET`.
        if carried == Carried::Continue && began.elapsed() < BATCH_BUDGET && wakes.joining_waiting()
        {
            continue;
        }
        draw(terminal, session, state, graphics)?;
        undrawn_since = None;
        wakes.mark(Phase::Drawn);
    }
}

/// Do what one frame's commands ask for, and return the state they left.
///
/// Most go straight to the job runner and are forgotten — `dispatch` starts
/// the work and returns, which is the line that keeps the keyboard responsive
/// during a two-minute fetch (ARCHITECTURE.md). The few the loop carries out
/// itself are output, not work: they need the terminal, which no worker
/// thread may touch. Those answer straight back into `update`, and what it
/// answers with joins the same queue.
pub(super) fn carry_out<B, D>(
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
            // These six restore the terminal, hand it over and take it
            // back (`super::outside`).
            command @ (Command::EditCommitMessage { .. }
            | Command::OpenExternalEditor { .. }
            | Command::OpenMergetool { .. }
            | Command::OpenConflictEditor { .. }
            | Command::ContinueInGit { .. }
            | Command::Suspend) => {
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

/// Whether the turn may take another queued wake after this one's commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Carried {
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

/// While nothing is waiting, rasterise the strips a page key would show:
/// `PageDown`'s and `PageUp`'s next screens, as `update` would draw them
/// (their cursor moves, so their thread and fold do too, and only strips
/// keyed exactly like that frame's are hits). Asks `wakes` before every
/// strip, so a key that arrives waits behind one strip at most; it never
/// uploads anything. Returns the strips rasterised.
pub(super) fn prefetch<W, R>(
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
    wakes: &mut W,
) -> usize
where
    W: Wakes,
    R: GraphicsReplies,
{
    // The search prompt takes page keys as text: nothing to ready there.
    if !draws_pixels(session, state) || state.search_prompt.is_some() {
        return 0;
    }
    let mut rasterised = 0;
    for page in [KeyCode::PageDown, KeyCode::PageUp] {
        // A look before each page's what-if too: its `update` clones the
        // state (every ref badge with it), work a key must not queue behind.
        if wakes.waiting() {
            break;
        }
        // What the screen would be after `page`; never shown. Its commands
        // (rows to fetch) are dropped: the real key asks for them again.
        let (paged, _unsent) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(page))));
        rasterised += graphics.prefetch(session, &paged, &mut || wakes.waiting());
    }
    rasterised
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

/// Delete every image Graft uploaded, as one whole write.
fn release<B, R>(terminal: &mut Terminal<B>, graphics: &mut Graphics<R>) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    let mut bytes = Vec::new();
    graphics.release(&mut bytes);
    if bytes.is_empty() {
        return Ok(());
    }
    let _whole = redetect::hold_output();
    let backend = terminal.backend_mut();
    backend
        .write_all(&bytes)
        .and_then(|()| Write::flush(backend))
}

/// Draw one frame, wrapped in a synchronized update.
///
/// The begin/end pair goes out unconditionally (DESIGN.md §4.1): a terminal
/// that does not understand DEC 2026 ignores the sequence, and probing for it
/// would cost a round-trip on every frame to learn something that changes
/// nothing.
///
/// At the pixel tier the images go out after the cells, inside the same
/// update, so text and graph appear together; never after a clear.
pub(super) fn draw<B, R>(
    terminal: &mut Terminal<B>,
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    // A whole frame, so a re-query cannot land inside one of its sequences.
    let _frame = redetect::hold_output();
    terminal
        .backend_mut()
        .write_all(BEGIN_SYNCHRONIZED_UPDATE)?;

    let mut graph = None;
    let drawn = terminal
        .draw(|frame| graph = view_with_graphics(session, state, graphics.status(), frame))
        .map(|_| ());
    let placed = drawn.as_ref().map_or(Ok(()), |()| {
        let mut images = Vec::new();
        graphics.frame(session, state, graph, &mut images);
        terminal.backend_mut().write_all(&images)
    });

    // The end marker goes out even when the draw failed. Leaving the terminal
    // inside a synchronized update would freeze the user's screen on whatever
    // was there before, with no way to tell that anything is wrong.
    let ended = terminal
        .backend_mut()
        .write_all(END_SYNCHRONIZED_UPDATE)
        .and_then(|()| Write::flush(terminal.backend_mut()));

    drawn.and(placed).and(ended)
}

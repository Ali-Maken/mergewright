//! The loop's body: draw, wait, update, repeat. Split out from
//! [`super::startup::run`] so it can be driven over any backend, any source
//! of wakes and any job runner — which is what lets the frame bytes be
//! asserted in a test.

use std::io::{self, Write};
use std::path::Path;

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_graph::raster::cache::Flush;
use graft_term::Transport;
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::graphics::{Graphics, GraphicsReplies, NoReplies};
use super::redetect;
use super::{Exit, Wake, Wakes};
use crate::jobs::Dispatch;
use crate::session::{Session, TerminalChange};
use crate::update::{AppEvent, AppState, Command, Quit, update};
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
        None,
    )
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
    common_dir: Option<&Path>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    let exit = frames(terminal, wakes, jobs, session, state, graphics, common_dir);
    let released = release(terminal, graphics);
    // When the loop itself failed, its error is the one to report; a free
    // that fails too failed writing to the same broken terminal.
    exit.and_then(|exit| released.map(|()| exit))
}

/// The loop's body: draw, wait, update, repeat.
fn frames<B, W, D, R>(
    terminal: &mut Terminal<B>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    mut state: AppState,
    graphics: &mut Graphics<R>,
    common_dir: Option<&Path>,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    // Draw before waiting: the first frame must be on screen while Graft sits
    // idle waiting for the user's first key.
    draw(terminal, session, &state, graphics)?;

    loop {
        // Idle: nothing is waiting, so get the next page ready.
        prefetch(session, &state, graphics, wakes);
        let Some(wake) = wakes.next_wake() else {
            // Nothing can wake the loop again. Leave as if the user had quit
            // rather than spinning on a dead channel.
            return Ok(Exit::Quit);
        };
        // Replies to the frames already drawn: an `ENOENT` here makes the
        // next frame transmit that strip again.
        let replied = graphics.drain()?;

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
                    draw(terminal, session, &state, graphics)?;
                }
                continue;
            }
        };

        let (next, commands) = update(&state, event);
        state = next;

        for command in commands {
            // Output, not work: the clipboard request goes to the terminal
            // now, and a failed write ends the loop like any other.
            let command = match command {
                Command::CopyToClipboard(text) => {
                    write_clipboard(terminal, session, &text)?;
                    continue;
                }
                Command::SaveLayout(split) => {
                    if let Some(common_dir) = common_dir {
                        // Best-effort (`crate::layout`'s module doc): a
                        // missing or unwritable `graft` directory is the
                        // expected case here, not a failure worth surfacing.
                        drop(crate::layout::save_split(common_dir, &split));
                    }
                    continue;
                }
                command => command,
            };
            // Handed over and forgotten. `dispatch` starts the work and
            // returns: there is no handle here to join and no result to wait
            // for, because the result comes back as another `Event`
            // (ARCHITECTURE.md). This is the line that keeps the keyboard
            // responsive during a two-minute fetch.
            jobs.dispatch(command);
        }

        match &state.quit {
            Some(Quit::Requested) => return Ok(Exit::Quit),
            Some(Quit::HandedBack(key)) => return Ok(Exit::HandedBack(*key)),
            // Reported by the caller, once the terminal is the user's again.
            Some(Quit::Fatal(reason)) => return Ok(Exit::Fatal(reason.clone())),
            None => {}
        }

        draw(terminal, session, &state, graphics)?;
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

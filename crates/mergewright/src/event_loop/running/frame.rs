//! One frame's bytes: `SYNC_BEGIN` · text · image commands · `SYNC_END` ·
//! fence (DESIGN.md §4.7, ADR-0054), and the frame trace's line for it.

use std::io::{self, Write};

use mergewright_term::{SYNC_BEGIN, SYNC_END};
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::super::graphics::{Graphics, GraphicsReplies};
use super::super::redetect;
use crate::session::Session;
use crate::trace::Tracer;
use crate::update::{AppState, Status};
use crate::view::view_with_graphics;

/// End a synchronized update, which is what makes the frame appear. The
/// tests find frames by it.
#[cfg(test)]
pub(in crate::event_loop) const END_SYNCHRONIZED_UPDATE: &[u8] = SYNC_END;

/// [`draw_frame`], with the frame's line in the trace when one is being
/// written. Returns `true` when the status row changed after the frame was
/// built, so another frame is owed.
///
/// A trace that could not be opened, or stopped on a failed write, says so
/// on the status row (`trace: <reason>`) — once, as a sticky failure `Esc`
/// dismisses — and the app runs on: the trace is a measurement, never a
/// reason to stop. The loop sets the status itself because the failure is
/// the loop's own (its file), not an event `update` was told about.
pub(super) fn draw_traced<B, R>(
    terminal: &mut Terminal<B>,
    session: &Session,
    state: &mut AppState,
    graphics: &mut Graphics<R>,
    tracer: &mut Tracer,
    fence: bool,
) -> io::Result<bool>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    if let Some(reason) = tracer.unreported() {
        state.status = Status::sticky_failure(format!("trace: {reason}"));
    }
    let began = tracer.clock();
    draw_frame(terminal, session, state, graphics, fence)?;
    tracer.drew(began, state);
    Ok(match tracer.unreported() {
        Some(reason) => {
            state.status = Status::sticky_failure(format!("trace: {reason}"));
            true
        }
        None => false,
    })
}

/// [`draw_frame`] with no fence: one frame, as a test draws it.
#[cfg(test)]
pub(in crate::event_loop) fn draw<B, R>(
    terminal: &mut Terminal<B>,
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    draw_frame(terminal, session, state, graphics, false)
}

/// Draw one frame: `SYNC_BEGIN` · text · image commands · `SYNC_END` ·
/// fence (DESIGN.md §4.7).
///
/// The begin/end pair goes out unconditionally (DESIGN.md §4.1): a terminal
/// that does not understand DEC 2026 ignores the sequence, and probing for it
/// would cost a round-trip on every frame to learn something that changes
/// nothing.
///
/// At the pixel tier the images go out after the cells, inside the same
/// update, so text and graph appear together; never after a clear. The
/// fence (`CSI 5 n`, when `fence`) goes after the end marker: its answer
/// says the terminal has read the whole frame.
pub(in crate::event_loop) fn draw_frame<B, R>(
    terminal: &mut Terminal<B>,
    session: &Session,
    state: &AppState,
    graphics: &mut Graphics<R>,
    fence: bool,
) -> io::Result<()>
where
    B: Backend<Error = io::Error> + Write,
    R: GraphicsReplies,
{
    // A whole frame, so a re-query cannot land inside one of its sequences.
    let _frame = redetect::hold_output();
    terminal.backend_mut().write_all(SYNC_BEGIN)?;

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
    let backend = terminal.backend_mut();
    let ended = backend
        .write_all(SYNC_END)
        .and_then(|()| {
            if fence {
                mergewright_term::write_fence(backend)
            } else {
                Ok(())
            }
        })
        .and_then(|()| Write::flush(backend));

    drawn.and(placed).and(ended)
}

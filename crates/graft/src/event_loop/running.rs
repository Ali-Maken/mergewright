//! The loop's entry points: [`pump`] and its test and pixel-tier forms,
//! the idle work between wakes ([`prefetch`]) and the free at exit. The
//! loop itself is split by what it does:
//!
//! - [`turn`]: the paced turn — when a wake is applied, when a frame is
//!   drawn, the fence settled before a handover (ADR-0054, G-settle).
//! - [`carry`]: the commands the loop carries out itself, which need the
//!   terminal: the six handovers and the clipboard.
//! - [`frame`]: one frame's bytes — bracket, text, images, fence.
//!
//! Split out from [`super::startup::run`] so it can be driven over any
//! backend, any source of wakes and any job runner — which is what lets the
//! frame bytes be asserted in a test.

mod carry;
mod frame;
mod quit_clock;
mod turn;

use std::io::{self, Write};

use graft_core::event::{Event, KeyCode, KeyPress};
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::graphics::{Graphics, GraphicsReplies, NoReplies};
use super::outside::Outside;
use super::redetect;
use super::{Exit, Wakes};
use crate::jobs::Dispatch;
use crate::session::Session;
use crate::trace::Tracer;
use crate::update::{AppEvent, AppState, update};
use crate::view::draws_pixels;

#[cfg(test)]
pub(super) use carry::{Carried, carry_out};
#[cfg(test)]
pub(super) use frame::{END_SYNCHRONIZED_UPDATE, draw};
use turn::frames;

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
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    pump_with(
        Screen {
            terminal,
            graphics: &mut graphics,
        },
        wakes,
        jobs,
        session,
        state,
        Outside::default(),
        &mut Tracer::off(),
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
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let exit = frames(
        Screen {
            terminal,
            graphics: &mut graphics,
        },
        wakes,
        jobs,
        session,
        &mut state,
        Outside::default(),
        &mut Tracer::off(),
    );
    (exit, state)
}

/// Where a frame goes: its text through `terminal`, the pixel tier's images
/// through `graphics`. Both are one frame, inside one synchronized update and
/// behind one fence (ADR-0054), so they travel together.
pub(super) struct Screen<'s, B: Backend, R> {
    pub(super) terminal: &'s mut Terminal<B>,
    pub(super) graphics: &'s mut Graphics<R>,
}

/// [`pump`], with the pixel tier's images and the replies about them; every
/// upload is freed however the loop ends. `tracer` writes the frame trace
/// (`crate::trace`); [`Tracer::off`] for none.
pub(super) fn pump_with<B, W, D, R>(
    screen: Screen<'_, B, R>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: AppState,
    outside: Outside<'_>,
    tracer: &mut Tracer,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    let Screen { terminal, graphics } = screen;
    let mut state = state;
    let exit = frames(
        Screen {
            terminal: &mut *terminal,
            graphics: &mut *graphics,
        },
        wakes,
        jobs,
        session,
        &mut state,
        outside,
        tracer,
    );
    let released = release(terminal, graphics);
    // When the loop itself failed, its error is the one to report; a free
    // that fails too failed writing to the same broken terminal.
    exit.and_then(|exit| released.map(|()| exit))
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

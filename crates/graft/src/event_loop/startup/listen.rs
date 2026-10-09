//! The loop's input side and its reach outside itself: the key forwarder,
//! the frame trace, the fence listener, and the input handed back after.

use std::io;
use std::sync::mpsc::Sender;

use graft_term::input::{FenceAnswers, InputMux, Inputs};

use super::super::outside::{Handover, Outside};
use super::super::redetect::Redetector;
use super::super::{ChannelWakes, Wake, input};
use crate::session::Session;
use crate::terminal::TerminalSession;
use crate::trace::{self, TerminalOut, Tracer};

/// What the loop may reach outside itself: the repository's directories,
/// the terminal to hand to `$EDITOR` or a mergetool, and the process's
/// first-launch switch.
pub(super) fn outside<'a>(
    session: &'a TerminalSession,
    mux: &'a InputMux,
    common_dir: Option<&'a std::path::Path>,
    workdir: Option<&'a std::path::Path>,
    first_launch: Option<&'a crate::first_launch::FirstLaunch>,
) -> Outside<'a> {
    Outside {
        common_dir,
        first_launch,
        handover: Some(Handover {
            session,
            mux,
            // The common directory, not the worktree's own: `crate::layout`
            // already keeps Graft's files there, and the message file is
            // written and read back within one key press.
            git_dir: common_dir,
            workdir,
        }),
    }
}

/// The loop's input side: the key forwarder, the frame trace and the fence
/// listener.
pub(super) struct Listening {
    pub(super) forwarder: input::Forwarder,
    pub(super) tracer: Tracer,
    /// The terminal's writer, metered when tracing.
    pub(super) out: TerminalOut,
    /// Held for the loop's life; `None` when the terminal is never fenced.
    pub(super) fences: Option<FenceAnswers>,
}

/// Start forwarding input to the loop. The frame trace (`crate::trace`) is
/// opened first: the forwarder stamps keys only when there is a trace to
/// stamp them for. Fence answers (ADR-0054) wake the loop straight from the
/// reader thread; a terminal that never answered the startup DSR is never
/// fenced.
pub(super) fn listen(
    mux: &InputMux,
    inputs: Inputs,
    sender: Sender<Wake>,
    redetector: Option<Redetector>,
    settled: &Session,
) -> io::Result<Listening> {
    let trace::Opened {
        tracer,
        meter,
        stamps,
    } = Tracer::from_env();
    let fences = settled
        .capabilities
        .answers_dsr
        .then(|| answer_fences(mux, sender.clone()));
    let forwarder = input::spawn_reader(inputs, sender, redetector, stamps)?;
    Ok(Listening {
        forwarder,
        tracer,
        out: TerminalOut::for_meter(meter),
        fences,
    })
}

/// Send every fence answer the mux reads to the loop as a [`Wake::Fence`],
/// until the loop has gone.
fn answer_fences(mux: &InputMux, wakes: Sender<Wake>) -> FenceAnswers {
    mux.answer_fences(Box::new(move |answers| {
        (0..answers).all(|_| wakes.send(Wake::Fence).is_ok())
    }))
}

/// This run is done reading (ticket m1-the-graph/35c). A key typed from here
/// on is kept for the next `on_stdin` (a wrapper's next run), and so is any
/// key already forwarded past the loop when it stopped: `]q` typed in one go
/// must step once and then quit, not step and lose the `q`. Joining the
/// forwarder first makes the hand-back complete — it ends once `hold` drops
/// its `Inputs` sender, after forwarding all it had.
pub(in crate::event_loop) fn hand_back_input<T>(
    mux: &InputMux,
    forwarder: input::Forwarder,
    wakes: ChannelWakes,
    outcome: io::Result<T>,
) -> io::Result<T> {
    mux.hold();
    let outcome = outcome.and_then(|exit| forwarder.finish().map(|()| exit));
    mux.give_back(wakes.unhandled_input());
    outcome
}

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
use graft_term::{SYNC_BEGIN, SYNC_END, Transport};
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::graphics::{Graphics, GraphicsReplies, NoReplies};
use super::outside::{Handover, Outside};
use super::pace::{FENCE_TIMEOUT, Pacer};
use super::redetect;
use super::rest::{Mover, Rest};
use super::{Exit, Phase, Waited, Wake, Wakes, joins_frame};
use crate::jobs::Dispatch;
use crate::session::{Session, TerminalChange};
use crate::trace::Tracer;
use crate::update::{
    AppEvent, AppState, Command, Product, Quit, Status, follow_behind, follow_key, update,
};
use crate::view::{draws_pixels, view_with_graphics};

/// End a synchronized update, which is what makes the frame appear. The
/// tests find frames by it.
#[cfg(test)]
pub(super) const END_SYNCHRONIZED_UPDATE: &[u8] = SYNC_END;

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

/// The most update time a turn spends on queued wakes before its frame shows
/// them — job answers and keys alike. Half a 60 Hz frame: a burst longer than
/// this still draws as it goes, so the screen never sits stale behind a long
/// queue.
const BATCH_BUDGET: Duration = Duration::from_millis(8);

/// The loop's body: draw, wait, update, repeat.
fn frames<B, W, D, R>(
    Screen { terminal, graphics }: Screen<'_, B, R>,
    wakes: &mut W,
    jobs: &D,
    session: &mut Session,
    state: &mut AppState,
    outside: Outside<'_>,
    tracer: &mut Tracer,
) -> io::Result<Exit>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    let pacer = Pacer::new(session.capabilities.answers_dsr, FENCE_TIMEOUT);
    Run {
        terminal,
        wakes,
        jobs,
        session,
        state,
        graphics,
        outside,
        tracer,
        pacer,
        rest: Rest::new(),
        undrawn_since: None,
        deferred: VecDeque::new(),
        now: false,
    }
    .run()
}

/// One run of the loop: its collaborators, and what stands between the state
/// and the screen.
struct Run<'r, 'o, B: Backend, W, D, R> {
    terminal: &'r mut Terminal<B>,
    wakes: &'r mut W,
    jobs: &'r D,
    session: &'r mut Session,
    state: &'r mut AppState,
    graphics: &'r mut Graphics<R>,
    outside: Outside<'o>,
    tracer: &'r mut Tracer,
    /// When the next frame may be written (ADR-0054).
    pacer: Pacer,
    /// When the cursor last moved, and so when it has rested (G-settle):
    /// mid-burst the pane's follow is owed, and the loop applies
    /// `AppEvent::Rested` once the moves stop.
    rest: Rest,
    /// When the first change not yet drawn was applied; `None` while the
    /// screen shows the state. Queued wakes share one frame, up to
    /// `BATCH_BUDGET` of updates.
    undrawn_since: Option<Instant>,
    /// Wakes that came while a frame was held and may not be applied ahead
    /// of it: a click acts on what he saw, so the frame before it is drawn
    /// first; what came after the click waits behind it, in order.
    deferred: VecDeque<Wake>,
    /// The next frame goes out without waiting for the pacer: the terminal
    /// was just handed back, and shows someone else's screen.
    now: bool,
}

impl<B, W, D, R> Run<'_, '_, B, W, D, R>
where
    B: Backend<Error = io::Error> + Write,
    W: Wakes,
    D: Dispatch,
    R: GraphicsReplies,
{
    fn run(mut self) -> io::Result<Exit> {
        // Draw before waiting: the first frame must be on screen while Graft
        // sits idle waiting for the user's first key.
        self.draw()?;
        loop {
            if let Some(exit) = self.turn()? {
                return Ok(exit);
            }
        }
    }

    /// One step: draw what is undrawn if the turn is over and the terminal
    /// is ready for it, else take one wake. `Some` when the loop ends.
    fn turn(&mut self) -> io::Result<Option<Exit>> {
        let rest_at = self.rest_deadline();
        if rest_at.is_some_and(|at| self.wakes.now() >= at) {
            return self.rested();
        }
        let Some(began) = self.undrawn_since else {
            // The screen shows the state. What was held back comes first;
            // else, idle: get the next page ready, then wait.
            if let Some(wake) = self.deferred.pop_front() {
                return self.take(wake);
            }
            // A follow is owed: wait for the next wake or for rest, which
            // comes first. The idle work waits for the turn after.
            if let Some(at) = rest_at {
                return match self.wakes.next_wake_by(at) {
                    Waited::Woke(wake) => self.take(wake),
                    Waited::TimedOut => self.rested(),
                    Waited::Ended => Ok(Some(Exit::Quit)),
                };
            }
            prefetch(self.session, self.state, self.graphics, self.wakes);
            let Some(wake) = self.wakes.next_wake() else {
                // Nothing can wake the loop again. Leave as if the user had
                // quit rather than spinning on a dead channel.
                return Ok(Some(Exit::Quit));
            };
            return self.take(wake);
        };

        // One turn (m7-power-tools/43): while the next wake joins the frame
        // — a job's answer, a key, a wheel scroll — apply it before drawing,
        // so a backlog of held-key repeats is shown by one frame, not one
        // frame each. A handover ends the turn (what follows waits for the
        // terminal to be ours again), and so does `BATCH_BUDGET`.
        let elapsed = self.wakes.now().saturating_duration_since(began);
        if !self.now && elapsed < BATCH_BUDGET && self.next_joins() {
            let wake = match self.deferred.pop_front() {
                Some(wake) => Some(wake),
                None => self.wakes.next_wake(),
            };
            return match wake {
                Some(wake) => self.take(wake),
                None => Ok(Some(Exit::Quit)),
            };
        }

        let now = self.wakes.now();
        if self.now || self.pacer.may_draw(now) {
            self.draw()?;
            return Ok(None);
        }
        // The terminal has not read the last frame yet (ADR-0054). Wait for
        // it, applying what comes meanwhile; the answer, or the pacer's
        // deadline, releases one frame showing all of it.
        // Rest may come first; the next turn applies it.
        let paced = self.pacer.deadline().unwrap_or(now);
        let deadline = rest_at.map_or(paced, |at| at.min(paced));
        match self.wakes.next_wake_by(deadline) {
            Waited::TimedOut => Ok(None),
            Waited::Ended => Ok(Some(Exit::Quit)),
            Waited::Woke(wake) => self.held(wake),
        }
    }

    /// Does the next wake join the frame being built? Held-back wakes are
    /// next, ahead of anything queued after them.
    fn next_joins(&mut self) -> bool {
        match self.deferred.front() {
            Some(wake) => joins_frame(wake),
            None => self.wakes.joining_waiting(),
        }
    }

    /// A wake that came while a frame was held: applied if it joins the
    /// frame (and nothing is held back ahead of it), acted on at once if it
    /// ends the loop or answers the fence, else held back until the frame
    /// is drawn.
    fn held(&mut self, wake: Wake) -> io::Result<Option<Exit>> {
        let at_once = matches!(
            wake,
            Wake::Fence | Wake::Shutdown(_) | Wake::InputFailed(_) | Wake::TerminalFailed(_)
        );
        if at_once || (self.deferred.is_empty() && joins_frame(&wake)) {
            return self.take(wake);
        }
        self.deferred.push_back(wake);
        Ok(None)
    }

    /// Apply one wake. `Some` when the loop ends.
    fn take(&mut self, wake: Wake) -> io::Result<Option<Exit>> {
        if let Wake::Fence = wake {
            let waited = self.pacer.answered(self.wakes.now());
            self.tracer.fenced(waited);
            // Replies to the frames already drawn: an `ENOENT` here makes
            // the next frame transmit that strip again.
            if self.graphics.drain()? {
                self.undrawn_since.get_or_insert_with(|| self.wakes.now());
            }
            return Ok(None);
        }
        // Read before the wake is taken apart: whether it ends a burst
        // (an input acting on the cursor's commit, G-settle), and what it
        // is as a mover.
        let rest_first = follow_behind(self.state) && self.rest.rest_first(&wake);
        let mover = Mover::of(&wake);
        self.tracer.took(&wake);
        let replied = self.graphics.drain()?;
        self.wakes.mark(Phase::Drained);

        let event = match wake {
            Wake::Event(event) => event,
            Wake::Shutdown(signal) => return Ok(Some(Exit::Signal(signal))),
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            Wake::Terminal(change) => {
                let changed = self.session.apply(change);
                if changed {
                    self.graphics.flush(match change {
                        TerminalChange::CellSize(_) => Flush::CellSize,
                        TerminalChange::Theme(_) => Flush::Theme,
                    });
                }
                if changed || replied {
                    self.undrawn_since.get_or_insert_with(|| self.wakes.now());
                }
                return Ok(None);
            }
            // Taken above; never reaches here.
            Wake::Fence => return Ok(None),
        };

        if rest_first && let Some(exit) = self.rested()? {
            return Ok(Some(exit));
        }
        // G-settle: a move within a burst is deferred, and one that changed
        // what the pane follows is the burst's latest.
        let now = self.wakes.now();
        self.state.defer_follow = self.rest.in_burst(now);
        let before = follow_key(self.state);
        let exit = self.apply(event)?;
        if follow_key(self.state) != before {
            self.rest.moved(now, mover);
        }
        Ok(exit)
    }

    /// When a follow is owed, the moment the cursor counts as rested.
    fn rest_deadline(&self) -> Option<Instant> {
        if follow_behind(self.state) {
            self.rest.deadline()
        } else {
            None
        }
    }

    /// The cursor has rested: the owed follow runs, once, and the next move
    /// is a leading edge again.
    fn rested(&mut self) -> io::Result<Option<Exit>> {
        self.state.defer_follow = false;
        let exit = self.apply(AppEvent::Rested)?;
        self.rest.rested();
        Ok(exit)
    }

    /// Update the state with one event and carry out what it asks for.
    /// `Some` when the loop ends.
    fn apply(&mut self, event: AppEvent) -> io::Result<Option<Exit>> {
        self.undrawn_since.get_or_insert_with(|| self.wakes.now());
        let clock = self.tracer.clock();
        let (next, commands) = update(self.state, event);
        self.tracer.updated(clock, commands.len());
        *self.state = next;
        self.wakes.mark(Phase::Updated);
        if commands.iter().any(hands_over) {
            self.settle_fences();
        }

        let (next, carried) = carry_out(
            self.terminal,
            self.session,
            self.jobs,
            std::mem::take(self.state),
            self.outside,
            commands,
        )?;
        *self.state = next;
        self.wakes.mark(Phase::CarriedOut);

        match &self.state.quit {
            Some(Quit::Requested) => return Ok(Some(Exit::Quit)),
            Some(Quit::HandedBack(key)) => return Ok(Some(Exit::HandedBack(*key))),
            // Reported by the caller, once the terminal is the user's again.
            Some(Quit::Fatal(reason)) => return Ok(Some(Exit::Fatal(reason.clone()))),
            None => {}
        }
        if carried == Carried::EndsBatch {
            // Handed over and back: answers owed to fences may have gone to
            // the other program, and the screen is theirs until redrawn.
            self.pacer.forget();
            self.now = true;
        }
        Ok(None)
    }

    /// Before the terminal goes to another program: take the answers still
    /// owed to fences, up to the pacer's timeout. An answer that arrived
    /// after the reader was paused would be read by `$EDITOR` as typing.
    /// What else comes meanwhile is held back, in order, for after.
    fn settle_fences(&mut self) {
        while let Some(deadline) = self.pacer.owed() {
            match self.wakes.next_wake_by(deadline) {
                Waited::Woke(Wake::Fence) => {
                    let waited = self.pacer.answered(self.wakes.now());
                    self.tracer.fenced(waited);
                }
                Waited::Woke(wake) => self.deferred.push_back(wake),
                // Unanswered: hand over anyway; `forget` follows the
                // handover. Ended: the loop finds out at its next wait.
                Waited::TimedOut | Waited::Ended => return,
            }
        }
    }

    /// Write one frame, fenced when the pacer says so.
    fn draw(&mut self) -> io::Result<()> {
        let fence = self.pacer.fenced();
        let again = draw_traced(
            self.terminal,
            self.session,
            self.state,
            self.graphics,
            self.tracer,
            fence,
        )?;
        let now = self.wakes.now();
        self.pacer.drew(now);
        // A trace failure changed the status row after the frame was built:
        // the next frame the pacer allows shows it.
        self.undrawn_since = again.then_some(now);
        self.now = false;
        self.wakes.mark(Phase::Drawn);
        Ok(())
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
fn hands_over(command: &Command) -> bool {
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

/// [`draw_frame`], with the frame's line in the trace when one is being
/// written. Returns `true` when the status row changed after the frame was
/// built, so another frame is owed.
///
/// A trace that could not be opened, or stopped on a failed write, says so
/// on the status row (`trace: <reason>`) — once, as a sticky failure `Esc`
/// dismisses — and the app runs on: the trace is a measurement, never a
/// reason to stop. The loop sets the status itself because the failure is
/// the loop's own (its file), not an event `update` was told about.
fn draw_traced<B, R>(
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
        state.status = Status::sticky(format!("trace: {reason}"));
    }
    let began = tracer.clock();
    draw_frame(terminal, session, state, graphics, fence)?;
    tracer.drew(began, state);
    Ok(match tracer.unreported() {
        Some(reason) => {
            state.status = Status::sticky(format!("trace: {reason}"));
            true
        }
        None => false,
    })
}

/// [`draw_frame`] with no fence: one frame, as a test draws it.
#[cfg(test)]
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
pub(super) fn draw_frame<B, R>(
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
                graft_term::write_fence(backend)
            } else {
                Ok(())
            }
        })
        .and_then(|()| Write::flush(backend));

    drawn.and(placed).and(ended)
}

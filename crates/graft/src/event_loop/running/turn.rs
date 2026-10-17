//! The paced turn (ADR-0054, G-settle): one wake at a time, applied to the
//! state; a frame drawn once the turn is over and the terminal has read the
//! last one; wakes that come while a frame is held applied or held back in
//! order; the fence answers owed settled before the terminal is handed over.

use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use graft_graph::raster::cache::Flush;
use ratatui::Terminal;
use ratatui::backend::Backend;

use super::super::graphics::{Graphics, GraphicsReplies};
use super::super::outside::Outside;
use super::super::pace::{FENCE_TIMEOUT, Pacer};
use super::super::rest::{Mover, Rest};
use super::super::{Exit, Phase, Waited, Wake, Wakes, input, joins_frame};
use super::carry::{Carried, carry_out, hands_over};
use super::frame::draw_traced;
use super::quit_clock::QuitClock;
use super::{Screen, prefetch};
use crate::jobs::Dispatch;
use crate::session::{Session, TerminalChange};
use crate::trace::Tracer;
use crate::update::{
    AppEvent, AppState, Command, Quit, follow_behind, follow_key, rest_owed, update,
};

/// The most update time a turn spends on queued wakes before its frame shows
/// them — job answers and keys alike. Half a 60 Hz frame: a burst longer than
/// this still draws as it goes, so the screen never sits stale behind a long
/// queue.
const BATCH_BUDGET: Duration = Duration::from_millis(8);

/// The loop's body: draw, wait, update, repeat.
pub(in crate::event_loop) fn frames<B, W, D, R>(
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
        quit_clock: QuitClock::default(),
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
    /// mid-burst the pane's follow and the next fold count are owed, and
    /// the loop applies `AppEvent::Rested` once the moves stop.
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
    /// The quit wait's count toward the stop offer (m8/13b review fix).
    quit_clock: QuitClock,
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
        let offer_at = self.offer_deadline();
        if offer_at.is_some_and(|at| self.wakes.now() >= at) {
            self.waited_long();
            return Ok(None);
        }
        // Whichever the loop owes first; the turn after the wait acts on it.
        let due = match (rest_at, offer_at) {
            (Some(rest), Some(offer)) => Some(rest.min(offer)),
            (rest, offer) => rest.or(offer),
        };
        let Some(began) = self.undrawn_since else {
            return self.idle(due);
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

        if self.now || self.pacer.may_draw(self.wakes.now()) {
            self.draw()?;
            return Ok(None);
        }
        self.paced(due)
    }

    /// The screen shows the state. What was held back comes first; else,
    /// idle: get the next page ready, then wait.
    fn idle(&mut self, due: Option<Instant>) -> io::Result<Option<Exit>> {
        if let Some(wake) = self.deferred.pop_front() {
            return self.take(wake);
        }
        // A follow is owed, or the quit wait's offer: wait for the next wake
        // or for that moment, which comes first. The next turn acts on the
        // moment (rest, or `waited_long`); the idle work waits for the turn
        // after.
        if let Some(at) = due {
            return match self.wakes.next_wake_by(at) {
                Waited::Woke(wake) => self.take(wake),
                Waited::TimedOut => Ok(None),
                Waited::Ended => Ok(Some(Exit::Quit)),
            };
        }
        prefetch(self.session, self.state, self.graphics, self.wakes);
        match self.wakes.next_wake() {
            Some(wake) => self.take(wake),
            // Nothing can wake the loop again. Leave as if the user had
            // quit rather than spinning on a dead channel.
            None => Ok(Some(Exit::Quit)),
        }
    }

    /// The terminal has not read the last frame yet (ADR-0054). Wait for
    /// it, applying what comes meanwhile; the answer, or the pacer's
    /// deadline, releases one frame showing all of it. Rest may come first;
    /// the next turn applies it.
    fn paced(&mut self, due: Option<Instant>) -> io::Result<Option<Exit>> {
        let paced = self.pacer.deadline().unwrap_or_else(|| self.wakes.now());
        let deadline = due.map_or(paced, |at| at.min(paced));
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
            self.fence_answered()?;
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

        match wake {
            Wake::Event(event) => self.input(event, rest_first, mover),
            Wake::Shutdown(signal) => Ok(Some(Exit::Signal(signal))),
            // The terminal hung up: the same exit as its `SIGHUP` (13g).
            Wake::InputFailed(error) if input::hung_up(&error) => {
                Ok(Some(Exit::Signal(signal_hook::consts::SIGHUP)))
            }
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => Err(error),
            Wake::Terminal(change) => {
                self.terminal_changed(change, replied);
                Ok(None)
            }
            // Taken above.
            Wake::Fence => Ok(None),
        }
    }

    /// The terminal has read every frame up to the oldest fence owed.
    fn fence_answered(&mut self) -> io::Result<()> {
        let waited = self.pacer.answered(self.wakes.now());
        self.tracer.fenced(waited);
        // Replies to the frames already drawn: an `ENOENT` here makes the
        // next frame transmit that strip again.
        if self.graphics.drain()? {
            self.undrawn_since.get_or_insert_with(|| self.wakes.now());
        }
        Ok(())
    }

    /// A re-detected cell size or theme: the strips drawn for the old one
    /// go, and a frame is owed if anything changed or a reply asked for one.
    /// A new background hint that leaves the theme as it was (his palette
    /// choice or a forced `ui.theme` wins) owes a frame — the hint's own
    /// words may change — but keeps the strips: they are painted from the
    /// theme (`StripPaint::of`), not the hint.
    fn terminal_changed(&mut self, change: TerminalChange, replied: bool) {
        let theme_before =
            matches!(change, TerminalChange::Theme(_)).then(|| self.session.theme.clone());
        let changed = self.session.apply(change);
        let flush = match change {
            TerminalChange::CellSize(_) => changed.then_some(Flush::CellSize),
            TerminalChange::Theme(_) => {
                (theme_before.as_ref() != Some(&self.session.theme)).then_some(Flush::Theme)
            }
        };
        if let Some(flush) = flush {
            self.graphics.flush(flush);
        }
        if changed || replied {
            self.undrawn_since.get_or_insert_with(|| self.wakes.now());
        }
    }

    /// The palette's `Theme: …` rows (`Command::Retheme`, m8/11 decision 6):
    /// the theme is the loop's, so the loop carries them out here — the
    /// same two steps as a re-detected background (`terminal_changed`): the
    /// session rebuilds the theme, and the strips drawn in the old one go.
    /// Every other command is handed back, in order.
    fn rethemed(&mut self, commands: Vec<Command>) -> Vec<Command> {
        let (themes, rest): (Vec<Command>, Vec<Command>) = commands
            .into_iter()
            .partition(|command| matches!(command, Command::Retheme(_)));
        for command in themes {
            if let Command::Retheme(kind) = command
                && self.session.choose(kind)
            {
                self.graphics.flush(Flush::Theme);
            }
        }
        rest
    }

    /// An event for `update`: rest first when it ends a burst, then applied,
    /// and the cursor's move noted for rest. `Some` when the loop ends.
    fn input(
        &mut self,
        event: AppEvent,
        rest_first: bool,
        mover: Mover,
    ) -> io::Result<Option<Exit>> {
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

    /// When work is owed to rest (a follow, a folding merge's count), the
    /// moment the cursor counts as rested. No burst, nothing deferred.
    fn rest_deadline(&self) -> Option<Instant> {
        let deadline = self.rest.deadline()?;
        rest_owed(self.state).then_some(deadline)
    }

    /// When the quit wait's stop offer is due (m8/13b review fix). The loop
    /// owns the clock ([`QuitClock`]); `update` stays pure.
    fn offer_deadline(&mut self) -> Option<Instant> {
        self.quit_clock.due(self.state, self.wakes.now())
    }

    /// The wait has stood `STOP_OFFER_AFTER`: its line now offers the stop,
    /// and a frame is owed to show it.
    fn waited_long(&mut self) {
        crate::update::quit::waited_long(self.state);
        self.undrawn_since.get_or_insert_with(|| self.wakes.now());
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
        let commands = self.rethemed(commands);

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
            // The tour (m8/19b): `drive` builds or removes the practice
            // repository; this repository is where the tour comes back to.
            Some(Quit::Practise { cursor }) => {
                return Ok(Some(Exit::Practise {
                    from: super::super::Open {
                        path: self.state.repo_path.clone(),
                        cursor: *cursor,
                    },
                }));
            }
            Some(Quit::LeaveTour { finished }) => {
                return Ok(Some(Exit::LeaveTour {
                    finished: *finished,
                }));
            }
            Some(Quit::StoppedHalfWay { job, what }) => {
                // The stop box opens only after the offer, which only the
                // clock makes, so a count is always there. Were it not, the
                // write was waited on no time this loop knows of: zero says
                // that, and the stop itself has already been asked.
                let after = self
                    .quit_clock
                    .after(self.wakes.now())
                    .unwrap_or(Duration::ZERO);
                return Ok(Some(Exit::StoppedHalfWay {
                    job: *job,
                    what: what.clone(),
                    after,
                }));
            }
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

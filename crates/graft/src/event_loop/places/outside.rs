//! The empty-state place's loop: the screen run outside a repository, the
//! pool its clone runs on, the steps it carries out, and how it hands on to
//! `drive`. Moved out of `places.rs` (m8/18e B1).

use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use graft_core::capabilities::RenderingTier;
use graft_core::event::Event;
use ratatui::backend::CrosstermBackend;

use super::super::exit_wait::Pool;
use super::super::startup::{Ran, Terminal, UNDO, hand_back_input};
use super::super::{ChannelWakes, Exit, Open, Wake, Wakes, input, is_job_event, signals};
use super::{Carried, OutsideStart, Place};
use crate::config::Loaded;
use crate::empty_state::{
    OutsideState, OutsideStep, outside_edge, outside_job, outside_paste, render,
};
use crate::first_launch::FirstLaunch;
use crate::recent::{self, Recent};
use crate::session::Session;
use crate::update::AppEvent;

mod carry;

pub(super) use carry::{OutsideJobs, carry_out};
// Only the tests reach these through `places`; `carry` uses them.
#[cfg(test)]
pub(super) use carry::{AnswersAsks, answer_ask};

/// Where `drive` goes once the empty state has ended.
pub(super) enum Next {
    /// On to this place.
    Place(Place),
    /// Into the practice tour, coming back to this (m8/19b).
    Practise(Open),
    /// Graft ends: close the terminal on this.
    End(Box<Ran>),
}

/// How the empty state ended, for `drive`: a repository opened goes on, with
/// the pool stopped now (a clone's, its job done: at once) and the notice
/// taken on; anything else ends the process with the pool, if any, stopped
/// once the terminal is back (`wind_down`) — a clone still running among
/// it, stoppable, so with no waiting line (rulings R4).
pub(super) fn leave_outside(left: io::Result<Left>, carried: &mut Carried) -> io::Result<Next> {
    match left {
        Ok(Left {
            outcome: Ok(Outcome::Opened { place, notice }),
            pool,
        }) => {
            Ran {
                outcome: Ok(Exit::Quit),
                pool,
            }
            .stop_for_the_next_place()?;
            carried.notice = notice;
            Ok(Next::Place(place))
        }
        Ok(Left {
            outcome: Ok(Outcome::Practise { from }),
            pool,
        }) => {
            Ran {
                outcome: Ok(Exit::Quit),
                pool,
            }
            .stop_for_the_next_place()?;
            Ok(Next::Practise(from))
        }
        Ok(Left {
            outcome: Ok(Outcome::Exit(exit)),
            pool,
        }) => Ok(Next::End(Box::new(Ran {
            outcome: Ok(exit),
            pool,
        }))),
        Ok(Left {
            outcome: Err(error),
            pool,
        }) => Ok(Next::End(Box::new(Ran {
            outcome: Err(error),
            pool,
        }))),
        Err(error) => Ok(Next::End(Box::new(Ran::without_jobs(Err(error))))),
    }
}

/// What the empty state shows besides its folder: the keys, and his
/// config's problems (the palette's `YOUR CONFIG`; still said on the first
/// repository screen after).
pub(super) struct Shown<'a> {
    pub(super) keymap: &'a std::sync::Arc<graft_ui::keymap::Keymap>,
    pub(super) problems: &'a [String],
    /// The process's first-launch line.
    pub(super) first_launch: &'a FirstLaunch,
    /// The count could not be read or written: said once, in the bar row.
    pub(super) warning: Option<String>,
}

/// How the empty state ended.
pub(super) enum Outcome {
    /// A repository was opened (by `⏎`, `o`, `i` or a clone).
    Opened {
        place: Place,
        /// Said once on its first frame (a clone's askpass close failure).
        notice: Option<String>,
    },
    /// `t` or the palette's `Practice tour`: into the practice tour,
    /// coming back here (m8/19b).
    Practise { from: Open },
    /// Graft is leaving.
    Exit(Exit),
}

/// What the empty state leaves: how it ended, and its job pool when a clone
/// started one (rulings R4), to stop as a repository's is stopped.
pub(super) struct Left {
    outcome: io::Result<Outcome>,
    pool: Option<Pool>,
}

/// The empty state's loop on `term`: draw, take a key, act, until a
/// repository opens or Graft quits. An open or init that fails stays here,
/// with the reason in the bar row and the cursor where it was: the screen is
/// never left, so it visibly stays (decision 4, done inside the loop rather
/// than by re-entering it, which would clear and redraw).
pub(super) fn run_outside(
    term: &mut Terminal,
    start: OutsideStart,
    config: &Loaded,
    shown: Shown<'_>,
    config_dir: Option<&Path>,
) -> io::Result<Left> {
    let mut settled = Session::settle(config, term.capabilities.clone());
    let ascii = settled.rendering.tier == RenderingTier::Ascii;
    let (recents, said) = read_recents(config_dir);
    let cursor = start.cursor.min(recents.len().saturating_sub(1));
    let mut state = OutsideState {
        folder: start.folder,
        missing: start.missing,
        home: std::env::var_os("HOME").map(PathBuf::from),
        recents,
        cursor,
        prompt: None,
        problem: start.problem.or(said).or(shown.warning),
        palette: None,
        config_problems: shown.problems.to_vec(),
        first_launch: shown.first_launch.line(),
        credential: None,
    };

    let inputs = term.inputs();
    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let resizes = signals::watch_resizes(sender.clone(), None)?;
    let mut jobs = OutsideJobs::new(sender.clone());
    let forwarder = input::spawn_reader(inputs, sender, None, None)?;
    let mut screen = ratatui::Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let outcome = outside_loop(
        &mut screen,
        &mut wakes,
        &mut state,
        (&mut settled, ascii),
        (config, &mut jobs, shown.first_launch),
        shown.keymap,
    );
    let pool = jobs.into_pool(signals, resizes, shown.keymap.spell(&UNDO).0);
    let outcome = hand_back_input(&term.mux, forwarder, wakes, outcome).and_then(|outcome| {
        if matches!(outcome, Outcome::Opened { .. }) {
            // The repository's screen starts from a blank one: its first
            // frame only draws cells that differ from blank. The backend's
            // clear, not `ratatui::Terminal::clear`: that one asks the
            // terminal where its cursor is (`ESC [6n`) and waits on stdin
            // for the answer, which the input mux owns — it hung the switch.
            ratatui::backend::Backend::clear(screen.backend_mut())?;
        }
        Ok(outcome)
    });
    Ok(Left { outcome, pool })
}

/// The recent list, and what to say about it once.
pub(super) fn read_recents(config_dir: Option<&Path>) -> (Vec<Recent>, Option<String>) {
    let Some(config_dir) = config_dir else {
        return (Vec::new(), Some("recent: no config folder".to_owned()));
    };
    match recent::read_recent(config_dir) {
        Ok(list) if list.skipped > 0 => (
            list.entries,
            Some(format!("recent: {} unreadable lines skipped", list.skipped)),
        ),
        Ok(list) => (list.entries, None),
        Err(error) => (Vec::new(), Some(format!("recent: {error}"))),
    }
}

pub(super) type Screen = ratatui::Terminal<CrosstermBackend<io::Stdout>>;

/// One frame as one synchronized update (ADR-0054 §3, DESIGN.md §4.1), as
/// the repository's frames are (`running::frame::draw_frame`): a frame
/// reaches the terminal in pieces, so without the bracket it can show the
/// top of a frame over the bottom of the last one (13h). The end marker goes out
/// even when the draw failed: a terminal left inside an update freezes on
/// what it showed before.
pub(super) fn draw_whole(
    screen: &mut Screen,
    render: impl FnOnce(&mut ratatui::Frame<'_>),
) -> io::Result<()> {
    use std::io::Write;
    screen.backend_mut().write_all(graft_term::SYNC_BEGIN)?;
    let drawn = screen.draw(render).map(|_| ());
    let backend = screen.backend_mut();
    let ended = backend
        .write_all(graft_term::SYNC_END)
        .and_then(|()| Write::flush(backend));
    drawn.and(ended)
}

pub(super) fn outside_loop(
    screen: &mut Screen,
    wakes: &mut ChannelWakes,
    state: &mut OutsideState,
    (settled, ascii): (&mut Session, bool),
    (config, jobs, line): (&Loaded, &mut OutsideJobs, &FirstLaunch),
    keymap: &std::sync::Arc<graft_ui::keymap::Keymap>,
) -> io::Result<Outcome> {
    loop {
        let now = SystemTime::now();
        draw_whole(screen, |frame| {
            let area = frame.area();
            render(
                state,
                keymap,
                &settled.theme,
                ascii,
                now,
                area,
                frame.buffer_mut(),
            );
        })?;
        let Some(wake) = wakes.next_wake() else {
            return Err(io::Error::other("the terminal's input ended"));
        };
        let step = match wake {
            Wake::Event(AppEvent::Core(Event::Key(key))) => outside_edge(state, keymap, key),
            Wake::Event(AppEvent::Core(Event::Paste(text))) => {
                outside_paste(state, keymap, &text);
                OutsideStep::Stay
            }
            Wake::Shutdown(signal) => return Ok(Outcome::Exit(Exit::Signal(signal))),
            // The terminal hung up: the same exit as its `SIGHUP` (13g).
            Wake::InputFailed(error) if input::hung_up(&error) => {
                return Ok(Outcome::Exit(Exit::Signal(signal_hook::consts::SIGHUP)));
            }
            Wake::InputFailed(error) | Wake::TerminalFailed(error) => return Err(error),
            // The clone's progress, its question and its answer (m8/18b).
            wake @ Wake::Event(_) if is_job_event(&wake) => outside_job(state, &wake),
            // A resize redraws (ratatui resizes on `draw`); focus, mouse,
            // fences and re-detection answers change nothing here — no
            // re-detector or fence listener runs on this screen.
            Wake::Event(_) | Wake::Terminal(_) | Wake::Fence => OutsideStep::Stay,
        };
        match step {
            // The next pass round this loop draws in it, changed or not.
            // This screen draws no pixels, so there are no strips to flush
            // (`turn::rethemed` does that in a repository).
            OutsideStep::Retheme(kind) => {
                settled.choose(kind);
            }
            step => {
                if let Some(outcome) = carry_out(step, state, jobs, config, line) {
                    return Ok(outcome);
                }
            }
        }
    }
}

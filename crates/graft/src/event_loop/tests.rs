use std::io::Write;
use std::sync::{Arc, Mutex};

use graft_core::event::{Event, KeyCode, KeyPress};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{TerminalOptions, Viewport};

use super::graphics::{Graphics, GraphicsReplies, NoReplies};
use super::headless::headless_settled;
use super::running::{END_SYNCHRONIZED_UPDATE, draw, prefetch, pump_with};
use super::*;
use crate::history::Phase;
use crate::jobs::Dispatch;
use crate::session::for_tests::session;
use crate::session::{Session, TerminalChange};
use crate::update::{AppEvent, AppState, Command, Size, update};

/// A terminal's output stream, kept in memory and readable while the
/// `Terminal` still owns the backend.
#[derive(Clone, Default)]
struct Recorder(Arc<Mutex<Vec<u8>>>);

impl Recorder {
    fn written(&self) -> String {
        let bytes = self.0.lock().expect("no test panics while holding this");
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Write for Recorder {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes = self.0.lock().expect("no test panics while holding this");
        bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A job runner that runs nothing and remembers everything, so a test can
/// see exactly what the loop handed over.
#[derive(Default)]
struct Recorded(Mutex<Vec<Command>>);

impl Recorded {
    fn commands(&self) -> Vec<Command> {
        self.0
            .lock()
            .expect("no test panics while holding this")
            .clone()
    }
}

impl Dispatch for Recorded {
    fn dispatch(&self, command: Command) {
        self.0
            .lock()
            .expect("no test panics while holding this")
            .push(command);
    }
}

/// A scripted source of wakes: hands out what it was given, then reports
/// that its producers are gone.
///
/// Every scripted wake is already queued, so `waiting` says `true` —
/// unless `idle` says how many times it answers `false` first.
struct ScriptedWakes {
    remaining: std::vec::IntoIter<Wake>,
    /// `idle[i]`: `waiting` calls that find nothing before wake `i`.
    idle: std::collections::VecDeque<usize>,
    asked: usize,
}

impl ScriptedWakes {
    fn new(wakes: Vec<Wake>) -> Self {
        Self::idle(wakes, Vec::new())
    }

    fn idle(wakes: Vec<Wake>, idle: Vec<usize>) -> Self {
        Self {
            remaining: wakes.into_iter(),
            idle: idle.into(),
            asked: 0,
        }
    }
}

impl Wakes for ScriptedWakes {
    fn next_wake(&mut self) -> Option<Wake> {
        self.idle.pop_front();
        self.remaining.next()
    }

    fn waiting(&mut self) -> bool {
        self.asked += 1;
        match self.idle.front_mut() {
            Some(left) if *left > 0 => {
                *left -= 1;
                false
            }
            _ => true,
        }
    }
}

/// A terminal that writes its bytes into memory, plus the handle that
/// reads them back. `with_options` and a fixed viewport are what let this
/// exist with no tty to ask for a size.
fn terminal(columns: u16, rows: u16) -> (Terminal<CrosstermBackend<Recorder>>, Recorder) {
    let recorder = Recorder::default();
    let terminal = Terminal::with_options(
        CrosstermBackend::new(recorder.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, columns, rows)),
        },
    )
    .expect("a fixed viewport never asks the backend for its size");
    (terminal, recorder)
}

fn quit_key() -> Wake {
    key(KeyPress::plain(KeyCode::Char('q')))
}

fn key(press: KeyPress) -> Wake {
    Wake::Event(AppEvent::Core(Event::Key(press)))
}

fn resized(columns: u16, rows: u16) -> Wake {
    Wake::Event(AppEvent::Core(Event::Resize { columns, rows }))
}

/// Replies handed out one batch per wake.
struct ScriptedReplies(std::collections::VecDeque<Vec<u8>>);

impl GraphicsReplies for ScriptedReplies {
    fn take(&mut self) -> io::Result<Vec<u8>> {
        Ok(self.0.pop_front().unwrap_or_default())
    }
}

const PIXEL_COLUMNS: u16 = 80;
const PIXEL_ROWS: u16 = 24;

/// A Kitty-protocol terminal at the pixel tier, showing 100 commits
/// requested in whole strips.
fn at_pixel_tier() -> (Session, AppState) {
    use graft_core::capabilities::{PixelProtocol, RenderingTier};
    let mut session = session();
    session.rendering.tier = RenderingTier::Pixel;
    session.capabilities.pixel_protocol = Some(PixelProtocol::Kitty);
    let mut history = crate::view::tests::history_of(100, 0);
    history.strip_rows = Some(graft_graph::raster::cache::STRIP_ROWS);
    let state = AppState {
        history: Some(history),
        settled_tier: Some(RenderingTier::Pixel),
        size: Size {
            columns: PIXEL_COLUMNS,
            rows: PIXEL_ROWS,
        },
        ..AppState::default()
    };
    (session, state)
}

/// Runs the loop at the pixel tier through `wakes`, answering with
/// `replies`; returns every frame (split on the end of each synchronized
/// update) and whatever was written after the last one.
fn pixel_run(wakes: Vec<Wake>, replies: Vec<Vec<u8>>) -> (Vec<String>, String) {
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let (mut session, state) = at_pixel_tier();
    let mut graphics = Graphics::new(ScriptedReplies(replies.into()));
    let exit = pump_with(
        &mut terminal,
        &mut ScriptedWakes::new(wakes),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
    )
    .expect("writing to memory cannot fail");
    assert_eq!(exit, Exit::Quit);
    let written = recorder.written();
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let mut frames: Vec<String> = written.split(&end).map(str::to_owned).collect();
    let tail = frames.pop().expect("split yields at least one piece");
    (frames, tail)
}

/// Image ids a stream transmits, in order.
fn transmitted(stream: &str) -> Vec<u32> {
    stream
        .split("a=t,f=32,t=d,i=")
        .skip(1)
        .map(|rest| {
            rest.split(',')
                .next()
                .and_then(|id| id.parse().ok())
                .expect("an id follows i=")
        })
        .collect()
}

/// Strip-cache flushes over a pixel-tier run through `wakes`, and every
/// frame it drew.
fn pixel_flushes(wakes: Vec<Wake>) -> (u64, Vec<String>) {
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let (mut session, state) = at_pixel_tier();
    let mut graphics = Graphics::new(NoReplies);
    pump_with(
        &mut terminal,
        &mut ScriptedWakes::new(wakes),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
    )
    .expect("writing to memory cannot fail");
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let frames = recorder.written().split(&end).map(str::to_owned).collect();
    (graphics.strip_stats().flushes, frames)
}

/// A pixel-tier run on `protocol` through `wakes`: every frame drawn,
/// the graphics state after, and how long each `draw` took.
fn pixel_frames(
    protocol: graft_core::capabilities::PixelProtocol,
    wakes: Vec<Wake>,
) -> (Vec<String>, Graphics<NoReplies>, Vec<std::time::Duration>) {
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.pixel_protocol = Some(protocol);
    let mut graphics = Graphics::new(NoReplies);
    let mut times = Vec::new();
    let started = std::time::Instant::now();
    draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
    times.push(started.elapsed());
    for wake in wakes {
        let Wake::Event(event) = wake else {
            panic!("only events are scripted here");
        };
        state = update(&state, event).0;
        let started = std::time::Instant::now();
        draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
        times.push(started.elapsed());
    }
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let frames = recorder.written().split(&end).map(str::to_owned).collect();
    (frames, graphics, times)
}

#[test]
fn a_resize_re_places_strips_without_retransmitting_them() {
    use graft_core::capabilities::PixelProtocol;
    let (_, still, _) = pixel_frames(PixelProtocol::Kitty, vec![]);
    let (frames, graphics, times) = pixel_frames(
        PixelProtocol::Kitty,
        vec![resized(PIXEL_COLUMNS - 20, PIXEL_ROWS - 4)],
    );
    assert!(!transmitted(&frames[0]).is_empty());
    assert!(
        transmitted(&frames[1]).is_empty(),
        "a resize transmits nothing: {:?}",
        frames[1]
    );
    assert!(frames[1].contains("a=p,"), "the strips are placed again");
    assert_eq!(graphics.strip_stats().flushes, 0);
    assert_eq!(
        graphics.strip_stats().rasterised,
        still.strip_stats().rasterised,
        "no strip rasterised again"
    );
    // Measured, not asserted: a debug build without a terminal's write
    // cost (m1-the-graph/25 reports the release numbers).
    eprintln!(
        "kitty first frame {:?}, resize frame {:?}",
        times[0], times[1]
    );
}

#[test]
fn iterm2_resends_every_band_but_encodes_an_unchanged_one_once() {
    use graft_core::capabilities::PixelProtocol;
    let (frames, graphics, times) = pixel_frames(
        PixelProtocol::Iterm2,
        vec![
            resized(PIXEL_COLUMNS, PIXEL_ROWS),
            resized(PIXEL_COLUMNS - 20, PIXEL_ROWS),
        ],
    );
    let bands = frames[0].matches("\x1b]1337;File=").count();
    assert!(bands > 0, "{:?}", frames[0]);
    assert_eq!(frames[1].matches("\x1b]1337;File=").count(), bands);
    assert_eq!(frames[2].matches("\x1b]1337;File=").count(), bands);
    assert_eq!(
        graphics.bands_encoded(),
        u64::try_from(bands).unwrap(),
        "re-sent twice, encoded once"
    );
    assert!(!frames[1].contains("a=t,"), "no Kitty bytes on iTerm2");
    eprintln!(
        "iterm2 first frame {:?}, unchanged frame {:?}, resize frame {:?}",
        times[0], times[1], times[2]
    );
}

#[test]
fn the_loop_no_longer_flushes_strips_on_a_resize() {
    let (flushes, frames) = pixel_flushes(vec![resized(PIXEL_COLUMNS, PIXEL_ROWS), quit_key()]);
    assert_eq!(flushes, 0);
    assert!(transmitted(&frames[1]).is_empty(), "{:?}", frames[1]);
}

#[test]
fn a_new_cell_size_or_theme_flushes_the_strips() {
    use graft_core::capabilities::{Theme, ThemeHint, ThemeSource};
    let (session, _) = at_pixel_tier();
    let mut cell = session.capabilities.cell_size;
    cell.width += 1;
    let theme = ThemeHint {
        theme: Theme::Light,
        source: ThemeSource::Assumed,
    };
    let (flushes, _) = pixel_flushes(vec![
        Wake::Terminal(TerminalChange::CellSize(cell)),
        Wake::Terminal(TerminalChange::Theme(theme)),
        // Unchanged: nothing to flush.
        Wake::Terminal(TerminalChange::Theme(theme)),
        quit_key(),
    ]);
    assert_eq!(flushes, 2);
}

#[test]
fn forced_pixels_on_a_silent_terminal_attempt_kitty_and_f3_recovers() {
    let (mut terminal, recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let (mut session, mut state) = at_pixel_tier();
    session.capabilities.pixel_protocol = None;
    state.settled_tier = Some(session.rendering.tier);
    let mut graphics = Graphics::new(NoReplies);
    pump_with(
        &mut terminal,
        &mut ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Function(3))), quit_key()]),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
    )
    .expect("writing to memory cannot fail");
    let end = String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE).into_owned();
    let written = recorder.written();
    let frames: Vec<&str> = written.split(&end).collect();
    assert!(!transmitted(frames[0]).is_empty(), "Kitty attempted");
    assert!(
        frames[1].contains("a=d,d=I,i="),
        "uploads freed: {:?}",
        frames[1]
    );
    assert!(
        frames[1].contains("renderer: unicode (F3 cycles)"),
        "{:?}",
        frames[1]
    );
    assert!(!frames[1].contains("gfx"), "no pixels, no gfx status");
}

fn unbound_key() -> Wake {
    key(KeyPress::plain(KeyCode::Char('z')))
}

#[test]
fn at_the_pixel_tier_strips_are_sent_once_and_freed_on_exit() {
    let (frames, tail) = pixel_run(vec![unbound_key(), unbound_key(), quit_key()], vec![]);

    assert_eq!(frames.len(), 3);
    let sent = transmitted(&frames[0]);
    assert!(!sent.is_empty(), "the first frame uploads its strips");
    assert!(frames[0].contains("a=p,i="), "and places them");
    assert!(frames[0].contains("gfx ok"), "{}", frames[0]);
    for frame in &frames[1..] {
        assert!(
            transmitted(frame).is_empty(),
            "steady state re-sends nothing"
        );
    }
    let everything = frames.concat() + &tail;
    assert!(!everything.contains("\x1b[2J"), "never a clear");
    assert!(!everything.contains("q=2"), "never silenced replies");
    for id in &sent {
        assert!(
            tail.contains(&format!("a=d,d=I,i={id},")),
            "{id} freed: {tail:?}"
        );
    }
}

#[test]
fn an_enoent_reply_re_sends_the_strip_and_says_so() {
    let (frames, _tail) = pixel_run(
        vec![unbound_key(), quit_key()],
        vec![b"\x1b_Gi=1000;ENOENT:image not found\x1b\\".to_vec()],
    );

    assert_eq!(transmitted(&frames[0]).first(), Some(&1000));
    assert!(!transmitted(&frames[1]).is_empty(), "{:?}", frames[1]);
    assert!(
        frames[1].contains("gfx: ENOENT:image not found"),
        "{:?}",
        frames[1]
    );
}

/// Pixels, then `F3` three times (Unicode, ASCII, pixels again), then
/// quit: the four frames and the tail.
fn f3_round_trip() -> (Vec<String>, String) {
    let f3 = || key(KeyPress::plain(KeyCode::Function(3)));
    let (frames, tail) = pixel_run(vec![f3(), f3(), f3(), quit_key()], vec![]);
    assert_eq!(frames.len(), 4);
    (frames, tail)
}

#[test]
fn f3_off_pixels_frees_every_upload_and_names_the_tier() {
    let (frames, _tail) = f3_round_trip();
    let first = transmitted(&frames[0]);
    assert!(!first.is_empty(), "{:?}", frames[0]);
    for id in &first {
        assert!(
            frames[1].contains(&format!("a=d,d=I,i={id},")),
            "{id} freed leaving pixels: {:?}",
            frames[1]
        );
    }
    assert!(frames[1].contains("renderer: unicode (F3 cycles)"));
    assert!(!frames[1].contains("gfx"), "no pixels, no gfx status");
    // Later frames are diffs: only the changed cells are written.
    assert!(frames[2].contains("ascii"), "{:?}", frames[2]);
    assert!(transmitted(&frames[2]).is_empty());
}

#[test]
fn f3_back_onto_pixels_transmits_afresh_and_frees_every_id_by_exit() {
    let (frames, tail) = f3_round_trip();
    let first = transmitted(&frames[0]);
    let again = transmitted(&frames[3]);
    assert!(!again.is_empty(), "back on pixels, strips sent again");
    assert!(
        again.iter().all(|id| !first.contains(id)),
        "fresh ids, never a freed one: {first:?} {again:?}"
    );
    assert!(frames[3].contains("a=p,i="), "and placed");
    assert!(frames[3].contains("gfx ok"), "{:?}", frames[3]);
    assert!(!frames[3].contains("F3 cycles"), "nothing extra at the top");

    let everything = frames.concat() + &tail;
    for id in first.iter().chain(&again) {
        assert!(
            everything.contains(&format!("a=d,d=I,i={id},")),
            "{id} freed by exit"
        );
    }
}

#[test]
fn leaving_the_graph_screen_removes_the_placements() {
    let (frames, _tail) = pixel_run(
        vec![key(KeyPress::plain(KeyCode::Function(2))), quit_key()],
        vec![],
    );

    assert!(frames[1].contains("a=d,d=i,i="), "{:?}", frames[1]);
    assert!(!frames[1].contains("gfx"), "no status off the pixel graph");
}

#[test]
fn the_quit_key_ends_the_loop() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let mut wakes = ScriptedWakes::new(vec![quit_key()]);

    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    assert_eq!(exit, Exit::Quit);
}

#[test]
fn a_signal_ends_the_loop_and_says_which_one() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let mut wakes = ScriptedWakes::new(vec![Wake::Shutdown(signals::WATCHED[1])]);

    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    assert_eq!(exit, Exit::Signal(signals::WATCHED[1]));
}

#[test]
fn a_signal_wins_over_a_key_that_arrived_first() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let mut wakes = ScriptedWakes::new(vec![
        resized(20, 3),
        Wake::Shutdown(signals::WATCHED[0]),
        quit_key(),
    ]);

    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    assert_eq!(exit, Exit::Signal(signals::WATCHED[0]));
}

#[test]
fn a_failed_read_stops_the_loop_and_is_reported() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let failure = io::Error::new(io::ErrorKind::BrokenPipe, "the terminal went away");
    let mut wakes = ScriptedWakes::new(vec![Wake::InputFailed(failure)]);

    let error = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect_err("a failed read must not be swallowed");

    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn losing_every_producer_ends_the_loop_rather_than_spinning() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let mut wakes = ScriptedWakes::new(Vec::new());

    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    assert_eq!(exit, Exit::Quit);
}

/// The state Graft starts in, before `Task::ReadHistory`'s `Started`
/// event arrives: `history` is `None`. `headless_settled` must not count
/// this as settled, or `graft repo | cat` prints an "opening…" frame with
/// no rows and exits 0 before the walk has said anything (M1 review,
/// M4, `KNOWN_GAPS.md:181`).
#[test]
fn headless_is_not_settled_before_history_exists() {
    assert!(!headless_settled(&AppState::default()));
}

#[test]
fn headless_is_not_settled_while_history_is_streaming() {
    let state = AppState {
        history: Some(crate::history::History::started()),
        ..AppState::default()
    };
    assert!(!headless_settled(&state));
}

#[test]
fn headless_is_settled_once_history_completes() {
    let mut history = crate::history::History::started();
    history.phase = Phase::Complete;
    let state = AppState {
        history: Some(history),
        ..AppState::default()
    };
    assert!(headless_settled(&state));
}

#[test]
fn headless_is_settled_when_the_walk_fails() {
    let mut history = crate::history::History::started();
    history.phase = Phase::Failed("disk read error".to_string());
    let state = AppState {
        history: Some(history),
        ..AppState::default()
    };
    assert!(headless_settled(&state));
}

#[test]
fn every_frame_is_wrapped_in_a_synchronized_update() {
    let (mut terminal, recorder) = terminal(20, 3);
    // One frame before the wait, one after the resize, and none after the
    // quit — three wakes, two frames.
    let mut wakes = ScriptedWakes::new(vec![resized(20, 3), quit_key()]);

    pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    let bytes = recorder.written();
    assert_eq!(bytes.matches("\u{1b}[?2026h").count(), 2);
    assert_eq!(bytes.matches("\u{1b}[?2026l").count(), 2);
    assert!(
        bytes.starts_with("\u{1b}[?2026h"),
        "the first bytes out must open a synchronized update: {bytes:?}"
    );
    assert!(
        bytes.ends_with("\u{1b}[?2026l"),
        "the last bytes out must close it: {bytes:?}"
    );
}

#[test]
fn graft_starts_at_the_terminals_size_not_zero_by_zero() {
    // Found under a pty: with a 0×0 starting size the history walk was
    // asked for one row, and the graph drew one row, until a resize.
    let size = crate::update::Size {
        columns: 80,
        rows: 24,
    };

    let state = opening_repository(&Recorded::default(), Path::new("/code/api"), size);

    assert_eq!(state.size, size);
    assert_eq!(state.graph_rows(), 22);
}

#[test]
fn the_status_screen_reaches_the_terminal() {
    let (mut terminal, recorder) = terminal(60, 12);
    let mut wakes = ScriptedWakes::new(vec![quit_key()]);
    let state = AppState {
        screen: crate::update::Screen::Status,
        ..AppState::default()
    };

    pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        state,
    )
    .expect("writing to memory cannot fail");

    // The backend skips blank cells and moves the cursor instead, so the
    // screen arrives as words rather than as whole lines.
    let bytes = recorder.written();
    for word in ["Graft", "Repository", "opening", "Rendering", "quit"] {
        assert!(bytes.contains(word), "{word:?} never reached the terminal");
    }
}

#[test]
fn a_terminal_with_no_cells_draws_without_panicking() {
    // ADR-0012's degenerate pty, driven through the whole loop rather than
    // just through `view`.
    let (mut terminal, _recorder) = terminal(0, 0);
    let mut wakes = ScriptedWakes::new(vec![resized(0, 0), quit_key()]);

    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    assert_eq!(exit, Exit::Quit);
}

#[test]
fn a_command_from_update_reaches_the_job_runner() {
    let (mut terminal, _recorder) = terminal(20, 3);
    let jobs = Recorded::default();
    let mut wakes =
        ScriptedWakes::new(vec![key(KeyPress::control(KeyCode::Char('r'))), quit_key()]);
    let state = AppState {
        repo_path: std::path::PathBuf::from("/code/api"),
        ..AppState::default()
    };

    pump(&mut terminal, &mut wakes, &jobs, &mut session(), state)
        .expect("writing to memory cannot fail");

    assert_eq!(
        jobs.commands(),
        vec![
            Command::OpenRepository,
            Command::ReadHistory(Order::Topological),
            Command::ReadRefs,
            Command::ReadWorkingTree,
        ]
    );
}

#[test]
fn a_job_result_is_just_another_wake() {
    let (mut terminal, recorder) = terminal(60, 14);
    let mut wakes = ScriptedWakes::new(vec![
        Wake::Event(AppEvent::Core(Event::JobFailed(
            graft_core::job::JobId::new(1),
            graft_core::job::JobFailure::Failed("not a git repository".to_owned()),
        ))),
        quit_key(),
    ]);

    pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        AppState::default(),
    )
    .expect("writing to memory cannot fail");

    let bytes = recorder.written();
    for word in ["failed", "repository"] {
        assert!(bytes.contains(word), "{word:?} never reached the terminal");
    }
}

/// A pixel-tier loop through `wakes`, `idle` as [`ScriptedWakes::idle`],
/// starting from `state`: strips rasterised, strips served from the cache.
fn prefetch_run(state: AppState, wakes: Vec<Wake>, idle: Vec<usize>) -> (u64, u64) {
    let (mut terminal, _recorder) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    let (mut session, _) = at_pixel_tier();
    let mut graphics = Graphics::new(NoReplies);
    pump_with(
        &mut terminal,
        &mut ScriptedWakes::idle(wakes, idle),
        &Recorded::default(),
        &mut session,
        state,
        &mut graphics,
    )
    .expect("writing to memory cannot fail");
    let counters = graphics.strip_stats();
    (counters.rasterised, counters.hits)
}

/// 255 commits, the screen showing row `top` with the cursor on it.
fn scrolled_to(top: usize) -> AppState {
    let (_, mut state) = at_pixel_tier();
    let mut history = crate::view::tests::history_of(255, top);
    history.top = top;
    history.strip_rows = Some(graft_graph::raster::cache::STRIP_ROWS);
    state.history = Some(history);
    state
}

#[test]
fn no_strip_is_prefetched_while_input_is_queued() {
    let queued = prefetch_run(scrolled_to(100), vec![quit_key()], vec![0]);
    let idle = prefetch_run(scrolled_to(100), vec![quit_key()], vec![usize::MAX]);

    let (session, _) = at_pixel_tier();
    let mut drawn_only = Graphics::new(NoReplies);
    let (mut terminal, _) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
    draw(&mut terminal, &session, &scrolled_to(100), &mut drawn_only).expect("memory");

    assert_eq!(
        queued.0,
        drawn_only.strip_stats().rasterised,
        "with a key queued the loop rasterises only what the frame shows"
    );
    assert!(
        idle.0 > queued.0,
        "idle, the loop does prefetch (else this proves nothing): {idle:?} vs {queued:?}"
    );
}

#[test]
fn a_key_arriving_mid_prefetch_waits_for_at_most_one_strip() {
    // The key arrives after `checks` looks at the queue: each look more
    // may let one more strip through, never two.
    let rasterised: Vec<u64> = (0..12)
        .map(|checks| prefetch_run(scrolled_to(100), vec![quit_key()], vec![checks]).0)
        .collect();
    for pair in rasterised.windows(2) {
        assert!(
            pair[1] <= pair[0] + 1,
            "one more look at the queue let more than one strip through: {rasterised:?}"
        );
    }
    let whole = prefetch_run(scrolled_to(100), vec![quit_key()], vec![usize::MAX]).0;
    assert!(
        whole >= rasterised[0] + 2,
        "an uninterrupted prefetch does at least two strips (else this proves nothing): {whole} vs {rasterised:?}"
    );
}

#[test]
fn page_keys_find_their_strips_already_rasterised() {
    for page in [KeyCode::PageDown, KeyCode::PageUp] {
        let (session, _) = at_pixel_tier();
        let (mut terminal, _) = terminal(PIXEL_COLUMNS, PIXEL_ROWS);
        let mut graphics = Graphics::new(NoReplies);
        let state = scrolled_to(100);
        draw(&mut terminal, &session, &state, &mut graphics).expect("memory");
        let done = prefetch(
            &session,
            &state,
            &mut graphics,
            &mut ScriptedWakes::idle(Vec::new(), vec![usize::MAX]),
        );
        let before = graphics.strip_stats();

        let paged = update(&state, AppEvent::Core(Event::Key(KeyPress::plain(page)))).0;
        draw(&mut terminal, &session, &paged, &mut graphics).expect("memory");
        let after = graphics.strip_stats();

        assert!(done > 0, "{page:?}: something was prefetched");
        assert_eq!(
            after.rasterised, before.rasterised,
            "{page:?}'s frame rasterised nothing: its strips were prefetched"
        );
        assert!(
            after.hits > before.hits,
            "{page:?}'s frame drew from the cache"
        );
    }
}

#[test]
fn a_wake_seen_while_checking_is_handed_out_next_and_kept_as_unhandled_input() {
    let (mut wakes, sender) = ChannelWakes::new();
    assert!(!wakes.waiting(), "nothing sent yet");
    sender.send(quit_key()).expect("receiver alive");
    sender
        .send(key(KeyPress::plain(KeyCode::Char('j'))))
        .expect("receiver alive");
    assert!(wakes.waiting());
    assert!(wakes.waiting(), "asking again loses nothing");
    drop(sender);

    let unhandled = wakes.unhandled_input();
    assert_eq!(
        unhandled.len(),
        2,
        "the wake seen while checking is kept: {unhandled:?}"
    );

    let (mut wakes, sender) = ChannelWakes::new();
    sender.send(quit_key()).expect("receiver alive");
    assert!(wakes.waiting());
    drop(sender);
    assert!(matches!(wakes.next_wake(), Some(Wake::Event(_))));
    assert!(wakes.next_wake().is_none());
    assert!(
        wakes.waiting(),
        "an ended source is worth finding out about"
    );
}

/// The rule the whole architecture rests on, checked rather than reviewed:
/// nothing on the UI thread's path waits for a job. A result arrives as a
/// wake or it does not arrive; the loop is never parked on one.
#[test]
fn the_ui_thread_never_waits_on_a_job() {
    let source = include_str!("mod.rs");
    let (code, _tests) = source
        .split_once("#[cfg(test)]")
        .expect("this module has a test section");

    for forbidden in [".join()", "recv_timeout", "wait_timeout", "thread::sleep"] {
        assert!(
            !code.contains(forbidden),
            "the loop must never wait on a job, but it mentions `{forbidden}`"
        );
    }
}

#[test]
fn a_channel_with_no_senders_left_reports_that_it_is_finished() {
    let (mut wakes, sender) = ChannelWakes::new();
    drop(sender);

    assert!(wakes.next_wake().is_none());
}

#[test]
fn the_input_a_finished_loop_never_handled_is_returned_in_order_without_the_rest() {
    let (mut wakes, sender) = ChannelWakes::new();
    sender
        .send(key(KeyPress::plain(KeyCode::Char(']'))))
        .unwrap();
    sender.send(Wake::Shutdown(15)).unwrap();
    sender.send(resized(80, 24)).unwrap();
    sender
        .send(key(KeyPress::plain(KeyCode::Char('q'))))
        .unwrap();
    assert!(matches!(wakes.next_wake(), Some(Wake::Event(_))));

    let unhandled = wakes.unhandled_input();
    assert_eq!(
        unhandled,
        vec![
            Event::Resize {
                columns: 80,
                rows: 24
            },
            Event::Key(KeyPress::plain(KeyCode::Char('q'))),
        ]
    );
}

/// The first of 255 commits selected, at the default (Outside) session.
fn copy_run(session: &mut Session) -> (Result<Exit, io::Error>, Recorder, String) {
    let (mut terminal, recorder) = terminal(80, 24);
    let history = crate::view::tests::history_of(255, 0);
    let sha = history
        .selected()
        .expect("the cursor row is on hand")
        .commit
        .id
        .to_string();
    let state = AppState {
        history: Some(history),
        ..AppState::default()
    };
    let mut wakes = ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Char('y'))), quit_key()]);
    let exit = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        session,
        state,
    );
    (exit, recorder, sha)
}

#[test]
fn y_writes_the_osc_52_request_to_the_terminal_and_not_to_the_jobs() {
    let (exit, recorder, sha) = copy_run(&mut session());
    exit.expect("writing to memory cannot fail");
    let sequence =
        String::from_utf8(graft_term::clipboard::osc52_copy(&sha)).expect("OSC 52 is ASCII");
    let written = recorder.written();
    assert!(written.contains(&sequence), "no OSC 52 for {sha}");
    assert!(written.contains(" → clipboard (osc 52)"), "no notice drawn");
}

#[test]
fn inside_tmux_the_osc_52_request_is_wrapped_in_the_passthrough() {
    let mut inside = session();
    inside.capabilities.tmux = graft_core::capabilities::TmuxStatus::Passthrough {
        version: "3.4".to_owned(),
    };
    let (exit, recorder, sha) = copy_run(&mut inside);
    exit.expect("writing to memory cannot fail");
    let mut wrapped = Vec::new();
    graft_term::tmux_passthrough(&graft_term::clipboard::osc52_copy(&sha), &mut wrapped);
    let wrapped = String::from_utf8(wrapped).expect("ASCII");
    assert!(recorder.written().contains(&wrapped));
}

/// A terminal whose clipboard write fails: frames go through, the OSC 52
/// request does not.
struct RefusesOsc52;

impl Write for RefusesOsc52 {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.windows(4).any(|window| window == b"]52;") {
            return Err(io::Error::other("the terminal went away"));
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_failed_clipboard_write_ends_the_loop_with_its_error() {
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(RefusesOsc52),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, 80, 24)),
        },
    )
    .expect("a fixed viewport never asks the backend for its size");
    let state = AppState {
        history: Some(crate::view::tests::history_of(10, 0)),
        ..AppState::default()
    };
    let mut wakes = ScriptedWakes::new(vec![key(KeyPress::plain(KeyCode::Char('y'))), quit_key()]);
    let error = pump(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session(),
        state,
    )
    .expect_err("the write error reaches the caller");
    assert_eq!(error.to_string(), "the terminal went away");
}

//! One scroll: the repository opened and ready, the real loop pumped over
//! the script's keys, and what it measured ([`Run`]).

use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mergewright::config::Loaded;
use mergewright::event_loop::{Exit, Wake, pump};
use mergewright::history::Phase as WalkPhase;
use mergewright::jobs::{Dispatch, JobRunner, Shutdown};
use mergewright::refs::{RefBadges, badges};
use mergewright::session::Session;
use mergewright::update::{AppState, update};
use mergewright_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, PixelProtocol, ProbeOutcome, RenderingTier,
    Rgb, Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history::AheadBehind;
use mergewright_git::repo::HeadState;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::controls::{Control, Spinner};
use super::paced::{Paced, Watching};
use super::sink::{Counts, Sink};
use super::{PATIENCE, Pane, REFS, Refs, answering_dsr, keys, script};
use crate::budget_run;
use crate::frame_trace::{KeyFeed, Post, Posted};

/// What one scroll measured.
pub(super) struct Run {
    /// Per key, in [`keys`] order: due time → the key's own frame.
    pub(super) latencies: Vec<Duration>,
    /// Per key, in [`keys`] order: where its latency went.
    pub(super) breakdown: Vec<String>,
    /// Keys whose own frame lacked the cursor's row or part of the viewport.
    pub(super) waited: Vec<String>,
    /// Kitty graphics commands written (`ESC _ G`): images at the pixel tier.
    pub(super) kitty_commands: usize,
    /// Every byte the loop wrote.
    pub(super) bytes: usize,
    /// Per frame: the idle prefetch after it (the loop's first look for a
    /// wake to its `next_wake`).
    pub(super) prefetch: Vec<Duration>,
    /// Per trigger (key name, `Window`, `Other`): frames, and what they wrote.
    pub(super) by_trigger: Vec<(String, usize, Counts)>,
    /// Per key: its phases, as `MWR_FRAME_TRACE` lines (see [`Paced::trace`]).
    pub(super) trace: Vec<String>,
    /// Every non-key wake from the first window of rows to key 1's frame,
    /// and the burst's length (see [`Paced::startup`]).
    pub(super) startup: Vec<String>,
    /// Per idle pause after `LaidOut`: the wakes nothing asked for (see
    /// [`Paced::idle_gaps`]).
    pub(super) idle_gaps: Vec<Vec<&'static str>>,
}

pub(super) fn scroll(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    pane: Pane,
    control: Control,
    traced: bool,
) -> Result<Run, String> {
    // One channel for job results and keys, as in the app
    // (`impl JobEvents for Sender<Wake>`, event_loop/mod.rs).
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, before_loop) =
        ready_to_scroll(repo, tier, refs, Post(post.clone()), &events)?;

    let mut session = Session::settle(&Loaded::default(), answering_dsr(capabilities(tier)));
    let sink = Sink::timed(traced).answering(post.clone());
    let mut terminal = in_memory(&sink, &state)?;

    let watching = Watching {
        inner: &jobs,
        asked: Mutex::default(),
    };
    let initial = state.clone();
    let feed = KeyFeed::start(keys(pane, control), post)
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(
        &events,
        keys(pane, control),
        &watching,
        sink.clone(),
        feed.log(),
    );
    wakes.no_prefetch = control == Control::NoPrefetch;
    let spinner = (control == Control::Busy).then(Spinner::start);
    let exit = pump(&mut terminal, &mut wakes, &watching, &mut session, state);
    drop(spinner);
    // Before any early return: a loop that stopped mid-script still leaves
    // the thread sending into a live channel, so this waits at most the
    // script's remaining pauses.
    let stamps = feed.finish();
    let exit = exit.map_err(|why| format!("the loop failed: {why}"))?;
    stamps?;
    if !matches!(exit, Exit::Quit) {
        return Err(format!(
            "the loop ended with {exit:?}, not at the end of the script"
        ));
    }
    if let Some(failure) = wakes.failure.take() {
        return Err(failure);
    }
    let run = measured(&mut wakes, &sink, initial, pane, &before_loop)?;

    let shutdown = jobs.shutdown(
        Duration::from_secs(10),
        mergewright::jobs::EXIT_WRITE_WAIT,
        &(),
    );
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}, forced {:?}",
            shutdown.stalled, shutdown.forced
        ));
    }
    Ok(run)
}

/// What the scroll measured, read from `wakes`' stamps once the loop has
/// stopped. `initial` is the state the loop started from, replayed to find
/// keys drawn before their rows arrived.
fn measured<D>(
    wakes: &mut Paced<'_, D>,
    sink: &Sink,
    initial: AppState,
    pane: Pane,
    before_loop: &[StartupWake],
) -> Result<Run, String> {
    let latencies = wakes.latencies()?;
    let prefetch = std::mem::take(&mut wakes.prefetch);
    // The scroll is over and every latency stamped: only now read what the
    // terminal was sent.
    let (written, counted) = sink.count_fenced(&wakes.written)?;
    let breakdown = wakes.breakdown(&counted);
    let waited = wakes.waited(initial);
    let by_trigger = wakes.written_by_trigger(&counted);
    let trace = wakes.trace(script(pane).len());
    let startup = wakes.startup(before_loop);
    let idle_gaps = wakes.idle_gaps(script(pane).len());
    Ok(Run {
        latencies,
        prefetch,
        breakdown,
        waited,
        kitty_commands: written.kitty_commands,
        bytes: written.bytes,
        by_trigger,
        trace,
        startup,
        idle_gaps,
    })
}

/// One non-key wake near startup: what it was, when it was sent and taken,
/// and what the loop spent on it (`draw` is `None` before the loop starts:
/// [`ready_to_scroll`] takes events in without drawing).
#[derive(Clone, Copy)]
pub(crate) struct StartupWake {
    pub(super) label: &'static str,
    pub(super) sent: Instant,
    pub(super) recv: Instant,
    pub(super) update: Option<Duration>,
    pub(super) draw: Option<Duration>,
}

/// Opens `repo` with its jobs reporting into `post`, and pumps `events`
/// until the screen is ready to scroll: rows on hand and the refs job
/// answered, so no stray refs answer lands mid-scroll.
///
/// Every event it takes is stamped ([`StartupWake`]) for the startup trace.
pub(super) fn ready_to_scroll(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    post: Post,
    events: &Receiver<Posted>,
) -> Result<(JobRunner<Post>, AppState, Vec<StartupWake>), String> {
    ready_sized(repo, tier, refs, post, events, super::real_size()?)
}

/// [`ready_to_scroll`] at `size` (`(columns, rows)`), whatever
/// `MWR_REAL_SIZE` says: a parameter, never the process-wide env
/// (m7-power-tools/42 ruling 3).
pub(crate) fn ready_to_scroll_at(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    post: Post,
    events: &Receiver<Posted>,
    size: (u16, u16),
) -> Result<(JobRunner<Post>, AppState, Vec<StartupWake>), String> {
    ready_sized(repo, tier, refs, post, events, Some(size))
}

/// The one body: `size` is what `MWR_REAL_SIZE` gave before (`None`:
/// the opened state's own size).
fn ready_sized(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    post: Post,
    events: &Receiver<Posted>,
    size: Option<(u16, u16)>,
) -> Result<(JobRunner<Post>, AppState, Vec<StartupWake>), String> {
    let (jobs, mut state) = budget_run::open_into(repo, post)?;
    state.settled_tier = Some(tier);
    if let Some((columns, rows)) = size {
        let (next, commands) = update(
            &state,
            mergewright::update::AppEvent::Core(mergewright_core::event::Event::Resize {
                columns,
                rows,
            }),
        );
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
    let ready = |state: &AppState| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .is_some_and(|h| h.window.as_ref().is_some_and(|w| !w.rows.is_empty()))
    };
    // `budget_run::pump_until_from`'s loop, with each event timed.
    let mut taken = Vec::new();
    loop {
        if let Some(history) = &state.history
            && let WalkPhase::Failed(why) = &history.phase
        {
            return Err(format!("the walk failed: {why}"));
        }
        if ready(&state) {
            break;
        }
        let posted = events
            .recv_timeout(PATIENCE)
            .map_err(|why| format!("no event for {PATIENCE:?} ({why}); state: {state:?}"))?;
        let recv = Instant::now();
        let label = posted.wake.label();
        let Wake::Event(event) = posted.wake else {
            return Err(format!("a wake no job sends: {label}"));
        };
        let started = Instant::now();
        let (next, commands) = update(&state, event);
        let update_took = started.elapsed();
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        taken.push(StartupWake {
            label,
            sent: posted.at,
            recv,
            update: Some(update_took),
            draw: None,
        });
        if let Some(quit) = &state.quit {
            return Err(format!("Mergewright quit: {quit:?}"));
        }
    }
    if let Refs::FiftyThousand = refs {
        state.refs = Some(Arc::new(fifty_thousand(&state)?));
    }
    print_state_clone(tier, refs, &state);
    Ok((jobs, state, taken))
}

/// A terminal of `state`'s size writing into `sink`.
pub(crate) fn in_memory(
    sink: &Sink,
    state: &AppState,
) -> Result<Terminal<CrosstermBackend<Sink>>, String> {
    Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, state.size.columns, state.size.rows)),
        },
    )
    .map_err(|why| format!("in-memory terminal: {why}"))
}

/// [`REFS`] refs, as `refs::read` would hand them over: half local branches
/// each tracking a remote-tracking ref, half those remote-tracking refs.
/// Nearly all point outside the first screen, as in a real repository with
/// that many; the rows on hand carry a few each, so badges are drawn and
/// looked up on every frame.
fn fifty_thousand(state: &AppState) -> Result<RefBadges, String> {
    let on_screen: Vec<Oid> = state
        .history
        .as_ref()
        .and_then(|h| h.window.as_ref())
        .map(|w| w.rows.iter().map(|row| row.commit.id).collect())
        .unwrap_or_default();
    if on_screen.is_empty() {
        return Err("no rows on hand to put badges on".to_owned());
    }
    let elsewhere = |index: usize| {
        let mut bytes = [0xEEu8; Oid::SHA1_BYTES];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        Oid::from_bytes(&bytes).map_err(|why| why.to_string())
    };
    let name = |text: String| RefName::new(text).map_err(|why| why.to_string());

    let branches = REFS / 2;
    let mut refs = Vec::with_capacity(REFS);
    let mut upstreams = HashMap::with_capacity(branches);
    for index in 0..branches {
        // One branch in 100 sits on a row on hand.
        let id = if index % 100 == 0 {
            on_screen[index / 100 % on_screen.len()]
        } else {
            elsewhere(index)?
        };
        let local = name(format!("refs/heads/topic/{index:05}"))?;
        let remote = name(format!("refs/remotes/origin/topic/{index:05}"))?;
        refs.push((local.clone(), id));
        refs.push((remote.clone(), id));
        upstreams.insert(local, remote);
    }
    let head = HeadState::Branch(refs[0].0.clone());

    let started = Instant::now();
    let built = badges(&refs, &head, &upstreams, &[], |_, _| {
        Ok(AheadBehind {
            ahead: 0,
            behind: 0,
        })
    })?;
    let building = started.elapsed();

    let started = Instant::now();
    let mut found = 0;
    for index in 0..branches {
        found += built.on(refs[index * 2].1).len();
    }
    let lookups = started.elapsed();
    println!(
        "refs: {REFS} refs, badges() {building:?}, {branches} lookups {lookups:?} ({} ns each), {found} labels found",
        lookups.as_nanos() / branches as u128
    );
    Ok(built)
}

/// A truecolour terminal that answered every probe, drawing at `tier`; at
/// the pixel tier over Kitty, with the user's measured cell (ADR-0012).
pub(crate) fn capabilities(tier: RenderingTier) -> Capabilities {
    let pixel = tier == RenderingTier::Pixel;
    Capabilities {
        tier,
        pixel_protocol: pixel.then_some(PixelProtocol::Kitty),
        kitty_graphics: if pixel { Supported::Yes } else { Supported::No },
        kitty_deflate: false,
        answers_dsr: false,
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::Yes,
        cell_size: CellSize {
            width: 17,
            height: 37,
            source: CellSizeSource::Reported,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x10,
                green: 0x10,
                blue: 0x14,
            }),
        },
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}

/// Prints what one `update`'s copy of the state costs: a clone of `state`
/// and dropping the state it replaces, which every event pays (`update`
/// clones, the loop drops the old one). Recorded, not asserted
/// (m1-the-graph/69 profiled the 50k-ref frames with it).
fn print_state_clone(tier: RenderingTier, refs: Refs, state: &AppState) {
    const LAPS: u32 = 64;
    let mut laps: Vec<Duration> = (0..LAPS)
        .map(|_| {
            let started = Instant::now();
            drop(state.clone());
            started.elapsed()
        })
        .collect();
    laps.sort();
    println!(
        "frames: tier={tier:?} refs={refs:?} state_clone_and_drop_us (recorded, not asserted): laps={LAPS} p50={} max={}",
        laps[laps.len() / 2].as_micros(),
        laps.last().map_or(0, Duration::as_micros),
    );
}

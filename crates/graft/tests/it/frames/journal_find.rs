//! J's find, measured (m7-power-tools/30p-b, review m7-30m-30o M2): J over
//! its caps, 2 000 reflog records and 500 lost commits, then a broad find
//! kept while `n`, `N` and `j` step through it. J's find computes its hits
//! once per find and once per list rebuild; this says whether that holds
//! the frame budget (ADR-0021's 16.7 ms frame, `Gate`'s p99 ≤ 16 ms).
//!
//! The same harness as the scroll ([`Paced`], [`KeyFeed`], one `Wake`
//! channel shared by the jobs and the keys), in two pumps, because J's
//! stages 2 and 3 are a wait, not a key:
//!
//! 1. `J`, measured; the loop ends once the key's frame is drawn.
//! 2. The wait: job events through `update` until `finding_lost()` is false
//!    (`budget_run::pump_until_from`). Not measured.
//! 3. `/`, the query, `⏎`, `n`×40, `N`×40, `j`×40, measured.
//!
//! The state between the pumps is the first pump's wakes replayed through
//! `update` from the state it started with, as `Paced::waited` replays them.
//!
//! The premises fail the test, never skip it: J's rows (≥ 2 000, of which
//! `✗` ≥ 400) are counted by the functions J's jobs run, with J's caps, on
//! the same repository (the list itself is `pub(crate)` state); the hits
//! are read from the frame's title after `⏎`, as drawn from the loop's own
//! state.
//!
//! Heavy tier only, as every `frames` scenario. Debug records the budget,
//! release enforces it (ADR-0021). The release run is
//! `scripts/m7-30p-perf.sh`'s.

use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use graft::config::Loaded;
use graft::event_loop::{Exit, Wake, pump};
use graft::jobs::{Dispatch, Shutdown};
use graft::session::Session;
use graft::update::{AppEvent, AppState, update};
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode};
use graft_fixtures::synthetic::{self, Scale};
use graft_git::repo::RepoHandle;
use graft_ops::journal_list::{
    LOST_CAP, ListRow, LostScan, RECORDS_CAP, build, find_lost, read_outside,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::controls::{Control, Spinner};
use super::paced::{Kind, Paced, Watching};
use super::run::capabilities;
use super::sink::Sink;
use super::{INPUT_TO_FRAME, TICK, answering_dsr};
use crate::budget_run;
use crate::captures::frame_at;
use crate::frame_trace::{FrameTrace, Gate, KeyFeed, Post, Posted, TRACE_VAR};
use crate::heavy::MEASURING;

/// J's cost does not hang on the graph's size, so the smallest fixture
/// whose `main` first-parent chain holds the 1 500 held records (the
/// synthetic history is 11 first-parent commits in 14): 10 000 commits,
/// with a commit-graph — the scale `heavy.rs`'s slope test already builds,
/// so the cache is shared.
const SCALE: Scale = Scale::of(10_000).with_commit_graph();

/// His daily view (memory: the Unicode tier is the daily view).
const COLUMNS: u16 = 187;
const ROWS: u16 = 50;

/// The find: every `$` row's subject is the fixture's `fixture: step <n>`,
/// and no `✗` row's (`lost <i>`) holds it.
const QUERY: &str = "step";

/// The premises (ticket 30p-b).
const MIN_ROWS: usize = 2_000;
const MIN_LOST_ROWS: usize = 400;
const MIN_HITS: usize = 1_500;

/// Each of `n`, `N` and `j`, pressed this many times.
const STEPS: usize = 40;

#[test]
fn j_with_a_broad_find_holds_the_frame_budget() {
    let control = Control::from_env().unwrap_or_else(|why| panic!("{why}"));
    let trace = FrameTrace::from_var(std::env::var_os(TRACE_VAR));
    let fixture = synthetic::with_reflog_of(SCALE, RECORDS_CAP, LOST_CAP)
        .unwrap_or_else(|why| panic!("{why}"));
    let repo = fixture.scratch.path();

    let (rows, lost_rows) = j_rows(repo).unwrap_or_else(|why| panic!("{why}"));
    assert!(
        rows >= MIN_ROWS && lost_rows >= MIN_LOST_ROWS,
        "premise: J has {rows} rows, {lost_rows} of them ✗; the scenario needs at least {MIN_ROWS}, {MIN_LOST_ROWS} ✗"
    );

    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let run = measure(repo, control).unwrap_or_else(|why| panic!("{why}"));
    drop(turn);

    let hits = hits_in(&run.title_after_enter).unwrap_or_else(|| {
        panic!(
            "premise: after /{QUERY}⏎ the title is not `… · find \"{QUERY}\" <at> of <n>`: {:?}",
            run.title_after_enter
        )
    });
    assert!(
        hits >= MIN_HITS,
        "premise: /{QUERY} finds {hits} rows; the scenario needs at least {MIN_HITS}"
    );
    assert_eq!(
        run.latencies.len(),
        1 + find_keys().len(),
        "every key got a frame"
    );

    let label = format!("tier=Unicode refs={rows} pane=JournalFind");
    let extra = format!("hits={hits} lost_rows={lost_rows}");
    record(&label, control, trace.as_ref(), &run, &extra);
}

/// Writes the trace, prints the `frames:` lines, and judges: release
/// enforces the p99, debug records it (ADR-0021).
fn record(label: &str, control: Control, trace: Option<&FrameTrace>, run: &Measured, extra: &str) {
    if let Some(trace) = &trace {
        let run_name = std::env::var("MWR_FRAME_RUN").unwrap_or_else(|_unset| "-".to_owned());
        let lines: Vec<String> = run
            .trace
            .iter()
            .map(|line| format!("trace {label} control={control:?} run={run_name} {line}"))
            .collect();
        trace
            .append(&lines)
            .unwrap_or_else(|why| panic!("{TRACE_VAR}: {why}"));
    }

    let gate = Gate::of(&run.latencies, INPUT_TO_FRAME);
    let late: Vec<String> = gate
        .over
        .iter()
        .map(|&key| format!("key {key} {:?}: {:?}", run.codes[key], run.latencies[key]))
        .collect();
    // The line `scripts/m7-30p-perf.sh` and `scripts/m7-perf-key.sh` read:
    // `tier= refs= pane=`, then the gate's `p50_us= p99_us= max_us=`.
    println!(
        "frames: {label} control={control:?} keys={} {} {extra}",
        run.latencies.len(),
        gate.line(),
    );
    println!(
        "frames: {label} keys over {INPUT_TO_FRAME:?} (recorded; the gate is the p99): {} of {}: {}",
        late.len(),
        run.latencies.len(),
        late.join(", ")
    );
    if cfg!(debug_assertions) {
        println!("frames: {label} debug build, budget recorded not asserted (ADR-0021)");
        return;
    }
    assert!(
        gate.holds(INPUT_TO_FRAME),
        "p99 {:?} is over the {INPUT_TO_FRAME:?} input-to-frame budget in J with /{QUERY} kept ({} of {} keys over): {}",
        gate.p99,
        late.len(),
        run.latencies.len(),
        late.join(", ")
    );
}

/// After the wait: `/`, the query, `⏎`, then `n`, `N` and `j` [`STEPS`]
/// times each, a key a tick, as the scroll paces its keys.
fn find_keys() -> Vec<KeyCode> {
    let mut keys = vec![KeyCode::Char('/')];
    keys.extend(QUERY.chars().map(KeyCode::Char));
    keys.push(KeyCode::Enter);
    for key in ['n', 'N', 'j'] {
        keys.extend(vec![KeyCode::Char(key); STEPS]);
    }
    keys
}

/// Where `⏎` sits in [`find_keys`].
fn enter_at() -> usize {
    1 + QUERY.chars().count()
}

/// J's rows and its `✗` rows on `repo`, from the functions J's stage 2 and
/// stage 3 jobs call (`jobs::journal_list`), with J's caps. The journal is
/// empty (Graft has done nothing here), so no entries.
fn j_rows(repo: &std::path::Path) -> Result<(usize, usize), String> {
    let handle = RepoHandle::open(repo).map_err(|why| format!("the fixture opens: {why}"))?;
    let outside =
        read_outside(&handle, RECORDS_CAP, &()).map_err(|why| format!("stage 2 reads: {why}"))?;
    let found = find_lost(&handle, &[], &outside, LOST_CAP, &())
        .map_err(|why| format!("stage 3 reads: {why}"))?;
    // Subjects and branch names change a row's words, not how many rows
    // there are.
    let journal = build(
        &[],
        &outside,
        Some(&found),
        &|_tip| None,
        &|_branch| true,
        &[],
        LostScan::Done,
    );
    let lost = journal
        .rows
        .iter()
        .filter(|row| matches!(row, ListRow::Lost { .. }))
        .count();
    Ok((journal.rows.len(), lost))
}

/// `n` in the title's `· find "<QUERY>" <at> of <n>`, if it ends so, with
/// `1 <= at <= n`.
///
/// `at` is not always 1 (the ticket's premise read `1 of <n>`): once stage
/// 3 is in, J's cursor starts on the newest `✗` row under a `$` row
/// (`update::journal::preselect`), here row 1 under the `$` row 0, and `⏎`
/// finds the first hit at or below the cursor (`update::journal::find`,
/// `KeyCode::Enter`), so the first `$` row after it: `2 of <n>`.
fn hits_in(title: &str) -> Option<usize> {
    let tail = title
        .trim_end()
        .rsplit_once(&format!("· find \"{QUERY}\" "))?
        .1;
    let (at, hits) = tail.split_once(" of ")?;
    let (at, hits): (usize, usize) = (at.parse().ok()?, hits.parse().ok()?);
    (1..=hits).contains(&at).then_some(hits)
}

/// What the two measured pumps gave.
struct Measured {
    /// Per key, `J` first: send → its own frame.
    latencies: Vec<Duration>,
    /// The key each latency is for.
    codes: Vec<KeyCode>,
    /// `MWR_FRAME_TRACE` lines, `phase=` naming the pump.
    trace: Vec<String>,
    /// J's title row as drawn right after `⏎`.
    title_after_enter: String,
}

/// A truecolour terminal that answered every probe, DSR too (ADR-0054),
/// drawing text at the Unicode tier: `run::capabilities`, fenced.
fn session() -> Session {
    let unicode = capabilities(RenderingTier::Unicode);
    Session::settle(&Loaded::default(), answering_dsr(unicode))
}

fn measure(repo: &std::path::Path, control: Control) -> Result<Measured, String> {
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state) = budget_run::open_into(repo, Post(post.clone()))?;
    let state = ready(&jobs, &events, &state)?;
    let mut session = session();
    let spinner = (control == Control::Busy).then(Spinner::start);

    let open = vec![KeyCode::Char('J')];
    let opened = pumped(
        &jobs,
        &events,
        &post,
        &mut session,
        state.clone(),
        &open,
        control,
    )?;
    let in_j = replayed(state, &opened.replay, None);
    if !in_j.journal.shown {
        return Err("J did not open".to_owned());
    }
    let settled = budget_run::pump_until_from(
        &jobs,
        |patience| next_event(&events, patience),
        in_j,
        |state| !state.journal.finding_lost(),
    )?;

    let find = find_keys();
    let found = pumped(
        &jobs,
        &events,
        &post,
        &mut session,
        settled.clone(),
        &find,
        control,
    )?;
    drop(spinner);
    let after_enter = replayed(settled, &found.replay, Some(enter_at()));
    let title_after_enter = title(&after_enter)?;
    let shutdown = jobs.shutdown(Duration::from_secs(10), graft::jobs::EXIT_WRITE_WAIT, &());
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}, forced {:?}",
            shutdown.stalled, shutdown.forced
        ));
    }

    let mut latencies = opened.latencies;
    latencies.extend(found.latencies);
    let codes = open.into_iter().chain(find).collect();
    let trace = opened
        .trace
        .iter()
        .map(|line| format!("phase=open {line}"))
        .chain(found.trace.iter().map(|line| format!("phase=find {line}")))
        .collect();
    Ok(Measured {
        latencies,
        codes,
        trace,
        title_after_enter,
    })
}

/// J's title row as `state` draws it at 187×50.
fn title(state: &AppState) -> Result<String, String> {
    let frame = frame_at(state, COLUMNS, ROWS, None);
    frame
        .iter()
        .find(|row| row.starts_with("JOURNAL ·"))
        .cloned()
        .ok_or_else(|| format!("no J title after /{QUERY}⏎:\n{}", frame.join("\n")))
}

/// The next job event on the shared channel, for `pump_until_from`. The
/// input thread has finished before any wait, so a key here is a harness
/// fault.
fn next_event(events: &Receiver<Posted>, patience: Duration) -> Result<AppEvent, String> {
    loop {
        let posted = events
            .recv_timeout(patience)
            .map_err(|why| why.to_string())?;
        match posted.wake {
            Wake::Event(event) => return Ok(event),
            // The last fence answer of a `pumped` loop that has ended
            // (ADR-0054): nothing waits for it, so it is stepped over.
            Wake::Fence => {}
            other => return Err(format!("a wake no job sends: {}", other.label())),
        }
    }
}

/// Opened at 187×50 at the Unicode tier, rows and refs on hand, the
/// journal's launch read answered.
fn ready(
    jobs: &impl Dispatch,
    events: &Receiver<Posted>,
    state: &AppState,
) -> Result<AppState, String> {
    let (mut state, commands) = update(
        state,
        AppEvent::Core(Event::Resize {
            columns: COLUMNS,
            rows: ROWS,
        }),
    );
    for command in commands {
        jobs.dispatch(command);
    }
    state.settled_tier = Some(RenderingTier::Unicode);
    budget_run::pump_until_from(
        jobs,
        |patience| next_event(events, patience),
        state,
        |state| {
            state.refs.is_some()
                && state.journal.reading.is_none()
                && state
                    .history
                    .as_ref()
                    .is_some_and(|h| h.window.as_ref().is_some_and(|w| !w.rows.is_empty()))
        },
    )
}

/// One pump's measurements.
struct Pumped {
    latencies: Vec<Duration>,
    trace: Vec<String>,
    /// Every wake handed to the loop, in order: its kind and its event.
    replay: Vec<(Kind, AppEvent)>,
}

/// The real loop over `keys`, sent by a [`KeyFeed`] a tick apart into the
/// channel the jobs report into. Ends once the last key's frame is drawn
/// and no rows it asked for are out.
fn pumped<D: Dispatch>(
    jobs: &D,
    events: &Receiver<Posted>,
    post: &mpsc::Sender<Posted>,
    session: &mut Session,
    state: AppState,
    keys: &[KeyCode],
    control: Control,
) -> Result<Pumped, String> {
    let script: Vec<(KeyCode, Duration)> = keys.iter().map(|key| (*key, TICK)).collect();
    let sink = Sink::timed(true).answering(post.clone());
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, COLUMNS, ROWS)),
        },
    )
    .map_err(|why| format!("in-memory terminal: {why}"))?;
    let watching = Watching {
        inner: jobs,
        asked: Mutex::default(),
    };
    let feed = KeyFeed::start(script.clone(), post.clone())
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(events, script, &watching, sink, feed.log());
    wakes.no_prefetch = control == Control::NoPrefetch;
    let exit = pump(&mut terminal, &mut wakes, &watching, session, state);
    // Before any early return: see `run::scroll`.
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
    if wakes.seen.is_some() {
        return Err("the loop ended with a wake taken off the channel and not handed".to_owned());
    }
    wakes.sink.were_fenced()?;
    let latencies = wakes.latencies()?;
    let trace = wakes.trace(keys.len());
    let replay = wakes
        .handed
        .iter()
        .map(|handed| handed.kind)
        .zip(std::mem::take(&mut wakes.replay))
        .collect();
    Ok(Pumped {
        latencies,
        trace,
        replay,
    })
}

/// `state` after `replay`'s events through `update`, up to and including
/// the wake of key `stop` when given. Commands are dropped: the loop
/// already carried them out.
fn replayed(mut state: AppState, replay: &[(Kind, AppEvent)], stop: Option<usize>) -> AppState {
    for (kind, event) in replay {
        state = update(&state, event.clone()).0;
        if let (Some(at), Kind::Key(key, _)) = (stop, kind)
            && *key == at
        {
            break;
        }
    }
    state
}

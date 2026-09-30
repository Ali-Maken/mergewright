//! Throwaway (housekeeping/02): where the heap slope's bytes per commit go.
//!
//! Runs the same gated path the heavy tier's slope guard measures (open →
//! first paint → `G` → `g`, `budget_run::run`), then attributes the live
//! heap at the end to one structure at a time by dropping them in order and
//! reading a counting allocator's live total before and after each drop.
//!
//! Run it on the 10k and the 100k commit-graph fixtures; the difference
//! between the two, divided by 90,000 commits, is each structure's
//! B/commit. `scripts/heap-slope.sh` does that.
//!
//! Live heap is not the gated number (the guard asserts on
//! `ri_lifetime_max_phys_footprint`), so the run prints both: the
//! per-structure live accounting, and the footprint peak it has to explain.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::mpsc;
use std::time::Duration;

use std::alloc::{GlobalAlloc, Layout, System};

use graft::history::{History, Phase};
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::repos::RepoStore;
use graft::update::{AppEvent, AppState, Size, update};
use graft_core::event::{Event, KeyCode, KeyPress};

// ---------------------------------------------------------------- allocator

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            LIVE.fetch_add(l.size(), Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) };
        LIVE.fetch_sub(l.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            if n >= l.size() {
                LIVE.fetch_add(n - l.size(), Relaxed);
            } else {
                LIVE.fetch_sub(l.size() - n, Relaxed);
            }
        }
        q
    }
}

#[global_allocator]
static A: Counting = Counting;

fn live() -> usize {
    LIVE.load(Relaxed)
}

/// Prints one accounting line: what was dropped, and the live bytes it held.
fn account(what: &str, before: usize) -> usize {
    let now = live();
    let freed = before.saturating_sub(now);
    println!("hold\t{what}\t{freed}");
    now
}

// ------------------------------------------------------------------ the run

const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};
const PATIENCE: Duration = Duration::from_secs(300);

fn main() -> std::process::ExitCode {
    let Some(repo) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: heapslope <repository>");
        return std::process::ExitCode::FAILURE;
    };
    match run(&repo) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("heapslope: {why}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(repo: &std::path::Path) -> Result<(), String> {
    let (sender, events) = mpsc::channel();
    let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
    let jobs = JobRunner::start(sender, 4, Arc::clone(&store)).map_err(|why| why.to_string())?;
    // The app's own opening (m7/12e): the reads now carry the `JobId`s the
    // state awaits, so they are asked through `update::opening`.
    let (mut state, commands) = graft::update::opening(repo.to_path_buf(), SIZE);
    for command in commands {
        jobs.dispatch(command);
    }

    let painted = |h: &History| h.window.as_ref().is_some_and(|w| !w.rows.is_empty());
    state = pump(&jobs, &events, state, painted)?;

    state = press(&jobs, &state, KeyCode::Char('G'));
    let at_bottom = |h: &History| {
        settled(h)
            && h.phase == Phase::Complete
            && h.window
                .as_ref()
                .is_some_and(|w| w.done && w.end() == w.laid_out)
    };
    state = pump(&jobs, &events, state, at_bottom)?;

    state = press(&jobs, &state, KeyCode::Char('g'));
    let at_top = |h: &History| settled(h) && h.window.as_ref().is_some_and(|w| w.first == 0);
    let mut state = pump(&jobs, &events, state, at_top)?;

    // Let the radar's own jobs (context, duplicates, force-push probes)
    // land: they are what the M2 cache is built from, and the guard's run
    // pays for them too. Drain whatever is already queued, then stop.
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        let Ok(event) = events.recv_timeout(Duration::from_millis(200)) else {
            break;
        };
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }

    match jobs.shutdown(Duration::from_secs(30)) {
        Shutdown::Clean => {}
        other => return Err(format!("the job pool did not stop cleanly: {other:?}")),
    }

    let footprint = graft_fixtures::footprint().map_err(|why| why.to_string())?;
    println!("total\tfootprint_peak\t{}", footprint.peak);
    println!("total\tfootprint_now\t{}", footprint.current);
    println!("total\tlive_at_end\t{}", live());

    // Counts, before anything is dropped: the numbers the accounting below
    // has to be consistent with.
    if let graft::update::detail::Loading::Ready(context) = &state.radar.context {
        println!("count\tdefault_chain\t{}", context.chain_len());
        println!(
            "count\tprevious_default_tips\t{}",
            context.previous_default_tips.len()
        );
    } else {
        println!("count\tdefault_chain\t-1");
        println!("count\tprevious_default_tips\t-1");
    }
    println!("count\tref_findings\t{}", state.radar.ref_findings.len());
    println!(
        "count\tforce_push_findings\t{}",
        state.radar.force_push_findings.len()
    );
    println!(
        "count\tduplicate_findings\t{}",
        state.radar.duplicate_findings.len()
    );
    if let Ok(table) = store.table() {
        println!("count\toverflow_len\t{}", table.overflow_len());
        println!("count\ttable_bytes\t{}", table.approximate_bytes());
    }
    println!(
        "count\trows_in_window\t{}",
        state
            .history
            .as_ref()
            .and_then(|h| h.window.as_ref())
            .map_or(0, |w| w.rows.len())
    );

    // One structure at a time, biggest suspects first. Each line is the
    // live bytes that structure was holding on its own.
    let mut mark = live();
    state.radar.context = graft::update::detail::Loading::Idle;
    mark = account("radar.context (default_chain + previous tips)", mark);
    state.radar.ref_findings = std::collections::HashMap::new();
    mark = account("radar.ref_findings", mark);
    state.radar.force_push_findings = std::collections::HashMap::new();
    mark = account("radar.force_push_findings", mark);
    state.radar.force_push_not_checkable = std::collections::HashMap::new();
    mark = account("radar.force_push_not_checkable", mark);
    state.radar.duplicate_findings = std::collections::HashMap::new();
    mark = account("radar.duplicate_findings", mark);
    state.radar = graft::update::radar::RadarState::default();
    mark = account("radar (the rest)", mark);
    state.refs = None;
    mark = account("refs (RefBadges)", mark);
    state.detail = graft::update::detail::DetailState::default();
    mark = account("detail pane", mark);
    state.map = graft::update::map::MapState::default();
    mark = account("map view", mark);
    state.search = None;
    mark = account("search", mark);
    state.history = None;
    mark = account("history (layout, order, cache, checkpoints)", mark);
    drop(state);
    mark = account("the rest of AppState", mark);
    drop(store);
    mark = account("RepoStore (CommitTable)", mark);
    println!("total\tlive_unattributed\t{mark}");
    Ok(())
}

fn settled(history: &History) -> bool {
    history.jump.is_none() && history.promotion.is_none()
}

fn press(jobs: &impl Dispatch, state: &AppState, code: KeyCode) -> AppState {
    let (next, commands) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
    for command in commands {
        jobs.dispatch(command);
    }
    next
}

fn pump(
    jobs: &impl Dispatch,
    events: &mpsc::Receiver<AppEvent>,
    mut state: AppState,
    done: impl Fn(&History) -> bool,
) -> Result<AppState, String> {
    loop {
        if let Some(history) = &state.history
            && let Phase::Failed(why) = &history.phase
        {
            return Err(format!("the walk failed: {why}"));
        }
        if state.history.as_ref().is_some_and(&done) {
            return Ok(state);
        }
        let event = events
            .recv_timeout(PATIENCE)
            .map_err(|why| format!("no event for {PATIENCE:?} ({why})"))?;
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        if let Some(quit) = &state.quit {
            return Err(format!("Graft quit: {quit:?}"));
        }
    }
}

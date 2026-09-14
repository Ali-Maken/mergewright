//! `budgets --phases` (m1-the-graph/55): first paint cut into its phases, run
//! serially on this thread with no jobs, so each phase's cost is its own.
//!
//! Wall time is per phase. RSS is the process's peak (`getrusage`), so a
//! phase's delta is how far it raised the peak, not what it freed.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use graft::history::{WalkSource, seeds_for};
use graft::repository::facts;
use graft_git::branches::Branches;
use graft_git::history::{CommitTable, Order, Walk};
use graft_git::repo::SharedRepo;
use graft_graph::layout::stream::{StreamConfig, StreamingLayout};

/// Rows on the first screen at 120×40, as `budget_run` sizes it.
const SCREEN_ROWS: usize = 40;

struct Clock {
    at: Instant,
    peak: u64,
}

impl Clock {
    fn start() -> Result<Self, String> {
        Ok(Clock {
            at: Instant::now(),
            peak: peak()?,
        })
    }

    /// Prints the phase since the last lap and starts the next.
    fn lap(&mut self, name: &str, detail: &str) -> Result<Duration, String> {
        let took = self.at.elapsed();
        let now = peak()?;
        // Tenths of a MiB in integers: no float cast to lose precision.
        let tenths = now.saturating_sub(self.peak) * 10 / (1024 * 1024);
        println!(
            "{name:<22} {:>9.2} ms  peak RSS +{:>4}.{} MiB  {detail}",
            took.as_secs_f64() * 1e3,
            tenths / 10,
            tenths % 10,
        );
        self.peak = now;
        self.at = Instant::now();
        Ok(took)
    }
}

fn peak() -> Result<u64, String> {
    graft_fixtures::peak_rss().map_err(|why| why.to_string())
}

/// Opens `repo` the way the history job does and prints each phase.
pub fn run(repo: &Path) -> Result<(), String> {
    let never = || false;
    let total = Instant::now();
    let mut clock = Clock::start()?;

    let shared = SharedRepo::open(repo).map_err(|why| why.to_string())?;
    let handle = shared.handle();
    facts(&handle)?;
    clock.lap("open", "")?;

    // The history job's path since Phase B: one read of the refs serves the
    // branches, the seeds and the walk's tips.
    let snapshot = handle.ref_snapshot().map_err(|why| why.to_string())?;
    clock.lap(
        "refs.ref_snapshot",
        &format!("{} refs, {} tips", snapshot.refs.len(), snapshot.tips.len()),
    )?;
    let branches =
        Branches::resolve_from(&handle, &snapshot.refs, None).map_err(|why| why.to_string())?;
    clock.lap("refs.resolve_from", &format!("{:?}", branches.chosen_by()))?;
    let seeds = seeds_for(&snapshot.refs, &branches);
    clock.lap("refs.seeds_for", "")?;
    let tips = snapshot.tips;

    let table = Arc::new(CommitTable::open(&handle).map_err(|why| why.to_string())?);
    clock.lap("table", &format!("graph {}", table.graph_count()))?;

    let walk = Walk::from_tips(&handle, Order::Topological, &tips, Arc::clone(&table))
        .map_err(|why| why.to_string())?;
    clock.lap(
        "walk_new",
        &format!(
            "in force {:?}, explored {}",
            walk.order_in_force(),
            walk.explorations()
        ),
    )?;

    let mut layout = StreamingLayout::new(
        WalkSource::new(&handle, walk),
        seeds,
        StreamConfig::default(),
    );
    layout.set_viewport(0..SCREEN_ROWS);
    let advance = layout
        .advance(StreamConfig::default().chunk_rows, &never)
        .map_err(|why| why.to_string())?;
    let work = layout.work();
    clock.lap(
        "first_chunk",
        &format!(
            "{} rows, hydrated {}, merge_reads {}, links_read {}",
            advance.rows_emitted, work.hydrated, work.merge_reads, work.links_read
        ),
    )?;

    let rows = layout.rows(0..SCREEN_ROWS).map_err(|why| why.to_string())?;
    clock.lap("first_window", &format!("{} rows", rows.len()))?;
    println!(
        "total                  {:>9.2} ms",
        total.elapsed().as_secs_f64() * 1e3
    );

    discriminators(&handle, &tips, &table)
}

/// Laps that confirm the cause: a walk from tips already read, a second
/// ref enumeration, and a fresh walk's first link on its own.
fn discriminators(
    handle: &graft_git::repo::RepoHandle,
    tips: &[graft_core::oid::Oid],
    table: &Arc<CommitTable>,
) -> Result<(), String> {
    let never = || false;
    // Discriminator: the same walk from the tips already read, so the
    // difference from `walk_new` is `all_refs` reading every ref again.
    let mut clock = Clock::start()?;
    let reused = Walk::from_tips(handle, Order::Topological, tips, Arc::clone(table))
        .map_err(|why| why.to_string())?;
    clock.lap(
        "from_tips(tips read)",
        &format!("explored {}", reused.explorations()),
    )?;
    drop(reused);
    handle.references().map_err(|why| why.to_string())?;
    clock.lap("references, again", "")?;

    // The walk's own first link, on a walk of its own so the layout above
    // saw the untouched walk: how much of `first_chunk` is waiting for it.
    let mut clock = Clock::start()?;
    let mut walk = Walk::all_refs(handle, Order::Topological, Arc::clone(table))
        .map_err(|why| why.to_string())?;
    clock.lap("again: walk_new", "")?;
    let first = walk.next_links(1, &never).map_err(|why| why.to_string())?;
    clock.lap(
        "again: first_link",
        &format!(
            "{} links, explored {}",
            first.links.len(),
            walk.explorations()
        ),
    )?;
    Ok(())
}

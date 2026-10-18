//! `budgets --phases` (m1-the-graph/55): first paint cut into its phases, run
//! serially on this thread with no jobs, so each phase's cost is its own.
//!
//! Wall time is per phase. RSS is the process's peak (`getrusage`), so a
//! phase's delta is how far it raised the peak, not what it freed.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mergewright::history::{WalkSource, seeds_for};
use mergewright::repository::facts;
use mergewright_git::branches::Branches;
use mergewright_git::history::{CommitTable, Order, Walk};
use mergewright_git::repo::SharedRepo;
use mergewright_graph::layout::stream::{StreamConfig, StreamingLayout};

/// Rows on the first screen at 120×40, as `budget_run` sizes it.
const SCREEN_ROWS: usize = 40;

pub(super) struct Clock {
    at: Instant,
    peak: u64,
    /// Lifetime peak physical footprint at the last lap, or why it could not
    /// be read. Anonymous (heap) pages only: a phase that raises RSS but not
    /// this faulted in mapped file pages (m1-the-graph/58b).
    footprint_peak: Result<u64, String>,
}

impl Clock {
    pub(super) fn start() -> Result<Self, String> {
        Ok(Clock {
            at: Instant::now(),
            peak: peak()?,
            footprint_peak: footprint_peak(),
        })
    }

    /// Prints the phase since the last lap and starts the next.
    pub(super) fn lap(&mut self, name: &str, detail: &str) -> Result<Duration, String> {
        let took = self.at.elapsed();
        let now = peak()?;
        // Tenths of a MiB in integers: no float cast to lose precision.
        let tenths = now.saturating_sub(self.peak) * 10 / (1024 * 1024);
        let footprint_now = footprint_peak();
        let footprint = match (&self.footprint_peak, &footprint_now) {
            (Ok(before), Ok(after)) => {
                let tenths = after.saturating_sub(*before) * 10 / (1024 * 1024);
                format!("footprint +{:>4}.{} MiB", tenths / 10, tenths % 10)
            }
            (Err(why), _) | (_, Err(why)) => format!("footprint unreadable: {why}"),
        };
        println!(
            "{name:<22} {:>9.2} ms  peak RSS +{:>4}.{} MiB  {footprint}  {detail}",
            took.as_secs_f64() * 1e3,
            tenths / 10,
            tenths % 10,
        );
        self.peak = now;
        self.footprint_peak = footprint_now;
        self.at = Instant::now();
        Ok(took)
    }
}

/// The `refs.ref_snapshot` lap's detail: tips, and how the pass learned
/// them (m1-the-graph/58b).
fn snapshot_detail(snapshot: &mergewright_git::repo::RefSnapshot) -> String {
    let stats = snapshot.stats;
    format!(
        "{} tips  refs={} known={} tags={} peeled_packed={} objects_read={} find_header={}",
        snapshot.tips.len(),
        stats.refs,
        stats.known,
        stats.tags,
        stats.peeled_packed,
        stats.objects_read,
        stats.find_header
    )
}

fn footprint_peak() -> Result<u64, String> {
    mergewright_fixtures::footprint()
        .map(|footprint| footprint.peak)
        .map_err(|why| why.to_string())
}

fn peak() -> Result<u64, String> {
    mergewright_fixtures::peak_rss().map_err(|why| why.to_string())
}

/// This process's physical footprint right now, `<current MiB> / peak
/// <peak MiB>` (m1-the-graph/57), or why it could not be read. Shares
/// `budgets.rs`'s own formatting (`super::format_footprint`) so `--gate`
/// and `--phases` report the same shape.
fn footprint_now() -> String {
    format!(
        "footprint now: {}",
        super::format_footprint(mergewright_fixtures::footprint())
    )
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

    // Opened before the refs are read, so a tip the table already knows is a
    // commit costs no object read (m1-the-graph/58).
    let table = Arc::new(CommitTable::open(&handle).map_err(|why| why.to_string())?);
    clock.lap("table", &format!("graph {}", table.graph_count()))?;

    // The history job's path since Phase B: one read of the refs serves the
    // branches, the seeds and the walk's tips.
    let snapshot = handle
        .ref_snapshot(&*table)
        .map_err(|why| why.to_string())?;
    clock.lap("refs.ref_snapshot", &snapshot_detail(&snapshot))?;
    let branches =
        Branches::resolve_from(&handle, &snapshot.refs, None).map_err(|why| why.to_string())?;
    clock.lap("refs.resolve_from", &format!("{:?}", branches.chosen_by()))?;
    let seeds = seeds_for(&snapshot.refs, &branches);
    clock.lap("refs.seeds_for", "")?;
    let tips = snapshot.tips;

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
    println!("{}", footprint_now());

    discriminators(&handle, &tips, &table)
}

/// Laps that confirm the cause: a walk from tips already read, a second
/// ref enumeration, and a fresh walk's first link on its own.
fn discriminators(
    handle: &mergewright_git::repo::RepoHandle,
    tips: &[mergewright_core::oid::Oid],
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

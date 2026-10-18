//! `budgets --phases`, continued without a commit-graph (m1-the-graph/59):
//! the stages the no-commit-graph peak passes through, serially on this
//! thread, each with what holds heap at that point.
//!
//! The app's path, minus the job pool: one `RepoStore` (so the walk, the
//! promotion sort and both layouts share one `CommitTable`), `Views` over a
//! date walk, a full date walk to the end (`G` before promotion), the
//! promotion sort, the swap to the promoted order anchored on the bottom row
//! (the old layout alive until the new one reaches it, ADR-0008), then `g`.
//!
//! `footprint now` is current / lifetime peak. A stage's transient is how far
//! it raised the lifetime peak above the footprint it started from; what it
//! kept is the rise in current footprint.

use std::path::Path;
use std::sync::Arc;

use mergewright::history::{Promote, View, Views, WalkSource, seeds_for, topological_order};
use mergewright::repos::RepoStore;
use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::oid::Oid;
use mergewright_git::history::{CommitTable, Order, Walk};
use mergewright_git::repo::{RefSnapshot, RepoHandle};
use mergewright_graph::Included;
use mergewright_graph::Seeds;
use mergewright_graph::layout::StreamingLayout;
use mergewright_graph::layout::stream::{Anchor, StreamConfig};

use super::phases::Clock;

/// Rows on the first screen at 120×40, as `budget_run` sizes it.
const SCREEN_ROWS: usize = 40;

type Layout<'a> = StreamingLayout<Included<WalkSource<'a>>>;

/// Runs the no-commit-graph stages on `repo`; says so and returns if the
/// repository has a commit-graph (the stages describe the other path).
pub fn run(repo: &Path) -> Result<(), String> {
    let never = || false;
    println!("no-cg stages (m1-the-graph/59): {}", footprint_now());
    let mut clock = Clock::start()?;

    let store = RepoStore::new(repo.to_path_buf(), None);
    let table = store.table().map_err(|why| why.to_string())?;
    if table.has_commit_graph() {
        println!("no-cg stages: skipped, the repository has a commit-graph");
        return Ok(());
    }
    let refs = store.refs().map_err(|why| why.to_string())?;
    let branches = store.branches().map_err(|why| why.to_string())?;
    let shared = store.shared().map_err(|why| why.to_string())?;
    let handle = shared.handle();
    let seeds = seeds_for(&refs.refs, &branches);
    clock.lap("cg-less open", &detail(&table, None))?;

    let mut views = views(&handle, &table, refs, seeds)?;
    let view = View::default();
    let layout = views.layout(&view).map_err(|why| why.to_string())?;
    layout.set_viewport(0..SCREEN_ROWS);
    layout
        .advance(StreamConfig::default().chunk_rows, &never)
        .map_err(|why| why.to_string())?;
    layout.rows(0..SCREEN_ROWS).map_err(|why| why.to_string())?;
    clock.lap("first paint", &detail(&table, Some(layout)))?;

    let (rows, last) = walk_to_the_end(layout)?;
    clock.lap("date walk to end", &detail(&table, Some(layout)))?;
    println!("  {}", footprint_now());

    let order = topological_order(&store, &never)?.ok_or("the sort was stopped")?;
    clock.lap(
        "topological_order",
        &format!(
            "{} handles ({} MiB)  {}",
            order.len(),
            mib(order.len() * std::mem::size_of::<CommitHandle>()),
            detail(&table, None)
        ),
    )?;
    println!("  {}", footprint_now());

    let layout = swap(&mut views, &view, order, last)?;
    // `Views::promote`, the synchronous swap: the app promotes through
    // `begin_promotion` and idle steps (m7-power-tools/12 Part C), so this
    // lap times a path the app no longer takes (review m7-12 Minor 5).
    clock.lap(
        "promote swap (synchronous path, not the app's)",
        &format!("anchored at the bottom  {}", detail(&table, Some(layout))),
    )?;
    println!("  {}", footprint_now());

    layout.set_viewport(0..SCREEN_ROWS);
    layout.rows(0..SCREEN_ROWS).map_err(|why| why.to_string())?;
    clock.lap("g", &detail(&table, Some(layout)))?;
    println!(
        "  {}  overflow_len {} of {} rows (ADR-0026 2x check)",
        footprint_now(),
        table.overflow_len(),
        rows
    );
    Ok(())
}

/// The app's `Views` over a date walk from `refs`' tips, opening a promoted
/// order when given one (as `history::stream` does).
fn views<'a>(
    handle: &'a RepoHandle,
    table: &Arc<CommitTable>,
    refs: Arc<RefSnapshot>,
    seeds: Seeds,
) -> Result<Views<'a, WalkSource<'a>>, String> {
    let walk = Walk::from_tips(handle, Order::Topological, &refs.tips, Arc::clone(table))
        .map_err(|why| why.to_string())?;
    let table = Arc::clone(table);
    let mut started = Some(walk);
    let open = move |promoted: Option<&Arc<[CommitHandle]>>| {
        if let Some(order) = promoted {
            return Ok(WalkSource::in_order(
                handle,
                Arc::clone(&table),
                Arc::clone(order),
            ));
        }
        let walk = match started.take() {
            Some(walk) => walk,
            None => Walk::from_tips(handle, Order::Topological, &refs.tips, Arc::clone(&table))?,
        };
        Ok(WalkSource::new(handle, walk))
    };
    Ok(Views::new(seeds, StreamConfig::default(), Box::new(open)))
}

/// `G` before promotion: the date walk laid out to its end and the bottom
/// screen hydrated. Returns the rows laid out and the bottom row's commit.
fn walk_to_the_end(layout: &mut Layout<'_>) -> Result<(usize, Oid), String> {
    let never = || false;
    while !layout.is_done() {
        layout
            .advance(usize::MAX, &never)
            .map_err(|why| why.to_string())?;
    }
    let rows = layout.rows_emitted();
    let bottom = rows.saturating_sub(SCREEN_ROWS)..rows;
    layout.set_viewport(bottom.clone());
    layout.rows(bottom).map_err(|why| why.to_string())?;
    let last = layout
        .id_at(rows.saturating_sub(1))
        .ok_or("the date walk laid out no rows")?;
    Ok((rows, last))
}

/// Swaps `view` to `order`, anchored on the bottom row `last` (the old layout
/// alive until the new one reaches it), and returns the new layout.
fn swap<'v, 'a>(
    views: &'v mut Views<'a, WalkSource<'a>>,
    view: &View,
    order: Arc<[CommitHandle]>,
    last: Oid,
) -> Result<&'v mut Layout<'a>, String> {
    let promote = Promote {
        order,
        anchor: Some(Anchor {
            commit: last,
            offset: SCREEN_ROWS - 1,
        }),
        height: SCREEN_ROWS,
    };
    let swapped = views
        .promote(view, &promote, &|| false)
        .map_err(|why| why.to_string())?;
    if !swapped {
        return Err("the promotion swap was stopped".to_owned());
    }
    views.layout(view).map_err(|why| why.to_string())
}

/// The heap holders at hand: the interner, and the layout's order and
/// resident set when there is one.
fn detail(table: &CommitTable, layout: Option<&Layout<'_>>) -> String {
    let interner = format!(
        "table len {} overflow {} interner {} MiB",
        table.len(),
        table.overflow_len(),
        mib(table.approximate_bytes())
    );
    let Some(layout) = layout else {
        return interner;
    };
    let resident = layout.resident();
    format!(
        "{interner}  rows {} order {} MiB checkpoints {} ({} MiB) frontier {} KiB held rows {}",
        layout.rows_emitted(),
        mib(resident.order_bytes),
        resident.checkpoints,
        mib(resident.checkpoint_bytes),
        resident.frontier_bytes / 1024,
        resident.rows,
    )
}

/// Bytes as MiB to one decimal, in integers.
fn mib(bytes: usize) -> String {
    let tenths = bytes * 10 / (1024 * 1024);
    format!("{}.{}", tenths / 10, tenths % 10)
}

fn footprint_now() -> String {
    format!(
        "footprint now: {}",
        super::format_footprint(mergewright_fixtures::footprint())
    )
}

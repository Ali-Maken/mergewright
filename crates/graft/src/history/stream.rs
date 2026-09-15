//! Walking a repository's history: opening it, and working out the strict
//! topological order ADR-0008 promotes to.

use std::sync::Arc;
use std::sync::mpsc::Receiver;

use graft_core::commit_handle::CommitHandle;
use graft_git::history::{CHUNK, Order, Walk};
use graft_graph::layout::StreamConfig;

use crate::jobs::Delivery;
use crate::repos::RepoStore;

use super::seeds::seeds_for;
use super::source::WalkSource;
use super::state::{Ended, HistoryUpdate, RowRequest, WalkFacts};
use super::views::{Views, error_chain, serve};

/// Walk `store`'s repository in `order`, answering each [`RowRequest`] that
/// arrives on `requests` with the rows it names.
///
/// Returns once `requests` has no sender left (the job was cancelled, or Graft
/// is leaving), `stop` says stop, or a report finds nobody listening.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, its
/// refs or branches could not be read, or the walk or the layout failed.
/// Formed here because it crosses a thread as part of a job result.
pub fn stream(
    store: &RepoStore,
    order: Order,
    requests: &Receiver<RowRequest>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Ended, String> {
    // One read of the refs serves the branches, the seeds and the walk's
    // tips: at thousands of loose refs each read is tens of milliseconds, and
    // reading them three times was most of first paint (m1-the-graph/55).
    let refs = store.refs().map_err(|error| error.to_string())?;
    let branches = store.branches().map_err(|error| error.to_string())?;
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let seeds = seeds_for(&refs.refs, &branches);

    let table = store.table().map_err(|error| error.to_string())?;
    let walk = Walk::from_tips(&handle, order, &refs.tips, Arc::clone(&table))
        .map_err(|error| error.to_string())?;
    let facts = WalkFacts {
        requested: walk.requested_order(),
        order: walk.order_in_force(),
        default_branch: branches.default_branch().cloned(),
        integration: branches.integration().to_vec().into(),
    };
    if report(HistoryUpdate::Started(Box::new(facts))) == Delivery::NobodyListening {
        return Ok(Ended::NobodyListening);
    }

    // The first view opened takes the walk already started; a second one
    // (the other `Follow`) starts its own.
    let handle = &handle;
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
            None => Walk::from_tips(handle, order, &refs.tips, Arc::clone(&table))?,
        };
        Ok(WalkSource::new(handle, walk))
    };
    let mut views = Views::new(seeds, StreamConfig::default(), Box::new(open));
    serve(&mut views, requests, stop, report).map_err(|error| error_chain(&error))
}

/// The whole range's strict topological order, worked out without a
/// commit-graph file: what the view promotes to (ADR-0008). `Ok(None)` if
/// `stop` asked it to stop.
///
/// # Errors
/// The failure text, ready to show: the repository, its refs or a commit
/// could not be read. Formed here because it crosses a thread as part of a
/// job result.
pub fn topological_order(
    store: &RepoStore,
    stop: &dyn Fn() -> bool,
) -> Result<Option<Arc<[CommitHandle]>>, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    // The walk reads each commit once, so a cache would fill and never be hit
    // (m1-the-graph/27b, fix D).
    let handle = repo.handle_with_object_cache(0);
    let table = store.table().map_err(|error| error_chain(&error))?;
    let tips = handle
        .commit_tips(&*table)
        .map_err(|error| error.to_string())?;
    let mut walk = Walk::full_topological(&handle, &tips, Arc::clone(&table))
        .map_err(|error| error_chain(&error))?;
    let mut order = Vec::new();
    loop {
        let chunk = walk
            .next_links(CHUNK, stop)
            .map_err(|error| error_chain(&error))?;
        // Without a commit-graph the first chunk has read the whole range, so
        // the count is known: size the order once instead of doubling it up
        // to a million entries (m1-the-graph/59b). With one, `explorations`
        // is only what has been read so far, and this reserves less.
        // The count is only a size hint: one past `usize` (impossible on a
        // 64-bit target) reserves nothing and the order still grows.
        let explored = usize::try_from(walk.explorations()).unwrap_or(0);
        order.reserve_exact(explored.saturating_sub(order.len()).max(chunk.links.len()));
        order.extend(chunk.links.iter().map(|link| link.commit));
        if chunk.stopped {
            return Ok(None);
        }
        if chunk.done {
            return Ok(Some(order.into()));
        }
    }
}

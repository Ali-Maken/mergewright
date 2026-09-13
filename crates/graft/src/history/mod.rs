//! Reading history for the screen: the walk, laid out as it arrives.
//!
//! The walk lives in `graft-git` and the layout engine in `graft-graph`, and
//! neither may see the other (ARCHITECTURE.md). This is where they meet:
//! [`WalkSource`] puts a `Walk` behind the layout's `CommitSource` seam,
//! [`seeds_for`] turns ADR-0001's branch resolution into reserved lanes, and
//! [`stream`] drives a `StreamingLayout` a chunk at a time, handing each
//! chunk's rows to the UI thread as soon as it is laid out — so the first row
//! is on screen long before the last commit is read.
//!
//! Nothing here knows about jobs or threads beyond a `stop` check and a
//! `report` callback; `crate::jobs` supplies both.

mod seeds;
mod source;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use graft_core::ref_name::RefName;
use graft_git::history::{Order, Walk};
use graft_graph::Row;
use graft_graph::layout::{CommitSource, StreamConfig, StreamError, StreamingLayout};

use crate::jobs::Delivery;
use crate::repos::RepoStore;

pub use seeds::seeds_for;
pub use source::WalkSource;

/// What a walk is, known before its first commit: the order it is in and the
/// branches its lanes are anchored on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkFacts {
    /// The order that was asked for.
    pub requested: Order,
    /// The order the commits actually come in. Different from
    /// [`WalkFacts::requested`] when there is no commit-graph file and
    /// ADR-0008 fell back to commit date.
    pub order: Order,
    /// Which branch owns lane 0 (ADR-0001), or [`None`] in a repository with
    /// no branches.
    pub default_branch: Option<RefName>,
    /// The branches that sit beside lane 0 and never compete for it.
    pub integration: Arc<[RefName]>,
}

impl WalkFacts {
    /// Is this an order the user should be told is not the final one?
    #[must_use]
    pub fn is_provisional(&self) -> bool {
        self.requested != self.order
    }
}

/// One thing a running walk tells the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryUpdate {
    /// The walk has started; this is what it is. Always the first update.
    Started(Box<WalkFacts>),
    /// The next rows, in order, straight after the ones already sent.
    /// Shared rather than cloned: the state is copied on every event.
    Rows(Arc<[Row]>),
}

/// Where a walk is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Rows are still arriving.
    Streaming,
    /// Every commit has a row.
    Complete,
    /// The user stopped it. The rows that had arrived stay.
    Cancelled,
    /// It broke, and this is why.
    Failed(String),
}

/// The history on screen: what has arrived of the walk the screen waits for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// What the walk is, once it has said.
    pub walk: Option<WalkFacts>,
    /// The rows laid out so far, chunk by chunk as they arrived.
    pub chunks: Vec<Arc<[Row]>>,
    /// How many rows [`History::chunks`] holds between them.
    pub rows: usize,
    /// Where the walk is.
    pub phase: Phase,
}

impl History {
    /// A walk that has been asked for and has sent nothing yet.
    #[must_use]
    pub fn started() -> Self {
        History {
            walk: None,
            chunks: Vec::new(),
            rows: 0,
            phase: Phase::Streaming,
        }
    }

    /// Takes in one update from the walk.
    pub fn apply(&mut self, update: HistoryUpdate) {
        match update {
            HistoryUpdate::Started(facts) => self.walk = Some(*facts),
            HistoryUpdate::Rows(rows) => {
                self.rows += rows.len();
                self.chunks.push(rows);
            }
        }
    }
}

/// How a [`stream`] or [`lay_out`] call ended, when it did not fail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// Every commit was laid out and reported.
    Complete,
    /// `stop` asked it to stop. Nothing laid out after that was reported.
    Cancelled,
    /// A report found nobody listening: the loop has gone.
    NobodyListening,
}

/// Walk `store`'s repository in `order`, laying it out and reporting each
/// chunk of rows through `report` as it lands.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, its
/// refs or branches could not be read, or the walk or the layout failed.
/// Formed here because it crosses a thread as part of a job result.
pub fn stream(
    store: &RepoStore,
    order: Order,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Ended, String> {
    let branches = store.branches().map_err(|error| error.to_string())?;
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let refs = handle.references().map_err(|error| error.to_string())?;
    let seeds = seeds_for(&refs, &branches);

    let walk = Walk::all_refs(&handle, order).map_err(|error| error.to_string())?;
    let facts = WalkFacts {
        requested: walk.requested_order(),
        order: walk.order_in_force(),
        default_branch: branches.default_branch().cloned(),
        integration: branches.integration().to_vec().into(),
    };
    if report(HistoryUpdate::Started(Box::new(facts))) == Delivery::NobodyListening {
        return Ok(Ended::NobodyListening);
    }

    let source = WalkSource::new(&handle, walk);
    let mut layout = StreamingLayout::new(source, seeds, StreamConfig::default());
    lay_out(&mut layout, stop, report).map_err(|error| error_chain(&error))
}

/// Advances `layout` a chunk at a time until its source is exhausted,
/// reporting each chunk's rows the moment they are laid out.
///
/// `stop` is checked before every chunk and handed to the source within one.
/// Once it has said stop, nothing more is reported — the rows of a chunk it
/// interrupted are not a chunk anybody asked for.
///
/// # Errors
/// Whatever the layout reports: the source failed, repeated a commit or
/// stalled.
pub fn lay_out<S: CommitSource>(
    layout: &mut StreamingLayout<S>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Ended, StreamError<S::Error>> {
    let chunk = layout.config().chunk_rows;
    loop {
        let before = layout.rows_emitted();
        let advanced = layout.advance(chunk, stop)?;
        if advanced.stopped || stop() {
            return Ok(Ended::Cancelled);
        }
        let after = layout.rows_emitted();
        if after > before {
            // The newest chunk is the most recently touched, so it is still
            // resident: this reads the cache, it does not rebuild.
            let rows = layout.rows(before..after)?;
            if report(HistoryUpdate::Rows(rows.into())) == Delivery::NobodyListening {
                return Ok(Ended::NobodyListening);
            }
        }
        if advanced.done {
            return Ok(Ended::Complete);
        }
    }
}

/// An error and every cause under it, on one line: `StreamError::Source`
/// says only "reading history", and the reason is in its source.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    text
}

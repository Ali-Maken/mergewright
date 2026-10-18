//! Folding merged branches by default, worker side (ease-pass 06, ADR-0063):
//! laying out with the fold's repairs, and what the fold says about each
//! row a [`Window`](super::Window) carries.
//!
//! The rule itself is `graft_graph::inclusion::FoldWalk`; this is where the
//! history worker drives it. Every advance goes through [`lay_out`], never
//! plain `StreamingLayout::advance`: in the provisional order the fold can
//! hide a commit too early or leave out a merge's edge that turns out to
//! reach a shown commit, and only `repair` mends that (missing rows and
//! missing edges otherwise).

use std::collections::HashMap;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_graph::inclusion::{Whereabouts, repair};
use graft_graph::layout::{Advance, CommitSource, StreamError, StreamingLayout};
use graft_graph::{Included, Row, recover_branch_name};

/// What the fold says about one row. Default — nothing — on every row of a
/// view that does not fold by default.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowFold {
    /// On a merge row: `Some(true)` when a merged branch is folded behind it
    /// (or shown because it is opened), `Some(false)` when nothing is (a sync
    /// merge), `None` until every second parent has been walked — and on any
    /// row that is not a merge. Only `Some(true)` draws a tail.
    pub folds: Option<bool>,
    /// The innermost opened merge whose merged side shows this row.
    pub inside: Option<Inside>,
}

/// The opened merge a row is inside, and the branch name recovered from its
/// subject (`graft_graph::recover_branch_name`), if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inside {
    /// The opened merge.
    pub merge: Oid,
    /// Its merged branch's name; `None` when the subject names none.
    pub label: Option<Arc<[u8]>>,
}

/// [`StreamingLayout::advance`], then the fold's repair, until nothing is
/// left to mend (`graft_graph::inclusion::advance_folded`), also saying the
/// first row any repair threw away — rows a window already sent may hold.
pub(super) struct Laid {
    /// The last advance's.
    pub advance: Advance,
    /// The first row a repair laid out again, if one did.
    pub invalidated: Option<usize>,
}

/// Lays out up to `budget` rows the way a folding view must: advancing, and
/// repairing what the provisional order got wrong, until nothing is left.
/// A layout that does not fold has nothing to repair: plain `advance`.
///
/// # Errors
/// As `StreamingLayout::advance` and `graft_graph::inclusion::repair`.
pub(super) fn lay_out<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    budget: usize,
    stop: &dyn Fn() -> bool,
) -> Result<Laid, StreamError<S::Error>> {
    let mut invalidated: Option<usize> = None;
    loop {
        let advance = layout.advance(budget, stop)?;
        if advance.stopped {
            return Ok(Laid {
                advance,
                invalidated,
            });
        }
        match repair(layout)? {
            Some(thrown) => {
                invalidated =
                    Some(invalidated.map_or(thrown.from_row, |row| row.min(thrown.from_row)));
            }
            None => {
                return Ok(Laid {
                    advance,
                    invalidated,
                });
            }
        }
    }
}

/// The fold's facts for each of `rows`, in order: empty when the view does
/// not fold. `labels` caches each opened merge's recovered name, read from
/// its own row in `layout`.
///
/// # Errors
/// Reading an opened merge's row back from `layout` failed.
pub(super) fn row_folds<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    rows: &[Row],
    labels: &mut HashMap<Oid, Option<Arc<[u8]>>>,
) -> Result<Arc<[RowFold]>, StreamError<S::Error>> {
    let Some(fold) = layout.source().fold() else {
        return Ok(Arc::from([]));
    };
    let included = layout.source();
    let facts: Vec<(Option<bool>, Option<Oid>)> = rows
        .iter()
        .map(|row| {
            let Some(handle) = included.handle_of(&row.commit.id) else {
                return (None, None);
            };
            let inside = fold.inside(handle).map(|merge| included.id_of(merge));
            (fold.folds(handle), inside)
        })
        .collect();
    let mut folds = Vec::with_capacity(facts.len());
    for (merge_folds, inside) in facts {
        let inside = match inside {
            Some(merge) => Some(Inside {
                merge,
                label: label_of(layout, merge, labels)?,
            }),
            None => None,
        };
        folds.push(RowFold {
            folds: merge_folds,
            inside,
        });
    }
    Ok(folds.into())
}

/// `merge`'s recovered branch name, read from its row once and cached. An
/// opened merge always has a row above the rows inside it; were it ever not
/// laid out, it has no name to give yet.
fn label_of<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    merge: Oid,
    labels: &mut HashMap<Oid, Option<Arc<[u8]>>>,
) -> Result<Option<Arc<[u8]>>, StreamError<S::Error>> {
    if let Some(label) = labels.get(&merge) {
        return Ok(label.clone());
    }
    let Some(at) = layout.row_of(merge) else {
        return Ok(None);
    };
    let rows = layout.rows(at..at + 1)?;
    let label: Option<Arc<[u8]>> = rows
        .first()
        .and_then(|row| recover_branch_name(row.commit.subject()))
        .map(Arc::from);
    labels.insert(merge, label.clone());
    Ok(label)
}

/// The shown merge a walked, hidden `commit` folds behind — through any
/// hidden merges on the way — or `None` when `commit` is shown, not walked
/// yet, or the view does not fold. Where a promotion's anchor lands once
/// the topological order hides it (ADR-0008); `Included::whereabouts`'s
/// `Folded`, the one place that reads the chain.
pub(super) fn landing<S: CommitSource>(
    layout: &StreamingLayout<Included<S>>,
    commit: Oid,
) -> Option<Oid> {
    match layout.source().whereabouts(commit) {
        Whereabouts::Folded { behind } => Some(behind),
        Whereabouts::Shown | Whereabouts::Filtered | Whereabouts::NotWalked => None,
    }
}

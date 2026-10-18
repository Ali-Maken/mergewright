//! A row's parents for everything but drawing (ease-pass 06 D1): the
//! repository's, never the view's. A folded merge, and a merge in the
//! first-parent view, is a merge here; drawing reads `Row::drawn_parents`.

use mergewright_core::oid::Oid;
use mergewright_graph::Row;

use crate::refs::RefBadges;

/// The parents a reader that reasons about `row`'s commit — a menu, a
/// refusal, the radar, the detail pane, a git write — uses: every real
/// parent. A stash entry is the exception: its index and untracked
/// parents are not history, so it keeps its first parent only, as the
/// graph has always shown it. Before the refs are read no row is known to
/// be a stash entry, and the graph draws none as one either.
#[must_use]
pub fn of<'r>(row: &'r Row, refs: Option<&RefBadges>) -> &'r [Oid] {
    let parents = row.commit.parents.as_slice();
    if refs.is_some_and(|refs| refs.is_stash(row.commit.id)) {
        &parents[..parents.len().min(1)]
    } else {
        parents
    }
}

/// Whether `row` is a merge to everything but drawing: two or more of
/// [`of`]'s parents. A folded merge is one; a stash entry is not.
#[must_use]
pub fn is_merge(row: &Row, refs: Option<&RefBadges>) -> bool {
    of(row, refs).len() >= 2
}

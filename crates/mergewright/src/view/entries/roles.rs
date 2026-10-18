//! Each row's lanes by role (ease-pass G1, G2): what every lane is to the
//! branch the cursor row was made on. The focus moves with `j`/`k`; the
//! bases come from `AppState::route` once 09's read lands, and only while
//! it was read for the branch in focus.

use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::row::Row;
use mergewright_graph::{LaneRole, Route, made_on};

use crate::history::History;
use crate::update::AppState;

/// One role per column of `row.lanes` (real columns: the fold is planned at
/// render, so a column it drops is never drawn); an unnamed lane is
/// [`LaneRole::Other`].
pub(crate) fn lane_roles(row: &Row, route: &Route) -> Vec<LaneRole> {
    row.lanes
        .iter()
        .map(|lane| lane.map_or(LaneRole::Other, |identity| route.role(identity)))
        .collect()
}

/// The route this frame draws by: focused on the cursor row's branch
/// ([`focus`]), with the bases `state.look.route` holds when it was read for that
/// same branch, else none yet ([`Route::unread`]): another branch's chain
/// would light the wrong lanes.
pub(in crate::view) fn frame_route(state: &AppState, history: &History) -> Route {
    let focus = focus(history);
    match &state.look.route {
        Some(read) if read.focus() == focus => read.clone(),
        Some(read) => Route::unread(read.trunk(), focus),
        None => Route::unread(trunk(history), focus),
    }
}

/// The branch the cursor row was made on; on a row inside an opened merge,
/// the branch that merge's row was made on (G4), while that row is in the
/// window the walk last sent.
pub(in crate::view) fn focus(history: &History) -> Option<BranchIdentity> {
    let at = history.cursor;
    if let Some(inside) = crate::view::rows::inside_merge(history, at) {
        let merge_row = history
            .window
            .as_ref()
            .and_then(|window| window.rows.iter().find(|row| row.commit.id == inside.merge));
        if let Some(merge_row) = merge_row {
            return made_on(merge_row);
        }
    }
    history.selected().and_then(made_on)
}

/// The trunk: the default branch the walk resolved. Before it has, no
/// branch is the trunk — the empty name matches no lane — so only the
/// focus stands out.
fn trunk(history: &History) -> BranchIdentity {
    history
        .walk
        .as_ref()
        .and_then(|walk| walk.default_branch.as_ref())
        .map_or_else(
            || BranchIdentity::of_name(b""),
            |name| BranchIdentity::of_name(name.shorthand()),
        )
}

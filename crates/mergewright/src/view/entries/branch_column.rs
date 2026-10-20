//! The branch column and the message's dimming (ease-pass G4–G6): each run
//! of rows made on one branch names it once, in its role's colour, `(you)`
//! after `HEAD`'s; a row inside an opened merge names the merged branch's
//! last path segment, dim; a row made on a branch off the cursor's route
//! dims its message.

use std::collections::{HashMap, HashSet};

use mergewright_graph::row::Row;
use mergewright_graph::{BranchIdentity, LaneRole, Route, made_on, recover_branch_name};
use mergewright_term::sanitize_path_bytes;
use mergewright_ui::graph_screen::{BranchCell, GraphEntry};

use crate::history::History;
use crate::update::AppState;

/// Fills `entries` — one per row of `rows`, the visible history rows
/// starting at history row `top` — with their branch cells and message dim.
pub(super) fn fill(
    state: &AppState,
    history: &History,
    route: &Route,
    (rows, top): (&[Row], usize),
    entries: &mut [GraphEntry<'_>],
) {
    let wanted: HashSet<BranchIdentity> = rows.iter().filter_map(made_on).collect();
    let (names, head) = names_of(state, history, &wanted);
    let role = |made: Option<BranchIdentity>| made.map_or(LaneRole::Other, |id| route.role(id));
    // The branch the run above was made on; `None` before the first row, so
    // the top row on screen always names its branch.
    let mut last: Option<Option<BranchIdentity>> = None;
    for ((entry, row), index) in entries.iter_mut().zip(rows).zip(top..) {
        let made = made_on(row);
        if let Some(inside) = crate::view::rows::inside_merge(history, index) {
            entry.branch = inside.label.map(|label| BranchCell {
                name: sanitize_path_bytes(last_segment(&label)),
                role: role(made),
                you: false,
                dim: true,
            });
            entry.dim_message = true;
            continue;
        }
        entry.dim_message = role(made) == LaneRole::Other;
        if last == Some(made) {
            continue;
        }
        last = Some(made);
        entry.branch = made.and_then(|id| {
            names.get(&id).map(|name| BranchCell {
                name: sanitize_path_bytes(name),
                role: role(made),
                you: head == Some(id),
                dim: false,
            })
        });
    }
}

/// The name the branch column gives lane `made` (ease-pass/07b: the bar's
/// branch is named as the column names it). The same lookup [`fill`]
/// runs, for one lane.
pub(crate) fn branch_name(
    state: &AppState,
    history: &History,
    made: BranchIdentity,
) -> Option<Vec<u8>> {
    let wanted = HashSet::from([made]);
    let (mut names, _) = names_of(state, history, &wanted);
    names.remove(&made)
}

/// `label`'s last path segment: `sara/fix-toc` → `fix-toc`.
fn last_segment(label: &[u8]) -> &[u8] {
    label.rsplit(|byte| *byte == b'/').next().unwrap_or(label)
}

/// The name behind each `wanted` lane identity, and `HEAD`'s branch's
/// identity. A lane is named by the ref it was seeded from (the default
/// branch, a local or remote-tracking branch) or by the merge subject it
/// was recovered from (`mergewright_graph::recover_branch_name`): the same
/// names, hashed the same way, as the layout used. One pass over the refs
/// and the window's merges per frame; nothing is read. A lane named by a
/// short SHA (no ref, no merge subject) stays unnamed: its cell is blank.
fn names_of(
    state: &AppState,
    history: &History,
    wanted: &HashSet<BranchIdentity>,
) -> (HashMap<BranchIdentity, Vec<u8>>, Option<BranchIdentity>) {
    let mut names = HashMap::with_capacity(wanted.len());
    let offer = |names: &mut HashMap<BranchIdentity, Vec<u8>>, name: &[u8]| {
        let id = BranchIdentity::of_name(name);
        if wanted.contains(&id) {
            names.entry(id).or_insert_with(|| name.to_vec());
        }
    };
    if let Some(default) = history
        .walk
        .as_ref()
        .and_then(|walk| walk.default_branch.as_ref())
    {
        offer(&mut names, default.shorthand());
    }
    let mut head = None;
    if let Some(refs) = state.refs.as_deref() {
        for (_, name, current, _) in refs.branches() {
            offer(&mut names, name);
            if current {
                head = Some(BranchIdentity::of_name(name));
            }
        }
        for (_, name) in refs.remotes() {
            offer(&mut names, name);
        }
    }
    if names.len() < wanted.len()
        && let Some(window) = history.window.as_ref()
    {
        for row in window
            .rows
            .iter()
            .filter(|row| row.commit.parents.len() > 1)
        {
            if let Some(name) = recover_branch_name(row.commit.subject()) {
                offer(&mut names, &name);
            }
        }
    }
    (names, head)
}

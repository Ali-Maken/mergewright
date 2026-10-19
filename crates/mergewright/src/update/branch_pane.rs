//! The branch pane's read (ease-pass 09, G12): the focused branch's four
//! answers and its route, asked at rest from `update::settle` right after
//! the fold count, kept by [`StatusKey`] until the next refs read.
//!
//! - **Which branch:** the focus the lane colours use (`view::focus`, G4),
//!   matched to a local branch by [`identity_of`].
//! - **Candidates:** every local branch `RefBadges` holds, kind from the walk's
//!   default and integration names (settle test 1: held on `History.walk`).
//! - **Cache:** keyed by the branch and every candidate tip; cleared whole
//!   when a new `RefBadges` lands ([`refs_landed`]). An upstream that moves
//!   always arrives with a refs read, so no upstream tip is held here: the
//!   job reads it itself.
//! - **Route:** a read for the branch in focus sets `look.route`, so the
//!   lanes it names draw `│` (Under); `view::frame_route` already ignores a
//!   route read for another branch.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use mergewright_core::job::JobId;
use mergewright_core::ref_name::RefName;
use mergewright_graph::Route;
use mergewright_graph::identity::BranchIdentity;
use mergewright_ops::branch_status::{BranchStatus, Candidate, CandidateKind};

use super::events::{Command, RestRead};
use super::state::AppState;
use crate::history::WalkFacts;
use crate::refs::RefBadges;

/// Where a local branch's name lives.
const HEADS: &str = "refs/heads/";

/// The trunk and integration names the walk resolved (ADR-0001): what makes a
/// candidate [`CandidateKind::Trunk`] or [`CandidateKind::Integration`].
#[derive(Clone, Copy, Debug)]
pub struct BranchNames<'a> {
    /// The default branch, full ref name.
    pub trunk: Option<&'a RefName>,
    /// The integration branches, full ref names.
    pub integration: &'a [RefName],
}

impl<'a> BranchNames<'a> {
    /// The names the walk holds after open.
    #[must_use]
    pub fn of(walk: &'a WalkFacts) -> Self {
        Self {
            trunk: walk.default_branch.as_ref(),
            integration: &walk.integration,
        }
    }
}

/// What one branch's read was asked for: the branch and a hash of every
/// candidate tip. Two asks with the same key would get the same answer while
/// the refs stay as read.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatusKey {
    /// The branch, full ref name.
    pub branch: String,
    /// Every candidate's name and tip, hashed.
    pub tips: u64,
}

/// The job's input (`RestRead::BranchStatus`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusAsk {
    /// The key the answer comes back under.
    pub key: StatusKey,
    /// The branch read.
    pub branch: Candidate,
    /// Every branch that could be its base.
    pub candidates: Vec<Candidate>,
}

/// Where one key's read stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusRead {
    /// Asked; each line says `reading…`.
    Reading,
    /// The answers.
    Read(Arc<BranchStatus>),
    /// Git could not answer: each line says `can't tell: <reason>`.
    Failed(String),
}

/// The reads kept this refs generation, by key.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reads {
    by_key: HashMap<StatusKey, StatusRead>,
}

impl Reads {
    /// The read for `key`, if asked.
    #[must_use]
    pub fn get(&self, key: &StatusKey) -> Option<&StatusRead> {
        self.by_key.get(key)
    }
}

/// Every local branch in `refs`, as a candidate base: full name, tip, and
/// kind by `names`. Sorted by name, so the key and the job see one order.
#[must_use]
pub fn candidates(refs: &RefBadges, names: &BranchNames<'_>) -> Vec<Candidate> {
    let mut all: Vec<Candidate> = refs
        .branches()
        .map(|(tip, short, _, _)| {
            let name = format!("{HEADS}{}", String::from_utf8_lossy(short));
            let is = |full: &RefName| full.as_bytes() == name.as_bytes();
            let kind = if names.trunk.is_some_and(is) {
                CandidateKind::Trunk
            } else if names.integration.iter().any(is) {
                CandidateKind::Integration
            } else {
                CandidateKind::Other
            };
            Candidate { name, tip, kind }
        })
        .collect();
    all.sort_by(|a, b| a.name.cmp(&b.name));
    all
}

/// The lane identity a candidate's commits are drawn under: the layout's
/// own short form (`RefName::shorthand`), not ops' display `short_name`.
#[must_use]
pub fn identity_of(candidate: &Candidate) -> BranchIdentity {
    match RefName::new(candidate.name.as_bytes()) {
        Ok(name) => BranchIdentity::of_name(name.shorthand()),
        // Safe to fall back: candidates are built from names git listed,
        // which always parse; and `shorthand` of a name with no known
        // prefix is the whole name, which is what this draws.
        Err(_) => BranchIdentity::of_name(candidate.name.as_bytes()),
    }
}

/// `branch`'s key against `candidates` (each name and tip, in order).
#[must_use]
pub fn status_key(branch: &Candidate, candidates: &[Candidate]) -> StatusKey {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for candidate in candidates {
        candidate.name.hash(&mut hasher);
        candidate.tip.hash(&mut hasher);
    }
    StatusKey {
        branch: branch.name.clone(),
        tips: hasher.finish(),
    }
}

/// The branch in focus as a candidate, with its key and every candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focused {
    /// The focused branch.
    pub branch: Candidate,
    /// Its key.
    pub key: StatusKey,
    /// Every candidate, the branch included (the rules skip it).
    pub candidates: Vec<Candidate>,
}

/// The focused branch, when the cursor row's branch (G4's focus) is a local
/// branch and the refs and walk facts have landed.
#[must_use]
pub fn focused(state: &AppState) -> Option<Focused> {
    let history = state.history.as_ref()?;
    let focus = crate::view::focus(history)?;
    let walk = history.walk.as_ref()?;
    let refs = state.refs.as_deref()?;
    let candidates = candidates(refs, &BranchNames::of(walk));
    let branch = candidates
        .iter()
        .find(|candidate| identity_of(candidate) == focus)?
        .clone();
    let key = status_key(&branch, &candidates);
    Some(Focused {
        branch,
        key,
        candidates,
    })
}

/// At rest, after the fold count: ask the focused branch's read unless its
/// key was asked this refs generation; a read already held sets the route.
pub(super) fn at_rest(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    let Some(focused) = focused(next) else {
        return;
    };
    match next.look.reads.by_key.get(&focused.key) {
        Some(StatusRead::Read(status)) => {
            let status = Arc::clone(status);
            set_route(next, &focused, &status);
        }
        Some(StatusRead::Reading | StatusRead::Failed(_)) => {}
        None => {
            next.look
                .reads
                .by_key
                .insert(focused.key.clone(), StatusRead::Reading);
            commands.push(Command::AtRest(
                next_id(),
                RestRead::BranchStatus(Box::new(StatusAsk {
                    key: focused.key,
                    branch: focused.branch,
                    candidates: focused.candidates,
                })),
            ));
        }
    }
}

/// `Product::BranchStatusRead`: kept under its key; when it is the focused
/// branch's, the lanes take its route. A failure stays on the pane's lines
/// (`can't tell:`), not the status line: nothing else went wrong.
pub(super) fn read(next: &mut AppState, key: StatusKey, result: Result<Arc<BranchStatus>, String>) {
    let entry = match result {
        Ok(status) => StatusRead::Read(status),
        Err(reason) => StatusRead::Failed(reason),
    };
    if let (StatusRead::Read(status), Some(focused)) = (&entry, focused(next))
        && focused.key == key
    {
        let status = Arc::clone(status);
        set_route(next, &focused, &status);
    }
    next.look.reads.by_key.insert(key, entry);
}

/// A new `RefBadges` landed: every key may be stale (a fetch, push, pull,
/// commit or branch operation moved a tip or an upstream). The next rest
/// asks again.
pub(super) fn refs_landed(next: &mut AppState) {
    next.look.reads.by_key.clear();
}

/// The cell is in the branch pane's slot (`graph_screen::ScreenRows::pane`,
/// mirrored here without a graph view, as `geometry` mirrors the detail
/// split): right of the rows, on the rows' lines.
pub(super) fn pane_holds(state: &AppState, column: u16, row: u16) -> bool {
    if state.screen != super::state::Screen::Graph {
        return false;
    }
    let columns = state.size.columns;
    let slot = crate::view::branch_pane::pane_slot(state, columns);
    let top = super::geometry::rows_above_graph(state);
    slot > 0
        && column >= columns.saturating_sub(slot)
        && row >= top
        && usize::from(row - top) < state.graph_pane_rows()
}

/// A click or hover in the pane would otherwise pick the graph row on its
/// line: the pane swallows it and clears the hover. The wheel still scrolls
/// the graph.
pub(super) fn pane_swallows(
    next: &mut AppState,
    mouse: mergewright_core::event::MouseInput,
) -> bool {
    use mergewright_core::event::MouseKind;
    let wheel_turn = matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown);
    let swallowed = !wheel_turn && pane_holds(next, mouse.column, mouse.row);
    if swallowed {
        next.hover = None;
    }
    swallowed
}

/// The route the lanes draw by, from `status` for the focused branch.
fn set_route(next: &mut AppState, focused: &Focused, status: &BranchStatus) {
    let trunk = focused
        .candidates
        .iter()
        .find(|candidate| candidate.kind == CandidateKind::Trunk)
        .map_or_else(|| BranchIdentity::of_name(b""), identity_of);
    let built_on: Vec<BranchIdentity> = status.route.iter().map(identity_of).collect();
    next.look.route = Some(Route::new(
        trunk,
        &built_on,
        Some(identity_of(&focused.branch)),
    ));
}

#[cfg(test)]
mod tests;

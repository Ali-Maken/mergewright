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
use crate::refs::{BranchEntry, RefBadges};

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

    /// A landed read says `branch` (full ref name) is merged nowhere
    /// (`Merged::NotYet`): the row label `not merged` (09b). Only landed
    /// reads count — unread, reading or failed draw nothing. Every key
    /// held is this refs generation's ([`refs_landed`] clears them), so
    /// matching on the branch alone is exact.
    #[must_use]
    pub fn not_merged(&self, branch: &str) -> bool {
        self.by_key.iter().any(|(key, read)| {
            key.branch == branch
                && matches!(read, StatusRead::Read(status)
                    if matches!(status.merged, mergewright_ops::branch_status::Merged::NotYet { .. }))
        })
    }
}

/// Every branch in `refs` as a candidate base: each local branch, and each
/// remote-tracking branch no local branch has as its upstream (ease-pass 09
/// ruling R: a teammate's branch he never checked out is still a branch).
/// Full name, tip, and kind by `names`; a remote one's kind is read from
/// its name with the remote stripped. Sorted by name, so the key and the
/// job see one order. The list is the refs read's own
/// ([`RefBadges::branch_list`], built once per read, ease-pass/18); only the
/// kinds are given here.
#[must_use]
pub fn candidates(refs: &RefBadges, names: &BranchNames<'_>) -> Vec<Candidate> {
    refs.branch_list()
        .entries()
        .iter()
        .map(|entry| candidate(entry, names))
        .collect()
}

/// `entry` as a candidate, its kind by `names`.
fn candidate(entry: &BranchEntry, names: &BranchNames<'_>) -> Candidate {
    Candidate {
        name: entry.name.clone(),
        tip: entry.tip,
        kind: kind_of(entry.local(), names),
    }
}

/// The kind of a branch standing for local name `local`.
fn kind_of(local: &[u8], names: &BranchNames<'_>) -> CandidateKind {
    let is = |full: &RefName| full.as_bytes().strip_prefix(HEADS.as_bytes()) == Some(local);
    if names.trunk.is_some_and(is) {
        CandidateKind::Trunk
    } else if names.integration.iter().any(is) {
        CandidateKind::Integration
    } else {
        CandidateKind::Other
    }
}

/// The lane identity a candidate's commits are drawn under: the layout's
/// own short form (`RefName::shorthand`), not ops' display `short_name`.
#[must_use]
pub fn identity_of(candidate: &Candidate) -> BranchIdentity {
    crate::refs::identity_of_name(&candidate.name)
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

/// The branch in focus as a candidate, with its key and the trunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Focused {
    /// The focused branch.
    pub branch: Candidate,
    /// Its key.
    pub key: StatusKey,
    /// The first candidate, in name order, of kind
    /// [`CandidateKind::Trunk`]; `None` when no branch stands for the
    /// default branch.
    pub trunk: Option<Candidate>,
}

/// The focused branch, when the cursor row's branch (G4's focus) is a local
/// branch and the refs and walk facts have landed. Lookups in the refs
/// read's [`BranchList`](crate::refs::BranchList): asked every update and
/// every frame, it never walks the refs (ease-pass/18).
#[must_use]
pub fn focused(state: &AppState) -> Option<Focused> {
    let (refs, names, focus) = sources(state)?;
    let list = refs.branch_list();
    let branch = candidate(list.drawn_under(focus)?, &names);
    let key = StatusKey {
        branch: branch.name.clone(),
        tips: list.tips(),
    };
    let trunk = names
        .trunk
        .and_then(|trunk| trunk.as_bytes().strip_prefix(HEADS.as_bytes()))
        .and_then(|local| list.standing_for(local))
        .map(|entry| candidate(entry, &names));
    Some(Focused { branch, key, trunk })
}

/// What the focus is read from: the refs, the walk's names, and the lane
/// identity of the cursor row's branch (G4's focus).
fn sources(state: &AppState) -> Option<(&RefBadges, BranchNames<'_>, BranchIdentity)> {
    let history = state.history.as_ref()?;
    let focus = crate::view::focus(history)?;
    let walk = history.walk.as_ref()?;
    let refs = state.refs.as_deref()?;
    Some((refs, BranchNames::of(walk), focus))
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
            // Every candidate, once per ask: the job reads them all.
            // `focused` just read the same sources, so this always finds
            // them; without them there is nothing to ask, as there.
            let Some((refs, names, _)) = sources(next) else {
                return;
            };
            let every = candidates(refs, &names);
            next.look
                .reads
                .by_key
                .insert(focused.key.clone(), StatusRead::Reading);
            commands.push(Command::AtRest(
                next_id(),
                RestRead::BranchStatus(Box::new(StatusAsk {
                    key: focused.key,
                    branch: focused.branch,
                    candidates: every,
                })),
            ));
        }
    }
}

/// Is the focused branch's read owed to rest — its key not asked this refs
/// generation? A burst defers the ask ([`at_rest`] runs only at rest), so
/// the loop must wake for rest while this holds, as it does for a fold
/// count (`update::rest_owed`); otherwise the pane reads `reading…` until
/// the next key.
pub(crate) fn owed(state: &AppState) -> bool {
    focused(state).is_some_and(|focused| state.look.reads.get(&focused.key).is_none())
}

/// The focused branch's read has landed, or there is none to ask: what a
/// test drains before it reads a frame, so the pane never races its own
/// `reading…` (the answer comes from the job pool on its own time). Not
/// while a refs read is in flight: its landing clears every read
/// ([`refs_landed`]), so what is held now may be gone by the frame.
#[must_use]
pub fn settled(state: &AppState) -> bool {
    let refs_in_flight = matches!(state.refs_read, super::state::RefsRead::Pending(_));
    !refs_in_flight
        && focused(state).is_none_or(|focused| {
            matches!(
                state.look.reads.get(&focused.key),
                Some(StatusRead::Read(_) | StatusRead::Failed(_))
            )
        })
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
        .trunk
        .as_ref()
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

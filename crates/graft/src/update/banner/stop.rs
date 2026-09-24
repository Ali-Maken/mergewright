//! The stop and the landing after it (m6-conflicts/12; moved out of
//! `banner.rs` by m6-conflicts/24).
//!
//! Two things the banner's keys lead to but do not own: the words the footer
//! gives a stopped operation ([`stop_reason`], [`stop_line`]) and the owed
//! landing on the new `HEAD` after a finished `--continue`
//! ([`land_on_head`] and the two reads it waits on).

use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};

use super::{STUCK_KEYS, banner_focusable, live, unmerged_paths};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// **A stop lands in the resolver** (m6-conflicts/18, P5): with anything
/// unmerged, the resolver opens on `paths`' first — the stop's own answer,
/// read from git's status in its order, the same source and order as
/// [`super::first_conflicted`] — opened from the banner so `Esc` returns
/// there. Nothing unmerged (an `edit` step, an empty pick) keeps the banner.
///
/// Called once `Stopped` and the banner focus are set. A resolver already
/// open, or a box, is left as it is: `open_resolver` would word a refusal
/// over the stop.
pub(in crate::update) fn land_in_resolver(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    paths: &[std::path::PathBuf],
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(first) = paths.first() else {
        return;
    };
    // A box opened while the operation ran (`c` has no `operating` guard)
    // owns the input: the stop stays on the banner, as without a resolver,
    // and `open_resolver`'s refusal is never worded over it (review
    // m6-amend-18, Major 1).
    if state.resolver.is_some() || crate::update::resolver::box_open(state) {
        return;
    }
    // The returned bool only says whether a key was spent; there is no key
    // here. Its refusals cannot hold: both callers clear `operating` first,
    // and a box was ruled out above.
    crate::update::resolver::open_resolver(
        state,
        commands,
        first.clone(),
        crate::update::resolver::OpenedFrom::Banner,
        next_id,
    );
}

/// A resume that stopped on a later step: the banner keeps the keyboard and
/// the stop is recorded as a state (m6-conflicts/12), which the footer
/// words `… stopped again at step 4 of 7 · …` beside the banner's keys.
///
/// git's own refusal, when it gave one, is a problem on top of the stop and
/// keeps its sticky line; `Ready` otherwise, which also clears the resume's
/// `Busy`.
pub(super) fn stopped_again(
    state: &mut AppState,
    kind: InProgress,
    paths: usize,
    stalled: Option<&str>,
) {
    state.banner_focus = banner_focusable(kind);
    state.stopped = Some(crate::update::state::Stop {
        origin: crate::update::relaunch::LiveOrigin::ThisRun,
        again: true,
        kind,
        paths_at_stop: paths,
    });
    state.status = match stalled {
        Some(why) => Status::Sticky {
            reason: format!("stopped again: {why}"),
            keys: Some(STUCK_KEYS.to_owned()),
        },
        None => Status::Ready,
    };
}

/// A `--continue` finished: hand the keyboard to the graph and owe it a
/// landing on the new `HEAD`, spent once the refresh `resumed` just issued
/// has both answered its refs read and started its walk.
///
/// Both, because either alone can put the cursor wrong: refs from before the
/// `c` name the old `HEAD`, and a seek into the old walk's rows is thrown
/// away when the fresh walk replaces them (`update::history_started`
/// reanchors on whatever the old walk had selected).
pub(super) fn finished_lands_on_head(state: &mut AppState, commands: &[Command]) {
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    let refs = commands.iter().find_map(|command| match command {
        Command::ReadRefs(id) => Some(*id),
        _ => None,
    });
    let walk = commands.iter().find_map(|command| match command {
        Command::ReadHistory(id, _) => Some(*id),
        _ => None,
    });
    // A scope that re-reads neither has nothing to land on: the cursor
    // stays where it is rather than waiting on reads never asked for.
    state.land_on_head = refs.is_some() && walk.is_some();
    state.head_landing = crate::update::state::HeadLanding { refs, walk };
}

/// A refs read answered. The one [`AppState::head_landing`] waits on is
/// spent here; any other — an older read still in flight when `c` was
/// pressed — leaves the landing owed.
pub(in crate::update) fn refs_answered(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
) {
    if state.land_on_head && state.head_landing.refs == Some(id) {
        state.head_landing.refs = None;
        land_on_head(state, commands);
    }
}

/// A history walk started, and `update::history_started` has put it in.
///
/// Spends [`AppState::head_landing`]'s walk whether or not this is the walk
/// the landing named: `history_started` only lets through the walk
/// `AppState::awaiting` names, and after `resumed`'s refresh that is its
/// walk or one a later refresh put in its place — either way a walk begun
/// after the `c`, so fresh enough to land in. Matching the id alone left the
/// landing owed for good when `Ctrl+R` replaced the walk before it started:
/// the superseded `Started` never gets here (m6-conflicts/24, review
/// m6-amend-11-12 Minor 3; `a_refresh_before_the_walk_starts_still_lands_on_head`).
pub(in crate::update) fn walk_started(state: &mut AppState, commands: &mut Vec<Command>) {
    if state.land_on_head && state.head_landing.walk.is_some() {
        state.head_landing.walk = None;
        land_on_head(state, commands);
    }
}

/// Spends [`AppState::land_on_head`] once both reads are in: the graph
/// cursor to `HEAD`'s row through `History::seek` — at once when the rows
/// on hand hold it, else by a `Find` the fresh walk answers.
///
/// Takes `commands` because `seek` returns a request the walk must be sent.
/// Nothing while either read is still out.
fn land_on_head(state: &mut AppState, commands: &mut Vec<Command>) {
    if !state.land_on_head || state.head_landing != crate::update::state::HeadLanding::default() {
        return;
    }
    state.land_on_head = false;
    let Some(head) = state.refs.as_ref().and_then(|refs| refs.head()) else {
        // An unborn `HEAD` has no row: nothing to land on.
        return;
    };
    state.on_working_tree = false;
    let height = state.graph_rows();
    let Some(history) = state.history.as_mut() else {
        // Not reached: the walk half of the landing is spent only in
        // `walk_started`, which `update::history_started` calls just after
        // putting the fresh history in, and nothing clears it after. Were
        // it `None`, there would be no rows to put a cursor on, so dropping
        // the landing loses nothing the screen could show.
        return;
    };
    let request = history.seek(head, height);
    // With no walk running (it finished, failed or was cancelled) nothing
    // can answer a request, so it is dropped, the same way every cursor
    // motion drops one (`keys::move_cursor`); the seek has already moved the
    // cursor as far as the resident rows allow.
    if let (Some(request), Some(walk)) = (request, state.awaiting) {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// Why the live operation stopped, in the footer's words
/// (m6-conflicts/12, scope item 2): `merge stopped · 1 file conflicted`,
/// `rebase stopped again at step 4 of 7 · 2 files conflicted`,
/// `merge in progress · begun outside Graft · 1 file conflicted`.
///
/// `live` is the working-tree read's detail and its unmerged count. It is
/// used **only** when its kind is the stop's: then the step and the count
/// follow the read, so resolving a file shows at once. Otherwise the stop's
/// own kind and count are used and the step is left out — a step from a
/// different operation would be a wrong number.
#[must_use]
pub fn stop_reason(
    stop: &crate::update::state::Stop,
    live: Option<(&InProgressDetail, usize)>,
) -> String {
    Wording::of(stop, live).words(true, true, true)
}

/// [`stop_reason`] and the shorter wordings the footer falls back to when
/// the full one does not fit (m6-conflicts/24, review m6-amend-11-12 M1),
/// fullest first. Each drops whole parts, lowest value first: `begun outside
/// Graft` / `from a previous run`, then the step, then the count. So the
/// count is drawn whole or not at all, and the last wording — the only one
/// the footer ever cuts — carries no number.
#[must_use]
pub fn stop_wordings(
    stop: &crate::update::state::Stop,
    live: Option<(&InProgressDetail, usize)>,
) -> Vec<String> {
    let wording = Wording::of(stop, live);
    let mut rungs: Vec<String> = Vec::with_capacity(4);
    for (origin, step, count) in [
        (true, true, true),
        (false, true, true),
        (false, false, true),
        (false, false, false),
    ] {
        let words = wording.words(origin, step, count);
        // A part the stop does not have drops nothing: skip the repeat.
        if rungs.last() != Some(&words) {
            rungs.push(words);
        }
    }
    rungs
}

/// The facts [`stop_reason`] words, gathered once for every wording.
struct Wording {
    name: &'static str,
    origin: crate::update::relaunch::LiveOrigin,
    again: bool,
    at_step: Option<(u32, u32)>,
    count: usize,
}

impl Wording {
    /// `live` counts only when its kind is the stop's (see [`stop_reason`]).
    fn of(stop: &crate::update::state::Stop, live: Option<(&InProgressDetail, usize)>) -> Self {
        let same_kind = live.filter(|(detail, _)| detail.kind == stop.kind);
        Self {
            name: stop.kind.name(),
            origin: stop.origin,
            again: stop.again,
            at_step: same_kind.and_then(|(detail, _)| detail.step),
            count: same_kind.map_or(stop.paths_at_stop, |(_, count)| count),
        }
    }

    /// The words with or without each droppable part: who began it
    /// (`origin`), the step, the count.
    fn words(&self, origin: bool, step: bool, count: bool) -> String {
        use crate::update::relaunch::LiveOrigin;

        let name = self.name;
        let at_step = self.at_step.filter(|_| step);
        let mut reason = match self.origin {
            LiveOrigin::ThisRun => {
                let again = if self.again { " again" } else { "" };
                let at =
                    at_step.map_or_else(String::new, |(at, of)| format!(" at step {at} of {of}"));
                format!("{name} stopped{again}{at}")
            }
            LiveOrigin::PreviousRun | LiveOrigin::OutsideGraft => {
                let mut words = match (self.origin, origin) {
                    (LiveOrigin::PreviousRun, true) => {
                        format!("{name} still in progress from a previous run")
                    }
                    (LiveOrigin::PreviousRun, false) => format!("{name} still in progress"),
                    (_, true) => format!("{name} in progress \u{b7} begun outside Graft"),
                    (_, false) => format!("{name} in progress"),
                };
                if let Some((at, of)) = at_step {
                    words.push_str(&format!(" \u{b7} step {at} of {of}"));
                }
                words
            }
        };
        if count && self.count > 0 {
            let files = if self.count == 1 { "file" } else { "files" };
            reason.push_str(&format!(" \u{b7} {} {files} conflicted", self.count));
        }
        reason
    }
}

/// [`stop_reason`] for the stop standing in `state`, worded from its last
/// working-tree read. `None` while nothing is stopped.
#[must_use]
pub fn stop_line(state: &AppState) -> Option<String> {
    let stop = state.stopped.as_ref()?;
    let count = unmerged_paths(state).len();
    let live = state.in_progress.as_ref().map(|detail| (detail, count));
    Some(stop_reason(stop, live))
}

/// [`stop_wordings`] for the stop standing in `state`, worded like
/// [`stop_line`] (whose words are the first). `None` while nothing is
/// stopped.
#[must_use]
pub fn stop_line_wordings(state: &AppState) -> Option<Vec<String>> {
    let stop = state.stopped.as_ref()?;
    let count = unmerged_paths(state).len();
    let live = state.in_progress.as_ref().map(|detail| (detail, count));
    Some(stop_wordings(stop, live))
}

/// One `Tab` from here lands on the banner, and nothing modal stands in the
/// way: the graph footer then leads with `⇥ banner`
/// (`HintContext::OperationLive`, m6-conflicts/12 scope item 5).
///
/// Mirrors [`take_focus`]'s forward rule as well as the ticket's four
/// conditions: with the detail pane open and its file list focused, `Tab`
/// goes to the diff first, and a hint promising the banner there would name
/// a key that does something else.
#[must_use]
pub fn one_tab_to_banner(state: &AppState) -> bool {
    live(state).is_some()
        && !state.banner_focus
        && !state.modal_owns_keys()
        && (!state.detail.shown || state.detail.focus == graft_ui::detail::DetailFocus::Diff)
}

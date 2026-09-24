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

/// A history walk started. The one [`AppState::head_landing`] waits on is
/// spent here, after `update::history_started` has put the fresh walk in.
pub(in crate::update) fn walk_started(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
) {
    if state.land_on_head && state.head_landing.walk == Some(id) {
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
        return;
    };
    let request = history.seek(head, height);
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
    use crate::update::relaunch::LiveOrigin;

    let same_kind = live.filter(|(detail, _)| detail.kind == stop.kind);
    let at_step = same_kind.and_then(|(detail, _)| detail.step);
    let count = same_kind.map_or(stop.paths_at_stop, |(_, count)| count);
    let name = stop.kind.name();
    let mut reason = match stop.origin {
        LiveOrigin::ThisRun => {
            let again = if stop.again { " again" } else { "" };
            let at = at_step.map_or_else(String::new, |(at, of)| format!(" at step {at} of {of}"));
            format!("{name} stopped{again}{at}")
        }
        LiveOrigin::PreviousRun | LiveOrigin::OutsideGraft => {
            let mut words = if stop.origin == LiveOrigin::PreviousRun {
                format!("{name} still in progress from a previous run")
            } else {
                format!("{name} in progress \u{b7} begun outside Graft")
            };
            if let Some((at, of)) = at_step {
                words.push_str(&format!(" \u{b7} step {at} of {of}"));
            }
            words
        }
    };
    if count > 0 {
        let files = if count == 1 { "file" } else { "files" };
        reason.push_str(&format!(" \u{b7} {count} {files} conflicted"));
    }
    reason
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

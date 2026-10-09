//! The stop and the landing after it (m6-conflicts/12; moved out of
//! `banner.rs` by m6-conflicts/24).
//!
//! Two things the banner's keys lead to but do not own: the words the footer
//! gives a stopped operation ([`stop_reason`], [`stop_line`]) and the owed
//! landing on the new `HEAD` after a finished `--continue`
//! ([`land_on_head`] and the two reads it waits on).

use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};

use super::{STUCK_KEYS, banner_focusable, unmerged_paths};
use crate::update::events::Command;
use crate::update::operate::Landing;
use crate::update::state::{AppState, Status};

mod tab;
mod wording;
pub use tab::BannerWay;
pub use tab::one_tab_to_banner;
pub(crate) use tab::{TabTarget, banner_way, next_of, pane_place, prev_of, tab_target, way_keys};
pub use wording::{StopLadder, stop_line, stop_line_wordings, stop_reason, stop_wordings};

/// Where a stopped rewrite is (m7-power-tools/07b): the step and rows
/// (`graft_ops::rewrite::progress`) and, at an `edit` stop, the commit git
/// handed him (`edit_stop`). Read on the job thread by [`read_rewrite_stop`]
/// and carried by every stop-producing answer — a first stop, a stop after
/// `c`, and the working-tree read a relaunch lands from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteStop {
    /// The step, the row git is on, what is left, and the branch of the line.
    pub progress: graft_ops::rewrite::RewriteProgress,
    /// `Some` at an `edit` stop: the graph takes the keyboard and `a` amends.
    pub edit: Option<graft_ops::rewrite::EditStop>,
}

/// The rewrite stop git is at, read on a job thread: `None` when no `rebase
/// -i` is running (a merge, a cherry-pick, an `apply`-style rebase), `Err`
/// with the words when git's or Graft's files could not be read — carried to
/// the banner and said on the status line, never dropped (note R4: "a failed
/// progress read").
pub(crate) fn read_rewrite_stop(
    repo: &graft_git::repo::RepoHandle,
    stop: &dyn graft_git::run::Stop,
) -> Option<Result<RewriteStop, String>> {
    let progress = match graft_ops::rewrite::progress(repo, stop) {
        Ok(Some(progress)) => progress,
        Ok(None) => return None,
        Err(error) => return Some(Err(error.explain())),
    };
    Some(
        graft_ops::rewrite::edit_stop(repo, stop)
            .map(|edit| RewriteStop { progress, edit })
            .map_err(|error| error.explain()),
    )
}

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
    rewrite: Option<Result<RewriteStop, String>>,
) {
    let stop = crate::update::state::Stop {
        origin: crate::update::relaunch::LiveOrigin::ThisRun,
        again: true,
        kind,
        paths_at_stop: paths,
        rewrite,
    };
    // An `edit` stop hands the keyboard to the graph (07b); every other
    // stop focuses the banner, as it did.
    state.banner_focus = stop_takes_focus(kind, &stop);
    state.status = match stalled {
        Some(why) => Status::Sticky {
            reason: format!("stopped again: {why}"),
            keys: Some(STUCK_KEYS.to_owned()),
        },
        None => progress_unread(&stop).map_or(Status::Ready, Status::sticky),
    };
    state.stopped = Some(stop);
}

/// Whether a stop takes the keyboard for the banner: every stop that has a
/// banner does, except a rewrite's `edit` stop, which hands it to the graph
/// so `space`, `a` and `c` work on the commit (m7-power-tools/07b, note R4).
/// A failed progress read keeps the banner's focus — the safe default: its
/// `a` is behind a box.
pub(crate) fn stop_takes_focus(kind: InProgress, stop: &crate::update::state::Stop) -> bool {
    banner_focusable(kind) && edit_stop_of(stop).is_none()
}

/// The `edit` stop a stop is at, when its rewrite read says so.
pub(crate) fn edit_stop_of(
    stop: &crate::update::state::Stop,
) -> Option<&graft_ops::rewrite::EditStop> {
    match &stop.rewrite {
        Some(Ok(rewrite)) => rewrite.edit.as_ref(),
        Some(Err(_)) | None => None,
    }
}

/// `couldn't read the rewrite's progress: <error>`, when the stop's rewrite
/// read failed: the stop still lands, on today's banner.
pub(crate) fn progress_unread(stop: &crate::update::state::Stop) -> Option<String> {
    match &stop.rewrite {
        Some(Err(error)) => Some(format!("couldn't read the rewrite's progress: {error}")),
        Some(Ok(_)) | None => None,
    }
}

/// A `--continue` finished, or a write that moves `HEAD` did
/// (`operate::operated`'s `Done`, m7-power-tools/03): hand the keyboard to
/// the graph and owe it a landing on the new `HEAD`, spent once the refresh
/// just issued has both answered its refs read and started its walk.
///
/// Both, because either alone can put the cursor wrong: refs from before the
/// `c` name the old `HEAD`, and a seek into the old walk's rows is thrown
/// away when the fresh walk replaces them (`update::history_started`
/// reanchors on whatever the old walk had selected).
pub(in crate::update) fn finished_lands_on_head(state: &mut AppState, commands: &[Command]) {
    finished_lands_on(state, commands, Landing::Head);
}

/// [`finished_lands_on_head`], landing on `on` (m7-power-tools/24): `HEAD`,
/// or one commit — a put-back's restored commit, a new branch's tip — once
/// the same two reads are in, so the row is sought in the fresh walk.
///
/// `Changes` is [`lands_on_changes`]'s: it waits on the file list, not on
/// these reads, so here it owes nothing.
pub(in crate::update) fn finished_lands_on(
    state: &mut AppState,
    commands: &[Command],
    on: Landing,
) {
    if matches!(on, Landing::Changes(_)) {
        return;
    }
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
    state.head_landing = crate::update::state::HeadLanding {
        refs,
        walk,
        on,
        gives_way: false,
    };
}

/// An edit put back lands on the changes pane, its cursor on `path` under
/// UNSTAGED, where the edit now is (m7-power-tools/24). The same
/// [`KeptCursor`] shape `relaunch::land_on_the_work` uses, so the cursor goes
/// on when the list arrives, not before: `path` is a path, the list is a
/// job.
///
/// [`KeptCursor`]: crate::update::detail::KeptCursor
pub(in crate::update) fn lands_on_changes(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    path: std::path::PathBuf,
    next_id: &mut impl FnMut() -> JobId,
) {
    // The pane follows the selection at the tail of every update, so the
    // selection has to be the working-tree row.
    state.on_working_tree = true;
    let spec = crate::update::detail::DetailSpec::WorkingTree;
    if state.detail.shown && state.detail.spec == Some(spec) {
        commands.extend(state.detail.refresh(next_id));
    } else {
        commands.extend(state.detail.open(spec, next_id));
    }
    state.detail.kept_cursor = Some(crate::update::detail::KeptCursor {
        path: path.into_os_string().into_encoded_bytes(),
        section: Some(graft_ui::detail::WorkingTreeSection::Unstaged),
        position: 0,
        body: 0,
    });
}

/// At a rewrite's `edit` stop the graph gets the keyboard with its cursor
/// on `HEAD`, the commit the stop handed him (07b, Decided "Edit stop"):
/// owed through the landing a finished `c` uses, spent once `commands`'
/// refs read and walk are in. Nothing at any other stop.
pub(in crate::update) fn edit_stop_lands_on_head(state: &mut AppState, commands: &[Command]) {
    if state.stopped.as_ref().and_then(edit_stop_of).is_some() {
        finished_lands_on_head(state, commands);
    }
}

/// [`edit_stop_lands_on_head`] at a relaunch: no refresh of its own to wait
/// on, only the launch's reads — the refs read while it is still out, the
/// walk while it has not started. Spent at once when both are in.
pub(in crate::update) fn relaunch_lands_on_head(state: &mut AppState, commands: &mut Vec<Command>) {
    state.detail.focus = graft_ui::detail::DetailFocus::Graph;
    state.on_working_tree = false;
    let refs = match state.refs_read {
        crate::update::state::RefsRead::Pending(id) => Some(id),
        crate::update::state::RefsRead::Unknown | crate::update::state::RefsRead::Current(_) => {
            None
        }
    };
    let walk = if state.history.is_none() {
        state.awaiting
    } else {
        None
    };
    state.land_on_head = true;
    state.head_landing = crate::update::state::HeadLanding {
        refs,
        walk,
        on: Landing::Head,
        gives_way: false,
    };
    land_on_head(state, commands);
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

/// Where the graph cursor was before a key or click, while an arrival's
/// landing ([`HeadLanding::gives_way`]) is owed — `None` otherwise.
///
/// [`HeadLanding::gives_way`]: crate::update::state::HeadLanding::gives_way
pub(in crate::update) struct ArrivalWatch(Option<(bool, Option<graft_core::oid::Oid>)>);

impl ArrivalWatch {
    /// Take note before the key or click is answered.
    pub(in crate::update) fn of(state: &AppState) -> Self {
        Self((state.land_on_head && state.head_landing.gives_way).then(|| Self::cursor(state)))
    }

    /// The graph cursor: on the uncommitted row, or on which commit.
    fn cursor(state: &AppState) -> (bool, Option<graft_core::oid::Oid>) {
        let commit = state
            .history
            .as_ref()
            .and_then(|history| history.selected())
            .map(|row| row.commit.id);
        (state.on_working_tree, commit)
    }

    /// After it: the same arrival landing still owed and the cursor moved —
    /// he went somewhere himself, so the landing is dropped rather than
    /// yanking him back to `HEAD` when the reads answer. A landing the key
    /// itself owes (an operation's) never gives way, so it is not touched.
    pub(in crate::update) fn drop_if_moved(self, state: &mut AppState) {
        let Some(before) = self.0 else {
            return;
        };
        if state.land_on_head && state.head_landing.gives_way && Self::cursor(state) != before {
            state.land_on_head = false;
            state.head_landing = crate::update::state::HeadLanding::default();
            // He went somewhere himself: the commit the arrival sought is
            // no longer where he is going (`tour::check_return`).
            state.return_to = None;
        }
    }
}

/// Spends [`AppState::land_on_head`] once both reads are in: the graph
/// cursor to `HEAD`'s row through `History::seek` — at once when the rows
/// on hand hold it, else by a `Find` the fresh walk answers.
///
/// Takes `commands` because `seek` returns a request the walk must be sent.
/// Nothing while either read is still out.
fn land_on_head(state: &mut AppState, commands: &mut Vec<Command>) {
    let landing = &state.head_landing;
    if !state.land_on_head || landing.refs.is_some() || landing.walk.is_some() {
        return;
    }
    state.land_on_head = false;
    let on = std::mem::take(&mut state.head_landing.on);
    let target = match on {
        Landing::Commit(commit) => Some(commit),
        Landing::Head => state.refs.as_ref().and_then(|refs| refs.head()),
        // Never owed here (`finished_lands_on`).
        Landing::Changes(_) => None,
    };
    let Some(head) = target else {
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

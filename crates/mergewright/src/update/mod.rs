//! The middle of the loop: `Event ──▶ update(state) ──▶ (new state, [Command])`
//! (ARCHITECTURE.md).
//!
//! [`update`] is the only place [`AppState`] changes, and it is pure: given the
//! same state and the same event it returns the same answer, every time,
//! forever. It reads no clock, opens no file, spawns no thread and draws
//! nothing. Anything that has to touch the world leaves here as a [`Command`]
//! for the job runner (`crate::jobs`) to execute.
//!
//! That is what makes the application testable without a terminal: every test
//! below is a plain function call over plain data.
//!
//! The module is split by concern: [`events`] and [`state`] hold the plain
//! data; [`keys`], [`search`] and [`commit_graph`] answer their own slice of
//! input; [`history`] carries the walk's progress and the topological-sort
//! promotion (ADR-0008); [`mouse`] answers the mouse, over the pure screen
//! geometry [`geometry`] computes (PROMPT.md 6.2, m2-details-and-diffs review
//! M8 — both used to live in this file). This file keeps `update` and its
//! settling; [`answer`] matches the event, [`answers`], [`tree_answers`] and
//! [`facts`] take what jobs bring back, and [`dispatch`] routes keys and the
//! core events.

pub mod amend_into;
mod answer;
mod answers;
pub mod banner;
pub mod branch_pane;
pub(crate) mod clash;
pub mod commit_editor;
mod commit_graph;
pub mod confirm;
mod context_menu;
pub mod credential;
pub mod detail;
mod dispatch;
pub mod drag;
pub(crate) mod edge;
mod events;
mod facts;
pub mod finish;
pub(crate) mod fold_count;
mod geometry;
mod history;
mod in_view;
pub mod journal;
pub mod keys;
pub mod map;
mod mouse;
pub mod name_prompt;
pub(crate) mod op_keys;
pub mod operate;
pub(crate) mod palette;
pub mod picker;
pub mod place;
mod prelude;
pub mod preview;
pub mod quit;
pub mod radar;
pub(crate) mod recover;
mod relaunch;
pub mod remote;
pub mod resolver;
pub(crate) mod rewrite_editor;
pub(crate) mod row_tags;
mod search;
mod selection;
pub mod stash;
mod state;
mod status_read;
mod strip;
pub mod switch;
pub(crate) mod tour;
mod tree_answers;
pub mod undo;

pub use context_menu::verbs::can_take;
pub use events::{
    AppEvent, Command, CommitGraphOffer, GitCheck, Product, Progress, RestRead, SortedOrder,
    TakeAt, TakeRequest, Task, WholeFileAnswer,
};
pub use geometry::history_row_line;
pub(crate) use geometry::{context_menu_area, cursor_body_line};
pub(crate) use history::walk_is_reading;
pub use relaunch::LiveOrigin;
pub(crate) use selection::{FollowKey, follow_behind, follow_key, pane_follows};
pub use state::{
    AppState, Banner, CHROME_ROWS, ClashState, GraphOverlay, Harness, HeadLanding, Quit, RefsRead,
    Screen, Size, StashOnlyMarks, Status, Stop,
};

use mergewright_core::event::Event;
// The update tests and their children name it through `crate::update`.
#[cfg(test)]
use mergewright_core::job::JobId;

use answer::answer;
pub(crate) use dispatch::dispatch_key;
use dispatch::{dispatch, show_rows};

/// The state Mergewright starts in, and the jobs that fill it.
///
/// Both the real loop (`crate::event_loop`) and the integration harnesses go
/// through here rather than writing the opening list out again, because the
/// history walk's id is minted here: `AppState::awaiting` has to name the
/// walk from the moment it is asked for, and a second list that forgot to do
/// that would put the `Ctrl+R` race back for whoever used it
/// (m5-history-and-ghost-preview/05).
///
/// Every one of these reads the world — the repository, `git --version`, the
/// history, the refs, the working tree — so every one is a job: the first
/// frame is drawn while they are still running, and says so, rather than
/// waiting on the disk.
#[must_use]
pub fn opening(repo_path: std::path::PathBuf, size: Size) -> (AppState, Vec<Command>) {
    let mut state = AppState {
        size,
        repo_path,
        ..AppState::default()
    };
    let walk = state.local_jobs.next_id();
    state.awaiting = Some(walk);
    state.facts_pending = true;
    let refs = state.local_jobs.next_id();
    state.refs_read.asked(refs);
    let tree = state.local_jobs.next_id();
    let read_tree = status_read::ask(&mut state, tree);
    let commands = vec![
        Command::OpenRepository,
        Command::CheckSystemGit,
        // The one with work in it. Asked for in topological order, which
        // ADR-0008 downgrades to commit date when the repository has no
        // commit-graph file.
        Command::ReadHistory(walk, mergewright_git::history::Order::Topological),
        // Badges: ahead/behind walks commits.
        Command::ReadRefs(refs),
        // The uncommitted-changes row.
        read_tree,
    ];
    (state, commands)
}

/// A repository arrived at on a terminal — launched inside it, or opened
/// from the empty state (m8-polish-and-ship/08 and /12d, note Lands row
/// "Open / init succeeds"): the cursor goes to `HEAD`'s row once
/// [`opening`]'s own refs read and walk have answered — the same owed landing
/// a `--continue` spends (`banner::land_on_head`). Unlike that one, it gives
/// way to a key or click that moves the graph cursor first
/// ([`HeadLanding::gives_way`]). Off a terminal (`event_loop::run`) nothing
/// calls this: the top row, as before.
///
/// `at` is the tour's way back (m8-polish-and-ship/19b, Amendment §A4): the
/// commit his cursor was on, sought as `HEAD` is; `None` is every other
/// arrival, on `HEAD`. A commit the walk does not find falls back to `HEAD`
/// with a word (`tour::check_return`).
pub fn lands_at_open(state: &mut AppState, at: Option<mergewright_core::oid::Oid>) {
    let refs = match state.refs_read {
        RefsRead::Pending(id) => Some(id),
        RefsRead::Unknown | RefsRead::Current(_) => None,
    };
    state.land_on_head = true;
    state.head_landing = HeadLanding {
        refs,
        walk: state.awaiting,
        on: at.map_or(operate::Landing::Head, operate::Landing::Commit),
        gives_way: true,
    };
    state.return_to = at;
}

/// Answer one event: the new state, and the work it wants done.
///
/// Takes the old state by reference and returns a new one instead of mutating
/// in place, so a caller can always compare before with after — which is what
/// undo, and the tests below, rely on.
#[must_use]
pub fn update(state: &AppState, event: AppEvent) -> (AppState, Vec<Command>) {
    let mut next = state.clone();
    let mut commands = Vec::new();
    let cursor_before = state.history.as_ref().map_or(0, |history| history.cursor);
    answer(&mut next, &mut commands, event);
    settle(&mut next, &mut commands, cursor_before);
    // The harness's prelude (m2-details-and-diffs/23): each key gets the
    // same follow-up a real key does, so one that starts a job leaves the
    // rest waiting for the answer; `take_next` hands out nothing until then.
    while let Some(key) = prelude::take_next(&mut next) {
        let cursor_before = next.history.as_ref().map_or(0, |history| history.cursor);
        answer(&mut next, &mut commands, AppEvent::Core(Event::Key(key)));
        settle(&mut next, &mut commands, cursor_before);
    }
    // The commit-graph line costs a graph row: when it comes or goes, the
    // viewport changed height, so ask for the rows that now fit.
    if next.commit_graph_line().is_some() != state.commit_graph_line().is_some() {
        show_rows(&mut next, &mut commands);
    }
    // Any write ends the blame trail: the lines it points at may move.
    place::after_commands(&mut next, &mut commands);
    in_view::keep_cursor_in_view(state, &mut next, &mut commands);
    keep_unseen_failure(state, &mut next);
    tour::check_return(&mut next, &mut commands);
    tour::request(state, &mut next, &mut commands);
    (next, commands)
}

/// The one seam that keeps a failure from going unseen under a box
/// (m6-conflicts/78, Decided 1). Last thing [`update`] does, every event.
///
/// - A failure that **arrives** while a modal owns the bottom row — the
///   modal stands both before and after the event, and the status became a
///   failure it was not — is copied to `unseen`. A failure that stood before
///   the modal opened is not: the modal's act replacing it is 69's rule.
/// - When the modal closes, `unseen` goes if `status` still holds that very
///   failure (the status draws itself); otherwise it stays, and the row
///   draws it in place of whatever replaced it (`shown_status`).
fn keep_unseen_failure(before: &AppState, next: &mut AppState) {
    let boxed_before = before.modal_owns_bottom_row();
    let boxed_after = next.modal_owns_bottom_row();
    if boxed_before && boxed_after && next.status.is_failure() && next.status != before.status {
        next.unseen = Some(next.status.clone());
    }
    if boxed_before && !boxed_after && next.unseen.as_ref() == Some(&next.status) {
        next.unseen = None;
    }
}

/// Is work owed to rest (G-settle)? What [`settle`] defers mid-burst: the
/// cursor's follow, and the next folding merge's count. The loop wakes for
/// rest while either holds — a count asked mid-burst by nobody else (its
/// predecessor's answer landing inside the burst, with no follow owed)
/// would otherwise wait for an unrelated key (ease-pass 06f).
pub(crate) fn rest_owed(state: &AppState) -> bool {
    follow_behind(state) || fold_count::owed(state) || branch_pane::owed(state)
}

/// What follows every answered event: the selection reconciled with the
/// cursor, the pane and the radar asking for what they now need.
fn settle(next: &mut AppState, commands: &mut Vec<Command>, cursor_before: usize) {
    // However the write ended, its question goes with it.
    credential::close_if_orphaned(next);
    selection::reconcile(next, cursor_before);
    // `local_jobs` borrowed out for the closure: `follow_selection` also
    // wants `&mut next`, and the two can't overlap.
    let mut local_jobs = next.local_jobs;
    if selection::follow_waits(next) {
        // Mid-burst (G-settle): the follow is owed until the loop says the
        // cursor rested (`AppEvent::Rested`). What the last follow started
        // is for a row the cursor has left: cancelled, once — after this
        // nothing is pending, so later moves of the burst add nothing. A
        // pane that does not follow the cursor (hidden, or a menu compare)
        // keeps its reads: only the merge probe followed the cursor.
        if selection::pane_follows(next) {
            commands.extend(next.detail.abandon());
        } else {
            commands.extend(next.detail.abandon_probe());
        }
    } else {
        commands.extend(selection::follow_selection(next, &mut || {
            local_jobs.next_id()
        }));
        next.followed = selection::follow_key(next);
        // At rest — not merely a burst with no follow owed — the folding
        // merges on screen get their counts: a held `j` reads none.
        if !next.defer_follow {
            fold_count::at_rest(next, commands, &mut || local_jobs.next_id());
            // Then the focused branch's four answers (ease-pass 09 M1).
            branch_pane::at_rest(next, commands, &mut || local_jobs.next_id());
        }
    }
    // The other asks below do not read the cursor (G-settle grep): the
    // radar's context and probes and the remotes are about refs, the
    // journal about its file, and `open_explain` answers a menu choice —
    // an input, which ends a burst before it is applied.
    radar::request_context(next, commands, &mut || local_jobs.next_id());
    radar::request_force_push_probes(next, commands, &mut || local_jobs.next_id());
    radar::request_duplicates_probe(next, commands, &mut || local_jobs.next_id());
    journal::request(next, commands, &mut || local_jobs.next_id());
    remote::request_at_launch(next, commands, &mut || local_jobs.next_id());
    remote::request_leaves(next, commands, &mut || local_jobs.next_id());
    // Last of the radar's work: the findings the refs carry, rebuilt only
    // when the refs, `HEAD`, the default branch, the context or the policy
    // moved — so a frame looks a row up instead of walking every ref
    // (M2 review C1).
    radar::refresh_ref_findings(next);
    remote::forget_stale_divergence(next);
    commands.extend(detail::open_explain(next, &mut || local_jobs.next_id()));
    next.local_jobs = local_jobs;
    // After the pane has followed the selection: a landing opens the pane
    // itself, and a follow after it would move the pane off its spec.
    detail::advance(next, commands);
}

#[cfg(test)]
mod tests;

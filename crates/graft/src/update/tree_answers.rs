//! What `answer` hands the repository opening, the history walk starting,
//! the working tree read and the stash entries.

use crate::history::History;
use crate::repository::RepositoryFacts;

use super::facts::repository_open_failed;
use super::{
    AppState, Command, StashOnlyMarks, Status, banner, clash, commit_graph, keys, relaunch,
    status_read, undo,
};

/// The repository opened, or failed to — `answer` routes only
/// [`Product::Repository`] here.
pub(super) fn repository_answer(next: &mut AppState, result: Result<RepositoryFacts, String>) {
    match result {
        Ok(facts) => repository_opened(next, facts),
        Err(failure) => repository_open_failed(next, failure),
    }
}

/// The repository opened, or re-opened on a refresh.
pub(super) fn repository_opened(next: &mut AppState, facts: RepositoryFacts) {
    next.repository = Some(facts);
    // A read failure from last time has been superseded by a repository
    // that opened. Nothing else is: a write's `Sticky` failure stays until
    // `Esc` (PROMPT.md §3), and the refresh that very failure asked for is
    // what lands here — clearing it wiped `push rejected: …` before it could
    // be read (m4-remotes/05). A running write's `Busy` and a finished
    // one's `Done` are the write's line, not the repository's.
    if matches!(next.status, Status::Failed(_)) {
        next.status = Status::Ready;
    }
    commit_graph::decide_commit_graph_offer(next);
}

/// A history walk started: it replaces whatever walk came before.
///
/// The walk the screen is waiting for was named when its command was built
/// (`Command::ReadHistory`), so this event tells `update` nothing it does not
/// already know — except which walk it is from. A `Started` under any other
/// id belongs to a walk already superseded and cancelled, and taking it would
/// blank the fresh walk's rows and put its cursor back on the wrong commit
/// (m5-history-and-ghost-preview/05).
pub(super) fn history_started(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
) {
    if next.awaiting != Some(id) {
        return;
    }
    let mut history = History::started();
    history.strip_rows = keys::strip_rows_for(next.tier());
    // A refresh keeps what the user folded, hid or switched to, and puts the
    // cursor back on the commit it was on — including a `Ctrl+R` that
    // rebuilt the id table for a commit-graph the walk had not picked up yet
    // (m1-the-graph/54): `update` cannot see that the table was rebound
    // without new cross-thread plumbing, so every walk restart reanchors,
    // the same as `change_view` already does.
    if let Some(previous) = &next.history {
        history.view = previous.view.clone();
        history.reanchor = previous.selected().map(|row| row.commit.id);
    }
    // The refs may have landed before this walk: its first request already
    // knows the stash entries, so their internals are never drawn at all.
    if let Some(view) = keys::view_change::with_stash_entries(&history.view, &stash_entries(next)) {
        history.view = view;
    }
    if let Some(request) = history.show(next.graph_rows()) {
        commands.push(Command::ShowRows(id, request));
    }
    next.history = Some(history);
    // A finished `--continue` waits on this walk too: a seek into the
    // walk it replaced would be thrown away with it (m6-conflicts/12).
    banner::walk_started(next, commands);
}

/// A working-tree read came back: the status, git's in-progress state and
/// the mergetool, and whatever landing they settle.
pub(super) fn working_tree_read(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    read: crate::working_tree::WorkingTree,
) {
    // Read before the assignment: the launch read is the one that finds no
    // working tree yet. See `relaunch::land_on_live_operation`.
    let first_read = next.working_tree.is_none();
    if let Some(disagreement) = undo::reread_disagrees(next, read.in_progress.as_ref()) {
        next.status = disagreement;
    }
    next.working_tree = Some(read.status);
    next.in_progress = read.in_progress;
    next.mergetool = read.mergetool;
    // The banner cannot have focus once there is no banner: the operation
    // finished, was aborted, or was finished from another terminal. Leaving
    // the flag set would make every key vanish into a modal nothing
    // (m5-history-and-ghost-preview/09).
    if next.in_progress.is_none() {
        next.banner_focus = false;
        // Nor can anything be stopped: the operation ended, in another
        // terminal or by an answer this read confirms. A stop left standing
        // would word a reason for an operation git no longer has
        // (m6-conflicts/12).
        next.stopped = None;
        next.live_landing_owed = false;
        // Nor is there a merge left for the finish summary's memory: one
        // finished in the shell (`git commit`) ends here, not in
        // `undo::operation_ended`, and its choices and draft would otherwise
        // word — and an unedited `⏎` commit — the next merge
        // (m6-conflicts/36, review m6-amend-35 Major 1).
        next.choices.clear();
        next.finish_draft = None;
    } else {
        // The launch lands on git's word alone; the journal only says whose
        // operation it is (m6-conflicts/12).
        let mut local_jobs = next.local_jobs;
        relaunch::working_tree_read_landed(next, commands, first_read, read.rewrite, &mut || {
            local_jobs.next_id()
        });
        next.local_jobs = local_jobs;
    }
    // A stash clash lives while anything is unmerged with no operation
    // (m6-conflicts/72b): this read is one of its facts.
    let mut local_jobs = next.local_jobs;
    clash::refresh_and_land(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
}

/// Working-tree read `id` answered, with a status or a failure. Only the
/// newest read settles what waited on it — the status line's verdict and an
/// undo's landing — because an older one read the repository before the
/// write it would be judging (`status_read`, ticket 41).
pub(super) fn working_tree_answered(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: graft_core::job::JobId,
    answer: Result<Box<crate::working_tree::WorkingTree>, String>,
) {
    let newest = status_read::answered(next, id);
    match answer {
        Ok(read) => {
            // The shell hand-over's landing (07c) needs the rewrite read
            // this answer carries, which `working_tree_read` hands on.
            let rewrite = (newest && next.continue_in_git_owed.is_some())
                .then(|| read.rewrite.clone())
                .flatten();
            working_tree_read(next, commands, *read);
            if newest {
                let mut local_jobs = next.local_jobs;
                banner::land_after_continue_in_git(next, commands, rewrite, &mut || {
                    local_jobs.next_id()
                });
                next.local_jobs = local_jobs;
                undo::land_after_undo(next);
                status_read::settle(next);
            }
        }
        Err(failure) => {
            if newest {
                // The read the line waited on has no status to give: the
                // resolver's notice stops saying it is checking, and the
                // failure below replaces the line. An undo's landing that
                // waited on it is dropped (the failure is on screen), not
                // paid by some later read that is no longer its answer.
                status_read::abandon(next);
                next.undo_landing_owed = false;
            }
            // The graph still draws without the uncommitted row; the reason
            // goes on the status line rather than leaving it silently
            // missing.
            next.status = Status::Failed(failure);
        }
    }
}

/// Every stash entry's own commit, `stash@{0}` first, as the refs last
/// read list them; empty before they are read.
pub(super) fn stash_entries(state: &AppState) -> Vec<graft_core::oid::Oid> {
    state.refs.as_deref().map_or_else(Vec::new, |refs| {
        refs.stashes().iter().map(|&(commit, _)| commit).collect()
    })
}

/// The refs were read (m7-power-tools/16): the history view learns the stash
/// entries, so each keeps only its first parent, and — with at least one —
/// a read asks which commits only a stash reaches. With none, the marks go
/// at once and nothing is asked.
pub(super) fn stash_entries_read(next: &mut AppState, commands: &mut Vec<Command>) {
    let stashes = stash_entries(next);
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(view) = keys::view_change::with_stash_entries(&history.view, &stashes)
        && let Some(request) = history.change_view(view, height)
        && let Some(walk) = next.awaiting
    {
        commands.push(Command::ShowRows(walk, request));
    }
    if stashes.is_empty() {
        next.stash_only = StashOnlyMarks::default();
        return;
    }
    let id = next.local_jobs.next_id();
    next.stash_only.job = Some(id);
    commands.push(Command::ReadStashOnly(id, stashes));
}

/// Which commits only a stash reaches came back. An answer to any read but
/// the newest is dropped: the refs moved since it was asked. A failure
/// unmarks every row and says why on the problem line (rule 3); `capped` is
/// used as far as it goes.
pub(super) fn stash_only_answered(
    next: &mut AppState,
    id: graft_core::job::JobId,
    answer: Result<std::sync::Arc<graft_git::history_reads::StashOnly>, String>,
) {
    if next.stash_only.job != Some(id) {
        return;
    }
    next.stash_only.job = None;
    match answer {
        Ok(found) => {
            next.stash_only.commits = std::sync::Arc::new(found.commits.iter().copied().collect());
        }
        Err(why) => {
            next.stash_only.commits = std::sync::Arc::default();
            next.status = Status::Failed(format!(
                "couldn't read which commits only a stash holds: {why}"
            ));
        }
    }
}

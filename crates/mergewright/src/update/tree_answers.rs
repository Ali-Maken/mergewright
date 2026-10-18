//! What `answer` hands the repository opening, the history walk starting,
//! the working tree read and the stash entries.

use crate::history::{History, View};
use crate::repository::RepositoryFacts;

use super::facts::repository_open_failed;
use super::{
    AppState, Command, StashOnlyMarks, Status, banner, clash, commit_graph, keys, relaunch,
    status_read, undo,
};

/// The repository opened, or failed to — `answer` routes only
/// [`Product::Repository`] here.
pub(super) fn repository_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    result: Result<RepositoryFacts, String>,
) {
    next.facts_pending = false;
    let head = match result {
        Ok(facts) => {
            let head = facts.head_commit;
            repository_opened(next, facts);
            head
        }
        Err(failure) => {
            repository_open_failed(next, failure);
            None
        }
    };
    head_known(next, commands, head);
}

/// The opening said where `HEAD` is (`None`: unborn, or unreadable — the
/// facts say which, and a failed opening says why on the status line).
/// A walk whose first request waited for it asks now, with `HEAD` kept;
/// a walk already asking re-lays out only if `HEAD` really moved
/// (ease-pass 06 D2).
fn head_known(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    head: Option<mergewright_core::oid::Oid>,
) {
    let height = next.graph_rows();
    let kept = next.history.as_ref().and_then(|history| {
        let view = keys::view_change::with_head_kept(&history.view, head);
        (!history.head_pending && view.is_some()).then_some(view)
    });
    if kept.is_some() {
        // A view change: a finished search's folded count rides on it.
        super::search::ask_folded(next);
    }
    let Some(history) = next.history.as_mut() else {
        return;
    };
    let request = if history.head_pending {
        history.head_pending = false;
        if let Some(view) = keys::view_change::with_head_kept(&history.view, head) {
            history.view = view;
        }
        history.show(height)
    } else {
        kept.flatten()
            .and_then(|view| history.change_view(view, height))
    };
    if let Some(request) = request
        && let Some(walk) = next.awaiting
    {
        commands.push(Command::ShowRows(walk, request));
    }
}

/// The repository opened, or re-opened on a refresh.
pub(super) fn repository_opened(next: &mut AppState, facts: RepositoryFacts) {
    next.repository = Some(Box::new(facts));
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
    id: mergewright_core::job::JobId,
) {
    if next.awaiting != Some(id) {
        return;
    }
    let mut history = History::started(next.graph_mode);
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
    // `HEAD` is kept out of the fold from the first request on, so a launch
    // lays out once (ease-pass 06 D2): from this walk's own opening when it
    // has answered, or else the first request waits for it (`head_known`).
    let head = if next.facts_pending {
        history.head_pending = true;
        head(next)
    } else {
        next.repository
            .as_ref()
            .and_then(|facts| facts.head_commit)
            .or_else(|| head(next))
    };
    if let Some(view) = refs_view(&history.view, &stash_entries(next), head) {
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
    // An operation that was live and is not: finished or aborted in the
    // shell. Its picks go with it (ease-pass 05) — on the change only, not
    // on every read with nothing live, which is how a stash clash looks.
    if next.in_progress.is_some() && read.in_progress.is_none() {
        next.picks.clear();
    }
    next.working_tree = Some(read.status);
    next.in_progress = read.in_progress;
    next.mergetool = read.mergetool;
    // An open commit screen follows the files that came or went
    // (ease-pass/08 C6).
    let mut local_jobs = next.local_jobs;
    super::commit_editor::tree_read(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
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
    id: mergewright_core::job::JobId,
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
pub(super) fn stash_entries(state: &AppState) -> Vec<mergewright_core::oid::Oid> {
    state.refs.as_deref().map_or_else(Vec::new, |refs| {
        refs.stashes().iter().map(|&(commit, _)| commit).collect()
    })
}

/// The commit `HEAD` resolves to, as the refs last read say; `None` before
/// they are read, or while the branch is unborn.
fn head(state: &AppState) -> Option<mergewright_core::oid::Oid> {
    state.refs.as_deref().and_then(crate::refs::RefBadges::head)
}

/// `view` knowing what the refs say — the stash entries
/// (`with_stash_entries`) and `HEAD`, kept out of the fold
/// (`with_head_kept`, ease-pass 06) — or `None` when it already does.
fn refs_view(
    view: &View,
    stashes: &[mergewright_core::oid::Oid],
    head: Option<mergewright_core::oid::Oid>,
) -> Option<View> {
    let with_entries = keys::view_change::with_stash_entries(view, stashes);
    let base = with_entries.as_ref().unwrap_or(view);
    keys::view_change::with_head_kept(base, head).or(with_entries)
}

/// The refs were read (m7-power-tools/16): the history view learns the stash
/// entries, so each keeps only its first parent, and where `HEAD` is, so
/// its line is never folded (ease-pass 06); and — with at least one stash —
/// a read asks which commits only a stash reaches. With none, the marks go
/// at once and nothing is asked.
pub(super) fn stash_entries_read(next: &mut AppState, commands: &mut Vec<Command>) {
    let stashes = stash_entries(next);
    let head = head(next);
    let height = next.graph_rows();
    let changes = next.history.as_ref().is_some_and(|history| {
        !history.head_pending && refs_view(&history.view, &stashes, head).is_some()
    });
    if changes {
        // A view change: a finished search's folded count rides on it.
        super::search::ask_folded(next);
    }
    if let Some(history) = next.history.as_mut()
        && let Some(view) = refs_view(&history.view, &stashes, head)
    {
        if history.head_pending {
            // Nothing asked yet: the first request carries it
            // (`head_known`).
            history.view = view;
        } else if let Some(request) = history.change_view(view, height)
            && let Some(walk) = next.awaiting
        {
            commands.push(Command::ShowRows(walk, request));
        }
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
    id: mergewright_core::job::JobId,
    answer: Result<std::sync::Arc<mergewright_git::history_reads::StashOnly>, String>,
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

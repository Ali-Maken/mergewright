//! What `answer` hands a started job, a mergetool, the journal, a stash, an
//! undo, a rewrite, the commit editor or an operation.

use mergewright_core::job::JobId;

use super::credential::credential_needed;
use super::tree_answers::history_started;
use super::{
    AppEvent, AppState, Command, Product, Progress, Status, Task, banner, commit_editor, edge,
    finish, history, journal, operate, resolver, rewrite_editor, search, stash, status_read, undo,
};

/// `git mergetool` came back.
///
/// The working tree is re-read **either way**: a tool that exited non-zero
/// may still have written the file before it did, and a stale `CONFLICTS`
/// list is how a user comes to press `c` on a conflict they have already
/// resolved. A failure keeps `a abort` in its keys, like every other line
/// the conflict interim puts up.
pub(super) fn mergetool_returned(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    path: &std::path::Path,
    result: Result<(), String>,
) {
    match result {
        // Nothing to report: the banner and the file list say what the
        // state is now, and a "mergetool ok" line would be a status field
        // reading like news when nothing happened. The tool had the file:
        // the finish summary says so (m6-conflicts/19 scope 3).
        Ok(()) => {
            finish::record_edited(next, path);
            if matches!(next.status, Status::Busy(_)) {
                next.status = Status::Ready;
            }
        }
        // Spelled for where he is now: this arm moves no focus, so the
        // place is the one he pressed `M` in (m8/10j ruling 1).
        Err(why) => {
            let place = edge::context_of(next);
            next.status = Status::Sticky {
                reason: why,
                keys: banner::mergetool_fallback(place, path, &next.keymap),
            };
        }
    }
    let tree = next.local_jobs.next_id();
    commands.push(status_read::ask(next, tree));
    commands.extend(next.detail.refresh(&mut || next.local_jobs.next_id()));
    // The resolver's `M`: its document is as stale as the file list was.
    // Both handovers land here now — `e` through `Product::HandedOver` in
    // [`answer`] — so the two behave the same, which they never did
    // (m6-conflicts/04; `e` scheduled no read at all).
    if next.resolver.is_some() {
        let mut local_jobs = next.local_jobs;
        resolver::handed_back(next, commands, &mut || local_jobs.next_id());
        next.local_jobs = local_jobs;
    }
}

/// The journal read answered.
pub(super) fn journal_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    product: &Product,
) {
    let mut local_jobs = next.local_jobs;
    let taken: bool =
        journal::journal_event(next, commands, id, product, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    debug_assert!(taken, "the journal's own product");
}

/// The stash overlay's own product — the list, or the preview's file list.
pub(super) fn stash_answer(next: &mut AppState, id: JobId, product: &Product) {
    let taken: bool = stash::stash_event(next, id, product);
    debug_assert!(taken, "the stash overlay's own product");
}

/// The journal's time machine answered, the same way.
pub(super) fn reverted_to_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    outcome: crate::jobs::undo::RevertedTo,
) {
    let mut local_jobs = next.local_jobs;
    commands.extend(undo::reverted_to(next, id, outcome, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// One `u`/`U` answered, with `local_jobs` borrowed out for the closure the
/// way every other call site does it.
pub(super) fn undone_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    outcome: crate::jobs::undo::Undone,
) {
    let mut local_jobs = next.local_jobs;
    commands.extend(undo::undone(next, id, outcome, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// A job said it had started. Three tasks take a note of the id; the rest are
/// not waited on by `start`, and are welcome whenever they arrive.
///
/// The commit graph's two are answered before this, in [`answer`].
pub(super) fn started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId, task: &Task) {
    match task {
        Task::ReadHistory => history_started(next, commands, id),
        Task::PromoteHistory => history::sort_started(next, commands, id),
        Task::Search => search::search_started(next, commands, id),
        Task::OpenRepository
        | Task::CheckSystemGit
        | Task::Resume
        | Task::ReadRefs
        | Task::ReadWorkingTree
        | Task::ListFiles { .. }
        | Task::FileDiff { .. }
        | Task::Blame
        | Task::Trace
        | Task::WholeFile
        | Task::ContainedIn { .. }
        | Task::RadarContext { .. }
        | Task::EvilProbe { .. }
        | Task::ForcePushProbe { .. }
        | Task::DuplicatesProbe { .. }
        | Task::StashOnly
        | Task::Map
        | Task::Operate
        | Task::Undo
        | Task::ReadJournal
        | Task::ReadOutside
        | Task::TourProgress
        | Task::FindLost
        | Task::RecoverFor
        | Task::PlanPreview
        | Task::RevertTo
        | Task::ReadStashes
        | Task::ReadSwitchList
        | Task::ReadPaletteBranches
        | Task::PaletteNames
        | Task::ReadTrackedPaths
        | Task::PaletteCommits
        | Task::ReadIndexState
        | Task::ReadRemotes
        | Task::OpenConflict
        | Task::TakeSide
        | Task::UntakeSide
        | Task::LoadBase
        | Task::ReadConflictSet
        | Task::ReadFinishSummary
        | Task::ReadFinishLook
        | Task::ReadCommitTemplate
        | Task::ReadHeadMessage
        | Task::ReadCommitFiles
        | Task::PlanRewrite
        | Task::OpenRewrite
        | Task::ResolveRowTags
        | Task::WriteCommitGraph
        | Task::DeclineCommitGraph => {}
    }
}

/// An answer to one of the rewrite editor's jobs (`rewrite_editor::owns`).
pub(super) fn rewrite_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: mergewright_core::job::JobId,
    product: Product,
) {
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    match product {
        Product::RewriteOpened(answer) => {
            rewrite_editor::opened(next, commands, id, answer, &mut next_id);
        }
        Product::Preview(answer) => rewrite_editor::after_planned(next, id, answer),
        Product::Files(answer) => {
            rewrite_editor::files_listed(next, id, answer.map(|(_, list)| list));
        }
        Product::ExternalEditor(answer) => {
            rewrite_editor::message_returned(next, commands, id, answer, &mut next_id);
        }
        // No job of the editor's answers with anything else; one that did
        // is still over, and whatever waited on it stops waiting.
        other => {
            debug_assert!(false, "not the rewrite editor's: {other:?}");
            rewrite_editor::job_ended(next, id, "an answer the rewrite editor never asked for");
        }
    }
    next.local_jobs = local_jobs;
}

/// The three products the commit editor waits on
/// (m3-working-tree-undo-and-stash/05).
pub(super) fn commit_editor_answer(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    event: AppEvent,
) {
    match event {
        AppEvent::Produced(id, Product::CommitTemplate(template)) => {
            commit_editor::template_arrived(next, id, template);
        }
        AppEvent::Produced(id, Product::HeadMessage(message)) => {
            let mut local_jobs = next.local_jobs;
            commit_editor::head_message_arrived(next, commands, id, message, &mut || {
                local_jobs.next_id()
            });
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::ExternalEditor(result)) => {
            commit_editor::editor_returned(next, id, result);
        }
        // `e`'s file, or `Ctrl+Z`'s suspend: nothing came back but whether
        // the terminal handover worked, and a failure is the user's to see.
        AppEvent::Produced(_, Product::HandedOver(Err(why))) => {
            next.status = Status::sticky(why);
        }
        AppEvent::Produced(_, Product::HandedOver(Ok(()))) => {}
        other => debug_assert!(false, "not the commit editor's: {other:?}"),
    }
}

/// The events an operate job can send: a running write's description
/// (`Progress::Operating`), git's credential question
/// (`Progress::CredentialNeeded`, m4-remotes/06) and its answer
/// (`Product::Operated`). Split out of [`answer`] to keep that match within
/// the line budget.
pub(super) fn operate_answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        AppEvent::Produced(id, Product::Operated(outcome)) => {
            // Read before the outcome is handed over: only a write that
            // actually ran moves the resolver on. A `git add` that was
            // refused leaves the file exactly where it was, and jumping to
            // the next conflict would hide the refusal behind a new screen.
            let staged = matches!(
                *outcome,
                crate::jobs::operate::Operated::Done(_)
                    | crate::jobs::operate::Operated::StagingNotKept { .. }
                    | crate::jobs::operate::Operated::Incomplete { .. }
            );
            let mut local_jobs = next.local_jobs;
            commands.extend(operate::operated(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            // `⏎`'s `git add` on the open file, or a whole-file `o`/`t`
            // (which also stages): on to the next conflicted one, or out.
            // `d`'s restart is the other write a resolver can launch, and
            // goes the other way — back to the same file, freshly read.
            // Both compare the id with the one the resolver launched, so
            // only the write that matches acts.
            if staged {
                resolver::resolved_advance(next, commands, id, &mut || local_jobs.next_id());
                resolver::restart_landed(next, commands, id, &mut || local_jobs.next_id());
            } else {
                resolver::write_not_landed(next, id);
            }
            next.local_jobs = local_jobs;
        }
        // `launch` already put the write's words on the line; this only
        // restores them if something else took the line meanwhile. It never
        // overwrites a `Busy` — that would wipe a remote write's
        // `· esc cancels`, its progress, or `Esc`'s `cancelling`.
        AppEvent::Progress(id, Progress::Operating { description })
            if next.operating == Some(id) && !matches!(next.status, Status::Busy(_)) =>
        {
            next.status = Status::Busy(description);
        }
        // git is waiting on an answer only the user has.
        AppEvent::Progress(id, Progress::CredentialNeeded { prompt, pulse_seed }) => {
            credential_needed(next, id, &prompt, pulse_seed);
        }
        _ => {}
    }
}

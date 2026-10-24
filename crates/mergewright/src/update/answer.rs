//! [`answer`]: one arm per event, each handed straight to its module.

use mergewright_core::event::Event;

use super::answers::{
    commit_editor_answer, journal_answer, mergetool_returned, operate_answer, reverted_to_answer,
    rewrite_answer, started, stash_answer, undone_answer,
};
use super::dispatch::core_event;
use super::facts::took_fact;
use super::tree_answers::{repository_answer, stash_only_answered};
use super::{
    AppEvent, AppState, Command, Product, Progress, Status, Task, amend_into, banner, clash,
    commit_editor, commit_graph, detail, finish, fold_count, history, journal, map, palette,
    picker, place, preview, radar, remote, resolver, rewrite_editor, search, stash, tour,
};

/// The half of [`update`] that answers one event, before the viewport is
/// squared with the commit-graph line.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per event, matched exhaustively and dispatched straight to its \
              module: the way to shorten it is to hand a whole event to a module that \
              matches it again behind a catch-all, which is what m4-remotes/05's review \
              (M4) had taken out of here"
)]
pub(super) fn answer(next: &mut AppState, commands: &mut Vec<Command>, event: AppEvent) {
    match event {
        event @ (AppEvent::Started(_, Task::WriteCommitGraph | Task::DeclineCommitGraph)
        | AppEvent::Produced(
            _,
            Product::CommitGraph(_) | Product::CommitGraphDeclined(_),
        )) => {
            commit_graph::commit_graph_event(next, event);
        }
        // A key or click that moves the graph cursor or the pane's
        // selection drops a `show_line` still landing (m7-power-tools/20b),
        // and the graph cursor an arrival's `HEAD` landing still owed
        // (m8-polish-and-ship/12d).
        AppEvent::Core(event @ (Event::Key(_) | Event::Mouse(_))) => {
            let watch = detail::LandingWatch::of(next);
            let arrival = banner::ArrivalWatch::of(next);
            core_event(next, commands, event);
            watch.drop_if_moved(next);
            arrival.drop_if_moved(next);
        }
        AppEvent::Core(event) => core_event(next, commands, event),
        // Nothing to answer: the loop sends it with `defer_follow` off, so
        // `settle` runs the owed follow (G-settle).
        // A clone: only the empty state's own pool runs one (m8/18b); a
        // repository screen never submits one, so never hears of one.
        AppEvent::Rested | AppEvent::Produced(_, Product::Cloned(_)) => {}
        AppEvent::Started(id, task) => started(next, commands, id, &task),
        AppEvent::PixelsFailed(why) => super::keys::fall_back_from_pixels(next, commands, why),
        AppEvent::GraphModeKept { mode, result } => {
            palette::graph_mode_kept(next, mode, result);
        }
        // The rewrite editor's jobs, by id, before the handlers that share
        // their products: the preview's, the detail pane's and the stash
        // list's `Files`, the commit editor's `$EDITOR` (m7-power-tools/08).
        AppEvent::Produced(id, product) if rewrite_editor::owns(next, id) => {
            rewrite_answer(next, commands, id, product);
        }
        // An open read nobody waits for any more: `opened` drops it.
        AppEvent::Produced(id, product @ Product::RewriteOpened(_)) => {
            rewrite_answer(next, commands, id, product);
        }
        AppEvent::Produced(id, product @ Product::Stashes(_)) => stash_answer(next, id, &product),
        AppEvent::Produced(id, Product::SwitchList(result)) => picker::listed(next, id, result),
        AppEvent::Produced(id, Product::PaletteBranches(result)) => {
            palette::branches_read(next, commands, id, result);
        }
        AppEvent::Produced(id, Product::PaletteNames(hits)) => {
            palette::names_answered(next, id, hits);
        }
        AppEvent::Produced(id, Product::TrackedPaths(result)) => {
            palette::tracked_read(next, commands, id, result);
        }
        AppEvent::Produced(id, Product::PaletteCommits(result)) => {
            palette::commits_answered(next, id, result);
        }
        // The overlay's file preview reads through `ListFiles` like the
        // detail pane does, under its own id; the guard asks which of the
        // two is waiting for this one.
        AppEvent::Produced(id, product @ Product::Files(_))
            if stash::claims(next, id, &product) =>
        {
            stash_answer(next, id, &product);
        }
        // The commit screen reads the diff of the file under its cursor
        // through `ReadFileDiff` too, under its own id (ease-pass/08).
        AppEvent::Produced(id, Product::FileDiff(_, result))
            if commit_editor::claims_diff(next, id) =>
        {
            commit_editor::diff_arrived(next, id, result.map(|(view, _)| view));
        }
        AppEvent::Produced(id, Product::CommitFiles(result)) => {
            commit_editor::commit_files_arrived(next, id, result);
        }
        // The detail pane names its jobs itself, before they start.
        event @ AppEvent::Produced(
            _,
            Product::Files(_)
            | Product::FileDiff(..)
            | Product::ContainedIn(..)
            | Product::EvilProbe(..),
        ) => detail::detail_event(next, commands, event),
        AppEvent::Produced(id, Product::Blame(answer)) => {
            place::blame_arrived(next, commands, id, &answer);
        }
        AppEvent::Produced(id, Product::Trace(answer)) => {
            place::trace_arrived(next, commands, id, answer);
        }
        AppEvent::Produced(id, Product::WholeFile(answer)) => {
            place::whole_file_arrived(next, id, answer);
        }
        event @ AppEvent::Produced(
            _,
            Product::RadarContext(_) | Product::ForcePushProbe(..) | Product::DuplicatesProbe(_),
        ) => radar::radar_event(next, event),
        AppEvent::Produced(id, Product::StashOnly(answer)) => {
            stash_only_answered(next, id, answer);
        }
        AppEvent::Produced(
            id,
            product @ (Product::RewritePlanned(_) | Product::RowTagsResolved(_)),
        ) => {
            amend_into::answer(next, commands, id, product);
        }
        AppEvent::Produced(id, Product::TopologicalOrder(sorted)) => {
            history::sorted(next, commands, id, sorted);
        }
        AppEvent::Produced(_, Product::Repository(result)) => {
            repository_answer(next, commands, result);
        }
        AppEvent::Produced(id, Product::IndexState(read)) => {
            let mut local_jobs = next.local_jobs;
            clash::index_answered(next, commands, id, read, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(
            id,
            product @ (Product::SystemGit(_) | Product::Refs(_) | Product::WorkingTree(_)),
        ) => took_fact(next, commands, id, product),
        AppEvent::Progress(id, Progress::History(update)) => {
            history::walk_progress(next, commands, id, update);
        }
        AppEvent::Progress(id, Progress::Search(hits)) => {
            search::search_progress(next, commands, id, hits);
        }
        event @ (AppEvent::Produced(_, Product::Map(_))
        | AppEvent::Progress(_, Progress::Map { .. })) => map::map_event(next, event),
        event @ (AppEvent::Produced(_, Product::Operated(_))
        | AppEvent::Progress(
            _,
            Progress::Operating { .. } | Progress::CredentialNeeded { .. },
        )) => operate_answer(next, commands, event),
        AppEvent::Produced(id, Product::Resumed(outcome)) => {
            let mut local_jobs = next.local_jobs;
            commands.extend(banner::resumed(next, id, *outcome, &mut || {
                local_jobs.next_id()
            }));
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(_, Product::Mergetool { path, result }) => {
            mergetool_returned(next, commands, &path, result);
        }
        AppEvent::Produced(_, Product::ContinuedInGit(result)) => {
            banner::continued_in_git(next, commands, result);
        }
        AppEvent::Produced(id, Product::ConflictOpened(result)) => {
            resolver::conflict_opened(next, id, result);
        }
        AppEvent::Produced(id, Product::Taken(result)) => {
            let mut local_jobs = next.local_jobs;
            resolver::taken(next, id, result, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::Untaken(result)) => {
            let mut local_jobs = next.local_jobs;
            resolver::untaken(next, id, result, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::BaseLoaded(result)) => {
            resolver::base_loaded(next, id, result);
        }
        AppEvent::Produced(id, Product::ConflictSet(result)) => {
            resolver::conflict_set_read(next, id, result);
        }
        AppEvent::Produced(id, Product::FinishSummary(result)) => {
            finish::summary_landed(next, id, result);
        }
        AppEvent::Produced(id, Product::FinishLook(result)) => {
            finish::look_landed(next, id, result);
        }
        AppEvent::Produced(id, Product::Preview(result)) => {
            preview::preview_produced(next, id, result);
        }
        AppEvent::Produced(id, Product::RevertedTo(done)) => {
            reverted_to_answer(next, commands, id, *done);
        }
        AppEvent::Produced(id, Product::Undone(done)) => undone_answer(next, commands, id, *done),
        AppEvent::Produced(id, product @ Product::Journal(_)) => {
            journal_answer(next, commands, id, &product);
        }
        AppEvent::Produced(id, Product::Outside(result)) => {
            let mut local_jobs = next.local_jobs;
            journal::outside_landed(next, commands, id, &result, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::LostFound(result)) => {
            journal::lost_landed(next, id, &result);
        }
        AppEvent::Produced(id, Product::TourProgress(done)) => tour::answered(next, id, done),
        AppEvent::Produced(id, Product::RecoverFor(result)) => {
            let mut local_jobs = next.local_jobs;
            journal::recover_answered(next, commands, id, &result, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        AppEvent::Produced(id, Product::Remotes(result)) => {
            remote::remotes_arrived(next, id, result);
        }
        AppEvent::Produced(
            _,
            Product::Leaves {
                sent,
                expected,
                result,
            },
        ) => remote::leaves_arrived(next, sent, expected, result),
        AppEvent::Produced(_, Product::FoldedCounted { counts }) => {
            fold_count::counted(next, counts);
        }
        AppEvent::Produced(_, Product::BranchStatusRead { key, result }) => {
            super::branch_pane::read(next, key, result);
        }
        AppEvent::Progress(id, Progress::RemoteProgress { brief }) => {
            remote::remote_progress(next, id, &brief);
        }
        // `e` from inside the resolver: `$EDITOR` had the file, so the
        // document on screen is not what is on disk any more. Before the
        // commit editor's arm, which is where every other `HandedOver`
        // goes — the resolver is modal, so while it is open there is no
        // other handover this can be (`Ctrl+Z` is swallowed by
        // `resolver_key`, which runs before `suspend_on_ctrl_z`).
        AppEvent::Produced(_, Product::HandedOver(result)) if next.resolver.is_some() => {
            match result {
                Ok(()) => {
                    if let Some(path) = next.resolver.as_ref().map(|open| open.path.clone()) {
                        finish::record_edited(next, &path);
                    }
                }
                Err(why) => next.status = Status::sticky_failure(why),
            }
            let mut local_jobs = next.local_jobs;
            resolver::handed_back(next, commands, &mut || local_jobs.next_id());
            next.local_jobs = local_jobs;
        }
        // The banner's (or changes pane's) `e` on a conflicted file came
        // back: a clean return marks it edited for the finish summary, and
        // the rest is the commit editor arm's, as before.
        AppEvent::Produced(id, Product::HandedOver(result)) if next.conflict_editing.is_some() => {
            let path = next.conflict_editing.take();
            if let (Ok(()), Some(path)) = (&result, path) {
                finish::record_edited(next, &path);
            }
            commit_editor_answer(
                next,
                commands,
                AppEvent::Produced(id, Product::HandedOver(result)),
            );
        }
        event @ AppEvent::Produced(
            _,
            Product::CommitTemplate(_)
            | Product::HeadMessage(_)
            | Product::ExternalEditor(_)
            | Product::HandedOver(_),
        ) => commit_editor_answer(next, commands, event),
    }
    // The banner's `⏎` back to the first open clash hands on once the
    // reads it waits on have landed — whichever lands last (Q7 a).
    let mut local_jobs = next.local_jobs;
    resolver::seek_first_open(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
}

//! `Dispatch for JobRunner`: one exhaustive arm per `Command`.

use super::JobRunner;
use crate::jobs::kinds::check_system_git;
use crate::jobs::types::{Dispatch, JobEvents};
use crate::update::{Command, RestRead, Task};

/// The detail pane's reads, which `dispatch` hands to `dispatch_detail`: a
/// pattern, so `dispatch` stays one exhaustive match (review m7-16b..21b,
/// Major 2) — a new `Command` with no arm is a build failure, never a panic
/// on the event loop.
macro_rules! detail_read {
    () => {
        Command::ListFiles(..)
            | Command::ReadFileDiff(..)
            | Command::ReadBlame(..)
            | Command::TraceHistory { .. }
            | Command::ReadWholeFile { .. }
            | Command::ContainedIn(..)
            | Command::RadarContext(..)
            | Command::EvilProbe(..)
            | Command::ForcePushProbe(..)
            | Command::DuplicatesProbe(..)
    };
}

impl<S: JobEvents> Dispatch for JobRunner<S> {
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per `Command` variant, exhaustive with no `_` arm so a new command \
                  stops this compiling until it is routed; `RecoverFor`'s fourth field (m8/10e) \
                  is what rustfmt spreads over five lines"
    )]
    fn dispatch(&self, command: Command) {
        match command {
            Command::OpenRepository => self.open_repository(),
            Command::CheckSystemGit => {
                self.announce(Task::CheckSystemGit, check_system_git());
            }
            Command::ReadRefs(id) => self.read_refs(id),
            Command::ReadWorkingTree(id) => self.read_working_tree(id),
            Command::PromoteHistory => self.promote_history(),
            Command::WriteCommitGraph => self.write_commit_graph(),
            Command::DeclineCommitGraph => self.decline_commit_graph(),
            Command::ReadHistory(id, order) => self.read_history(id, order),
            Command::ShowRows(id, request) => self.show_rows(id, request),
            Command::Search(query) => self.start_search(query),
            command @ (detail_read!()) => self.dispatch_detail(command),
            Command::ReadStashOnly(id, stashes) => self.read_stash_only(id, stashes),
            Command::BuildMap(id, req) => self.build_map(id, req),
            Command::ReadCommitTemplate(id) => self.read_commit_template(id),
            Command::ReadHeadMessage(id) => self.read_head_message(id),
            Command::ReadCommitFiles(id, staged_parts) => self.read_commit_files(id, staged_parts),
            Command::PlanRewrite { id, request, head } => self.plan_rewrite(id, request, head),
            Command::OpenRewrite { id, req } => self.open_rewrite(id, req),
            Command::ResolveRowTags(id, offsets) => self.resolve_row_tags(id, offsets),
            Command::Operate(id, op, keys) => self.operate(id, op, keys),
            Command::Resume(id, what, keys) => self.resume(id, what, keys),
            Command::Undo(id, force, keys) => self.run_undo(id, false, force, keys),
            Command::Redo(id, force, keys) => self.run_undo(id, true, force, keys),
            Command::ReadJournal(id) => self.read_journal(id),
            Command::TourProgress(id) => self.tour_progress(id),
            Command::ReadOutside { id } => self.read_outside(id),
            Command::FindLost { id, outside } => self.find_lost(id, outside),
            Command::RecoverFor {
                id,
                list,
                row,
                keys,
            } => self.recover_for(id, list, row, keys),
            Command::PlanPreview(id, request) => self.plan_preview(id, request),
            Command::RevertTo(id, to, force, keys) => self.run_revert_to(id, to, force, keys),
            Command::ReadStashes(id) => self.read_stashes(id),
            Command::ReadSwitchList(id) => self.read_switch_list(id),
            Command::ReadPaletteBranches(id) => self.read_palette_branches(id),
            Command::PaletteNames(id, request) => self.palette_names(id, request),
            Command::ReadTrackedPaths(id) => self.read_tracked_paths(id),
            Command::PaletteCommits(id, query) => self.palette_commits(id, query),
            Command::ReadIndexState { id } => self.read_index_state(id),
            Command::OpenConflict(id, path) => self.open_conflict(id, path),
            Command::TakeSide(id, request) => self.take_side(id, request),
            Command::UntakeSide(id, request) => self.untake_side(id, request),
            Command::LoadBase(id, path, file) => self.load_base(id, path, *file),
            Command::ReadConflictSet(id) => self.read_conflict_set(id),
            Command::ReadFinishSummary(id) => self.read_finish_summary(id),
            Command::ReadFinishLook(id, ask) => self.read_finish_look(id, ask),
            Command::ReadRemotes(id) => self.read_remotes(id),
            Command::ReadLeaves { id, sent, expected } => self.read_leaves(id, sent, expected),
            Command::AtRest(id, RestRead::CountFolded { merge }) => self.count_folded(id, merge),
            Command::AtRest(id, RestRead::BranchStatus(ask)) => self.read_branch_status(id, *ask),
            Command::CancelJob(id) => self.cancel(id),
            Command::Unstoppable(id, what) => self.unstoppable(id, what),
            Command::ForceStop(id) => self.force_stop(id),
            Command::AnswerCredential(id, answer) => self.credential(id, Some(answer)),
            Command::CancelCredential(id) => self.credential(id, None),
            // The loop writes this to the terminal itself (`event_loop`).
            Command::CopyToClipboard(_) => not_a_job("CopyToClipboard is terminal output"),
            // The loop writes this to disk itself (`event_loop::running`).
            Command::SaveLayout(_) => not_a_job("SaveLayout is written by the loop"),
            Command::FirstLaunchDone(_) => not_a_job("FirstLaunchDone is written by the loop"),
            Command::KeepGraphMode(_) => not_a_job("KeepGraphMode is written by the loop"),
            // The theme is the loop's (`Session`), not a worker's.
            Command::Retheme(_) => not_a_job("Retheme is the loop's theme"),
            // Handing the terminal over and taking it back is the loop's own
            // business (`event_loop::outside`): a worker thread has no claim
            // on the modes the loop drew through (m3/05).
            Command::OpenExternalEditor { .. }
            | Command::OpenMergetool { .. }
            | Command::ContinueInGit { .. }
            | Command::OpenConflictEditor { .. }
            | Command::EditCommitMessage { .. }
            | Command::Suspend => not_a_job("the terminal is handed over by the loop"),
        }
    }
}

/// A command the loop carries out itself reached the runner: a routing bug,
/// not a runtime condition. `dispatch` names each such variant, so the match
/// has no catch-all and a new `Command` must be given an arm to build.
fn not_a_job(why: &str) -> ! {
    unreachable!("{why}, never a job")
}

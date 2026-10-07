//! Which commands change the repository's files or history: any of them
//! ends the trail, because the lines it points at may have moved.

use crate::update::events::Command;

/// True for a command that launches an operation, or hands the terminal to
/// a tool that edits files. Exhaustive on purpose, no `_` arm: a new
/// `Command` stops this compiling until it is classified.
#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one line per `Command` variant, exhaustive with no `_` arm on purpose (above); \
              shortening it means a catch-all that would classify a new write as a read"
)]
pub fn is_write(command: &Command) -> bool {
    match command {
        Command::Operate(..)
        | Command::Resume(..)
        | Command::RevertTo(..)
        | Command::Undo(..)
        | Command::Redo(..)
        | Command::TakeSide(..)
        | Command::OpenExternalEditor { .. }
        | Command::OpenConflictEditor { .. }
        | Command::OpenMergetool { .. }
        | Command::ContinueInGit { .. } => true,
        // Reads, the loop's own business, and answers to a write already
        // running. `WriteCommitGraph` writes git's lookup cache, never a
        // file's lines; `EditCommitMessage` writes Graft's own message file
        // and the commit it feeds is an `Operate`.
        Command::OpenRepository
        | Command::CheckSystemGit
        | Command::ReadHistory(..)
        | Command::ReadRefs(..)
        | Command::ReadWorkingTree(..)
        | Command::PromoteHistory
        | Command::ShowRows(..)
        | Command::CancelJob(..)
        | Command::Unstoppable(..)
        | Command::ForceStop(..)
        | Command::AnswerCredential(..)
        | Command::CancelCredential(..)
        | Command::Search(..)
        | Command::CopyToClipboard(..)
        | Command::SaveLayout(..)
        | Command::WriteCommitGraph
        | Command::DeclineCommitGraph
        | Command::ListFiles(..)
        | Command::ReadFileDiff(..)
        | Command::ContainedIn(..)
        | Command::RadarContext(..)
        | Command::EvilProbe(..)
        | Command::ForcePushProbe(..)
        | Command::DuplicatesProbe(..)
        | Command::ReadStashOnly(..)
        | Command::BuildMap(..)
        | Command::PlanPreview(..)
        | Command::ReadJournal(..)
        | Command::ReadOutside { .. }
        | Command::FindLost { .. }
        | Command::RecoverFor { .. }
        | Command::ReadStashes(..)
        | Command::ReadSwitchList(..)
        | Command::ReadIndexState { .. }
        | Command::ReadRemotes(..)
        | Command::ReadLeaves { .. }
        | Command::ReadCommitTemplate(..)
        | Command::ReadHeadMessage(..)
        | Command::PlanRewrite { .. }
        | Command::OpenRewrite { .. }
        | Command::ResolveRowTags(..)
        | Command::EditCommitMessage { .. }
        | Command::Suspend
        | Command::OpenConflict(..)
        | Command::LoadBase(..)
        | Command::ReadConflictSet(..)
        | Command::ReadFinishSummary(..)
        | Command::ReadFinishDiff(..)
        | Command::ReadBlame(..)
        | Command::TraceHistory { .. }
        | Command::ReadWholeFile { .. } => false,
    }
}

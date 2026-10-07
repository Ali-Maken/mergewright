//! A short name for each wake, for the `frames` startup trace
//! (m7-power-tools/12): what kind of thing woke the loop, never its payload
//! (`Debug` of a `Refs` answer prints 50k refs).

use graft_core::event::Event;

use super::Wake;
use crate::history::HistoryUpdate;
use crate::update::{AppEvent, Product, Progress, Task};

impl Wake {
    /// The variant, and for an event the kind inside it:
    /// `Produced::Refs`, `Started::ReadHistory`, `Progress::History::Window`.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Wake::Event(event) => event_label(event),
            Wake::Shutdown(_) => "Shutdown",
            Wake::InputFailed(_) => "InputFailed",
            Wake::Terminal(_) => "Terminal",
            Wake::TerminalFailed(_) => "TerminalFailed",
            Wake::Fence => "Fence",
        }
    }
}

fn event_label(event: &AppEvent) -> &'static str {
    match event {
        AppEvent::Core(event) => core_label(event),
        AppEvent::Started(_, task) => task_label(task),
        AppEvent::Produced(_, product) => product_label(product),
        AppEvent::Progress(_, progress) => progress_label(progress),
        AppEvent::Rested => "Rested",
    }
}

fn core_label(event: &Event) -> &'static str {
    match event {
        Event::Key(_) => "Core::Key",
        Event::Mouse(_) => "Core::Mouse",
        Event::Resize { .. } => "Core::Resize",
        Event::Paste(_) => "Core::Paste",
        Event::FocusGained => "Core::FocusGained",
        Event::FocusLost => "Core::FocusLost",
        Event::JobFinished(_) => "Core::JobFinished",
        Event::JobFailed(..) => "Core::JobFailed",
        Event::JobCancelled(_) => "Core::JobCancelled",
    }
}

fn progress_label(progress: &Progress) -> &'static str {
    match progress {
        Progress::History(HistoryUpdate::Started(_)) => "Progress::History::Started",
        Progress::History(HistoryUpdate::Progress { .. }) => "Progress::History::Progress",
        Progress::History(HistoryUpdate::Window(_)) => "Progress::History::Window",
        Progress::History(HistoryUpdate::LaidOut { .. }) => "Progress::History::LaidOut",
        Progress::Search(_) => "Progress::Search",
        Progress::Map { .. } => "Progress::Map",
        Progress::Operating { .. } => "Progress::Operating",
        Progress::RemoteProgress { .. } => "Progress::RemoteProgress",
        Progress::CredentialNeeded { .. } => "Progress::CredentialNeeded",
    }
}

fn task_label(task: &Task) -> &'static str {
    match task {
        Task::OpenRepository => "Started::OpenRepository",
        Task::CheckSystemGit => "Started::CheckSystemGit",
        Task::ReadHistory => "Started::ReadHistory",
        Task::ReadRefs => "Started::ReadRefs",
        Task::ReadWorkingTree => "Started::ReadWorkingTree",
        Task::Resume => "Started::Resume",
        Task::ReadCommitTemplate => "Started::ReadCommitTemplate",
        Task::ReadHeadMessage => "Started::ReadHeadMessage",
        Task::PlanRewrite => "Started::PlanRewrite",
        Task::OpenRewrite => "Started::OpenRewrite",
        Task::ResolveRowTags => "Started::ResolveRowTags",
        Task::PromoteHistory => "Started::PromoteHistory",
        Task::Search => "Started::Search",
        Task::WriteCommitGraph => "Started::WriteCommitGraph",
        Task::DeclineCommitGraph => "Started::DeclineCommitGraph",
        Task::ListFiles { .. } => "Started::ListFiles",
        Task::FileDiff { .. } => "Started::FileDiff",
        Task::Blame => "Started::Blame",
        Task::Trace => "Started::Trace",
        Task::WholeFile => "Started::WholeFile",
        Task::ContainedIn { .. } => "Started::ContainedIn",
        Task::RadarContext { .. } => "Started::RadarContext",
        Task::EvilProbe { .. } => "Started::EvilProbe",
        Task::ForcePushProbe { .. } => "Started::ForcePushProbe",
        Task::DuplicatesProbe { .. } => "Started::DuplicatesProbe",
        Task::StashOnly => "Started::StashOnly",
        Task::Map => "Started::Map",
        Task::Operate => "Started::Operate",
        Task::Undo => "Started::Undo",
        Task::ReadJournal => "Started::ReadJournal",
        Task::ReadOutside => "Started::ReadOutside",
        Task::FindLost => "Started::FindLost",
        Task::RecoverFor => "Started::RecoverFor",
        Task::PlanPreview => "Started::PlanPreview",
        Task::RevertTo => "Started::RevertTo",
        Task::ReadStashes => "Started::ReadStashes",
        Task::ReadSwitchList => "Started::ReadSwitchList",
        Task::ReadPaletteBranches => "Started::ReadPaletteBranches",
        Task::PaletteNames => "Started::PaletteNames",
        Task::ReadTrackedPaths => "Started::ReadTrackedPaths",
        Task::PaletteCommits => "Started::PaletteCommits",
        Task::ReadIndexState => "Started::ReadIndexState",
        Task::ReadRemotes => "Started::ReadRemotes",
        Task::OpenConflict => "Started::OpenConflict",
        Task::TakeSide => "Started::TakeSide",
        Task::LoadBase => "Started::LoadBase",
        Task::ReadConflictSet => "Started::ReadConflictSet",
        Task::ReadFinishSummary => "Started::ReadFinishSummary",
        Task::ReadFinishDiff => "Started::ReadFinishDiff",
    }
}

fn product_label(product: &Product) -> &'static str {
    match product {
        Product::Repository(_) => "Produced::Repository",
        Product::SystemGit(_) => "Produced::SystemGit",
        Product::Refs(_) => "Produced::Refs",
        Product::WorkingTree(_) => "Produced::WorkingTree",
        Product::TopologicalOrder(_) => "Produced::TopologicalOrder",
        Product::CommitGraph(_) => "Produced::CommitGraph",
        Product::CommitGraphDeclined(_) => "Produced::CommitGraphDeclined",
        Product::Files(_) => "Produced::Files",
        Product::FileDiff(..) => "Produced::FileDiff",
        Product::Blame(_) => "Produced::Blame",
        Product::Trace(_) => "Produced::Trace",
        Product::WholeFile(_) => "Produced::WholeFile",
        Product::ContainedIn(..) => "Produced::ContainedIn",
        Product::RadarContext(_) => "Produced::RadarContext",
        Product::EvilProbe(..) => "Produced::EvilProbe",
        Product::ForcePushProbe(..) => "Produced::ForcePushProbe",
        Product::DuplicatesProbe(_) => "Produced::DuplicatesProbe",
        Product::StashOnly(_) => "Produced::StashOnly",
        Product::Map(_) => "Produced::Map",
        Product::CommitTemplate(_) => "Produced::CommitTemplate",
        Product::HeadMessage(_) => "Produced::HeadMessage",
        Product::RewritePlanned(_) => "Produced::RewritePlanned",
        Product::RowTagsResolved(_) => "Produced::RowTagsResolved",
        Product::RewriteOpened(_) => "Produced::RewriteOpened",
        Product::ExternalEditor(_) => "Produced::ExternalEditor",
        Product::HandedOver(_) => "Produced::HandedOver",
        Product::Operated(_) => "Produced::Operated",
        Product::Resumed(_) => "Produced::Resumed",
        Product::Mergetool { .. } => "Produced::Mergetool",
        Product::ContinuedInGit(_) => "Produced::ContinuedInGit",
        Product::ConflictOpened(_) => "Produced::ConflictOpened",
        Product::Taken(_) => "Produced::Taken",
        Product::BaseLoaded(_) => "Produced::BaseLoaded",
        Product::ConflictSet(_) => "Produced::ConflictSet",
        Product::FinishSummary(_) => "Produced::FinishSummary",
        Product::FinishDiff(_) => "Produced::FinishDiff",
        Product::Preview(_) => "Produced::Preview",
        Product::RevertedTo(_) => "Produced::RevertedTo",
        Product::Undone(_) => "Produced::Undone",
        Product::Journal(_) => "Produced::Journal",
        Product::Outside(_) => "Produced::Outside",
        Product::LostFound(_) => "Produced::LostFound",
        Product::RecoverFor(_) => "Produced::RecoverFor",
        Product::Stashes(_) => "Produced::Stashes",
        Product::SwitchList(_) => "Produced::SwitchList",
        Product::PaletteBranches(_) => "Produced::PaletteBranches",
        Product::PaletteNames(_) => "Produced::PaletteNames",
        Product::TrackedPaths(_) => "Produced::TrackedPaths",
        Product::PaletteCommits(_) => "Produced::PaletteCommits",
        Product::IndexState(_) => "Produced::IndexState",
        Product::Remotes(_) => "Produced::Remotes",
        Product::Leaves { .. } => "Produced::Leaves",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::io;

    use graft_core::event::{KeyCode, KeyPress};
    use graft_core::job::JobId;

    use super::*;
    use crate::history::Window;

    /// Every wake names itself without its payload: the match above is
    /// exhaustive (a new variant does not compile without a label), and the
    /// labels of one of each kind are distinct and carry the kind inside.
    #[test]
    fn every_wake_has_a_label() {
        let id = JobId::new(1);
        let window = Window {
            request: 1,
            first: 0,
            rows: Vec::new().into(),
            laid_out: 0,
            done: false,
            promoted: false,
        };
        let wakes = [
            Wake::Event(AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(
                'j',
            ))))),
            Wake::Event(AppEvent::Started(id, Task::ReadHistory)),
            Wake::Event(AppEvent::Started(id, Task::ReadRefs)),
            Wake::Event(AppEvent::Produced(id, Product::Refs(Err("x".to_owned())))),
            Wake::Event(AppEvent::Produced(
                id,
                Product::Repository(Err("x".to_owned())),
            )),
            Wake::Event(AppEvent::Progress(
                id,
                Progress::History(HistoryUpdate::Window(window)),
            )),
            Wake::Event(AppEvent::Progress(
                id,
                Progress::History(HistoryUpdate::Progress {
                    request: 1,
                    laid_out: 0,
                }),
            )),
            Wake::Event(AppEvent::Progress(
                id,
                Progress::History(HistoryUpdate::LaidOut { rows: 0 }),
            )),
            Wake::Shutdown(15),
            Wake::InputFailed(io::Error::other("x")),
            Wake::TerminalFailed(io::Error::other("x")),
            Wake::Fence,
        ];
        let labels: Vec<&str> = wakes.iter().map(Wake::label).collect();
        assert_eq!(
            labels.iter().collect::<HashSet<_>>().len(),
            labels.len(),
            "one label per kind: {labels:?}"
        );
        assert_eq!(labels[1], "Started::ReadHistory");
        assert_eq!(labels[3], "Produced::Refs");
        assert_eq!(labels[5], "Progress::History::Window");
        assert!(labels.iter().all(|label| !label.is_empty()));
    }
}

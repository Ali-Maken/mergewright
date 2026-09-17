//! What flows through [`super::update`]: events in, commands out, and the
//! job-runner vocabulary that ties the two together.

use graft_core::event::Event;
use graft_core::job::JobId;

use crate::history::RowRequest;
use crate::repository::RepositoryFacts;
use crate::update::detail::PaneSize;
use graft_git::history::Order;
use graft_git::version::SystemGit;

/// Everything the loop can be told.
///
/// [`Event`] is what `core` can describe on its own — a key, a resize, a job
/// that ended. It cannot carry a repository: `core` does not know what one is,
/// and moving the git types down into it to make it possible would invert the
/// layering (ADR-0007). So `app` wraps it, and jobs that have something to
/// hand over hand it over here. Symmetric with [`Command`]: commands go out,
/// products come back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AppEvent {
    /// Something `core` can describe.
    Core(Event),
    /// A job has been queued, and this is the id its result will carry.
    ///
    /// Announced before the work is queued, so the state knows which id it is
    /// waiting for before any result can arrive — otherwise `Esc` has nothing
    /// to cancel until the job it would cancel has already finished. The
    /// runner sends it down the same channel as the result, so it is always
    /// the earlier of the two.
    Started(JobId, Task),
    /// A job that ended with something the application keeps.
    Produced(JobId, Product),
    /// A running job has something to show before it ends. Always between
    /// the job's [`AppEvent::Started`] and its one ending event.
    Progress(JobId, Progress),
}

/// What a running job hands over before it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The history walk has started, or laid out its next rows.
    History(crate::history::HistoryUpdate),
    /// A search read another chunk of history and found these commits.
    Search(std::sync::Arc<[graft_core::oid::Oid]>),
}

/// What a job was started to do.
///
/// Only enough to tell the one job whose result the screen waits for from the
/// ones that simply land when they land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Task {
    /// Open the repository and read what the status line shows.
    OpenRepository,
    /// Ask this machine's `git` what version it is.
    CheckSystemGit,
    /// Walk history.
    ReadHistory,
    /// Read the refs each commit's badges show.
    ReadRefs,
    /// Read the working tree's staged/unstaged/untracked counts.
    ReadWorkingTree,
    /// Work out the strict topological order a provisional walk promotes to.
    PromoteHistory,
    /// Search history for a query (`/`).
    Search,
    /// Build the commit-graph with `git commit-graph write` (m1-the-graph/54).
    WriteCommitGraph,
    /// Remember "never for this repository" for the commit-graph offer.
    DeclineCommitGraph,
    /// The detail pane's changed-file list.
    ListFiles {
        /// What was listed.
        spec: graft_git::diff::DiffSpec,
    },
    /// One file's diff for the detail pane.
    FileDiff {
        /// Which diff.
        key: super::detail::DiffKey,
    },
    /// Which refs contain a commit.
    ContainedIn {
        /// The commit asked about.
        oid: graft_core::oid::Oid,
    },
    /// The Mistake Radar's default-branch context: its first-parent chain
    /// and previous tips (`update::radar`).
    RadarContext {
        /// The default branch's tip when this was asked for.
        default_tip: graft_core::oid::Oid,
        /// The default branch itself.
        default_ref: graft_core::ref_name::RefName,
    },
    /// Is the open merge's recorded resolution what merging its parents
    /// gives? (`jobs::radar::run_evil_probe`.)
    EvilProbe {
        /// The merge being checked.
        merge: graft_core::oid::Oid,
    },
}

/// What a job brought back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Product {
    /// The repository opened, or it did not. A path that is not a repository
    /// is an ordinary answer to an ordinary question, not a job failure —
    /// [`Event::JobFailed`] stays for jobs that broke.
    Repository(Result<RepositoryFacts, String>),
    /// What the `git` on this machine turned out to be.
    SystemGit(GitCheck),
    /// The refs on each commit and how far each branch is from its
    /// upstream, or why they could not be read.
    Refs(Result<crate::refs::RefBadges, String>),
    /// The working tree's staged/unstaged/untracked counts, or why they
    /// could not be read.
    WorkingTree(Result<graft_git::status::WorkingTreeCounts, String>),
    /// Every commit in strict topological order, for the walk on screen to
    /// promote to (ADR-0008).
    TopologicalOrder(std::sync::Arc<[graft_core::commit_handle::CommitHandle]>),
    /// The commit-graph write finished, or why it failed (the error's
    /// `Display`, git's own words in it, unsanitised).
    CommitGraph(Result<graft_git::commit_graph::Written, String>),
    /// The decline was remembered, or why it could not be.
    CommitGraphDeclined(Result<(), String>),
    /// The detail pane's file tree and the git list behind it, or why not.
    Files(
        Result<
            (
                std::sync::Arc<graft_ui::detail::FileListView>,
                std::sync::Arc<graft_git::diff::FileList>,
            ),
            String,
        >,
    ),
    /// One file's assembled diff, or why not.
    FileDiff(
        super::detail::DiffKey,
        Result<std::sync::Arc<graft_ui::detail::DiffView>, String>,
    ),
    /// The refs containing a commit, or why not.
    ContainedIn(
        graft_core::oid::Oid,
        Result<graft_git::contained::Contained, String>,
    ),
    /// The Mistake Radar's default-branch context, or why it could not be
    /// read.
    RadarContext(Result<std::sync::Arc<crate::jobs::radar::RadarContext>, String>),
    /// One merge's evil-merge verdict. Never a `Result`: "could not check"
    /// is one of the verdict's own three answers, not a failure.
    EvilProbe(
        graft_core::oid::Oid,
        graft_ops::mistake_radar::probes::EvilVerdict,
    ),
}

/// What running `git --version` found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitCheck {
    /// `git` ran and said which version it is.
    Ran(SystemGit),
    /// `git` could not be run at all, and this is why. Not fatal at M0 —
    /// nothing has shelled out to it yet — but the user needs to know now
    /// rather than at the first operation that needs it.
    Unavailable(String),
}

/// Work for the job runner: something [`super::update`] wants done that it
/// cannot do itself, because doing it would mean touching the world.
///
/// Plain data, with no closure and no handle in it, so that `update` stays a
/// function whose answer can be compared with `assert_eq!`. Turning a command
/// into the code that runs it is the job runner's business (`crate::jobs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the repository Graft was pointed at and check it is one.
    ///
    /// No path: which repository this run is about is settled before the loop
    /// starts and is the job runner's to hold (`crate::repos`), not something
    /// a keystroke gets to change. This is also what `Ctrl+R` (manual
    /// refresh, DESIGN.md §2.1) re-runs, and it is where anything cached
    /// about the repository is thrown away.
    OpenRepository,
    /// Find out whether the `git` on this machine is new enough (2.38,
    /// DESIGN.md §3.1). Runs once, at startup: it shells out, so it is a job.
    CheckSystemGit,
    /// Walk history in this order and bring the commits back.
    ///
    /// The order asked for is not always the order that comes back: without a
    /// commit-graph file a topological walk falls to commit date, and says so
    /// (ADR-0008).
    ReadHistory(Order),
    /// Read the refs for the badges, counting ahead/behind off the loop's
    /// thread.
    ReadRefs,
    /// Read the working tree's staged/unstaged/untracked counts off the
    /// loop's thread.
    ReadWorkingTree,
    /// Work out the whole range's strict topological order, for a walk whose
    /// order is provisional (ADR-0008). Only the newest such job's answer is
    /// used.
    PromoteHistory,
    /// Hand history job `JobId` a request for rows. The rows come back as a
    /// [`crate::history::HistoryUpdate::Window`] tagged with the request's
    /// id.
    ShowRows(JobId, RowRequest),
    /// Stop a job whose result is no longer wanted. Cooperative: a job that has
    /// not started is dropped, and a running one stops at its next check.
    CancelJob(JobId),
    /// Search history for a query off the loop's thread (`/`).
    Search(crate::search::Query),
    /// Ask the terminal to put this text on its clipboard (OSC 52, `y`).
    /// Written by the loop itself, to the terminal, through the session's
    /// transport: it is output, not a job, and never reaches the job runner.
    CopyToClipboard(String),
    /// Persist the detail pane's split (ADR-0028: `<`/`>` or a divider drag
    /// changed it). Written by the loop itself, to `<common_dir>/graft/
    /// layout` (`crate::layout::save_split`); a failure is expected and
    /// non-fatal (see that module's doc), never dispatched to jobs.
    SaveLayout(PaneSize),
    /// Run `git commit-graph write --reachable` as a job (`B` on the offer).
    /// Never cancelled once git starts (`graft_git::commit_graph`).
    WriteCommitGraph,
    /// Remember the user declined the commit-graph offer (`X`).
    DeclineCommitGraph,
    /// List the files a diff touches, under an id `update` chose.
    ListFiles(
        JobId,
        graft_git::diff::DiffSpec,
        graft_git::diff::DiffOptions,
    ),
    /// Read and assemble one file's diff, under an id `update` chose.
    ReadFileDiff(JobId, super::detail::DiffKey, graft_git::diff::FileEntry),
    /// Ask which refs contain a commit, under an id `update` chose.
    ContainedIn(JobId, graft_core::oid::Oid),
    /// Read the Mistake Radar's default-branch context, under an id
    /// `update` chose: the default branch's tip and the branch itself, as
    /// `update` last knew them.
    RadarContext(JobId, graft_core::oid::Oid, graft_core::ref_name::RefName),
    /// Probe one merge for an evil resolution, under an id `update` chose.
    EvilProbe(JobId, graft_core::oid::Oid),
}

/// Where the offer to build a missing commit-graph file is (m1-the-graph/54).
///
/// The job ids are `None` from the key press until the runner announces the
/// job: `B` and `X` must leave `Offered` at once, so a second press does not
/// start a second job, and the id is not known until `Started`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommitGraphOffer {
    /// Nothing to offer: a commit-graph is there, the user declined before,
    /// the facts are not in yet, or a written line was cleared.
    #[default]
    NotNeeded,
    /// No commit-graph and no decline: `B` builds, `X` declines. The line
    /// says there is no git to build with when the git check did not pass.
    Offered,
    /// `git commit-graph write` is running as this job.
    Writing(Option<JobId>),
    /// The write finished; cleared by the next key.
    Written {
        /// Commits the new file holds.
        commits: u32,
    },
    /// The decline is being written as this job.
    Declining(Option<JobId>),
    /// Declined and remembered; no line.
    Declined,
    /// The write failed; the problem line says why. No retry this run.
    Failed,
    /// A harness asked for no offer (`Harness::offer_commit_graph`).
    Suppressed,
}

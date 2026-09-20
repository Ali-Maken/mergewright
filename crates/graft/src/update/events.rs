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
    /// The Map view finished another row's walk (`crate::map`).
    Map {
        /// Rows finished so far.
        done: u32,
        /// Rows the view is building in total.
        total: u32,
    },
    /// A working-tree write started running, described in the words the
    /// journal and the status line both use (`Operation::describe`).
    Operating {
        /// What is running, e.g. `"stage src/a.rs"`.
        description: String,
    },
    /// A fetch, pull or push reported how far it has got: git's own
    /// progress line, shortened (`RemoteProgress::brief`, m4-remotes/05).
    RemoteProgress {
        /// e.g. `"receiving objects 45%"`.
        brief: String,
    },
    /// A fetch, pull or push is waiting on git's credential question
    /// (m4-remotes/06): open the prompt; the job blocks until
    /// [`Command::AnswerCredential`] or [`Command::CancelCredential`] under
    /// the same id, or until it is cancelled.
    CredentialNeeded {
        /// git's own words, raw — remote-reachable, sanitised before it is
        /// kept (`update::credential`).
        prompt: String,
        /// Where the masked field's pulse starts, drawn fresh at random for
        /// this prompt by `jobs::askpass::pulse_seed`. It is carried on the
        /// event rather than drawn in `update` so `update` stays pure and
        /// its tests can pin it (ADR-0038 §2; see
        /// `graft_ui::text_input::SecretField::pulse`).
        pulse_seed: bool,
    },
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
    /// Read `commit.template`'s file, for the commit editor's empty body
    /// (m3-working-tree-undo-and-stash/05).
    ReadCommitTemplate,
    /// Read `HEAD`'s commit message, to prefill an amend
    /// (m3-working-tree-undo-and-stash/05).
    ReadHeadMessage,
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
    /// Did the remote this branch tracks rewrite its history?
    /// (`jobs::radar::run_force_push_probe`.)
    ForcePushProbe {
        /// The local branch, short (`main`).
        branch: String,
        /// The remote-tracking ref's tip as `update` last knew it.
        current: graft_core::oid::Oid,
    },
    /// The Mistake Radar's duplicate-commit probe: patch-ids over the
    /// newest commits (`jobs::radar::run_duplicates_probe`).
    DuplicatesProbe {
        /// How many commits back the probe looked.
        window: u32,
    },
    /// The Map view's per-branch bucket counts (`crate::map`, `M`).
    Map,
    /// One working-tree write (stage, unstage, discard, …), under an id
    /// `update` chose (m3-working-tree-undo-and-stash/04).
    Operate,
    /// `u`/`U`: take back the newest operation still in force, or put back
    /// the newest undo (m3-working-tree-undo-and-stash/06). A write like any
    /// other — it takes `AppState::operating` and journals its own entry.
    Undo,
    /// Read the undo journal, closing anything a previous run left open
    /// (m3-working-tree-undo-and-stash/06).
    ReadJournal,
    /// Read the stash list for the `S` overlay
    /// (m3-working-tree-undo-and-stash/06).
    ReadStashes,
    /// Read the configured remotes and `HEAD`'s upstream (m4-remotes/05).
    ReadRemotes,
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
    /// The working tree's full status (staged/unstaged/untracked, one entry
    /// per changed path), or why it could not be read. `Arc`: shared with
    /// the changes pane's file list, never copied per event
    /// (m3-working-tree-undo-and-stash/04, mirroring `Product::Refs`).
    WorkingTree(Result<crate::working_tree::WorkingTree, String>),
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
    /// One file's diff: the drawable view model and the raw diff git
    /// parsed, or why not. The raw one is what `hunk_patch` builds a
    /// stage/discard patch from (m3-working-tree-undo-and-stash/04) — the
    /// view model has been sanitised and cannot be turned back into bytes
    /// git would accept.
    FileDiff(
        super::detail::DiffKey,
        Result<
            (
                std::sync::Arc<graft_ui::detail::DiffView>,
                std::sync::Arc<graft_git::diff::FileDiff>,
            ),
            String,
        >,
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
    /// One branch's force-push verdict, by branch name. Never a `Result`,
    /// for the same reason [`Product::EvilProbe`] is not: "could not check"
    /// is one of the verdict's own answers.
    ForcePushProbe(String, graft_ops::mistake_radar::probes::ForcePushVerdict),
    /// The duplicate-commit probe's groups, or why it could not run.
    DuplicatesProbe(Result<std::sync::Arc<graft_ops::mistake_radar::probes::Duplicates>, String>),
    /// The Map view's per-branch bucket counts, or why it could not be built
    /// (`crate::map`). A `Stop` firing mid-walk is one more line here, the
    /// same as any other failure — not folded into a silent cancellation.
    Map(Result<std::sync::Arc<crate::map::MapData>, String>),
    /// `commit.template`'s contents, or `None` where the config names no
    /// template (m3-working-tree-undo-and-stash/05). A template that is
    /// configured but unreadable is `None` with the reason on the status
    /// line — the editor still opens, with an empty body.
    CommitTemplate(Result<Option<String>, String>),
    /// `HEAD`'s commit message, for an amend's prefill, or why it could not
    /// be read — an unborn `HEAD` among the reasons
    /// (m3-working-tree-undo-and-stash/05).
    HeadMessage(Result<String, String>),
    /// What `$EDITOR` left in the message file, or why Graft could not use
    /// it: the editor would not start, exited non-zero, or the file could
    /// not be read back (m3-working-tree-undo-and-stash/05).
    ExternalEditor(Result<String, String>),
    /// The terminal was handed to another program and came back, with
    /// nothing to bring back but whether it worked: `e`'s file in `$EDITOR`,
    /// and `Ctrl+Z`'s suspend (m3-working-tree-undo-and-stash/05).
    HandedOver(Result<(), String>),
    /// One working-tree write finished. Not a plain `Result`: see
    /// [`crate::jobs::operate::Operated`]'s doc for why an operation that
    /// ran but left part of its after-state unread still carries the
    /// [`graft_ops::apply::Applied`] the caller refreshes from.
    Operated(Box<crate::jobs::operate::Operated>),
    /// One undo or redo finished. Not a plain `Result` for the same reason
    /// [`Product::Operated`] is not: a restore that ran but could not read
    /// part of what it left behind still carries the
    /// [`graft_ops::apply::Applied`] the caller refreshes from, and "nothing
    /// to undo" and "the repository moved" are ordinary answers rather than
    /// failures (m3-working-tree-undo-and-stash/06).
    Undone(Box<crate::jobs::undo::Undone>),
    /// The undo journal, newest entry last, or why it could not be read.
    /// `Arc`: the journal browser holds it across frames, never copied per
    /// event (m3-working-tree-undo-and-stash/06).
    Journal(Result<std::sync::Arc<crate::jobs::undo::JournalRead>, String>),
    /// The stash list, `stash@{0}` first, or why it could not be read.
    /// `Arc` for the same reason as [`Product::Journal`]: the overlay holds
    /// it across frames (m3-working-tree-undo-and-stash/06).
    Stashes(Result<std::sync::Arc<Vec<graft_git::stash::StashEntry>>, String>),
    /// The remotes, `HEAD`'s branch and its upstream, or why they could not
    /// be read (m4-remotes/05).
    Remotes(Result<std::sync::Arc<crate::jobs::remote::RemoteFacts>, String>),
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
    /// Hand the user's answer to the credential question job `JobId` is
    /// waiting on (m4-remotes/06). Dropped, zeroed, if the job has ended.
    AnswerCredential(JobId, super::credential::Answer),
    /// Tell job `JobId`'s credential question that the user cancelled it.
    /// `update` always sends [`Command::CancelJob`] beside it: an operation
    /// is never left half-authenticated.
    CancelCredential(JobId),
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
    /// Probe one branch's upstream for a rewrite, under an id `update`
    /// chose: the branch's short name and the tip it stood at. The
    /// remote-tracking ref itself is resolved by the job, from config
    /// `update` cannot read.
    ForcePushProbe(JobId, String, graft_core::oid::Oid),
    /// Probe the newest `window` commits for shared patch-ids, under an id
    /// `update` chose.
    DuplicatesProbe(JobId, u32),
    /// Build the Map view's per-branch bucket counts, under an id `update`
    /// chose (`M`, `update::map`).
    BuildMap(JobId, crate::map::MapRequest),
    /// Run one working-tree write, under an id `update` chose: `ops` decides
    /// what it does (checks, plans, journals, snapshots); `app` only wires
    /// the key to the call (the seam rule, `crates/graft/tests/it/
    /// seam_rule.rs`, m3-working-tree-undo-and-stash/04).
    Operate(JobId, Box<graft_ops::operation::Operation>),
    /// Take back the newest operation still in force (`u`), under an id
    /// `update` chose. `true` forces past the drift guard — the user has
    /// seen what moved and said go ahead (m3-working-tree-undo-and-stash/06).
    Undo(JobId, bool),
    /// Put back the newest undo still in force (`U`), otherwise exactly
    /// [`Command::Undo`] (m3-working-tree-undo-and-stash/06).
    Redo(JobId, bool),
    /// Read the undo journal, under an id `update` chose. The job closes
    /// anything a previous run left `Pending` before it answers, so the
    /// browser never shows an entry that is waiting for a process that is
    /// gone (m3-working-tree-undo-and-stash/06).
    ReadJournal(JobId),
    /// Read the stash list, under an id `update` chose. Asked for when the
    /// `S` overlay opens and again after every write, because any of them
    /// may have moved `refs/stash` (m3-working-tree-undo-and-stash/06).
    ReadStashes(JobId),
    /// Read the remote facts `f`/`p`/`P` decide from, under an id `update`
    /// chose. Asked for wherever the refs are (m4-remotes/05).
    ReadRemotes(JobId),
    /// Read `commit.template`'s file for the commit editor's empty body,
    /// under an id `update` chose (m3-working-tree-undo-and-stash/05).
    ReadCommitTemplate(JobId),
    /// Read `HEAD`'s commit message to prefill an amend, under an id
    /// `update` chose (m3-working-tree-undo-and-stash/05).
    ReadHeadMessage(JobId),
    /// Hand the terminal to `$EDITOR` on this path, at this line, and take
    /// it back when the editor exits (m3-working-tree-undo-and-stash/05).
    ///
    /// Carried out by the frame loop itself, never by the job runner:
    /// restoring the terminal and re-entering it is the loop's own
    /// business, and a worker thread has no business touching the modes the
    /// loop drew through (`event_loop::outside`). `CopyToClipboard` and
    /// `SaveLayout` are the precedent.
    OpenExternalEditor {
        /// The file to open.
        path: std::path::PathBuf,
        /// The line to put the cursor on, where the editor takes one.
        line: Option<u32>,
    },
    /// Write this message to the repository's `graft/COMMIT_EDITMSG`, hand
    /// the terminal to `$EDITOR` on it, and bring back what was saved
    /// (`Ctrl+E`, m3-working-tree-undo-and-stash/05). The path is the
    /// loop's: `update` knows no repository directory.
    EditCommitMessage {
        /// The id the answering [`Product::ExternalEditor`] comes back under,
        /// so a message from an editor the user has since closed is dropped.
        id: JobId,
        /// The message as the fields have it now.
        message: String,
    },
    /// Suspend Graft the way `Ctrl+Z` does in any other terminal program
    /// (`keys.suspend_on_ctrl_z`, ADR-0002). Carried out by the frame loop,
    /// like `OpenExternalEditor`.
    Suspend,
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

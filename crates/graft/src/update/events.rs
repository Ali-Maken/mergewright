//! What flows through [`super::update`]: events in, commands out, and the
//! job-runner vocabulary that ties the two together.

use graft_core::event::Event;
use graft_core::job::JobId;

use crate::repository::RepositoryFacts;
use graft_git::version::SystemGit;

mod command;

pub use command::{Command, TakeAt, TakeRequest};

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
    /// Carry the live operation forward, skip its step, or abort it
    /// (m5-history-and-ghost-preview/09).
    Resume,
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
    /// Draw a Ghost Preview: plan what an operation would do, and read the
    /// commits it points at that are not on screen
    /// (m5-history-and-ghost-preview/08).
    PlanPreview,
    /// The journal's time machine: undo back past one entry
    /// (m5-history-and-ghost-preview/08).
    RevertTo,
    /// Read the stash list for the `S` overlay
    /// (m3-working-tree-undo-and-stash/06).
    ReadStashes,
    /// Read the configured remotes and `HEAD`'s upstream (m4-remotes/05).
    ReadRemotes,
    /// Read one conflicted path for the resolver (m6-conflicts/04).
    OpenConflict,
    /// Take one side of one conflict block and write the file back
    /// (m6-conflicts/04).
    TakeSide,
    /// Attach the merge base to every block of the open file (`B`,
    /// m6-conflicts/05).
    LoadBase,
    /// Read the set of files the operation in progress conflicted on, for
    /// the resolver's file list (m6-conflicts/13).
    ReadConflictSet,
    /// Read what the merge about to be committed changes, for the finish
    /// summary (m6-conflicts/19).
    ReadFinishSummary,
    /// Read one staged file's diff for the finish summary's `space`
    /// (m6-conflicts/19).
    ReadFinishDiff,
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
    /// Boxed: the read carries the incoming commit's summary
    /// (`InProgressDetail::theirs`), which would otherwise make every event
    /// as large as this one.
    WorkingTree(Result<Box<crate::working_tree::WorkingTree>, String>),
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
    /// A `--continue`/`--skip`/`--abort` finished
    /// (m5-history-and-ghost-preview/09). The same [`Operated`] shape as a
    /// write, because it is one, and its own product so that the banner —
    /// not `update::operate` — decides what the three outcomes mean for the
    /// operation the user is standing in.
    ///
    /// [`Operated`]: crate::jobs::operate::Operated
    Resumed(Box<crate::jobs::operate::Operated>),
    /// `git mergetool` was handed the terminal for one file and came back
    /// (m5-history-and-ghost-preview/09). `Err` is the tool refusing to
    /// start, exiting non-zero or being killed, in words — never swallowed,
    /// because a mergetool that did not run leaves the conflict exactly
    /// where it was and the user needs to know which of the two happened.
    ///
    /// The path rides along so the line can name the file, and so the
    /// working tree is re-read either way: a tool that failed at the end may
    /// still have written the file.
    Mergetool {
        /// The file the tool was opened on.
        path: std::path::PathBuf,
        /// Whether it ran, or why it did not.
        result: Result<(), String>,
    },
    /// One conflicted path, read for the resolver (m6-conflicts/04), or why
    /// it could not be opened — in `graft_ops`' own words
    /// ([`graft_ops::conflict::OpenError`]'s `Display`), never `{:?}`.
    ///
    /// `Arc`: the open resolver holds the parsed document across frames and
    /// the layout is built from it, so it is shared rather than copied per
    /// event (mirroring [`Product::Preview`]). The id it answers under
    /// rides on [`AppEvent::Produced`], like every other product's.
    ///
    /// The route — an unmerged document or card, or a done file read-only —
    /// is picked by the job from the status it reads (m6-conflicts/25).
    ConflictOpened(Result<crate::jobs::conflict::Opening, crate::jobs::conflict::OpenFailure>),
    /// One conflict block took a side and the file was written back
    /// (m6-conflicts/04): the document as it now stands, with its new
    /// fingerprint, or why nothing was written
    /// ([`graft_ops::conflict::TakeError`]'s `Display`).
    Taken(
        Result<
            std::sync::Arc<graft_git::conflict::ConflictFile>,
            crate::jobs::conflict::TakeFailure,
        >,
    ),
    /// The merge base was attached to every block of the open file, or it
    /// could not be (`B`, m6-conflicts/05): the document with every block's
    /// [`graft_git::conflict::Block::base`] filled in, and how many blocks
    /// got one, or why not — `graft_ops::conflict::BaseError`'s own words
    /// (`no base (added on both sides)`, `base unavailable after edits`, or
    /// git's own `diff3` failure), paired with whether they are a problem —
    /// only a failure is; no base and edited-past-git are facts.
    BaseLoaded(Result<(std::sync::Arc<graft_git::conflict::ConflictFile>, usize), (String, bool)>),
    /// The files the operation in progress conflicted on
    /// (`graft_ops::conflict::operation_conflicts`, m6-conflicts/13), or why
    /// they could not be read, in `graft_ops`' own words.
    ConflictSet(Result<std::sync::Arc<graft_ops::conflict::ConflictSet>, String>),
    /// What the merge about to be committed changes
    /// (`graft_ops::finish::finish_summary`, m6-conflicts/19), or why it
    /// could not be read, in `graft_ops`' own words.
    FinishSummary(Result<std::sync::Arc<graft_ops::finish::FinishSummary>, String>),
    /// One staged file's diff for the finish summary, or why not
    /// (m6-conflicts/19): the same view model the detail pane draws, built
    /// by the same code (`jobs::detail::assemble_diff`).
    FinishDiff(Result<std::sync::Arc<graft_ui::detail::DiffView>, String>),
    /// What a Ghost Preview's operation would do, or why no picture could be
    /// drawn (m5-history-and-ghost-preview/08). `Arc`: the open preview
    /// holds it across frames, and the slice is laid out from it once.
    Preview(Result<std::sync::Arc<super::preview::Planned>, String>),
    /// The journal's time machine finished, stopped part-way, or hit
    /// §6.7's drift rail (m5-history-and-ghost-preview/08). Not a plain
    /// `Result`, for [`Product::Undone`]'s reasons.
    RevertedTo(Box<crate::jobs::undo::RevertedTo>),
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

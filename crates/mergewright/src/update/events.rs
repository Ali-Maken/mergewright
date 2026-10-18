//! What flows through [`super::update`]: events in, commands out, and the
//! job-runner vocabulary that ties the two together.

use mergewright_core::event::Event;
use mergewright_core::job::JobId;

use crate::repository::RepositoryFacts;
use mergewright_git::version::SystemGit;

mod command;
mod commit_graph_offer;
mod take;

pub use command::Command;
pub use commit_graph_offer::CommitGraphOffer;
pub use take::{TakeAt, TakeRequest};

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
    /// The cursor has rested after a burst of moves (G-settle,
    /// m8-polish-and-ship/03): the follow `update::settle` deferred runs now,
    /// once, for the cursor's row. Only the loop sends it
    /// (`event_loop::rest`); nothing else happens.
    Rested,
}

/// What a running job hands over before it ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// The history walk has started, or laid out its next rows.
    History(crate::history::HistoryUpdate),
    /// A search read another chunk of history and found these commits.
    Search(std::sync::Arc<[mergewright_core::oid::Oid]>),
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
        /// `mergewright_ui::text_input::SecretField::pulse`).
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
    /// Plan `a` into an older commit, or absorb: the rewrite, its preview's
    /// words and its landing's (m7-power-tools/09).
    PlanRewrite,
    /// Read what the rewrite editor opens on (`R`, m7-power-tools/08).
    OpenRewrite,
    /// After a put-in landed: the commits it changed on the new line, for
    /// their row tags (m7-power-tools/09).
    ResolveRowTags,
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
        spec: mergewright_git::diff::DiffSpec,
    },
    /// One file's diff for the detail pane.
    FileDiff {
        /// Which diff.
        key: super::detail::DiffKey,
    },
    /// Which refs contain a commit.
    ContainedIn {
        /// The commit asked about.
        oid: mergewright_core::oid::Oid,
    },
    /// The Mistake Radar's default-branch context: its first-parent chain
    /// and previous tips (`update::radar`).
    RadarContext {
        /// The default branch's tip when this was asked for.
        default_tip: mergewright_core::oid::Oid,
        /// The default branch itself.
        default_ref: mergewright_core::ref_name::RefName,
    },
    /// Is the open merge's recorded resolution what merging its parents
    /// gives? (`jobs::radar::run_evil_probe`.)
    EvilProbe {
        /// The merge being checked.
        merge: mergewright_core::oid::Oid,
    },
    /// Did the remote this branch tracks rewrite its history?
    /// (`jobs::radar::run_force_push_probe`.)
    ForcePushProbe {
        /// The local branch, short (`main`).
        branch: String,
        /// The remote-tracking ref's tip as `update` last knew it.
        current: mergewright_core::oid::Oid,
    },
    /// The Mistake Radar's duplicate-commit probe: patch-ids over the
    /// newest commits (`jobs::radar::run_duplicates_probe`).
    DuplicatesProbe {
        /// How many commits back the probe looked.
        window: u32,
    },
    /// Which commits only a stash reaches (`jobs::stash_only`,
    /// m7-power-tools/16).
    StashOnly,
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
    /// J's stage 2, git's reflog records (m7-power-tools/24).
    ReadOutside,
    /// The practice tour's ticks, from the practice repository's journal
    /// (m8-polish-and-ship/19b).
    TourProgress,
    /// J's stage 3, the lost commits and edits (m7-power-tools/24).
    FindLost,
    /// What `⏎` on a J row puts back (m7-power-tools/24).
    RecoverFor,
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
    /// Read the branch picker's rows (m7-power-tools/10).
    ReadSwitchList,
    /// Read every branch for the palette's BRANCHES, once per `⌃P` open
    /// (m8-polish-and-ship/12).
    ReadPaletteBranches,
    /// Match the palette's query against branch and tag names
    /// (m8-polish-and-ship/12).
    PaletteNames,
    /// Read every tracked path for the palette's FILES, once per `⌃P` open
    /// (m8-polish-and-ship/12b).
    ReadTrackedPaths,
    /// The palette's first commits matching its query
    /// (m8-polish-and-ship/12b).
    PaletteCommits,
    /// Read the index now for a stash clash's tie (ADR-0045 §2).
    ReadIndexState,
    /// Read the configured remotes and `HEAD`'s upstream (m4-remotes/05).
    ReadRemotes,
    /// Read one conflicted path for the resolver (m6-conflicts/04).
    OpenConflict,
    /// Take one side of one conflict block and write the file back
    /// (m6-conflicts/04).
    TakeSide,
    /// Put the last pick back and write the file once (ease-pass 05).
    UntakeSide,
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
    /// Blame one file for a blame place (m7-power-tools/20).
    Blame,
    /// Trace a history place's commits (m7-power-tools/21).
    Trace,
    /// Read a history place's whole file at one commit (m7-power-tools/21).
    WholeFile,
    /// Read every change's line counts for the commit screen (ease-pass/08).
    ReadCommitFiles,
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
    /// promote to (ADR-0008), and how long the sort took.
    TopologicalOrder(SortedOrder),
    /// The commit-graph write finished, or why it failed (the error's
    /// `Display`, git's own words in it, unsanitised).
    CommitGraph(Result<mergewright_git::commit_graph::Written, String>),
    /// The decline was remembered, or why it could not be.
    CommitGraphDeclined(Result<(), String>),
    /// The detail pane's file tree and the git list behind it, or why not.
    Files(
        Result<
            (
                std::sync::Arc<mergewright_ui::detail::FileListView>,
                std::sync::Arc<mergewright_git::diff::FileList>,
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
                std::sync::Arc<mergewright_ui::detail::DiffView>,
                std::sync::Arc<mergewright_git::diff::FileDiff>,
            ),
            String,
        >,
    ),
    /// A blame place's blame, or why none (m7-power-tools/20).
    Blame(Result<std::sync::Arc<super::place::BlameAnswer>, super::place::BlameFailure>),
    /// A history place's trace, or why none (m7-power-tools/21).
    Trace(Result<std::sync::Arc<super::place::TraceAnswer>, super::place::TraceFailure>),
    /// A history place's whole file at one commit, or git's words.
    WholeFile(Result<WholeFileAnswer, String>),
    /// The refs containing a commit, or why not.
    ContainedIn(
        mergewright_core::oid::Oid,
        Result<mergewright_git::contained::Contained, String>,
    ),
    /// The Mistake Radar's default-branch context, or why it could not be
    /// read.
    RadarContext(Result<std::sync::Arc<crate::jobs::radar::RadarContext>, String>),
    /// One merge's evil-merge verdict. Never a `Result`: "could not check"
    /// is one of the verdict's own three answers, not a failure.
    EvilProbe(
        mergewright_core::oid::Oid,
        mergewright_ops::mistake_radar::probes::EvilVerdict,
    ),
    /// One branch's force-push verdict, by branch name, with the tip it was
    /// asked about. Never a `Result`, for the same reason
    /// [`Product::EvilProbe`] is not: "could not check" is one of the
    /// verdict's own answers.
    ForcePushProbe(
        String,
        mergewright_core::oid::Oid,
        mergewright_ops::mistake_radar::probes::ForcePushVerdict,
    ),
    /// The duplicate-commit probe's groups, or why it could not run.
    DuplicatesProbe(
        Result<std::sync::Arc<mergewright_ops::mistake_radar::probes::Duplicates>, String>,
    ),
    /// The commits only a stash reaches, each with the newest entry `n`
    /// (`stash@{n}`) reaching it, or why they could not be read
    /// (m7-power-tools/16).
    StashOnly(Result<std::sync::Arc<mergewright_git::history_reads::StashOnly>, String>),
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
    /// A put-in planned: the rewrite to preview, `plan`'s refusal, or —
    /// on the `HEAD` path — nothing built on his branch (today's amend); or
    /// why it could not be read (m7-power-tools/09).
    RewritePlanned(Result<crate::jobs::rewrite::RewritePlanned, String>),
    /// The commits a landed put-in changed, one per planned offset, or why
    /// they could not be read (m7-power-tools/09).
    RowTagsResolved(Result<Vec<mergewright_core::oid::Oid>, String>),
    /// What the rewrite editor opens on, `plan`'s refusal, or why the read
    /// failed (m7-power-tools/08).
    RewriteOpened(Result<crate::jobs::rewrite_open::RewriteOpened, String>),
    /// What `$EDITOR` left in the message file, or why Mergewright could not use
    /// it: the editor would not start, exited non-zero, or the file could
    /// not be read back (m3-working-tree-undo-and-stash/05).
    ExternalEditor(Result<String, String>),
    /// The terminal was handed to another program and came back, with
    /// nothing to bring back but whether it worked: `e`'s file in `$EDITOR`,
    /// and `Ctrl+Z`'s suspend (m3-working-tree-undo-and-stash/05).
    HandedOver(Result<(), String>),
    /// `git <kind> --continue` came back (`Command::ContinueInGit`): its
    /// exit code (`None`: killed by a signal), or why it could not run. A
    /// code, not `ExitStatus`: `update` stays pure
    /// (`update_cannot_reach_the_world`). The landing waits for the
    /// working-tree read it asks for (m7-power-tools/07c).
    ContinuedInGit(Result<Option<i32>, String>),
    /// One working-tree write finished. Not a plain `Result`: see
    /// [`crate::jobs::operate::Operated`]'s doc for why an operation that
    /// ran but left part of its after-state unread still carries the
    /// [`mergewright_ops::apply::Applied`] the caller refreshes from.
    Operated(Box<crate::jobs::operate::Operated>),
    /// A clone from the empty state ended (m8/18b): the new repository, or
    /// why not. Delivered even when the clone was stopped.
    Cloned(crate::jobs::clone::CloneAnswer),
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
    /// it could not be opened — in `mergewright_ops`' own words
    /// ([`mergewright_ops::conflict::OpenError`]'s `Display`), never `{:?}`.
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
    /// ([`mergewright_ops::conflict::TakeError`]'s `Display`).
    Taken(
        Result<
            std::sync::Arc<mergewright_git::conflict::ConflictFile>,
            crate::jobs::conflict::TakeFailure,
        >,
    ),
    /// The last pick was put back and the file written once (ease-pass
    /// 05): the document as it now stands, or why nothing was written.
    Untaken(
        Result<
            std::sync::Arc<mergewright_git::conflict::ConflictFile>,
            crate::jobs::conflict::UntakeFailure,
        >,
    ),
    /// The merge base was attached to every block of the open file, or it
    /// could not be (`B`, m6-conflicts/05): the document with every block's
    /// [`mergewright_git::conflict::Block::base`] filled in, and how many blocks
    /// got one, or why not — `mergewright_ops::conflict::BaseError`'s own words
    /// (`no base (added on both sides)`, `base unavailable after edits`, or
    /// git's own `diff3` failure), paired with whether they are a problem —
    /// only a failure is; no base and edited-past-git are facts.
    BaseLoaded(
        Result<
            (
                std::sync::Arc<mergewright_git::conflict::ConflictFile>,
                usize,
            ),
            (String, bool),
        >,
    ),
    /// The files the operation in progress conflicted on
    /// (`mergewright_ops::conflict::operation_conflicts`, m6-conflicts/13), or why
    /// they could not be read, in `mergewright_ops`' own words.
    ConflictSet(Result<std::sync::Arc<mergewright_ops::conflict::ConflictSet>, String>),
    /// What the merge about to be committed changes and the solved files
    /// its finish stages (`mergewright_ops::finish::merge_review`,
    /// m6-conflicts/19, ease-pass/05b), or why it could not be read.
    FinishSummary(Result<crate::update::finish::Reviewed, crate::update::finish::ReviewFailed>),
    /// One staged file's diff for the finish summary, or why not
    /// (m6-conflicts/19): the same view model the detail pane draws, built
    /// by the same code (`jobs::detail::assemble_diff`).
    FinishDiff(Result<std::sync::Arc<mergewright_ui::detail::DiffView>, String>),
    /// What a Ghost Preview's operation would do, or why no picture could be
    /// drawn (m5-history-and-ghost-preview/08). `Arc`: the open preview
    /// holds it across frames, and the slice is laid out from it once.
    Preview(Result<std::sync::Arc<super::preview::Planned>, super::preview::PreviewFailure>),
    /// The journal's time machine finished, stopped part-way, or hit
    /// §6.7's drift rail (m5-history-and-ghost-preview/08). Not a plain
    /// `Result`, for [`Product::Undone`]'s reasons.
    RevertedTo(Box<crate::jobs::undo::RevertedTo>),
    /// One undo or redo finished. Not a plain `Result` for the same reason
    /// [`Product::Operated`] is not: a restore that ran but could not read
    /// part of what it left behind still carries the
    /// [`mergewright_ops::apply::Applied`] the caller refreshes from, and "nothing
    /// to undo" and "the repository moved" are ordinary answers rather than
    /// failures (m3-working-tree-undo-and-stash/06).
    Undone(Box<crate::jobs::undo::Undone>),
    /// The undo journal, newest entry last, or why it could not be read.
    /// `Arc`: the journal browser holds it across frames, never copied per
    /// event (m3-working-tree-undo-and-stash/06).
    Journal(Result<std::sync::Arc<crate::jobs::undo::JournalRead>, String>),
    /// The tour's ticks (`Command::TourProgress`), in
    /// `mergewright_ops::tour::TourTask::ALL` order, or why the journal could not
    /// be read (m8-polish-and-ship/19b).
    TourProgress(Result<[bool; 8], String>),
    /// J's stage 2 answered (`Command::ReadOutside`): the reflog records, or
    /// why they could not be read. `Arc`: the list holds them across frames.
    Outside(Result<std::sync::Arc<Vec<mergewright_ops::journal_list::OutsideRecord>>, String>),
    /// J's stage 3 answered (`Command::FindLost`), or why the scan failed —
    /// the title says so (m7-power-tools/24).
    LostFound(Result<std::sync::Arc<crate::jobs::journal_list::LostFound>, String>),
    /// What `⏎` on a J row puts back (`Command::RecoverFor`); `Err` is the
    /// status row's words, a refusal or a failed read.
    RecoverFor(Result<mergewright_ops::journal_list::Recover, String>),
    /// The stash list, `stash@{0}` first, or why it could not be read.
    /// `Arc` for the same reason as [`Product::Journal`]: the overlay holds
    /// it across frames (m3-working-tree-undo-and-stash/06).
    Stashes(Result<std::sync::Arc<Vec<mergewright_git::stash::StashEntry>>, String>),
    /// The branch picker's rows in drawn order, or why they could not be
    /// read (`Command::ReadSwitchList`, m7-power-tools/10). `Arc`: the
    /// picker holds it across frames and filters it per key.
    SwitchList(Result<std::sync::Arc<mergewright_ops::switch_list::SwitchList>, String>),
    /// Every branch with its tip's date, for the palette's BRANCHES, or why
    /// they could not be read (`Command::ReadPaletteBranches`,
    /// m8-polish-and-ship/12).
    PaletteBranches(Result<std::sync::Arc<mergewright_git::switch_refs::SwitchRefs>, String>),
    /// The names job's answer for one query (`Command::PaletteNames`).
    PaletteNames(std::sync::Arc<crate::jobs::palette::NameHits>),
    /// Every tracked path, for the palette's FILES, or why they could not be
    /// read (`Command::ReadTrackedPaths`, m8-polish-and-ship/12b).
    TrackedPaths(Result<std::sync::Arc<mergewright_git::status::TrackedPaths>, String>),
    /// The commits job's answer for one query: the first
    /// `update::palette::COMMIT_HITS` in walk order, or why the walk failed
    /// (`Command::PaletteCommits`, m8-polish-and-ship/12b).
    PaletteCommits(Result<std::sync::Arc<[crate::search::CommitHit]>, String>),
    /// The index now, for a stash clash's tie, or why it could not be read
    /// (`Command::ReadIndexState`).
    IndexState(Result<mergewright_core::journal::IndexState, String>),
    /// The remotes, `HEAD`'s branch and its upstream, or why they could not
    /// be read (m4-remotes/05).
    Remotes(Result<std::sync::Arc<crate::jobs::remote::RemoteFacts>, String>),
    /// What a force-with-lease push would take off the remote, for the
    /// lease key it was asked for (`Command::ReadLeaves`), or why it could
    /// not be read.
    Leaves {
        /// The tip git refused.
        sent: mergewright_core::oid::Oid,
        /// The tracking tip the lease names.
        expected: mergewright_core::oid::Oid,
        /// The answer.
        result: Result<crate::update::remote::Leaves, String>,
    },
    /// How many commits a folded merge brought in
    /// (`Command::CountFolded`), or git's reason it could not be counted.
    FoldedCounted {
        /// The folded merge.
        merge: mergewright_core::oid::Oid,
        /// The answer.
        count: Result<usize, String>,
    },
    /// Every change against `HEAD` with its line counts
    /// (`Command::ReadCommitFiles`), or why it could not be read.
    CommitFiles(Result<std::sync::Arc<mergewright_git::diff::FileList>, String>),
}

/// A history place's whole file at one commit (m7-power-tools/21c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeFileAnswer {
    /// The file, assembled as the detail pane assembles a diff.
    pub view: std::sync::Arc<mergewright_ui::detail::DiffView>,
    /// `Some` only when the diff is binary. `Err`: the size read's `Display`.
    pub sizes: Option<Result<mergewright_git::trace::BlobSizes, String>>,
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

/// A finished topological sort: the order, and how long it took to make.
/// The time decides the commit-graph offer (m7-power-tools/15, :778): only a
/// sort the user had to wait for is worth a file to speed up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortedOrder {
    /// Every commit in strict topological order.
    pub order: std::sync::Arc<[mergewright_core::commit_handle::CommitHandle]>,
    /// Wall time of `history::topological_order`, measured in the job.
    pub took: core::time::Duration,
}

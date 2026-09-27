//! The operation shape: one enum, four properties (M3 brief point 1).
//!
//! Every write Graft makes is an [`Operation`] with, per kind, its
//! preconditions ([`Operation::check`]), its typed `git` invocations
//! ([`Operation::plan`]), what it changes ([`Operation::touches`] — which
//! decides the snapshot) and its inverse ([`crate::undo::inverse`], computed
//! from the journal entry, not from the operation). Everything here is pure:
//! the facts come in as [`RepoFacts`], nothing spawns `git`, nothing reads a
//! file, so every rule is testable without a repository.
//!
//! **Seam rule:** `ops` decides, `app` wires. `app` never builds a
//! `GitInvocation` for a write, never touches the journal file, never chooses
//! a snapshot; it turns a key into an `Operation` and hands it over.

mod commit;
mod conflict;
mod finish_merge;
mod history;
mod recover;
mod remote;
mod rewrite;
mod stage;
mod stash;

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::error::RefNameRule;
use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::remote::{Remote, TrackedUpstream};
use graft_git::run::GitArg;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::{UnmergedHow, WorkingTreeStatus};
use graft_git::version::GitVersion;

pub use conflict::{OneSided, ResolveWholeParams, RestartKind, RestartParams, Side};
pub(crate) use conflict::{check_restart, shape_of_stages};
pub use finish_merge::{AbortMergeParams, FinishMergeParams, MergeEnd};
pub use history::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, OnEdits, PickParams, RebaseParams, RefDeleteParams, ResetMode, ResetParams,
    ResetTarget, RevertParams, Source, TagCreateParams, Tip,
};
pub(crate) use history::{
    branch_ref, creates_at_head, merge_subject, rebase_invocation, short, short_ref, source_arg,
    tag_ref,
};
pub(crate) use recover::pins as recover_pins;
pub use recover::{FilesEffect, NoPutBack, RecoverParams, RecoverTarget};
pub use remote::{FetchParams, Lease, PullParams, PushParams, RemoteNames};

/// One write to the repository, as the user asked for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Paths or a patch into the index.
    Stage(Target),
    /// Paths or a patch out of the index.
    Unstage(Target),
    /// Paths or a patch dropped from the working tree (and the index).
    Discard(Target),
    /// A new commit from the index.
    Commit(CommitParams),
    /// `HEAD` rewritten from the index with a new message.
    Amend(CommitParams),
    /// `git stash push`.
    StashPush(StashParams),
    /// `git stash apply --index stash@{index}`; `commit` pins what the list
    /// showed, so a list that moved underneath is refused, never guessed at.
    StashApply {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git stash pop --index stash@{index}`.
    StashPop {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git stash drop stash@{index}`.
    StashDrop {
        /// Position in the stash list, `0` newest.
        index: usize,
        /// The stash commit the list showed at that position.
        commit: Oid,
    },
    /// `git fetch`: moves remote-tracking refs, nothing the user typed.
    /// Journaled, never undone.
    Fetch(FetchParams),
    /// A fast-forward pull: fetch the upstream's remote, then `merge
    /// --ff-only` onto its tracking ref. Undoable: the branch and the tree
    /// go back, the tracking ref stays where the fetch left it.
    Pull(PullParams),
    /// `git push`. Journaled, never undone — the remote keeps what it got.
    Push(PushParams),
    /// `git merge` into `HEAD`'s branch. One entry from start to finish: a
    /// conflict leaves it `Pending` until `resume` closes it (m5/07).
    Merge(MergeParams),
    /// `git rebase` of `HEAD`'s branch, through the rewrite engine: git's
    /// `rebase -i` on a todo of picks, the branches built on his following
    /// (ADR-0047 §6). One entry across every stop.
    Rebase(RebaseParams),
    /// A rewrite of `HEAD`'s branch — squash, fixup, reword, reorder, drop,
    /// edit, staged changes put into an older commit — planned by
    /// [`crate::rewrite::plan`] and run as git's `rebase -i` on the todo it
    /// planned, the branches built on his following (ADR-0047). One entry
    /// across every stop.
    Rewrite(crate::rewrite::RewriteParams),
    /// `git cherry-pick` onto `HEAD`.
    CherryPick(PickParams),
    /// `git revert` onto `HEAD`.
    Revert(RevertParams),
    /// `git reset` of `HEAD`'s branch, or `update-ref` of another branch.
    Reset(ResetParams),
    /// `git switch` to a branch or a detached commit.
    Checkout(CheckoutParams),
    /// `git branch <name> <at>`, optionally switched to.
    BranchCreate(BranchCreateParams),
    /// A branch deleted by compare-and-swap.
    BranchDelete(RefDeleteParams),
    /// `git tag`.
    TagCreate(TagCreateParams),
    /// A tag deleted by compare-and-swap.
    TagDelete(RefDeleteParams),
    /// `git stash branch`.
    BranchFromStash(BranchFromStashParams),
    /// One conflicted path resolved whole, to one side: the shapes a block
    /// take cannot reach — no markers, or binary (m6-conflicts/03).
    ResolveWholeFile(ResolveWholeParams),
    /// One conflicted path put back the way git left it: `git checkout
    /// --merge`, the resolver's "start this file over" (m6-conflicts/03).
    RestartConflict(RestartParams),
    /// A merge begun in the shell, finished: `git merge --continue`, or
    /// `git commit -F -` with an edited message. Its own entry, whose `u`
    /// puts the merge back in progress (m6-conflicts/20). `apply::resume`
    /// sends it when no entry of Graft's is live; a merge Graft began is
    /// closed on its own entry instead.
    FinishMerge(FinishMergeParams),
    /// A merge begun in the shell, aborted: `git merge --abort`. Its own
    /// entry with a full backup, whose `u` puts the merge back as it stood
    /// (m6-conflicts/57). `apply::resume(Abort)` sends it when no entry of
    /// Graft's is live; a merge Graft began is aborted on its own entry.
    AbortMerge(AbortMergeParams),
    /// A stash that clashed on its way back, dropped once the last clashed
    /// file is done (ADR-0045 §3): `stash drop stash@{n}`, `n` looked up by
    /// the commit in the facts `apply` reads, never assumed. Refused
    /// `NoSuchStash` when it is gone. Journaled as `StashDrop` with the stash
    /// pinned, so `u` stores it back.
    DropPutBack {
        /// The stash's commit.
        stash: Oid,
    },
    /// A put-back from the journal's list (ADR-0046 §3, m7-power-tools/23):
    /// a branch moved back, a deleted branch made again, or an edit an undo
    /// replaced restored from that undo's backup. `u` undoes it.
    Recover(RecoverParams),
}

/// What [`crate::facts::read_facts`] must read for one operation, beyond
/// the fixed picture: [`Operation::facts_scope`] names it, so a stage never
/// pays for remotes and a merge never pays for a pass over the ref store.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FactsScope {
    /// Read the remotes and `HEAD`'s upstream (fetch, pull, push).
    pub remotes: bool,
    /// Refs to read by exact name into [`RepoFacts::refs`]: the tips a
    /// history operation is pinned to, `HEAD` for a checkout.
    pub refs: Vec<String>,
}

/// What a stage, unstage or discard applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Whole files, as the status lists them.
    Paths(Vec<PathBuf>),
    /// One hunk, or selected lines of one, as a complete one-file patch from
    /// `graft_git::diff::hunk_patch`. Its `PatchDirection` names how `plan`
    /// applies it — `Forward` for a stage (`git apply --cached`), `Reverse`
    /// for an unstage or discard (`apply -R`) — so the direction and the
    /// `-R` are one choice, not two; it only changes the bytes for a
    /// selection of lines (a whole hunk is the same either way).
    Patch {
        /// The file the patch is for.
        path: PathBuf,
        /// The patch bytes, fed to `git apply` on stdin.
        patch: Vec<u8>,
        /// Which diff the hunk was cut from: a stage takes a worktree hunk,
        /// an unstage an index hunk, a discard either — and reverses an
        /// index hunk in both the index and the working tree.
        side: PatchSide,
    },
}

/// Which diff a [`Target::Patch`] was cut from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchSide {
    /// The unstaged diff: working tree against index. The Changes pane's
    /// body for an unstaged or untracked file.
    Worktree,
    /// The staged diff: index against `HEAD`. The body for a staged file.
    Index,
}

/// What a commit or amend needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitParams {
    /// The whole message; the first non-blank line is the subject.
    pub message: String,
    /// `--signoff`.
    pub sign_off: bool,
    /// `--no-verify`: skip the pre-commit and commit-msg hooks.
    pub no_verify: bool,
    /// `--allow-empty`: a commit with nothing staged.
    pub allow_empty: bool,
}

/// What a stash push needs.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StashParams {
    /// `-m <message>`; without one git names the stash after `HEAD`.
    pub message: Option<String>,
    /// What goes in.
    pub scope: StashScope,
}

/// What a stash push takes. Git refuses `--staged` together with
/// `--include-untracked` (verified), so the pair is not representable: the
/// prompt toggles one or the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StashScope {
    /// Every staged and unstaged change; with `include_untracked`, `-u`
    /// takes untracked files too.
    All {
        /// `-u`.
        include_untracked: bool,
    },
    /// `--staged`: only the index (git 2.35+).
    StagedOnly,
}

impl Default for StashScope {
    /// `git stash push` with no flags: tracked changes only.
    fn default() -> Self {
        Self::All {
            include_untracked: false,
        }
    }
}

/// One typed `git` invocation an operation plans: arguments and stdin, not a
/// string, so nothing is ever re-parsed on the way to a process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// The arguments after `git`.
    pub args: Vec<GitArg>,
    /// Bytes fed on stdin: a patch, or a commit message.
    pub stdin: Option<Vec<u8>>,
    /// Talks to a remote: the executor streams its progress and answers its
    /// credential prompts. `false` for everything local.
    pub remote: bool,
}

impl Invocation {
    fn new(args: Vec<GitArg>) -> Self {
        Self {
            args,
            stdin: None,
            remote: false,
        }
    }

    fn with_stdin(args: Vec<GitArg>, stdin: Vec<u8>) -> Self {
        Self {
            args,
            stdin: Some(stdin),
            remote: false,
        }
    }

    /// `git <args>` as a person would type it, shell-quoted: the Command
    /// Log's line, and what the plan tests compare.
    #[must_use]
    pub fn command_line(&self) -> String {
        graft_git::run::command_line(&self.args)
    }
}

/// What a rebase or checkout sets aside around itself
/// ([`Operation::sets_aside`], ADR-0045 §4): the tracked files changed.
/// Every put-back keeps the index (ADR-0045 amendment 2026-09-26 b,
/// m7-power-tools/30). When git cannot reinstate it, the after-state says so
/// (`AfterState::StagingNotKept`); the preview does not forecast that.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetAside {
    /// Tracked files with staged or unstaged changes.
    pub files: usize,
}

/// Everything `check`, `plan` and `touches` may look at, read once by the
/// executor before the operation and never re-read half way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoFacts {
    /// No working tree at all.
    pub bare: bool,
    /// Where `HEAD` points.
    pub head: HeadRef,
    /// A merge, rebase, cherry-pick, revert or bisect under way.
    pub in_progress: Option<InProgress>,
    /// The working tree's status, porcelain v2 with untracked files listed
    /// one by one.
    pub status: WorkingTreeStatus,
    /// The stash list, newest first.
    pub stashes: Vec<StashEntry>,
    /// The system git's version.
    pub git: GitVersion,
    /// The configured remotes, `origin` first. Read only for a remote
    /// operation (`read_facts`'s `with_remotes`); empty otherwise.
    pub remotes: Vec<Remote>,
    /// The upstream of `HEAD`'s branch; `None` when detached, unborn, not
    /// tracking anything, or not read (a local operation).
    pub upstream: Option<TrackedUpstream>,
    /// The refs [`FactsScope::refs`] asked for, by exact name; `None` for
    /// one that is absent. `HEAD` resolves to the commit it is on. Empty
    /// for an operation that named none (m5/07).
    pub refs: BTreeMap<String, Option<Oid>>,
    /// Unmerged paths whose file still holds a `<<<<<<< ` line. Read only
    /// when the status lists unmerged paths, so it costs nothing otherwise
    /// (m5/07).
    pub markers: Vec<PathBuf>,
}

/// The key that asked for a rewrite, so a refusal names what *that* key does
/// (note R1/R2: `R` and `a` each have their own words).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewriteKey {
    /// `R`: open the rewrite.
    Open,
    /// `a`: put staged changes into an older commit.
    AmendInto,
}

impl RewriteKey {
    /// What the key does, as the refusal's first half.
    #[must_use]
    pub fn what_it_does(self) -> &'static str {
        match self {
            Self::Open => "R rewrites your branch",
            Self::AmendInto => "a puts staged changes into a commit on your branch",
        }
    }

    /// The key itself, as a refusal names it (`a`).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Open => "R",
            Self::AmendInto => "a",
        }
    }
}

/// Why an operation will not run. Each one is a fact about the repository the
/// user can act on, never a git error re-worded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The repository has no working tree.
    Bare,
    /// A merge, rebase, cherry-pick, revert or bisect is under way; finishing
    /// or abandoning it is M5/M6's, so commit, amend, stash and discard wait.
    InProgress(InProgress),
    /// Paths with unresolved conflicts; resolving them is M6's.
    Unmerged {
        /// The conflicted paths.
        paths: Vec<PathBuf>,
    },
    /// A path the status does not list in a state this operation applies to.
    PathNotInState {
        /// The path.
        path: PathBuf,
    },
    /// The patch's `---`/`+++` path is not the target's path.
    PatchPathMismatch,
    /// The hunk was cut from the other diff: an index hunk given to a stage,
    /// or a worktree hunk to an unstage.
    PatchSideMismatch,
    /// A commit with nothing staged and `allow_empty` off.
    NothingStaged,
    /// A message with no subject line.
    EmptySubject,
    /// An amend or a stash push before the first commit.
    UnbornHead,
    /// A stash push with nothing to put in it: nothing staged for a
    /// staged-only push, otherwise nothing staged, unstaged or (when
    /// included) untracked.
    NothingToStash,
    /// The system git is too old for a flag the operation needs.
    NeedsGit {
        /// The git feature, as the user would search for it.
        feature: &'static str,
        /// The oldest git that has it.
        minimum: GitVersion,
    },
    /// No stash at the given index.
    NoSuchStash,
    /// The stash at that index is not the one the user chose.
    StashListChanged,
    /// Applying the stash would touch files that have uncommitted changes —
    /// a possible conflict, and conflicts are M6's.
    TouchesChangedFiles {
        /// The paths both the stash and the working tree changed.
        paths: Vec<PathBuf>,
    },
    /// A stash put-back refused before writing anything: unsaved edits
    /// (worktree ≠ index) to a path the stash writes, or a file where the
    /// stash would create an untracked one (m6-conflicts/86). Only the stash
    /// preview says it for the first; `apply` still refuses that with
    /// [`Refusal::TouchesChangedFiles`]. The second — ignored files
    /// included — both the preview and `apply` refuse, from
    /// [`crate::in_the_way::blocking`] (ADR-0049).
    ///
    /// The three sets are named apart (review 86, m4 and m5): edits of his
    /// read `your unsaved edits to …`, a file that is not his edit reads
    /// `<path> is in the way`, and staged-only edits `check` also refuses
    /// are named beside the unsaved ones, so the first refusal is the whole
    /// list.
    StashInTheWay {
        /// Tracked paths with unsaved edits.
        unsaved: Vec<PathBuf>,
        /// Tracked paths whose only changes are staged. Named only beside
        /// one of the other two sets; alone they stay
        /// [`Refusal::TouchesChangedFiles`].
        staged: Vec<PathBuf>,
        /// Untracked files where the stash creates one.
        untracked: Vec<PathBuf>,
        /// Ignored files where the stash creates one (review 28 m4): named
        /// first, tagged `(ignored)`, with the next step `move it aside
        /// first`, in the words of [`Refusal::InTheWay`].
        ignored: Vec<PathBuf>,
    },
    /// The stash's staged part no longer applies to the index as it stands:
    /// git's `stash apply --index` refuses (`conflicts in index`) and writes
    /// nothing (review 86, M1). Only the stash preview says it.
    StashIndexNoLongerApplies {
        /// Position in the stash list, `0` newest.
        index: usize,
    },
    /// A pull or push with `HEAD` not on a branch (detached or unborn).
    DetachedHead,
    /// No remote is configured at all.
    NoRemote,
    /// No remote by the name asked for.
    NoSuchRemote {
        /// The name asked for.
        name: String,
    },
    /// A pull on a branch with no upstream.
    NoUpstream {
        /// The branch, short name.
        branch: String,
    },
    /// The upstream the app read is not the one configured now.
    UpstreamChanged,
    /// `HEAD` is no longer on the branch the app read.
    BranchChanged,
    /// A force-push whose lease names a commit the local tracking ref is not
    /// at: fetched since the box was read, or never fetched at all. Refused
    /// before git runs, so work the user has not looked at is never
    /// overwritten (m4/04 review, M1).
    LeaseStale {
        /// The branch on the remote, as `origin/main`.
        remote_side: String,
    },
    /// A tip the preview was drawn against is no longer where it was: the
    /// picture the user confirmed is not what would run (m5/07, hard
    /// choice 3).
    Moved {
        /// The ref, full name or `HEAD`.
        name: String,
    },
    /// A switch to a new branch, or a branch from a stash, over changed
    /// tracked files; `S` stashes them first (m5/07, hard choice 4).
    /// Untracked files never count. A rebase or checkout sets them aside
    /// instead (ADR-0045 §4, m6-conflicts/73).
    DirtyTree {
        /// How many tracked files have staged or unstaged changes.
        count: usize,
    },
    /// A branch or tag name git would not accept.
    BadRefName {
        /// The name as typed.
        name: String,
        /// Which of git's rules it broke.
        rule: RefNameRule,
    },
    /// A branch or tag by that name already exists.
    RefExists {
        /// The name as typed.
        name: String,
    },
    /// A stage of an unmerged path whose file still has `<<<<<<<` in it.
    MarkersRemain {
        /// The path.
        path: PathBuf,
    },
    /// `resume` with no operation of Graft's in progress.
    NotInProgress,
    /// A re-push of a commit that is not on the branch being pushed.
    NotOnBranch {
        /// The commit asked for.
        commit: Oid,
        /// The branch, short name.
        branch: String,
    },
    /// `-m <n>` given for a commit that has no such parent, or is not a
    /// merge at all.
    MainlineOnNonMerge,
    /// Reverting a merge without saying which parent is the mainline.
    MergeNeedsMainline,
    /// A cherry-pick with no commits.
    NoCommits,
    /// A ref the operation names does not exist.
    NoSuchRef {
        /// Full name.
        name: String,
    },
    /// Deleting the branch `HEAD` is on.
    CheckedOut {
        /// Short name.
        branch: String,
    },
    /// `Resume::Skip` on an operation that has no step to skip (a merge).
    CannotSkip(InProgress),
    /// `Resume::ContinueWith` on an operation that is not a merge: a rebase,
    /// cherry-pick or revert keeps each commit's own message, so there is no
    /// one message to replace (m6-conflicts/09; a per-commit message is M7).
    MessageNotHere {
        /// What git has in progress.
        kind: InProgress,
    },
    /// `c` on an **octopus** merge begun in the shell (`git merge a b`:
    /// `MERGE_HEAD` names several commits). Graft finishes a merge of one
    /// branch; the finish summary compares each path against one
    /// `MERGE_HEAD`, so for an octopus it would show a result that is not
    /// the one being committed (m6-conflicts/38). `a` on one is refused too:
    /// `u` of the abort could not be checked to put every head back
    /// (m6-conflicts/57). `end` says which, so the words name the command.
    OctopusMerge {
        /// How many commits `MERGE_HEAD` names: branches being merged in.
        heads: usize,
        /// `c` or `a`.
        end: MergeEnd,
    },
    /// `c` or `a` on a shell-begun merge while `MERGE_AUTOSTASH` waits
    /// (`git merge --autostash`): git applies that stash when the merge
    /// ends either way, and `u` of the end could not give it back
    /// (m6-conflicts/57; before that, `c` said `NotInProgress` about a
    /// merge standing in front of the user).
    AutostashWaits {
        /// `c` or `a`.
        end: MergeEnd,
    },
    /// The Ghost Preview has no picture for this operation: a stage, a
    /// commit, an ordinary push. The M4 confirm boxes cover those; the
    /// preview draws the operations that move history.
    NotPreviewable(OperationKind),
    /// The journal's time machine cannot revert to the chosen entry, and
    /// why (one line, already in the user's terms).
    CannotRevertTo(String),
    /// A re-push asked for without a lease. A re-push moves the remote
    /// branch backwards, which is a force by any other name, and ADR-0038
    /// §4 allows only `--force-with-lease` — never a bare force.
    RepushNeedsLease,
    /// `--ff-only` where the branches have diverged: a merge commit would be
    /// needed, and the user said no (m5/07, from the preview).
    NotFastForward {
        /// The branch merged into, short name.
        branch: String,
    },
    /// A conflict operation on a path the status does not call unmerged —
    /// resolved already, never conflicted, or the status has moved on
    /// (m6-conflicts/03).
    NotUnmerged {
        /// The path.
        path: PathBuf,
    },
    /// `⏎` (mark resolved) on a file that still has blocks nobody has taken
    /// a side on. Said in words before any `git` process runs;
    /// [`Refusal::MarkersRemain`] stays as the second line of defence
    /// (m6-conflicts/03).
    ConflictsOpen {
        /// The path.
        path: PathBuf,
        /// How many blocks are still open.
        open: usize,
    },
    /// A block take, or a restart, on a shape that has no markers: every
    /// `UnmergedHow` but `BothModified` and `BothAdded`, and a binary file
    /// of either of those. [`Operation::ResolveWholeFile`] is the way out
    /// (m6-conflicts/03).
    ///
    /// A restart lands here for the same reason at one remove: `checkout
    /// --merge` needs more than one stage to merge, and a one-sided shape has
    /// one. It used to reach git and come back as a raw
    /// `does not have all necessary versions` (m6 g03 review, M1);
    /// [`conflict::check_restart`] holds the measurements.
    NotMarkerShape {
        /// The path.
        path: PathBuf,
        /// The shape the status read.
        how: UnmergedHow,
    },
    /// `d` on a path that is no longer unmerged and whose sides no recorded
    /// stage tree holds either: there is nothing to put back, and
    /// `checkout --merge` on a stage-0 path git kept no resolve-undo for
    /// exits 0 and silently does nothing (measured, git 2.54.0). Said in
    /// words instead (m6-conflicts/08).
    NoStagesKept {
        /// The path.
        path: PathBuf,
    },
    /// `d` sent a [`RestartKind`] the stages recovered now no longer spell:
    /// the gate's answer went stale between the offer and the act. Nothing
    /// was written (m6-conflicts/58).
    RestartChanged {
        /// The path.
        path: PathBuf,
    },
    // ── The rewrite's refusals (ADR-0047; m7 R1's words) ──────────────
    /// `R` or `a` on a commit that is not on the branch `HEAD` is on.
    NotOnYourBranch {
        /// The commit.
        commit: Oid,
        /// Which key asked: each says what it does in its own words.
        by: RewriteKey,
    },
    /// A merge inside the range: `--rebase-merges` is not in M7.
    MergeInRange {
        /// The first merge in the range, oldest first.
        merge: Oid,
    },
    /// Two branches both built on his, neither on the other: Graft moves
    /// one line and does not guess which.
    ForkedDependents {
        /// One of them.
        a: RefName,
        /// The other.
        b: RefName,
    },
    /// A branch built on his is checked out in another worktree: git will
    /// not move a branch checked out elsewhere, and leaving it behind is the
    /// duplicated history the line exists to prevent.
    DependentElsewhere {
        /// The branch.
        branch: RefName,
        /// The worktree it is checked out in.
        worktree: PathBuf,
    },
    /// `a` on an older commit with nothing staged. (The ticket's
    /// `NothingStaged{target}`; the unit [`Refusal::NothingStaged`] is the
    /// commit's and already taken.)
    NothingStagedFor {
        /// The commit that would have gained the changes.
        target: Oid,
    },
    /// `HEAD` is detached (or on a branch with no commit): a rewrite
    /// rewrites a branch.
    NotOnABranch,
    /// Branches built on his would follow, and this git's `rebase -i` has
    /// no `update-ref` (git ≥ 2.38). A rewrite with none still runs.
    GitTooOld {
        /// The version needed, as the words say it.
        need: &'static str,
    },
    /// `a` into `HEAD` with nothing built on it: not a rewrite — the app's
    /// plain amend does it.
    PlainAmend,
    /// `a` into the repository's first commit: there is nothing to replay
    /// it onto.
    RootCommit {
        /// The commit.
        commit: Oid,
    },
    /// The todo cannot run as written.
    BadTodo(crate::rewrite::TodoProblem),
    /// Absorb placed none of the staged hunks: every one stays staged.
    NothingToAbsorb,
    /// Absorb, and the commit at `HEAD` is on a remote-tracking ref: every
    /// commit below it is too, so there is no unpushed commit to put lines
    /// into (06c Question 2; on every base since the 06e ruling). Words:
    /// `every commit on <branch> is already pushed · absorb only changes
    /// unpushed commits · c commits them`.
    AllPushed {
        /// The branch, short name (`main`).
        branch: String,
    },
    /// Absorb on a branch with no base below `HEAD` that is neither the
    /// default branch nor the only one — a new branch at `main`'s tip:
    /// nothing on it is its own (06e ruling, Q1). Words: `<branch> has no
    /// commits of its own yet · c commits them`.
    NoCommitsOfItsOwn {
        /// The branch, short name (`fix`).
        branch: String,
    },
    /// `a` into a commit older than `HEAD` that a remote-tracking ref holds:
    /// a teammate may have it (06c ruling; on every base since 06e). Only
    /// `a` gives it — absorb refuses with [`Refusal::AllPushed`] — and never
    /// for `HEAD`, which `a` amends whatever a remote holds (06e ruling 5).
    /// Words: `<short> is already pushed · a only changes older commits you
    /// haven't pushed`.
    AlreadyPushed {
        /// The commit.
        commit: Oid,
    },
    /// `a` on the default or only branch with no base below `HEAD`, into an
    /// unpushed commit more than [`crate::rewrite::ABSORB_FALLBACK_DEPTH`]
    /// back (06c ruling; 06e names the branch, the only place it fires).
    /// Words: `<short> is more than 10 commits back · on <branch> a reaches
    /// the last 10 unpushed commits`.
    BeyondReach {
        /// The commit.
        commit: Oid,
        /// The branch, short name (`main`).
        branch: String,
    },
    // ── The journal list's put-backs (ADR-0046 §3; m7-power-tools/22) ──
    /// A put-back's branch is no longer where the list saw it: it moved on
    /// since, to `now`. The compare-and-swap the put-back runs under.
    MovedOnSince {
        /// The branch, short name.
        name: String,
        /// Where it is now.
        now: Oid,
    },
    /// The lost commit is reachable again — a branch, tag or stash holds it
    /// — so there is nothing to put back.
    HeldAgain {
        /// The commit.
        commit: Oid,
        /// What holds it, in words (`main`).
        by: String,
    },
    /// The row has no put-back of its own, and why.
    NoPutBack(NoPutBack),
    /// A file put-back over files with changes of his own — staged,
    /// unstaged or untracked: the restore would write over them, and there
    /// is no set-aside here, since the edit *is* the file (m7/23).
    UnsavedEditsTo {
        /// The files in the way.
        paths: Vec<PathBuf>,
    },
    /// Files git doesn't keep stand where the operation would write its own
    /// (ADR-0049): git would write over them, and the backup can't hold an
    /// ignored one. Read by [`crate::in_the_way::blocking`], in the preview
    /// and again at `y`, before anything is written.
    InTheWay {
        /// What the operation is doing, in words: `switching to other`.
        doing: String,
        /// Ignored files in the way, sorted. They decide the next step:
        /// neither `S` nor `c` takes one.
        ignored: Vec<PathBuf>,
        /// Untracked files in the way, sorted: `S` stashes them.
        untracked: Vec<PathBuf>,
    },
}

impl Refusal {
    /// One plain sentence saying what stands in the way and what to do, naming
    /// the shell command where a later milestone owns the fix.
    #[must_use]
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per refusal, matched exhaustively here: splitting the remote \
                  ones out needed a catch-all that re-entered this function (m4/04 review \
                  Minor 7)"
    )]
    pub fn explain(&self) -> String {
        match self {
            Self::Bare => "This repository has no working tree, so there is nothing here to \
                           stage, commit, discard or stash."
                .to_owned(),
            Self::InProgress(kind) => format!(
                "A {} is in progress; finish it with `{}` or abandon it with `{}` first.",
                kind.name(),
                kind.continue_command(),
                kind.abort_command()
            ),
            // ADR-0045 / m6-conflicts/72: Graft resolves them now, so the
            // words say what to do here, not in the shell.
            Self::Unmerged { paths } => match paths.split_first() {
                Some((first, rest)) if !rest.is_empty() => format!(
                    "{} +{} still has conflicts · resolve it first",
                    first.display(),
                    rest.len()
                ),
                _ => format!(
                    "{} still has conflicts · resolve it first",
                    name_paths(paths)
                ),
            },
            Self::PathNotInState { path } => format!(
                "{} has no change this can apply to; the status may have moved on, so \
                 refresh and try again.",
                path.display()
            ),
            Self::PatchPathMismatch => "The patch is for a different file than the one \
                                        selected; refresh and pick the hunk again."
                .to_owned(),
            Self::PatchSideMismatch => "The hunk comes from the other diff (a stage takes an \
                                        unstaged hunk, an unstage a staged one); refresh and \
                                        pick it again."
                .to_owned(),
            Self::NothingStaged => {
                "Nothing is staged; stage something first, or allow an empty commit.".to_owned()
            }
            Self::EmptySubject => "The commit message needs a subject line.".to_owned(),
            Self::UnbornHead => "There is no commit yet; amend and stash both need one.".to_owned(),
            Self::NothingToStash => "There are no changes to stash: untracked files count only \
                                     when they are included, and a staged-only stash needs \
                                     something staged."
                .to_owned(),
            Self::NeedsGit { feature, minimum } => {
                format!("`git {feature}` needs git {minimum} or newer; the system git is older.")
            }
            Self::NoSuchStash => "That stash is no longer in the list; look again.".to_owned(),
            Self::StashListChanged => {
                "The stash list changed since it was read; look again before choosing.".to_owned()
            }
            Self::TouchesChangedFiles { paths } => format!(
                "The stash changes {}, which {} uncommitted changes now; stage, stash or \
                 discard those first, since applying over them could conflict.",
                name_paths(paths),
                if paths.len() == 1 { "has" } else { "have" }
            ),
            Self::StashInTheWay {
                unsaved,
                staged,
                untracked,
                ignored,
            } => in_the_way_words(unsaved, staged, untracked, ignored),
            Self::StashIndexNoLongerApplies { index } => {
                format!("the staged part of stash@{{{index}}} no longer applies")
            }
            // The remote refusals, in the M4 design note's words.
            Self::DetachedHead => "HEAD is detached — check out a branch first".to_owned(),
            Self::NoRemote => {
                "no remote — add one in the shell: git remote add origin <url>".to_owned()
            }
            Self::NoSuchRemote { name } => format!("no remote named '{name}'"),
            Self::NoUpstream { branch } => {
                format!("{branch} has no upstream — P pushes and sets one")
            }
            Self::UpstreamChanged => {
                "the branch's upstream changed since it was read — press p again".to_owned()
            }
            Self::BranchChanged => {
                "HEAD moved to another branch since it was read — look again".to_owned()
            }
            Self::LeaseStale { remote_side } => format!(
                "force-push refused: {remote_side} is not where the box said \
                 (fetched since, or never fetched) · f to fetch, then look"
            ),
            // The history refusals, in the M5 design note's words.
            Self::Moved { name } => format!(
                "{} moved since the preview was drawn — press the key again",
                history::short_ref(name)
            ),
            Self::DirtyTree { count } => format!(
                "needs a clean tree · S stashes {count} changed {} first",
                if *count == 1 { "file" } else { "files" }
            ),
            Self::BadRefName { name, rule } => format!("bad name '{name}': {rule}"),
            Self::RefExists { name } => format!("{name} exists"),
            Self::MarkersRemain { path } => format!(
                "{} still has conflict markers (<<<<<<<) — e edits, M opens the mergetool",
                path.display()
            ),
            Self::NotInProgress => {
                "nothing Graft started is in progress; an operation begun in the shell is \
                 finished there"
                    .to_owned()
            }
            Self::NotOnBranch { commit, branch } => format!(
                "{} is not on {branch}; there is no old tip to re-push",
                history::short(*commit)
            ),
            Self::MainlineOnNonMerge => {
                "-m names a parent this commit has not got; only a merge takes a mainline"
                    .to_owned()
            }
            Self::MergeNeedsMainline => {
                "reverting a merge needs a mainline: Fix… offers -m 1".to_owned()
            }
            Self::NoCommits => "nothing to cherry-pick".to_owned(),
            Self::NoSuchRef { name } => format!("{} does not exist", history::short_ref(name)),
            Self::CheckedOut { branch } => {
                format!("{branch} is checked out; switch away before deleting it")
            }
            Self::NotPreviewable(kind) => {
                format!("{} has no preview", crate::apply::kind_name(*kind))
            }
            Self::CannotRevertTo(why) => format!("cannot revert to that point: {why}"),
            Self::RepushNeedsLease => {
                "re-pushing an old tip rewrites the remote branch; it runs only with a lease"
                    .to_owned()
            }
            Self::CannotSkip(kind) => {
                format!(
                    "a {} has nothing to skip · c continues, a aborts",
                    kind.name()
                )
            }
            Self::MessageNotHere { kind } => {
                format!(
                    "a {} keeps each commit's own message · c continues",
                    kind.name()
                )
            }
            Self::OctopusMerge { heads, end } => format!(
                "this merges {heads} branches at once (an octopus merge); {}",
                end.in_the_shell()
            ),
            Self::AutostashWaits { end } => format!(
                "git set your changes aside for this merge (autostash); {}",
                end.in_the_shell()
            ),
            Self::NotFastForward { branch } => {
                format!("{branch} has diverged; a fast-forward is not possible — m merges instead")
            }
            // The M6 conflict refusals, in the design note's words.
            Self::NotUnmerged { path } => format!("{} is not conflicted", path.display()),
            Self::ConflictsOpen { path, open } => {
                let noun = if *open == 1 { "conflict" } else { "conflicts" };
                format!("{open} {noun} still open in {}", path.display())
            }
            Self::NotMarkerShape { path, how } => format!(
                "{}: {} — no markers to take from",
                path.display(),
                how.words()
            ),
            Self::NoStagesKept { path } => {
                format!("{}: the conflict's sides were not kept", path.display())
            }
            Self::RestartChanged { path } => {
                format!(
                    "{} changed since it was opened · open it again",
                    path.display()
                )
            }
            Self::NotOnYourBranch { commit, by } => {
                format!("{} · {} isn't on it", by.what_it_does(), short(*commit))
            }
            Self::MergeInRange { merge } => {
                format!("can't rewrite past a merge ({})", short(*merge))
            }
            Self::ForkedDependents { a, b } => format!(
                "{} and {} are both built on yours · Graft moves one line",
                ref_words(a),
                ref_words(b)
            ),
            Self::DependentElsewhere { branch, worktree } => format!(
                "{} is checked out in another worktree ({}) · it can't follow",
                ref_words(branch),
                worktree.display()
            ),
            Self::NothingStagedFor { target } => {
                format!(
                    "nothing staged to put into {} · space stages",
                    short(*target)
                )
            }
            Self::NotOnABranch => "you're not on a branch · switch first (C)".to_owned(),
            Self::GitTooOld { need } => {
                format!("branches built on yours can follow only with git {need} or newer")
            }
            Self::PlainAmend => {
                "that's the newest commit · a plain amend puts the changes in".to_owned()
            }
            Self::RootCommit { commit } => format!(
                "{} is the first commit · there is nothing below it to replay onto",
                short(*commit)
            ),
            Self::BadTodo(problem) => problem.explain(),
            Self::NothingToAbsorb => {
                "none of your staged lines belong to a commit on your branch · c commits them"
                    .to_owned()
            }
            Self::AllPushed { branch } => format!(
                "every commit on {branch} is already pushed · absorb only changes unpushed \
                 commits · c commits them"
            ),
            Self::NoCommitsOfItsOwn { branch } => {
                format!("{branch} has no commits of its own yet · c commits them")
            }
            Self::AlreadyPushed { commit } => format!(
                "{} is already pushed · a only changes older commits you haven't pushed",
                short(*commit)
            ),
            Self::BeyondReach { commit, branch } => format!(
                "{} is more than {} commits back · on {branch} a reaches the last {} unpushed \
                 commits",
                short(*commit),
                crate::rewrite::ABSORB_FALLBACK_DEPTH,
                crate::rewrite::ABSORB_FALLBACK_DEPTH
            ),
            Self::MovedOnSince { name, now } => {
                format!("{name} moved on since (now {})", short(*now))
            }
            Self::HeldAgain { commit, by } => {
                format!("{} is on {by} again · nothing to put back", short(*commit))
            }
            Self::NoPutBack(NoPutBack::Graft) => {
                "done in Graft · revert to before it instead".to_owned()
            }
            Self::NoPutBack(NoPutBack::Switch { back_to }) => {
                format!("a switch moves no branch · switch back to {back_to} instead")
            }
            Self::NoPutBack(NoPutBack::Created { name }) => {
                format!("this step made {name} · there was nothing before it to go back to")
            }
            Self::NoPutBack(NoPutBack::Nothing) => "nothing here to put back".to_owned(),
            Self::UnsavedEditsTo { paths } => {
                let verb = if paths.len() == 1 { "has" } else { "have" };
                format!(
                    "{} {verb} unsaved edits · c commits or S stashes them first",
                    name_paths(paths)
                )
            }
            Self::InTheWay {
                doing,
                ignored,
                untracked,
            } => in_the_way_of(doing, ignored, untracked),
        }
    }
}

/// [`Refusal::InTheWay`] in words. The first ignored path names the line
/// when there is one, since an ignored file decides the next step; `+N more`
/// counts every other path, ignored or not (the form of Recover's
/// `put back your edit to <path> +N more`).
fn in_the_way_of(doing: &str, ignored: &[PathBuf], untracked: &[PathBuf]) -> String {
    let Some(first) = ignored.first().or(untracked.first()) else {
        return format!("nothing is in the way of {doing}");
    };
    let first = first.display();
    let more = ignored.len() + untracked.len() - 1;
    match (!ignored.is_empty(), more) {
        (true, 0) => format!("{first} (ignored) is in the way of {doing} · move it aside first"),
        (true, more) => format!(
            "{first} (ignored) +{more} more are in the way of {doing} · move them aside first"
        ),
        (false, 0) => format!("{first} is in the way of {doing} · S stashes it first"),
        (false, more) => {
            format!("{first} +{more} more are in the way of {doing} · S stashes them first")
        }
    }
}

/// `src/a.rs`, `src/a.rs and src/b.rs`, `src/a.rs and 2 more files`.
/// [`Refusal::StashInTheWay`] in words: each non-empty set as its own
/// clause, joined by `and`. `is` only for one file that is not his edit
/// (`new.txt is in the way`); every other shape is plural. An ignored file
/// reads as in [`Refusal::InTheWay`] (review 28 m4): the first ignored one
/// names the files, `(ignored) +N more`, and the next step is `move it
/// aside first`, since `S` can't take it.
fn in_the_way_words(
    unsaved: &[PathBuf],
    staged: &[PathBuf],
    untracked: &[PathBuf],
    ignored: &[PathBuf],
) -> String {
    let mut clauses = Vec::new();
    if !unsaved.is_empty() {
        clauses.push(format!("your unsaved edits to {}", name_paths(unsaved)));
    }
    if !staged.is_empty() {
        clauses.push(format!("your staged edits to {}", name_paths(staged)));
    }
    let files = untracked.len() + ignored.len();
    match (ignored.first(), files) {
        (Some(first), 1) => clauses.push(format!("{} (ignored)", first.display())),
        (Some(first), files) => {
            clauses.push(format!("{} (ignored) +{} more", first.display(), files - 1));
        }
        (None, 0) => {}
        (None, _) => clauses.push(name_paths(untracked)),
    }
    let verb = if unsaved.is_empty() && staged.is_empty() && files == 1 {
        "is"
    } else {
        "are"
    };
    let sentence = format!("{} {verb} in the way", clauses.join(" and "));
    match (ignored.is_empty(), files) {
        (true, _) => sentence,
        (false, 1) => format!("{sentence} · move it aside first"),
        (false, _) => format!("{sentence} · move them aside first"),
    }
}

/// A branch as the words name it: `feature-b`, not `refs/heads/feature-b`.
fn ref_words(name: &RefName) -> String {
    String::from_utf8_lossy(name.shorthand()).into_owned()
}

fn name_paths(paths: &[PathBuf]) -> String {
    match paths {
        [] => "no files".to_owned(),
        [one] => one.display().to_string(),
        [first, second] => format!("{} and {}", first.display(), second.display()),
        [first, rest @ ..] => format!("{} and {} more files", first.display(), rest.len()),
    }
}

/// The in-progress state an operation of this kind opens when git stops on
/// a conflict: what the executor matches `in_progress` against to tell
/// "stopped, still live" from "failed" (m5/07). `None` for every other kind.
///
/// One table, asked two ways: [`Operation::starts_in_progress`] for an
/// operation about to run, this for a journal entry already written — a
/// second copy is a second thing to keep in step (m5/07 review, M11).
#[must_use]
pub(crate) fn opens_in_progress(kind: OperationKind) -> Option<InProgress> {
    match kind {
        OperationKind::Merge => Some(InProgress::Merge),
        // A rewrite runs git's `rebase -i` (ADR-0047).
        OperationKind::Rebase | OperationKind::Rewrite => Some(InProgress::Rebase),
        OperationKind::CherryPick => Some(InProgress::CherryPick),
        OperationKind::Revert => Some(InProgress::Revert),
        _ => None,
    }
}

impl Operation {
    /// Which journal kind this records as.
    #[must_use]
    pub fn kind(&self) -> OperationKind {
        match self {
            Self::Stage(_) => OperationKind::Stage,
            Self::Unstage(_) => OperationKind::Unstage,
            Self::Discard(_) => OperationKind::Discard,
            Self::Commit(_) => OperationKind::Commit,
            Self::Amend(_) => OperationKind::Amend,
            Self::StashPush(_) => OperationKind::StashPush,
            Self::StashApply { .. } => OperationKind::StashApply,
            Self::StashPop { .. } => OperationKind::StashPop,
            Self::StashDrop { .. } | Self::DropPutBack { .. } => OperationKind::StashDrop,
            Self::Fetch(_) => OperationKind::Fetch,
            Self::Pull(_) => OperationKind::Pull,
            Self::Push(_) => OperationKind::Push,
            Self::Merge(_) => OperationKind::Merge,
            Self::Rebase(_) => OperationKind::Rebase,
            Self::Rewrite(_) => OperationKind::Rewrite,
            Self::CherryPick(_) => OperationKind::CherryPick,
            Self::Revert(_) => OperationKind::Revert,
            Self::Reset(_) => OperationKind::Reset,
            Self::Checkout(_) => OperationKind::Checkout,
            Self::BranchCreate(_) => OperationKind::BranchCreate,
            Self::BranchDelete(_) => OperationKind::BranchDelete,
            Self::TagCreate(_) => OperationKind::TagCreate,
            Self::TagDelete(_) => OperationKind::TagDelete,
            Self::BranchFromStash(_) => OperationKind::BranchFromStash,
            Self::ResolveWholeFile(_) => OperationKind::ResolveConflict,
            Self::RestartConflict(_) => OperationKind::RestartConflict,
            Self::FinishMerge(_) => OperationKind::FinishMerge,
            Self::AbortMerge(_) => OperationKind::AbortMerge,
            Self::Recover(_) => OperationKind::Recover,
        }
    }

    /// The in-progress state this operation itself opens when git stops on
    /// a conflict: what the executor matches `in_progress` against to tell
    /// "stopped, still live" from "failed" (m5/07). `None` for every other
    /// operation.
    #[must_use]
    pub fn starts_in_progress(&self) -> Option<InProgress> {
        opens_in_progress(self.kind())
    }

    /// A history operation (m5/07): its preview is drawn from ghosts and
    /// ref moves, and the executor re-reads everything after it.
    #[must_use]
    pub fn is_history(&self) -> bool {
        matches!(
            self,
            Self::Merge(_)
                | Self::Rebase(_)
                | Self::Rewrite(_)
                | Self::CherryPick(_)
                | Self::Revert(_)
                | Self::Reset(_)
                | Self::Checkout(_)
                | Self::BranchCreate(_)
                | Self::BranchDelete(_)
                | Self::TagCreate(_)
                | Self::TagDelete(_)
                | Self::BranchFromStash(_)
        )
    }

    /// True when finishing this write moves `HEAD` — its ref or its commit —
    /// so the graph cursor lands on the new `HEAD`'s row after it (UX §3.1,
    /// m7-power-tools/03). False for everything that leaves `HEAD` where it
    /// was: stage, stash, tag, fetch, push, a branch moved or deleted that
    /// `HEAD` is not on, and the resolver's per-file writes. A put-back
    /// (`Recover`) is false: the journal screen owns its own landing.
    ///
    /// No `_` arm: a new kind must say which it is.
    #[must_use]
    pub fn moves_head(&self) -> bool {
        match self {
            Self::Commit(_)
            | Self::Amend(_)
            | Self::Merge(_)
            | Self::Rebase(_)
            | Self::Rewrite(_)
            | Self::CherryPick(_)
            | Self::Revert(_)
            | Self::Pull(_)
            | Self::Checkout(_)
            | Self::BranchFromStash(_)
            // The shell-begun merge, committed: a new merge commit on
            // `HEAD`. Sent by `apply::resume`, whose finish lands already.
            | Self::FinishMerge(_) => true,
            Self::Reset(params) => matches!(params.target, ResetTarget::Head { .. }),
            // A new branch switched to moves `HEAD`, as a checkout does.
            Self::BranchCreate(params) => params.checkout,
            Self::Stage(_)
            | Self::Unstage(_)
            | Self::Discard(_)
            | Self::StashPush(_)
            | Self::StashApply { .. }
            | Self::StashPop { .. }
            | Self::StashDrop { .. }
            | Self::Fetch(_)
            | Self::Push(_)
            | Self::BranchDelete(_)
            | Self::TagCreate(_)
            | Self::TagDelete(_)
            | Self::ResolveWholeFile(_)
            | Self::RestartConflict(_)
            // `merge --abort` puts the tree back to `HEAD`; `HEAD` stays.
            | Self::AbortMerge(_)
            | Self::DropPutBack { .. }
            | Self::Recover(_) => false,
        }
    }

    /// What a rebase, rewrite, previewed switch or switching create away
    /// from `HEAD` would set aside before it runs and put back after
    /// (ADR-0045 §4): `Some` when tracked files are changed,
    /// staged or not; `None` on a clean tree, over untracked files only
    /// (they stay where they are), and for every other kind. Pure over the
    /// facts — the preview and `apply` ask the same question.
    #[must_use]
    pub fn sets_aside(&self, facts: &RepoFacts) -> Option<SetAside> {
        let wraps = match self {
            Self::Rebase(_) | Self::Rewrite(_) => true,
            // A switch sets aside only when it was previewed (the switch
            // rule, m7-power-tools/10b); a new branch at `HEAD`'s own commit
            // leaves the edits where they are.
            Self::Checkout(params) => params.on_edits == OnEdits::SetAside,
            Self::BranchCreate(params) => {
                params.checkout
                    && params.on_edits == OnEdits::SetAside
                    && !history::creates_at_head(params, facts)
            }
            // A branch put back while checked out is a `reset --hard`.
            Self::Recover(params) => recover::moves_checked_out(params, facts),
            _ => false,
        };
        if !wraps {
            return None;
        }
        let files = facts
            .status
            .entries
            .iter()
            .filter(|entry| entry.is_staged() || entry.is_unstaged())
            .count();
        (files > 0).then_some(SetAside { files })
    }

    /// `refs/stash`, when this operation sets edits aside.
    fn stash_ref_if_set_aside(&self, facts: &RepoFacts) -> Option<String> {
        self.sets_aside(facts).map(|_| stash::STASH_REF.to_owned())
    }

    /// `touches`, with `refs/stash` added when this operation sets edits
    /// aside (a switch's).
    fn and_stash_if_set_aside(&self, mut touches: Touches, facts: &RepoFacts) -> Touches {
        touches.refs.extend(self.stash_ref_if_set_aside(facts));
        touches
    }

    /// Every [`Tip`] this operation is pinned to: what the Ghost Preview was
    /// drawn against, and what `check` refuses [`Refusal::Moved`] over
    /// (hard choice 3). Empty for an operation with nothing pinned.
    #[must_use]
    pub fn tips(&self) -> Vec<Tip> {
        match self {
            Self::Merge(params) => vec![params.into.clone()],
            Self::Rebase(params) => history::tips_of_rebase(params),
            Self::Rewrite(params) => rewrite::tips(params),
            Self::CherryPick(params) => vec![params.head.clone()],
            Self::Revert(params) => vec![params.head.clone()],
            Self::Reset(params) => match &params.target {
                ResetTarget::Head { branch, .. } => vec![branch.clone()],
                ResetTarget::Branch(tip) => vec![tip.clone()],
            },
            Self::BranchDelete(params) | Self::TagDelete(params) => vec![Tip {
                name: params.name.clone(),
                oid: params.expected,
            }],
            _ => vec![],
        }
    }

    /// What the facts must carry for this operation: the remotes for a
    /// remote one, the named refs a history one is pinned to.
    #[must_use]
    pub fn facts_scope(&self) -> FactsScope {
        let refs = match self {
            Self::Merge(params) => history::refs_of_merge(params),
            Self::Rebase(params) => history::refs_of_rebase(params),
            Self::Rewrite(params) => rewrite::tips(params)
                .into_iter()
                .map(|tip| tip.name)
                .collect(),
            Self::CherryPick(params) => vec![params.head.name.clone()],
            Self::Revert(params) => vec![params.head.name.clone()],
            Self::Reset(params) => history::refs_of_reset(params),
            Self::Checkout(params) => history::refs_of_checkout(params),
            Self::BranchCreate(params) => history::refs_of_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => vec![params.name.clone()],
            Self::TagCreate(params) => vec![history::tag_ref(&params.name)],
            // `HEAD` as well: the preview reaches the stash commit by
            // asking for what is on it and not on `HEAD`, and the branch
            // starts at the stash's base, not at `HEAD` (m5/07).
            Self::BranchFromStash(params) => {
                vec![history::branch_ref(&params.name), "HEAD".to_owned()]
            }
            // The branch being pushed: a re-push has to prove its commit is
            // on it before it will draw a force-push (ADR-0038 §4).
            Self::Push(params) => vec![params.branch.clone()],
            Self::Recover(params) => recover::refs(params),
            _ => vec![],
        };
        FactsScope {
            remotes: self.is_remote(),
            refs,
        }
    }

    /// [`Operation::describe`], with what only the facts know: a
    /// `DropPutBack` is `drop stash@{n}`, `n` the position `check` found by
    /// commit — as a `StashDrop` reads (review 72+72b m6). `apply` writes
    /// this on the journal line.
    #[must_use]
    pub fn describe_in(&self, facts: &RepoFacts) -> String {
        match self {
            Self::DropPutBack { stash } => match stash::position_of(*stash, facts) {
                Ok(index) => format!("drop stash@{{{index}}}"),
                // Not listed: `check` refuses before anything is written,
                // so this line is never journaled.
                Err(_) => self.describe(),
            },
            _ => self.describe(),
        }
    }

    /// The journal's and the browser's one-line text: `stage src/a.rs`,
    /// `commit "fix the thing"`, `pop stash@{0}`. A `DropPutBack` has no
    /// position without the facts, so it is named by its commit here; the
    /// journal line uses [`Operation::describe_in`].
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Stage(target) => stage::describe("stage", target),
            Self::Unstage(target) => stage::describe("unstage", target),
            Self::Discard(target) => stage::describe("discard", target),
            Self::Commit(params) => format!("commit \"{}\"", commit::subject(&params.message)),
            Self::Amend(params) => format!("amend \"{}\"", commit::subject(&params.message)),
            Self::StashPush(params) => stash::describe_push(params),
            Self::StashApply { index, .. } => format!("apply stash@{{{index}}}"),
            Self::StashPop { index, .. } => format!("pop stash@{{{index}}}"),
            Self::StashDrop { index, .. } => format!("drop stash@{{{index}}}"),
            Self::DropPutBack { stash } => format!("drop stash {}", history::short(*stash)),
            Self::Fetch(params) => remote::describe_fetch(params),
            Self::Pull(params) => remote::describe_pull(params),
            Self::Push(params) => remote::describe_push(params),
            Self::Merge(params) => history::describe_merge(params),
            Self::Rebase(params) => history::describe_rebase(params),
            Self::Rewrite(params) => rewrite::describe(params),
            Self::CherryPick(params) => history::describe_pick(params),
            Self::Revert(params) => history::describe_revert(params),
            Self::Reset(params) => history::describe_reset(params),
            Self::Checkout(params) => history::describe_checkout(params),
            Self::BranchCreate(params) => history::describe_branch_create(params),
            Self::BranchDelete(params) => history::describe_delete("branch", params),
            Self::TagCreate(params) => history::describe_tag_create(params),
            Self::TagDelete(params) => history::describe_delete("tag", params),
            Self::BranchFromStash(params) => history::describe_branch_from_stash(params),
            Self::ResolveWholeFile(params) => conflict::describe_resolve(params),
            Self::RestartConflict(params) => conflict::describe_restart(params),
            Self::FinishMerge(params) => finish_merge::describe(params),
            Self::AbortMerge(params) => finish_merge::describe_abort(params),
            Self::Recover(params) => recover::describe(params),
        }
    }

    /// The two gates every operation passes before its own rules: unmerged
    /// paths, and an operation git already has in progress.
    ///
    /// A stage or unstage of exactly the unmerged paths is "mark resolved"
    /// (m5/07), and the two conflict operations *are* the resolution, so
    /// their own `check` is the one that refuses. A stage of a path that
    /// still carries markers is `MarkersRemain`. Everything else waits for
    /// the operation in progress to finish.
    fn check_conflict_state(&self, facts: &RepoFacts) -> Result<(), Refusal> {
        let unmerged: Vec<PathBuf> = facts
            .status
            .entries
            .iter()
            .filter(|entry| entry.unmerged)
            .map(|entry| entry.path.clone())
            .collect();
        if !unmerged.is_empty() {
            let resolving = match self {
                Self::Stage(Target::Paths(paths)) | Self::Unstage(Target::Paths(paths)) => {
                    paths.iter().all(|path| unmerged.contains(path))
                }
                // The two conflict operations are the resolution; aborting
                // is one of the merge's two ends, and a merge stopped on a
                // conflict is exactly what it ends.
                Self::ResolveWholeFile(_) | Self::RestartConflict(_) | Self::AbortMerge(_) => true,
                _ => false,
            };
            if !resolving {
                return Err(Refusal::Unmerged { paths: unmerged });
            }
            if let Self::Stage(Target::Paths(paths)) = self {
                history::markers_in(paths, facts)?;
            }
        }
        // Finishing or aborting the merge is what the in-progress state
        // waits *for*.
        let waits = !matches!(
            self,
            Self::Stage(_)
                | Self::Unstage(_)
                | Self::ResolveWholeFile(_)
                | Self::RestartConflict(_)
                | Self::FinishMerge(_)
                | Self::AbortMerge(_)
        );
        match facts.in_progress.filter(|_| waits) {
            Some(kind) => Err(Refusal::InProgress(kind)),
            None => Ok(()),
        }
    }

    /// Property 1: the preconditions. `Ok` means `plan` will produce
    /// invocations git is expected to accept.
    ///
    /// # Errors
    /// The first [`Refusal`] found, in this order: a bare repository, unmerged
    /// paths (a stage or unstage of exactly those paths is exempt — that is
    /// "mark resolved", m5/07 — and a stage of one that still has conflict
    /// markers is `MarkersRemain`), an operation in progress (stage and
    /// unstage are allowed mid-rebase, everything else waits), then the
    /// kind's own rules. A fetch skips the first three: it changes no
    /// branch, index or file, so it runs in a bare repository and in the
    /// middle of a rebase.
    pub fn check(&self, facts: &RepoFacts) -> Result<(), Refusal> {
        if let Self::Fetch(params) = self {
            return remote::check_fetch(params, facts);
        }
        if facts.bare {
            return Err(Refusal::Bare);
        }
        self.check_conflict_state(facts)?;
        match self {
            Self::Stage(target) => stage::check(target, &facts.status, stage::State::Stageable),
            Self::Unstage(target) => stage::check(target, &facts.status, stage::State::Staged),
            Self::Discard(target) => stage::check(target, &facts.status, stage::State::Discardable),
            Self::Commit(params) => commit::check_commit(params, &facts.status),
            Self::Amend(params) => commit::check_amend(params, &facts.head),
            Self::StashPush(params) => stash::check_push(params, facts),
            Self::StashApply { index, commit } | Self::StashPop { index, commit } => {
                stash::check_apply(*index, *commit, facts)
            }
            Self::StashDrop { index, commit } => stash::find(*index, *commit, facts).map(|_| ()),
            Self::DropPutBack { stash } => stash::position_of(*stash, facts).map(|_| ()),
            Self::Fetch(params) => remote::check_fetch(params, facts),
            Self::Pull(params) => remote::check_pull(params, facts),
            Self::Push(params) => remote::check_push(params, facts),
            Self::Merge(params) => history::check_merge(params, facts),
            Self::Rebase(params) => history::check_rebase(params, facts),
            Self::Rewrite(params) => rewrite::check(params, facts),
            Self::CherryPick(params) => history::check_pick(params, facts),
            Self::Revert(params) => history::check_revert(params, facts),
            Self::Reset(params) => history::check_reset(params, facts),
            Self::Checkout(params) => history::check_checkout(params, facts),
            Self::BranchCreate(params) => history::check_branch_create(params, facts),
            Self::BranchDelete(params) => history::check_branch_delete(params, facts),
            Self::TagCreate(params) => history::check_tag_create(params, facts),
            Self::TagDelete(params) => history::check_delete(params, facts),
            Self::BranchFromStash(params) => history::check_branch_from_stash(params, facts),
            Self::ResolveWholeFile(params) => conflict::check_resolve(params, facts),
            Self::RestartConflict(params) => conflict::check_restart(&params.path, facts),
            Self::FinishMerge(_) | Self::AbortMerge(_) => finish_merge::check(facts),
            Self::Recover(params) => recover::check(params, facts),
        }
    }

    /// Property 2: the typed invocations, in the order they run. Exact
    /// arguments are the rules in the group spec; each is verified against
    /// real git in a throwaway repository before it was written down.
    #[must_use]
    pub fn plan(&self, facts: &RepoFacts) -> Vec<Invocation> {
        match self {
            Self::Stage(target) => stage::plan_stage(target, &facts.status),
            Self::Unstage(target) => stage::plan_unstage(target, facts),
            Self::Discard(target) => stage::plan_discard(target, facts),
            Self::Commit(params) => vec![commit::plan(params, false)],
            Self::Amend(params) => vec![commit::plan(params, true)],
            Self::StashPush(params) => vec![stash::plan_push(params)],
            Self::StashApply { index, .. } => vec![stash::plan_at("apply", *index, true)],
            Self::StashPop { index, .. } => vec![stash::plan_at("pop", *index, true)],
            Self::StashDrop { index, .. } => vec![stash::plan_at("drop", *index, false)],
            Self::DropPutBack { stash } => vec![stash::plan_drop_by_commit(*stash, facts)],
            Self::Fetch(params) => remote::plan_fetch(params),
            Self::Pull(params) => remote::plan_pull(params),
            Self::Push(params) => remote::plan_push(params),
            Self::Merge(params) => history::plan_merge(params),
            Self::Rebase(params) => history::plan_rebase(params, facts.git),
            Self::Rewrite(params) => rewrite::plan(params, facts.git),
            Self::CherryPick(params) => history::plan_pick(params),
            Self::Revert(params) => history::plan_revert(params),
            Self::Reset(params) => history::plan_reset(params),
            Self::Checkout(params) => history::plan_checkout(params),
            Self::BranchCreate(params) => history::plan_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => history::plan_delete(params),
            Self::TagCreate(params) => history::plan_tag_create(params),
            Self::BranchFromStash(params) => history::plan_branch_from_stash(params),
            Self::ResolveWholeFile(params) => conflict::plan_resolve(params),
            Self::RestartConflict(params) => conflict::plan_restart(params),
            Self::FinishMerge(params) => finish_merge::plan(params),
            Self::AbortMerge(_) => finish_merge::plan_abort(),
            Self::Recover(params) => recover::plan(params, facts),
        }
    }

    /// Fetch, pull or push: an operation that talks to a remote. The
    /// executor re-reads everything after one (`Refresh::Everything`).
    #[must_use]
    pub fn is_remote(&self) -> bool {
        matches!(self, Self::Fetch(_) | Self::Pull(_) | Self::Push(_))
    }

    /// The branch and remote names a remote operation's failure line uses,
    /// from its params; `None` for a local operation.
    #[must_use]
    pub fn remote_names(&self) -> Option<RemoteNames> {
        match self {
            Self::Fetch(params) => Some(remote::names_of_fetch(params)),
            Self::Pull(params) => Some(remote::names_of_pull(params)),
            Self::Push(params) => Some(remote::names_of_push(params)),
            _ => None,
        }
    }

    /// The command lines a remote operation will run, known before any
    /// facts are read — the push box shows them. Equal to `plan`'s lines,
    /// because a remote operation's plan reads nothing from the facts.
    /// `None` for a local operation, whose plan depends on the status.
    #[must_use]
    pub fn preview(&self) -> Option<Vec<String>> {
        let plan = match self {
            Self::Fetch(params) => remote::plan_fetch(params),
            Self::Pull(params) => remote::plan_pull(params),
            Self::Push(params) => remote::plan_push(params),
            _ => return None,
        };
        Some(plan.iter().map(Invocation::command_line).collect())
    }

    /// Property 3: what the operation changes — the snapshot's scope and the
    /// generic inverse's steps. Refs are full names; a detached `HEAD` is
    /// `HEAD` itself.
    #[must_use]
    pub fn touches(&self, facts: &RepoFacts) -> Touches {
        let (refs, index, worktree) = match self {
            Self::Stage(_) | Self::Unstage(_) => (vec![], true, false),
            // A discard, and the two conflict operations: the index (three
            // stages collapsing to one entry, or going) and the working tree
            // (a side written out, or the file removed). No ref moves for
            // either conflict operation — the operation the conflict belongs
            // to is what moves one, when it finishes.
            Self::Discard(_) | Self::ResolveWholeFile(_) | Self::RestartConflict(_) => {
                (vec![], true, true)
            }
            Self::Commit(_) | Self::Amend(_) => {
                (vec![commit::head_ref_name(&facts.head)], true, false)
            }
            Self::StashPush(_) | Self::StashApply { .. } | Self::StashPop { .. } => {
                (vec![stash::STASH_REF.to_owned()], true, true)
            }
            Self::StashDrop { .. } | Self::DropPutBack { .. } => {
                (vec![stash::STASH_REF.to_owned()], false, false)
            }
            // Tracking refs are not recorded: a fetch can move thousands.
            Self::Fetch(_) => (vec![], false, false),
            // For the browser's `origin/main a1b2c3d → 9f8e7d6`, not for undo.
            Self::Push(params) => (remote::push_tracking_ref(params, facts), false, false),
            // The branch (or a detached `HEAD`), the index and the tree.
            // A pull never records the tracking ref: undo must not move it
            // back, since the remote still has those commits. Finishing a
            // shell-begun merge commits the resolved index, so `u` has the
            // resolved tree to put back. Aborting one moves no ref, but the
            // branch is recorded all the same: a commit made on it since is
            // drift, and putting the merge back onto another `HEAD` would
            // be a merge nobody began.
            Self::Pull(_)
            | Self::Merge(_)
            | Self::CherryPick(_)
            | Self::Revert(_)
            | Self::FinishMerge(_)
            | Self::AbortMerge(_) => (vec![commit::head_ref_name(&facts.head)], true, true),
            // A set-aside records `refs/stash` too: its before and after
            // say whether `S` was left in the list, which is what the
            // inverse's stash step reads (ADR-0045 §4 step 2, §6).
            //
            // A rebase's branches built on his join these in `apply`, which
            // finds them when it runs (`apply::engine`).
            Self::Rebase(_) => {
                let mut refs = vec![commit::head_ref_name(&facts.head)];
                refs.extend(self.stash_ref_if_set_aside(facts));
                (refs, true, true)
            }
            // Every branch of the line, his first (ADR-0047 §2).
            Self::Rewrite(params) => {
                let mut refs: Vec<String> = rewrite::tips(params)
                    .into_iter()
                    .map(|tip| tip.name)
                    .collect();
                refs.extend(self.stash_ref_if_set_aside(facts));
                (refs, true, true)
            }
            Self::Reset(params) => return history::touches_reset(params),
            Self::Checkout(params) => {
                return self.and_stash_if_set_aside(history::touches_checkout(params), facts);
            }
            Self::BranchCreate(params) => {
                return self.and_stash_if_set_aside(history::touches_branch_create(params), facts);
            }
            Self::BranchDelete(params) | Self::TagDelete(params) => {
                (vec![params.name.clone()], false, false)
            }
            Self::TagCreate(params) => (vec![history::tag_ref(&params.name)], false, false),
            Self::BranchFromStash(params) => return history::touches_branch_from_stash(params),
            Self::Recover(params) => {
                let mut touches = recover::touches(params, facts);
                touches.refs.extend(self.stash_ref_if_set_aside(facts));
                return touches;
            }
        };
        Touches {
            refs,
            index,
            worktree,
        }
    }
}

#[cfg(test)]
mod tests;

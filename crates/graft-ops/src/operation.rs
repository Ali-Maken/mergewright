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
mod history;
mod remote;
mod stage;
mod stash;

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::error::RefNameRule;
use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::remote::{Remote, TrackedUpstream};
use graft_git::run::GitArg;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::WorkingTreeStatus;
use graft_git::version::GitVersion;

pub use history::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, PickParams, RebaseParams, RefDeleteParams, ResetMode, ResetParams, ResetTarget,
    RevertParams, Source, TagCreateParams, Tip,
};
pub(crate) use history::{branch_ref, merge_subject, short, short_ref, tag_ref};
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
    /// `git rebase` of `HEAD`'s branch. One entry across every stop.
    Rebase(RebaseParams),
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
    /// A rebase or a switch over changed tracked files; `S` stashes them
    /// first (m5/07, hard choice 4). Untracked files never count.
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
            Self::Unmerged { paths } => format!(
                "{} unresolved conflicts; resolve and `git add` them in the shell first \
                 (Graft resolves conflicts in a later milestone).",
                name_paths(paths)
            ),
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
            Self::NotFastForward { branch } => {
                format!("{branch} has diverged; a fast-forward is not possible — m merges instead")
            }
        }
    }
}

/// `src/a.rs`, `src/a.rs and src/b.rs`, `src/a.rs and 2 more files`.
fn name_paths(paths: &[PathBuf]) -> String {
    match paths {
        [] => "no files".to_owned(),
        [one] => one.display().to_string(),
        [first, second] => format!("{} and {}", first.display(), second.display()),
        [first, rest @ ..] => format!("{} and {} more files", first.display(), rest.len()),
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
            Self::StashDrop { .. } => OperationKind::StashDrop,
            Self::Fetch(_) => OperationKind::Fetch,
            Self::Pull(_) => OperationKind::Pull,
            Self::Push(_) => OperationKind::Push,
            Self::Merge(_) => OperationKind::Merge,
            Self::Rebase(_) => OperationKind::Rebase,
            Self::CherryPick(_) => OperationKind::CherryPick,
            Self::Revert(_) => OperationKind::Revert,
            Self::Reset(_) => OperationKind::Reset,
            Self::Checkout(_) => OperationKind::Checkout,
            Self::BranchCreate(_) => OperationKind::BranchCreate,
            Self::BranchDelete(_) => OperationKind::BranchDelete,
            Self::TagCreate(_) => OperationKind::TagCreate,
            Self::TagDelete(_) => OperationKind::TagDelete,
            Self::BranchFromStash(_) => OperationKind::BranchFromStash,
        }
    }

    /// The in-progress state this operation itself opens when git stops on
    /// a conflict: what the executor matches `in_progress` against to tell
    /// "stopped, still live" from "failed" (m5/07). `None` for every other
    /// operation.
    #[must_use]
    pub fn starts_in_progress(&self) -> Option<InProgress> {
        match self {
            Self::Merge(_) => Some(InProgress::Merge),
            Self::Rebase(_) => Some(InProgress::Rebase),
            Self::CherryPick(_) => Some(InProgress::CherryPick),
            Self::Revert(_) => Some(InProgress::Revert),
            _ => None,
        }
    }

    /// A history operation (m5/07): its preview is drawn from ghosts and
    /// ref moves, and the executor re-reads everything after it.
    #[must_use]
    pub fn is_history(&self) -> bool {
        matches!(
            self,
            Self::Merge(_)
                | Self::Rebase(_)
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

    /// Every [`Tip`] this operation is pinned to: what the Ghost Preview was
    /// drawn against, and what `check` refuses [`Refusal::Moved`] over
    /// (hard choice 3). Empty for an operation with nothing pinned.
    #[must_use]
    pub fn tips(&self) -> Vec<Tip> {
        match self {
            Self::Merge(params) => vec![params.into.clone()],
            Self::Rebase(params) => vec![params.branch.clone()],
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
            Self::CherryPick(params) => vec![params.head.name.clone()],
            Self::Revert(params) => vec![params.head.name.clone()],
            Self::Reset(params) => history::refs_of_reset(params),
            Self::Checkout(params) => history::refs_of_checkout(params),
            Self::BranchCreate(params) => history::refs_of_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => vec![params.name.clone()],
            Self::TagCreate(params) => vec![history::tag_ref(&params.name)],
            Self::BranchFromStash(params) => vec![history::branch_ref(&params.name)],
            _ => vec![],
        };
        FactsScope {
            remotes: self.is_remote(),
            refs,
        }
    }

    /// The journal's and the browser's one-line text: `stage src/a.rs`,
    /// `commit "fix the thing"`, `pop stash@{0}`.
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
            Self::Fetch(params) => remote::describe_fetch(params),
            Self::Pull(params) => remote::describe_pull(params),
            Self::Push(params) => remote::describe_push(params),
            Self::Merge(params) => history::describe_merge(params),
            Self::Rebase(params) => history::describe_rebase(params),
            Self::CherryPick(params) => history::describe_pick(params),
            Self::Revert(params) => history::describe_revert(params),
            Self::Reset(params) => history::describe_reset(params),
            Self::Checkout(params) => history::describe_checkout(params),
            Self::BranchCreate(params) => history::describe_branch_create(params),
            Self::BranchDelete(params) => history::describe_delete("branch", params),
            Self::TagCreate(params) => history::describe_tag_create(params),
            Self::TagDelete(params) => history::describe_delete("tag", params),
            Self::BranchFromStash(params) => history::describe_branch_from_stash(params),
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
                _ => false,
            };
            if !resolving {
                return Err(Refusal::Unmerged { paths: unmerged });
            }
            if let Self::Stage(Target::Paths(paths)) = self {
                history::markers_in(paths, facts)?;
            }
        }
        let waits_for_in_progress = !matches!(self, Self::Stage(_) | Self::Unstage(_));
        if let Some(kind) = facts.in_progress.filter(|_| waits_for_in_progress) {
            return Err(Refusal::InProgress(kind));
        }
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
            Self::Fetch(params) => remote::check_fetch(params, facts),
            Self::Pull(params) => remote::check_pull(params, facts),
            Self::Push(params) => remote::check_push(params, facts),
            Self::Merge(params) => history::check_merge(params, facts),
            Self::Rebase(params) => history::check_rebase(params, facts),
            Self::CherryPick(params) => history::check_pick(params, facts),
            Self::Revert(params) => history::check_revert(params, facts),
            Self::Reset(params) => history::check_reset(params, facts),
            Self::Checkout(params) => history::check_checkout(params, facts),
            Self::BranchCreate(params) => history::check_branch_create(params, facts),
            Self::BranchDelete(params) => history::check_branch_delete(params, facts),
            Self::TagCreate(params) => history::check_tag_create(params, facts),
            Self::TagDelete(params) => history::check_delete(params, facts),
            Self::BranchFromStash(params) => history::check_branch_from_stash(params, facts),
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
            Self::Fetch(params) => remote::plan_fetch(params),
            Self::Pull(params) => remote::plan_pull(params),
            Self::Push(params) => remote::plan_push(params),
            Self::Merge(params) => history::plan_merge(params),
            Self::Rebase(params) => history::plan_rebase(params),
            Self::CherryPick(params) => history::plan_pick(params),
            Self::Revert(params) => history::plan_revert(params),
            Self::Reset(params) => history::plan_reset(params),
            Self::Checkout(params) => history::plan_checkout(params),
            Self::BranchCreate(params) => history::plan_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => history::plan_delete(params),
            Self::TagCreate(params) => history::plan_tag_create(params),
            Self::BranchFromStash(params) => history::plan_branch_from_stash(params),
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
            Self::Discard(_) => (vec![], true, true),
            Self::Commit(_) | Self::Amend(_) => {
                (vec![commit::head_ref_name(&facts.head)], true, false)
            }
            Self::StashPush(_) | Self::StashApply { .. } | Self::StashPop { .. } => {
                (vec![stash::STASH_REF.to_owned()], true, true)
            }
            Self::StashDrop { .. } => (vec![stash::STASH_REF.to_owned()], false, false),
            // Tracking refs are not recorded: a fetch can move thousands.
            Self::Fetch(_) => (vec![], false, false),
            // The branch and the tree, never the tracking ref: undo must not
            // move it back, since the remote still has those commits.
            Self::Pull(_) => (vec![commit::head_ref_name(&facts.head)], true, true),
            // For the browser's `origin/main a1b2c3d → 9f8e7d6`, not for undo.
            Self::Push(params) => (remote::push_tracking_ref(params, facts), false, false),
            Self::Merge(_) | Self::Rebase(_) | Self::CherryPick(_) | Self::Revert(_) => {
                (vec![commit::head_ref_name(&facts.head)], true, true)
            }
            Self::Reset(params) => return history::touches_reset(params),
            Self::Checkout(_) => return history::touches_checkout(),
            Self::BranchCreate(params) => return history::touches_branch_create(params),
            Self::BranchDelete(params) | Self::TagDelete(params) => {
                (vec![params.name.clone()], false, false)
            }
            Self::TagCreate(params) => (vec![history::tag_ref(&params.name)], false, false),
            Self::BranchFromStash(params) => return history::touches_branch_from_stash(params),
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

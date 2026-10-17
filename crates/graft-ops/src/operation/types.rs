//! Support types: targets, params, facts, invocations.

use super::{
    BTreeMap, EditStop, GitArg, GitVersion, HeadRef, InProgress, KeyNames, Oid, OpKey, Operation,
    PathBuf, Remote, StashEntry, TrackedUpstream, WorkingTreeStatus, clause,
};

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
///
/// The default: an empty message, no flags, [`Include::Staged`].
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CommitParams {
    /// The whole message; the first non-blank line is the subject.
    pub message: String,
    /// `--signoff`.
    pub sign_off: bool,
    /// `--no-verify`: skip the pre-commit and commit-msg hooks.
    pub no_verify: bool,
    /// `--allow-empty`: a commit with nothing staged.
    pub allow_empty: bool,
    /// What goes in: the index as he staged it, or the files he chose.
    pub include: Include,
}

/// What a commit or amend takes in (ease-pass/04).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum Include {
    /// The index as it is: today's `git commit`, hooks and all.
    #[default]
    Staged,
    /// These files, each whole or as staged; every other staged entry stays
    /// staged. Built on a scratch copy of the index ([`IndexFile::Scratch`]).
    Chosen(Vec<Chosen>),
}

/// One file chosen to go in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chosen {
    /// The path as the status lists it (the new path for a rename).
    pub path: PathBuf,
    /// Which of its changes.
    pub part: Part,
}

/// How much of a chosen file goes in. The note's `Take`, renamed so it never
/// meets `graft_git::Take` in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The working file as it is: modified, deleted or new alike.
    Whole,
    /// Only what is staged; its unstaged changes stay out.
    StagedPart,
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
    /// The index this line reads and writes.
    pub index: IndexFile,
}

/// Which index a git line reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IndexFile {
    /// The repository's own index.
    #[default]
    Real,
    /// A copy of it the executor makes before the plan's first `Scratch`
    /// line, runs every `Scratch` line on with `GIT_INDEX_FILE`, and removes
    /// when the plan ends.
    Scratch,
}

/// What [`Invocation::command_line`] writes for the scratch index's path,
/// which only the executor knows ([`Invocation::command_line_in`]).
pub const SCRATCH_PLACEHOLDER: &str = "<scratch index>";

impl Invocation {
    pub(super) fn new(args: Vec<GitArg>) -> Self {
        Self {
            args,
            stdin: None,
            remote: false,
            index: IndexFile::Real,
        }
    }

    pub(super) fn with_stdin(args: Vec<GitArg>, stdin: Vec<u8>) -> Self {
        Self {
            args,
            stdin: Some(stdin),
            remote: false,
            index: IndexFile::Real,
        }
    }

    /// The same line, run on the scratch index.
    pub(super) fn on_scratch(self) -> Self {
        Self {
            index: IndexFile::Scratch,
            ..self
        }
    }

    /// `git <args>` as a person would type it, shell-quoted: what the plan
    /// tests compare and the preview shows. A [`IndexFile::Scratch`] line
    /// leads with `GIT_INDEX_FILE=<scratch index> `, the path only the
    /// executor knows.
    #[must_use]
    pub fn command_line(&self) -> String {
        self.spelled(SCRATCH_PLACEHOLDER)
    }

    /// The Command Log's line: [`Invocation::command_line`] with the scratch
    /// index's real path, so a `Scratch` line runs as shown.
    #[must_use]
    pub fn command_line_in(&self, scratch_index: &std::path::Path) -> String {
        use std::os::unix::ffi::OsStrExt;
        self.spelled(&graft_git::run::quote_word(
            scratch_index.as_os_str().as_bytes(),
        ))
    }

    fn spelled(&self, scratch_index: &str) -> String {
        let line = graft_git::run::command_line(&self.args);
        match self.index {
            IndexFile::Real => line,
            IndexFile::Scratch => format!("GIT_INDEX_FILE={scratch_index} {line}"),
        }
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
    /// What a pull's fetch brought (m7-power-tools/03b). `Some` only for a
    /// pull, only once `apply` has run its fetch, and only when tracked
    /// files are changed; `read_facts` never fills it.
    pub incoming: Option<Incoming>,
    /// The rebase in progress stopped on an `edit` row (m7-power-tools/07a,
    /// [`crate::rewrite::edit_stop`]). `None` outside a rebase and at a
    /// conflict stop. P14's one exception keys on it (`check`).
    pub edit_stop: Option<EditStop>,
}

/// What a pull's fetch brought: the tracking ref's tip after it, whether
/// `HEAD` fast-forwards to it, and the paths that differ between the two
/// (`diff --name-only --no-renames`, so a rename is both of its paths).
/// Read only for a fast-forward: `paths` is empty when `fast_forward` is
/// false.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Incoming {
    /// The tracking ref's tip after the fetch.
    pub tip: Oid,
    /// `HEAD` is an ancestor of `tip`.
    pub fast_forward: bool,
    /// Every path the fast-forward changes.
    pub paths: Vec<PathBuf>,
}

/// P14's one exception (m7-power-tools/07a, note R4): an amend inside a
/// rebase stopped on an `edit` row, while `HEAD` is the commit git stopped
/// on or an amend of it. A commit on top of it, a conflict stop, and every
/// other operation in progress still wait.
pub(super) fn amends_at_edit_stop(op: &Operation, facts: &RepoFacts) -> bool {
    matches!(op, Operation::Amend(_))
        && facts.in_progress == Some(InProgress::Rebase)
        && facts.edit_stop.as_ref().is_some_and(|stop| stop.head_on_it)
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
    /// What the key does, as the refusal's first half (`R rewrites your
    /// branch`); `None` when he has the key unbound (W3).
    #[must_use]
    pub fn what_it_does(self, keys: &dyn KeyNames) -> Option<String> {
        match self {
            Self::Open => clause(keys, self.key(), "rewrites your branch"),
            Self::AmendInto => clause(
                keys,
                self.key(),
                "puts staged changes into a commit on your branch",
            ),
        }
    }

    /// The key itself, by what it does.
    #[must_use]
    pub fn key(self) -> OpKey {
        match self {
            Self::Open => OpKey::Rewrite,
            Self::AmendInto => OpKey::Amend,
        }
    }
}

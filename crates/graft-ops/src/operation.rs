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
mod stage;
mod stash;

use std::path::PathBuf;

use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::run::GitArg;
use graft_git::stash::StashEntry;
use graft_git::state::InProgress;
use graft_git::status::WorkingTreeStatus;
use graft_git::version::GitVersion;

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
}

/// What a stage, unstage or discard applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// Whole files, as the status lists them.
    Paths(Vec<PathBuf>),
    /// One hunk, or selected lines of one, as a complete one-file patch from
    /// `graft_git::diff::hunk_patch` (forward for a stage, reverse for an
    /// unstage or discard).
    Patch {
        /// The file the patch is for.
        path: PathBuf,
        /// The patch bytes, fed to `git apply` on stdin.
        patch: Vec<u8>,
    },
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
    /// `-u`: untracked files go in too.
    pub include_untracked: bool,
    /// `--staged`: only the index (git 2.35+).
    pub staged_only: bool,
}

/// One typed `git` invocation an operation plans: arguments and stdin, not a
/// string, so nothing is ever re-parsed on the way to a process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// The arguments after `git`.
    pub args: Vec<GitArg>,
    /// Bytes fed on stdin: a patch, or a commit message.
    pub stdin: Option<Vec<u8>>,
}

impl Invocation {
    fn new(args: Vec<GitArg>) -> Self {
        Self { args, stdin: None }
    }

    fn with_stdin(args: Vec<GitArg>, stdin: Vec<u8>) -> Self {
        Self {
            args,
            stdin: Some(stdin),
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
    /// A commit with nothing staged and `allow_empty` off.
    NothingStaged,
    /// A message with no subject line.
    EmptySubject,
    /// An amend before the first commit.
    UnbornHead,
    /// A stash push with nothing to put in it.
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
}

impl Refusal {
    /// One plain sentence saying what stands in the way and what to do, naming
    /// the shell command where a later milestone owns the fix.
    #[must_use]
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
            Self::NothingStaged => {
                "Nothing is staged; stage something first, or allow an empty commit.".to_owned()
            }
            Self::EmptySubject => "The commit message needs a subject line.".to_owned(),
            Self::UnbornHead => "There is no commit to amend yet.".to_owned(),
            Self::NothingToStash => "There are no changes to stash (untracked files count only \
                                     when they are included)."
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
        }
    }

    /// Property 1: the preconditions. `Ok` means `plan` will produce
    /// invocations git is expected to accept.
    ///
    /// # Errors
    /// The first [`Refusal`] found, in this order: a bare repository, unmerged
    /// paths, an operation in progress (stage and unstage are allowed
    /// mid-rebase, everything else waits), then the kind's own rules.
    pub fn check(&self, facts: &RepoFacts) -> Result<(), Refusal> {
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
            return Err(Refusal::Unmerged { paths: unmerged });
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
        }
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

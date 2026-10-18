//! Stage, unstage and discard: paths or a patch, against the status.

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use mergewright_core::journal::HeadRef;
use mergewright_git::run::GitArg;
use mergewright_git::status::{StatusEntry, WorkingTreeStatus};

use super::{Invocation, PatchSide, Refusal, RepoFacts, Target};

/// Which status a path must be in for the operation to apply to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    /// Stage: an unstaged change, or an untracked file (whole files only —
    /// a patch needs a tracked file, ADR-0034).
    Stageable,
    /// Unstage: a staged change.
    Staged,
    /// Discard: any change at all; a patch needs a change on the side it
    /// was cut from.
    Discardable,
}

impl State {
    /// `hunk` is the side a hunk came from; `None` for whole files.
    fn admits(self, entry: &StatusEntry, hunk: Option<PatchSide>) -> bool {
        match (self, hunk) {
            // An unmerged path is stageable whole: that is "mark resolved"
            // (m5/07); `Operation::check` has already refused it while the
            // file still carries markers.
            (Self::Stageable, None) => entry.is_unstaged() || entry.untracked || entry.unmerged,
            (Self::Discardable, None) => {
                entry.is_unstaged() || entry.is_staged() || entry.untracked
            }
            (Self::Stageable, Some(_)) | (Self::Discardable, Some(PatchSide::Worktree)) => {
                entry.is_unstaged()
            }
            (Self::Staged, _) | (Self::Discardable, Some(PatchSide::Index)) => entry.is_staged(),
        }
    }

    /// The side a hunk must come from, where the operation fixes it.
    fn wanted_side(self) -> Option<PatchSide> {
        match self {
            Self::Stageable => Some(PatchSide::Worktree),
            Self::Staged => Some(PatchSide::Index),
            Self::Discardable => None,
        }
    }
}

pub(super) fn describe(verb: &str, target: &Target) -> String {
    match target {
        Target::Paths(paths) => match paths.as_slice() {
            [one] => format!("{verb} {}", one.display()),
            many => format!("{verb} {} files", many.len()),
        },
        Target::Patch { path, .. } => format!("{verb} a hunk of {}", path.display()),
    }
}

pub(super) fn check(
    target: &Target,
    status: &WorkingTreeStatus,
    state: State,
) -> Result<(), Refusal> {
    match target {
        Target::Paths(paths) => paths
            .iter()
            .try_for_each(|path| check_path(path, status, state, None)),
        Target::Patch { path, patch, side } => {
            check_path(path, status, state, Some(*side))?;
            if state.wanted_side().is_some_and(|wanted| wanted != *side) {
                return Err(Refusal::PatchSideMismatch);
            }
            if patch_names(patch, path) {
                Ok(())
            } else {
                Err(Refusal::PatchPathMismatch)
            }
        }
    }
}

fn check_path(
    path: &Path,
    status: &WorkingTreeStatus,
    state: State,
    hunk: Option<PatchSide>,
) -> Result<(), Refusal> {
    match status.entry(path) {
        Some(entry) if state.admits(entry, hunk) => Ok(()),
        _ => Err(Refusal::PathNotInState {
            path: path.to_path_buf(),
        }),
    }
}

/// Whether the patch's `---`/`+++` headers name `path` (`a/<path>`,
/// `b/<path>`, or `/dev/null` on one side). Only the first header pair
/// counts: a hunk patch is one file.
fn patch_names(patch: &[u8], target: &Path) -> bool {
    let wanted = target.as_os_str().as_bytes();
    let mut before = None;
    let mut after = None;
    for line in patch.split(|&b| b == b'\n') {
        if let Some(rest) = line.strip_prefix(b"--- ") {
            before.get_or_insert(rest);
        } else if let Some(rest) = line.strip_prefix(b"+++ ") {
            after.get_or_insert(rest);
            break;
        }
    }
    let (Some(before), Some(after)) = (before, after) else {
        return false;
    };
    let names = |side: &[u8], prefix: &[u8]| side.strip_prefix(prefix) == Some(wanted);
    let null = |side: &[u8]| side == b"/dev/null";
    (names(before, b"a/") || null(before))
        && (names(after, b"b/") || null(after))
        && !(null(before) && null(after))
}

pub(super) fn plan_stage(target: &Target, status: &WorkingTreeStatus) -> Vec<Invocation> {
    match target {
        Target::Paths(paths) => vec![Invocation::new(with_paths(
            vec![GitArg::Flag("add")],
            &pathspec(paths, status),
        ))],
        Target::Patch { patch, .. } => vec![Invocation::with_stdin(
            vec![GitArg::Flag("apply"), GitArg::Flag("--cached"), stdin_arg()],
            patch.clone(),
        )],
    }
}

pub(super) fn plan_unstage(target: &Target, facts: &RepoFacts) -> Vec<Invocation> {
    match target {
        Target::Paths(paths) => {
            let args = if matches!(facts.head, HeadRef::Unborn(_)) {
                // `restore --staged` needs a `HEAD` to restore from; before the
                // first commit the index entry simply goes (verified: leaves
                // the file untracked).
                vec![
                    GitArg::Flag("rm"),
                    GitArg::Flag("--cached"),
                    GitArg::Flag("--quiet"),
                ]
            } else {
                vec![GitArg::Flag("restore"), GitArg::Flag("--staged")]
            };
            vec![Invocation::new(with_paths(
                args,
                &pathspec(paths, &facts.status),
            ))]
        }
        Target::Patch { patch, .. } => vec![Invocation::with_stdin(
            vec![
                GitArg::Flag("apply"),
                GitArg::Flag("--cached"),
                GitArg::Flag("-R"),
                stdin_arg(),
            ],
            patch.clone(),
        )],
    }
}

/// Discard, one invocation per class of path: untracked files are cleaned;
/// tracked files with a staged change go back to `HEAD` in both index and
/// working tree (this also removes a staged new file and puts a staged rename
/// back — verified); unstaged-only changes go back to the index. Before the
/// first commit there is no `HEAD`: staged entries leave the index and are
/// cleaned with the untracked ones.
///
/// A hunk is reversed where it came from: a worktree hunk in the working
/// tree; an index hunk in the working tree first, then the index. The
/// working tree goes first because it is the step that can fail — an
/// unstaged edit over the same lines — and a failure there leaves nothing
/// changed, where the other order would leave the hunk reversed in the index
/// only.
pub(super) fn plan_discard(target: &Target, facts: &RepoFacts) -> Vec<Invocation> {
    let paths = match target {
        Target::Paths(paths) => paths,
        Target::Patch { patch, side, .. } => {
            let reverse = |cached: bool| {
                let mut args = vec![GitArg::Flag("apply")];
                if cached {
                    args.push(GitArg::Flag("--cached"));
                }
                args.push(GitArg::Flag("-R"));
                args.push(stdin_arg());
                Invocation::with_stdin(args, patch.clone())
            };
            return match side {
                PatchSide::Worktree => vec![reverse(false)],
                PatchSide::Index => vec![reverse(false), reverse(true)],
            };
        }
    };
    let classes = Classes::split(paths, &facts.status);
    let mut plan = Vec::new();
    if matches!(facts.head, HeadRef::Unborn(_)) {
        if !classes.staged.is_empty() {
            plan.push(Invocation::new(with_paths(
                vec![
                    GitArg::Flag("rm"),
                    GitArg::Flag("--cached"),
                    GitArg::Flag("--quiet"),
                ],
                &classes.staged,
            )));
        }
        let mut to_clean = classes.staged;
        to_clean.extend(classes.untracked);
        if !to_clean.is_empty() {
            plan.push(clean(&to_clean));
        }
        return plan;
    }
    if !classes.staged.is_empty() {
        plan.push(Invocation::new(with_paths(
            vec![
                GitArg::Flag("restore"),
                GitArg::Flag("--staged"),
                GitArg::Flag("--worktree"),
                GitArg::Flag("--source=HEAD"),
            ],
            &classes.staged,
        )));
    }
    if !classes.unstaged_only.is_empty() {
        plan.push(Invocation::new(with_paths(
            vec![GitArg::Flag("restore"), GitArg::Flag("--worktree")],
            &classes.unstaged_only,
        )));
    }
    if !classes.untracked.is_empty() {
        plan.push(clean(&classes.untracked));
    }
    plan
}

/// The paths of a discard, by what `git` must do to each.
#[derive(Default)]
struct Classes {
    /// Tracked, with a staged change (and maybe an unstaged one too).
    staged: Vec<PathBuf>,
    /// Tracked, with only an unstaged change.
    unstaged_only: Vec<PathBuf>,
    /// Not in the index.
    untracked: Vec<PathBuf>,
}

impl Classes {
    fn split(paths: &[PathBuf], status: &WorkingTreeStatus) -> Self {
        let mut classes = Self::default();
        for path in paths {
            // `check` already refused a path the status does not list; an
            // unlisted one here has nothing to discard, so it plans nothing.
            let Some(entry) = status.entry(path) else {
                continue;
            };
            let class = if entry.untracked {
                &mut classes.untracked
            } else if entry.is_staged() {
                &mut classes.staged
            } else {
                &mut classes.unstaged_only
            };
            class.push(entry.path.clone());
            class.extend(entry.original_path.clone());
        }
        classes
    }
}

/// `clean -f -- <paths>`. With a pathspec `-d` is irrelevant (git's own words;
/// verified: a named untracked directory goes without it).
fn clean(paths: &[PathBuf]) -> Invocation {
    Invocation::new(with_paths(
        vec![GitArg::Flag("clean"), GitArg::Flag("-f")],
        paths,
    ))
}

/// The target paths plus, for a renamed or copied entry, the path it came
/// from — `restore --staged -- new` alone would leave `old` staged as deleted.
fn pathspec(paths: &[PathBuf], status: &WorkingTreeStatus) -> Vec<PathBuf> {
    let mut spec = Vec::with_capacity(paths.len());
    for path in paths {
        spec.push(path.clone());
        if let Some(original) = status.entry(path).and_then(|e| e.original_path.clone()) {
            spec.push(original);
        }
    }
    spec
}

/// `git --literal-pathspecs <args> -- <paths>`: each path is exactly the
/// file the status listed. Without the flag `*`, `?`, `[` and a leading `:`
/// are pathspec magic even after `--` — verified: `add -- 'a*b'` also stages
/// `ab` and `acb`, and `clean -f` would remove them; with the flag each
/// command takes `a*b` alone.
pub(super) fn with_paths(mut args: Vec<GitArg>, paths: &[PathBuf]) -> Vec<GitArg> {
    args.insert(0, GitArg::Flag("--literal-pathspecs"));
    args.push(GitArg::Separator);
    args.extend(
        paths
            .iter()
            .map(|path| GitArg::Path(path.as_os_str().as_bytes().to_vec())),
    );
    args
}

/// `-` as `git apply`'s "read the patch from stdin".
fn stdin_arg() -> GitArg {
    GitArg::Word("-".to_owned())
}

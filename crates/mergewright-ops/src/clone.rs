//! `git clone` into a folder, from the empty state's `c`
//! (m8-polish-and-ship/18a; rulings R1, R3).
//!
//! **Not an [`Operation`](crate::operation::Operation)**, as [`init`](crate::init):
//! there is no repository, so no journal and no undo. It runs through the
//! same remote runner as fetch, pull and push ([`RemoteRun`]), so a
//! credential prompt and progress work exactly as they do there.
//!
//! **Mergewright never deletes the folder.** On a failure or a stop git removes
//! what it made (from its signal handler and `atexit`); if something is
//! still there, the error says so (`left`) and the user decides.

use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use mergewright_git::remote::RemoteFailure;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::apply::AfterState;
use crate::apply::remote::{self, RemoteHooks, RemoteRun};
use crate::operation::Invocation;

/// What to clone, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloneParams {
    /// The remote, as the user typed it.
    pub url: String,
    /// The folder the clone becomes. Absolute; its parent exists. It may
    /// exist already only as an empty folder.
    pub into: PathBuf,
}

/// A finished clone.
#[derive(Debug)]
pub struct Cloned {
    /// The new repository's working tree: `CloneParams::into`.
    pub path: PathBuf,
    /// The credential prompt's socket could not be cleaned up after git
    /// exited. The clone itself is whole; this is reported, never dropped.
    pub askpass: Option<std::io::Error>,
}

/// Why a clone did not happen. `left` is the folder git left behind, when it
/// did: `Some` only when the folder is there after the failure and git made
/// it (or it was empty before and now holds files).
#[derive(Debug)]
pub enum CloneError {
    /// Empty, starts with `-` (it would read as an option), or no folder
    /// name can be taken from it. Nothing ran.
    BadUrl,
    /// The target exists and is not an empty folder. Nothing ran.
    TargetNotEmpty(PathBuf),
    /// git ran and failed: its failure, in the remote operations' terms.
    Remote {
        /// What git's stderr says went wrong.
        failure: RemoteFailure,
        /// The last few stderr segments, for a detail view.
        stderr_tail: Vec<String>,
        /// The folder git left behind, if any.
        left: Option<PathBuf>,
    },
    /// The user stopped it.
    Stopped {
        /// The folder git left behind, if any.
        left: Option<PathBuf>,
    },
    /// git could not run, or ended some other way (a signal).
    Run {
        /// Why.
        error: RunError,
        /// The folder git left behind, if any.
        left: Option<PathBuf>,
    },
}

impl CloneError {
    /// The one line the empty state shows. Not sanitised: the app does that,
    /// as for `ApplyError`.
    #[must_use]
    pub fn explain(&self) -> String {
        let (said, left) = match self {
            CloneError::BadUrl => return "can't name a folder from this URL".to_owned(),
            CloneError::TargetNotEmpty(path) => {
                let name = path.file_name().unwrap_or(path.as_os_str());
                return format!("{} already exists here", name.to_string_lossy());
            }
            CloneError::Remote { failure, left, .. } => {
                (format!("clone failed: {}", remote::short(failure)), left)
            }
            // The stop's words are "nothing kept" only when that is true.
            CloneError::Stopped { left: Some(path) } => {
                return format!("clone stopped · {} was left behind", path.display());
            }
            CloneError::Stopped { left: None } => return "clone stopped · nothing kept".to_owned(),
            CloneError::Run { error, left } => (format!("clone: {error}"), left),
        };
        match left {
            Some(path) => format!("{said} · {} was left behind", path.display()),
            None => said,
        }
    }
}

/// The folder name a clone of `url` gets, as git would pick it: the last
/// path part, `.git` removed. `None` when there is no usable name.
///
/// Trailing whitespace and `/` go; then a trailing `/.git`, then `.git`.
/// The name is what follows the last `/` of the path — for a `scheme://`
/// URL the path after the host, so `https://h/` has none — or, in scp form
/// (`host:path`, no `://`) with no `/`, what follows the `:`. Empty, `.`,
/// `..` or a control character is no name.
#[must_use]
pub fn name_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim_end().trim_end_matches('/');
    let trimmed = trimmed.strip_suffix("/.git").unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let (path, scp) = match trimmed.split_once("://") {
        // After the scheme: the host, then the path. No `/` after the host
        // is no path, so no name.
        Some((_scheme, rest)) => (rest.split_once('/').map(|(_host, path)| path)?, false),
        None => (trimmed, true),
    };
    let name = match path.rsplit_once('/') {
        Some((_, last)) => last,
        None if scp => path.rsplit_once(':').map_or(path, |(_, last)| last),
        None => path,
    };
    if name.is_empty() || name == "." || name == ".." || name.chars().any(char::is_control) {
        return None;
    }
    Some(name.to_owned())
}

/// Clone `params.url` into `params.into`: `git clone --progress -- <url>
/// <into>`, run in `into`'s parent through [`GitInvocation::in_folder`] and
/// the remote runner, with `hooks` (`None`: headless, git asks nothing).
///
/// Refused before anything runs — the stop is not polled: [`CloneError::BadUrl`],
/// then [`CloneError::TargetNotEmpty`].
///
/// # Errors
/// See [`CloneError`].
pub fn clone(
    params: &CloneParams,
    hooks: Option<&RemoteHooks>,
    stop: &dyn Stop,
) -> Result<Cloned, CloneError> {
    let url = params.url.as_str();
    if url.is_empty() || url.starts_with('-') || name_from_url(url).is_none() {
        return Err(CloneError::BadUrl);
    }
    let into = params.into.as_path();
    let existed = into.exists();
    if existed && !is_empty_folder(into) {
        return Err(CloneError::TargetNotEmpty(into.to_path_buf()));
    }

    let args = vec![
        GitArg::Flag("clone"),
        GitArg::Flag("--progress"),
        GitArg::Separator,
        GitArg::Word(url.to_owned()),
        GitArg::Path(into.as_os_str().as_bytes().to_vec()),
    ];
    // A path with no parent is the root, which is never an empty folder and
    // was refused above; `into` itself stands in only to keep this total.
    let parent = into.parent().unwrap_or(into);
    let git = GitInvocation::in_folder(parent).args(args.iter().cloned());
    let invocation = Invocation {
        args,
        stdin: None,
        remote: true,
        index: crate::operation::IndexFile::Real,
    };
    let mut cleanup = Vec::new();
    let ran = remote::run(RemoteRun { hooks, stop }, git, &invocation, &mut cleanup);
    // The runner pushes only `AskPass` (`apply/remote.rs`, its one `push`);
    // nothing else can be in `cleanup` to lose.
    let askpass = cleanup.into_iter().find_map(|after| match after {
        AfterState::AskPass(error) => Some(error),
        _ => None,
    });

    match ran {
        Ok(()) => Ok(Cloned {
            path: into.to_path_buf(),
            askpass,
        }),
        // Ignored on a failure, deliberately: `CloneError` (as the ticket
        // gives it) has no slot for it, and a leftover askpass socket file in
        // the temp folder is harmless next to the failure the user is told.
        // Flagged in 18a's NOTES as a question.
        Err(failed) => {
            let left = left_behind(into, existed);
            Err(match failed.error {
                RunError::Stopped => CloneError::Stopped { left },
                RunError::Exited { .. } => CloneError::Remote {
                    failure: remote::failure_of(&failed.error, &failed.segments),
                    stderr_tail: remote::tail(&failed.segments),
                    left,
                },
                error => CloneError::Run { error, left },
            })
        }
    }
}

/// `into` is a folder with nothing in it. Unreadable counts as not empty:
/// the clone is refused rather than run over something it cannot see.
fn is_empty_folder(into: &Path) -> bool {
    std::fs::read_dir(into).is_ok_and(|mut entries| entries.next().is_none())
}

/// What git left at `into` after a failure: the folder, when it is there and
/// git made it, or it was empty before and is not now.
fn left_behind(into: &Path, existed: bool) -> Option<PathBuf> {
    let there = into.exists() && (!existed || !is_empty_folder(into));
    there.then(|| into.to_path_buf())
}

#[cfg(test)]
mod tests;

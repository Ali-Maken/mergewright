//! What the commit editor needs from the repository: `commit.template`'s
//! contents and `HEAD`'s message for an amend
//! (m3-working-tree-undo-and-stash/05).
//!
//! Under `jobs/` because it builds [`GitInvocation`]s: reads, not writes, but
//! the seam guard (`crates/mergewright/tests/it/seam_rule.rs`) draws the line by
//! directory rather than by intent, and that is the cheaper rule to keep.
//!
//! Every function here returns the failure as text ready to show, the way
//! [`crate::working_tree::read`] does: it crosses a thread as part of a job
//! result and there is one thing every caller does with it.

use std::path::{Path, PathBuf};

use mergewright_git::repo::{HeadState, RefTarget};
use mergewright_git::run::{GitArg, GitInvocation, RunError};

use crate::repos::RepoStore;

/// `commit.template`'s contents, or `None` when the config names no template.
///
/// `--path` so git expands `~` and `~user` itself; a template that is
/// configured but missing is a failure the user should see, not a silent
/// empty body.
///
/// # Errors
/// git could not be run, or the configured template could not be read.
pub(super) fn template(store: &RepoStore) -> Result<Option<String>, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let configured = GitInvocation::new(&handle)
        .args([
            GitArg::Flag("config"),
            GitArg::Flag("--get"),
            GitArg::Flag("--path"),
            GitArg::Flag("commit.template"),
        ])
        .run(&());
    let output = match configured {
        Ok(output) => output,
        // `git config --get` exits 1 for "no such key". That is an answer,
        // not a failure; every other exit code is reported.
        Err(RunError::Exited { status: 1, .. }) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if path.is_empty() {
        return Ok(None);
    }
    let path = absolute(&handle, Path::new(&path));
    std::fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| format!("{}: {error}", path.display()))
}

/// A relative `commit.template` is relative to the working tree, the way git
/// resolves it.
fn absolute(handle: &mergewright_git::repo::RepoHandle, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    handle
        .workdir()
        .map_or_else(|| path.to_path_buf(), |root| root.join(path))
}

/// `HEAD`'s commit message, for an amend's prefill.
///
/// # Errors
/// `HEAD` is unborn (nothing to amend), or it could not be read.
pub(super) fn head_message(store: &RepoStore) -> Result<String, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let id = match handle.head().map_err(|error| error.to_string())? {
        HeadState::Detached(id) => id,
        HeadState::Branch(name) => {
            match handle.reference(&name).map_err(|error| error.to_string())? {
                Some(RefTarget::Object(id)) => id,
                Some(RefTarget::Symbolic(to)) => {
                    return Err(format!(
                        "HEAD points at {}, which points at another ref",
                        String::from_utf8_lossy(to.as_bytes())
                    ));
                }
                None => return Err("HEAD names a branch that does not exist".to_owned()),
            }
        }
        HeadState::Unborn(_) => {
            return Err("nothing committed yet, so there is nothing to amend".to_owned());
        }
    };
    let commit =
        mergewright_git::history::read_commit(&handle, id).map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&commit.message).into_owned())
}

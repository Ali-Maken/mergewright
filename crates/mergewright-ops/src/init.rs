//! `git init` in a folder that is not a repository yet — the empty state's
//! `i` (m8-polish-and-ship/08, keymap §2.13).
//!
//! **Not an [`Operation`](crate::operation::Operation)**: it is not journaled
//! and not undoable. There is no journal before a repository exists, and the
//! only way to reverse it would be deleting `.git`, which Mergewright never does.
//! It lives here rather than in the app because of the seam rule: the app
//! never builds a write's `GitInvocation`.

use std::path::{Path, PathBuf};

use mergewright_git::run::{GitArg, GitInvocation};

/// Why a folder could not be made a repository. `Display` is the line the
/// empty state's bar row reads.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum InitError {
    /// The folder does not exist (or is not a directory). Nothing is created:
    /// a typo must not become a repository.
    #[error("init: no such folder {}", .0.display())]
    Missing(PathBuf),
    /// git refused or could not run; its first stderr line, or the reason.
    #[error("init: {0}")]
    Git(String),
}

/// Run `git init` in `dir`. `Ok` is the new repository's working tree
/// (`dir` itself, canonicalised).
///
/// Runs through [`GitInvocation::in_folder`], so an inherited `GIT_DIR` or
/// `GIT_WORK_TREE` cannot send it elsewhere. Synchronous: a local init takes
/// milliseconds, and the empty state has no job runner.
///
/// # Errors
/// [`InitError::Missing`] when `dir` is not a directory, [`InitError::Git`]
/// when git fails.
pub fn init_here(dir: &Path) -> Result<PathBuf, InitError> {
    if !dir.is_dir() {
        return Err(InitError::Missing(dir.to_path_buf()));
    }
    GitInvocation::in_folder(dir)
        .args([GitArg::Flag("init"), GitArg::Flag("--quiet")])
        .run(&())
        .map_err(|error| InitError::Git(error.to_string()))?;
    dir.canonicalize()
        .map_err(|error| InitError::Git(format!("{}: {error}", dir.display())))
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        reason = "a scratch directory that cannot be made leaves nothing to test"
    )]

    use super::{InitError, init_here};

    #[test]
    fn init_here_creates_a_git_directory_in_the_folder() {
        let dir = tempfile::tempdir().expect("a scratch directory");

        let made = init_here(dir.path()).expect("git init succeeds");

        assert!(dir.path().join(".git").is_dir(), "no .git in the folder");
        assert_eq!(made, dir.path().canonicalize().expect("canonical"));
    }

    #[test]
    fn a_missing_folder_is_missing_and_nothing_is_created() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let missing = dir.path().join("typo");

        let answer = init_here(&missing);

        assert_eq!(answer, Err(InitError::Missing(missing.clone())));
        assert!(!missing.exists(), "a typo became a folder");
    }

    /// A stray `GIT_DIR` (a hook, a script) must not redirect the init: the
    /// folder the user was shown is the one that becomes a repository.
    /// Checked through a child process of git with the variable set, the
    /// same way the app's own process would hand it down.
    #[test]
    fn a_stray_git_dir_in_the_environment_does_not_redirect_it() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let target = dir.path().join("here");
        let elsewhere = dir.path().join("elsewhere.git");
        std::fs::create_dir(&target).expect("target folder");

        // Set for this test's process only through the invocation's own
        // child: `in_folder` must remove what it inherits, so the variable
        // is put in the parent environment of a re-run of this test binary.
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "init::tests::child_inits_target_from_env",
                "--nocapture",
                "--include-ignored",
            ])
            .env("GIT_DIR", &elsewhere)
            .env("GIT_WORK_TREE", dir.path())
            .env("MWR_INIT_TARGET", &target)
            .status()
            .expect("the child test runs");

        assert!(status.success(), "the child init failed");
        assert!(
            target.join(".git").is_dir(),
            "the folder was not made a repository"
        );
        assert!(!elsewhere.exists(), "GIT_DIR redirected the init");
    }

    /// The child half of the test above; does nothing unless it was started
    /// by it.
    #[test]
    #[ignore = "run only as the child of a_stray_git_dir_in_the_environment_does_not_redirect_it"]
    fn child_inits_target_from_env() {
        let Some(target) = std::env::var_os("MWR_INIT_TARGET") else {
            return;
        };
        assert!(
            std::env::var_os("GIT_DIR").is_some(),
            "the parent set GIT_DIR"
        );
        init_here(std::path::Path::new(&target)).expect("init succeeds");
    }
}

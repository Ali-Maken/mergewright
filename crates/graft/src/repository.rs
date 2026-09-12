//! What Graft reads from a repository at startup, and keeps.
//!
//! A [`RepoHandle`] cannot live in [`AppState`](crate::update::AppState): it is
//! not `Send`, the state is cloned on every event, and `update` must stay pure.
//! So the job reads what the screen needs, hands back these plain facts, and
//! drops the handle. M1's rev-walk is the ticket that needs a handle to stay
//! open, and owns that decision (KNOWN_GAPS.md).

use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_git::repo::{HeadState, RepoHandle};

/// What one open repository told Graft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryFacts {
    /// The working tree's root, or the repository directory itself when it is
    /// bare.
    pub path: PathBuf,
    /// No working tree.
    pub bare: bool,
    /// Where `HEAD` points.
    pub head: Head,
}

/// Where `HEAD` points — the three states a repository can be in, plus the one
/// where it could not be read.
///
/// Names are carried as the bytes git stored, not as `String`: a ref name is
/// only *mostly* UTF-8, and deciding what to do about that is the screen's
/// business, one step before the sanitiser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// On a branch with commits on it, by its shorthand (`main`).
    Branch(Vec<u8>),
    /// Not on a branch: this commit.
    Detached(Oid),
    /// On a branch that does not exist yet, because nothing is committed.
    Unborn(Vec<u8>),
    /// `HEAD` could not be read, and this is why. A repository can open and
    /// still have an unreadable `HEAD` — a ref file with a typo in it — and
    /// showing an empty row instead of the reason would be hiding a failure.
    Unreadable(String),
}

/// Open the repository at `path` and read what the status screen shows.
///
/// # Errors
/// The failure text, ready to show: the path is not a repository, or the
/// repository is there but cannot be used. The message is formed here rather
/// than passed on as a typed error because it crosses a thread as part of a
/// job result, and there is exactly one thing every caller does with it —
/// tell the user.
pub fn facts(path: &Path) -> Result<RepositoryFacts, String> {
    let repository = RepoHandle::open(path).map_err(|error| error.to_string())?;

    let location = repository
        .workdir()
        .unwrap_or_else(|| repository.git_dir())
        .to_path_buf();
    // Made absolute, because the path Graft was given is usually `.` and a
    // screen that says the repository is at `.` tells the user nothing.
    // Absolute rather than canonical: symlinks are left alone, so a user whose
    // home is a symlink sees the path they know rather than the one behind it.
    let location = std::path::absolute(&location)
        .map_err(|error| format!("cannot work out where {} is: {error}", location.display()))?;

    Ok(RepositoryFacts {
        path: location,
        bare: repository.is_bare(),
        head: match repository.head() {
            Ok(HeadState::Branch(name)) => Head::Branch(name.shorthand().to_vec()),
            Ok(HeadState::Detached(id)) => Head::Detached(id),
            Ok(HeadState::Unborn(name)) => Head::Unborn(name.shorthand().to_vec()),
            Err(error) => Head::Unreadable(error.to_string()),
        },
    })
    // The handle is dropped here, on the worker thread that opened it.
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// `git init` in a temporary directory. Real `git`, because the shape of a
    /// `.git` directory is easy to write down wrongly, and cheap enough for
    /// the fast tier (PROMPT.md §7.1).
    fn initialised(arguments: &[&str]) -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let status = Command::new("git")
            .arg("init")
            .args(arguments)
            .arg("-b")
            .arg("main")
            .current_dir(directory.path())
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .status()
            .expect("git is installed — Graft does not run without it");
        assert!(status.success(), "git init failed");
        directory
    }

    #[test]
    fn a_path_that_is_not_a_repository_says_so() {
        let directory = tempfile::tempdir().expect("a temporary directory");

        let error = facts(directory.path()).expect_err("an empty directory is not a repository");

        assert!(error.contains("not inside a git repository"), "{error}");
    }

    #[test]
    fn a_fresh_repository_is_on_a_branch_with_no_commits() {
        let directory = initialised(&[]);
        let expected = directory.path().to_path_buf();

        let facts = facts(directory.path()).expect("git init makes a repository");

        assert_eq!(facts.path, expected);
        assert!(!facts.bare);
        assert_eq!(facts.head, Head::Unborn(b"main".to_vec()));
    }

    #[test]
    fn a_bare_repository_reports_its_git_directory() {
        let directory = initialised(&["--bare"]);
        let expected = directory.path().to_path_buf();

        let facts = facts(directory.path()).expect("git init --bare makes a repository");

        assert!(facts.bare);
        // No working tree, so the path shown is the repository itself.
        assert_eq!(facts.path, expected);
    }
}

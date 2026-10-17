//! What Graft reads from a repository at startup, and keeps.
//!
//! A [`RepoHandle`] cannot live in [`AppState`](crate::update::AppState): it is
//! not `Send`, the state is cloned on every event, and `update` must stay pure.
//! So the job borrows a handle from the [`RepoStore`](crate::repos::RepoStore),
//! reads what the screen needs, hands back these plain facts, and drops the
//! handle. The repository behind it stays open.

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::commit_graph::{self, CommitGraphStatus, Offer};
use graft_git::mailmap::Mailmap;
use graft_git::repo::{HeadState, RepoHandle};
use graft_git::shallow::ShallowList;

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
    /// The commit `HEAD` resolves to; `None` while unborn, or when `HEAD`
    /// could not be read or resolved (then `head` is
    /// [`Head::Unreadable`] and says why). The history walk starts with it
    /// kept out of the fold (ease-pass 06 D2).
    pub head_commit: Option<Oid>,
    /// The repository's resolved `.mailmap`, for the author chip
    /// (m1-the-graph/10).
    pub mailmap: Mailmap,
    /// Whether history has a commit-graph to sort by, and whether the user
    /// declined Graft's offer to build one (m1-the-graph/54, ADR-0008).
    pub commit_graph: CommitGraphFact,
}

/// The commit-graph file, as the offer needs to know it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitGraphFact {
    /// A commit-graph covering this many commits: nothing to offer.
    Present {
        /// Commits the file holds.
        commits: u32,
    },
    /// No commit-graph; `declined` when the user said "never for this
    /// repository".
    Absent {
        /// The decline file is there.
        declined: bool,
    },
    /// A shallow clone: git and Graft read no commit-graph here, so none is
    /// offered (m7-power-tools/20j; git's `commit_graph_compatible`).
    Shallow,
    /// The file, or the decline file, could not be read, and this is why.
    /// Shown as a problem; nothing is offered.
    Unreadable(String),
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

/// Read what the status screen shows from an open repository.
///
/// # Errors
/// The failure text, ready to show. The message is formed here rather than
/// passed on as a typed error because it crosses a thread as part of a job
/// result, and there is exactly one thing every caller does with it — tell
/// the user.
pub fn facts(repository: &RepoHandle) -> Result<RepositoryFacts, String> {
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

    let head = match repository.head() {
        Ok(HeadState::Branch(name)) => Head::Branch(name.shorthand().to_vec()),
        Ok(HeadState::Detached(id)) => Head::Detached(id),
        Ok(HeadState::Unborn(name)) => Head::Unborn(name.shorthand().to_vec()),
        Err(error) => Head::Unreadable(error.to_string()),
    };
    // A `HEAD` that reads but does not resolve is unreadable too: the
    // reason goes where the status screen shows `HEAD`, never dropped.
    let (head, head_commit) = match (head, repository.head_commit()) {
        (Head::Unreadable(why), _) => (Head::Unreadable(why), None),
        (head, Ok(commit)) => (head, commit),
        (_, Err(error)) => (Head::Unreadable(error.to_string()), None),
    };
    Ok(RepositoryFacts {
        path: location,
        bare: repository.is_bare(),
        head,
        head_commit,
        mailmap: repository.mailmap(),
        commit_graph: commit_graph_fact(repository),
    })
    // The handle is dropped here, on the worker thread that opened it.
}

/// What the commit-graph offer needs: the file's status, and the decline
/// only when there is no file to speak of. A shallow clone is its own fact:
/// a commit-graph written there would never be read.
fn commit_graph_fact(repository: &RepoHandle) -> CommitGraphFact {
    match ShallowList::read(repository) {
        Ok(list) if !list.is_empty() => return CommitGraphFact::Shallow,
        Ok(_) => {}
        Err(error) => return CommitGraphFact::Unreadable(error.to_string()),
    }
    match commit_graph::status(repository) {
        Ok(CommitGraphStatus::Present { commits }) => CommitGraphFact::Present { commits },
        Ok(CommitGraphStatus::Absent) => match commit_graph::offer_state(repository) {
            Ok(offer) => CommitGraphFact::Absent {
                declined: offer == Offer::Declined,
            },
            Err(error) => CommitGraphFact::Unreadable(format!(
                "cannot read the commit-graph offer choice: {error}"
            )),
        },
        Err(error) => CommitGraphFact::Unreadable(error.to_string()),
    }
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
    fn a_fresh_repository_is_on_a_branch_with_no_commits() {
        let directory = initialised(&[]);
        let expected = directory.path().to_path_buf();

        let handle = RepoHandle::open(directory.path()).expect("git init makes a repository");
        let facts = facts(&handle).expect("an open repository has facts");

        assert_eq!(facts.path, expected);
        assert!(!facts.bare);
        assert_eq!(facts.head, Head::Unborn(b"main".to_vec()));
        assert_eq!(
            facts.commit_graph,
            CommitGraphFact::Absent { declined: false }
        );
    }

    #[test]
    fn a_bare_repository_reports_its_git_directory() {
        let directory = initialised(&["--bare"]);
        let expected = directory.path().to_path_buf();

        let handle =
            RepoHandle::open(directory.path()).expect("git init --bare makes a repository");
        let facts = facts(&handle).expect("an open repository has facts");

        assert!(facts.bare);
        // No working tree, so the path shown is the repository itself.
        assert_eq!(facts.path, expected);
    }
}

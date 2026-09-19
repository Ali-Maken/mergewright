//! A throwaway repository and the "read the repository back" helpers every
//! execution test asserts through.
//!
//! Nothing here trusts a return value: an operation is judged by what `git`
//! says the repository holds afterwards (PROMPT.md §7.2 item 4).

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use graft_core::journal::EntryId;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::GitVersion;
use graft_ops::apply::Context;
use graft_ops::journal::{Journal, journal_path};

/// A repository in a temporary directory, removed when it drops.
pub struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    /// A fresh `git init` with a hermetic local config: identity set, and the
    /// global-config knobs that would otherwise reach in (signing, CRLF
    /// translation) turned off locally, where local wins.
    pub fn init() -> TempRepo {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "graft-ops-exec-{}-{unique}",
            std::process::id()
        ));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("an old scratch clears");
        }
        std::fs::create_dir_all(&path).expect("the scratch is made");
        let repo = TempRepo { path };
        repo.git(&["init", "-q", "-b", "main", "."]);
        repo.harden(Path::new("."));
        repo
    }

    /// The same hermetic config, applied to a linked worktree's own git dir.
    pub fn harden(&self, relative: &Path) {
        for (key, value) in [
            ("user.name", "Tester"),
            ("user.email", "tester@example.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("core.hooksPath", "/dev/null"),
        ] {
            self.git_in(relative, &["config", key, value]);
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn open(&self) -> RepoHandle {
        RepoHandle::open(&self.path).expect("the scratch repository opens")
    }

    pub fn open_at(&self, relative: &str) -> RepoHandle {
        RepoHandle::open(&self.path.join(relative)).expect("the worktree opens")
    }

    /// Run `git` in the repository root and return its stdout, trimmed.
    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(Path::new("."), args)
    }

    /// Run `git` in a directory under the repository root.
    pub fn git_in(&self, relative: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(self.path.join(relative))
            .args(args)
            .output()
            .unwrap_or_else(|why| panic!("git runs: {why}"));
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim_end().to_owned()
    }

    /// Run `git`, returning `Err(stderr)` instead of panicking.
    pub fn try_git(&self, args: &[&str]) -> Result<String, String> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .args(args)
            .output()
            .unwrap_or_else(|why| panic!("git runs: {why}"));
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim_end().to_owned())
        }
    }

    pub fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.path.join(relative);
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory).expect("the directory is made");
        }
        std::fs::write(path, bytes).expect("the file is written");
    }

    pub fn read(&self, relative: &str) -> Vec<u8> {
        std::fs::read(self.path.join(relative)).expect("the file reads")
    }

    pub fn remove(&self, relative: &str) {
        std::fs::remove_file(self.path.join(relative)).expect("the file is removed");
    }

    pub fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }

    /// A first commit, so `HEAD` is born.
    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", message]);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        // Best effort: a surviving scratch directory is noise in `/tmp`, not
        // a test failure, and panicking in `Drop` would hide the real one.
        let _removed = std::fs::remove_dir_all(&self.path);
    }
}

/// A `Stop` that never stops.
pub struct Never;

impl Stop for Never {
    fn should_stop(&self) -> bool {
        false
    }
}

/// A `Stop` that answers `false` until it has been asked `after` times, then
/// `true` for ever: the user quitting part-way through an operation.
pub struct StopAfter {
    asked: AtomicUsize,
    after: usize,
}

impl StopAfter {
    pub fn new(after: usize) -> StopAfter {
        StopAfter {
            asked: AtomicUsize::new(0),
            after,
        }
    }
}

impl Stop for StopAfter {
    fn should_stop(&self) -> bool {
        self.asked.fetch_add(1, Ordering::Relaxed) >= self.after
    }
}

/// An `EntryId` from a fixed clock, so a test's ids are ordered and readable.
pub fn entry_id(at: i64) -> EntryId {
    EntryId::new(at, [0; 10])
}

/// Open a journal for a repository.
pub fn journal_for(repo: &RepoHandle) -> Journal {
    Journal::open(&journal_path(repo).expect("the journal path resolves")).expect("it opens")
}

/// A `Context` over a repository and its journal.
pub fn context<'a>(
    repo: &'a RepoHandle,
    journal: &'a mut Journal,
    now: i64,
    stop: &'a dyn Stop,
) -> Context<'a> {
    Context::open(repo, git_version(), journal, now, [0; 10], stop).expect("the context opens")
}

/// The system git's version, read once per test.
pub fn git_version() -> GitVersion {
    graft_git::version::system_git()
        .expect("git is on PATH")
        .version()
}

/// Everything about a repository that an undo has to put back exactly:
/// refs, the index, and the bytes of every file in the working tree,
/// untracked ones included.
#[derive(Debug, PartialEq, Eq)]
pub struct RepoState {
    pub refs: String,
    pub index: String,
    pub files: Vec<(String, Vec<u8>)>,
    pub head: String,
}

/// Read that state back out of the repository with `git` and the filesystem.
///
/// Backup refs are left out: they are Graft's own bookkeeping and are meant
/// to accumulate (§6.8), so an undo is not expected to remove them.
pub fn state_of(repo: &TempRepo) -> RepoState {
    let refs = repo
        .git(&["for-each-ref", "--format=%(refname) %(objectname)"])
        .lines()
        .filter(|line| !line.starts_with("refs/graft/"))
        .collect::<Vec<_>>()
        .join("\n");
    let index = repo.git(&["ls-files", "-s"]);
    let head = repo
        .try_git(&["symbolic-ref", "-q", "HEAD"])
        .unwrap_or_else(|_| format!("detached {}", repo.git(&["rev-parse", "HEAD"])));
    RepoState {
        refs,
        index,
        files: files_under(repo.path(), repo.path()),
        head,
    }
}

/// Every file under the working tree, `.git` excluded, path-sorted, with its
/// bytes.
fn files_under(root: &Path, directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(directory).expect("the directory reads");
    for entry in entries {
        let entry = entry.expect("the entry reads");
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        if path.is_dir() {
            found.extend(files_under(root, &path));
        } else {
            let relative = path
                .strip_prefix(root)
                .expect("the path is under the root")
                .to_string_lossy()
                .into_owned();
            found.push((relative, std::fs::read(&path).expect("the file reads")));
        }
    }
    found.sort();
    found
}

/// `git stash list` as `stash@{n}: <commit> <message>` lines.
pub fn stash_list(repo: &TempRepo) -> String {
    repo.git(&["stash", "list", "--format=%H %gs"])
}

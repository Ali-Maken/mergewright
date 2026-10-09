//! A throwaway repository and the "read the repository back" helpers every
//! execution test asserts through.
//!
//! Nothing here trusts a return value: an operation is judged by what `git`
//! says the repository holds afterwards (PROMPT.md §7.2 item 4).

// Helpers, not `#[test]` functions, so `clippy.toml`'s "expect is fine in
// tests" does not reach them. It is fine here for the same reason: a
// scaffold that cannot build the repository has to stop the test loudly,
// and there is no caller to hand an error to.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]
// Every helper here is used; no blanket `dead_code` allowance (m3/03 review, M3).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use graft_core::journal::JournalEntry;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::GitVersion;
use graft_ops::apply::{Applied, ApplyError, Context, apply};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::operation::Operation;
use graft_ops::undo::{self, UndoError};

/// The clock every execution test runs on, so entry ids are readable.
pub const NOW: i64 = 1_700_000_000_000;

/// A repository in a temporary directory, removed when it drops.
pub struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    /// A fresh `git init` with a hermetic local config: identity set, and the
    /// global-config knobs that would otherwise reach in (signing, CRLF
    /// translation) turned off locally, where local wins.
    pub fn init() -> TempRepo {
        let repo = TempRepo::scratch();
        repo.git(&["init", "-q", "-b", "main", "."]);
        repo.harden(Path::new("."));
        repo
    }

    /// A byte-for-byte copy (`cp -R`) in a scratch directory of its own: the
    /// same commits, refs, index and working tree, for running a second
    /// route to the same result beside the first (the rewrite oracle).
    pub fn copy(&self) -> TempRepo {
        let copy = TempRepo::scratch();
        let status = std::process::Command::new("cp")
            .arg("-R")
            .arg(self.path.join("."))
            .arg(&copy.path)
            .status()
            .unwrap_or_else(|why| panic!("cp runs: {why}"));
        assert!(status.success(), "cp -R copies the repository");
        copy
    }

    /// An empty scratch directory, unique to this process and call.
    fn scratch() -> TempRepo {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("graft-ops-exec-{}-{unique}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("an old scratch clears");
        }
        std::fs::create_dir_all(&path).expect("the scratch is made");
        TempRepo { path }
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
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
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
            Ok(String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_owned())
        } else {
            Err(String::from_utf8_lossy(&output.stderr)
                .trim_end()
                .to_owned())
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

    pub fn exists(&self, relative: &str) -> bool {
        self.path.join(relative).exists()
    }

    /// An empty file at every temporary name a raw write (`restore::raw`)
    /// in this process may use in `dir`, so it cannot land: `.~<6 hex>` of
    /// `pid × 16 + attempt`, attempts 0–15, as `temporary_name` builds them.
    /// Returns their repo-relative paths, to check each is left untouched.
    pub fn block_raw_writes_in(&self, dir: &str) -> Vec<String> {
        let pid = std::process::id();
        (0..16u32)
            .map(|attempt| {
                let n = pid.wrapping_mul(16).wrapping_add(attempt) & 0x00ff_ffff;
                let relative = format!("{dir}/.~{n:06x}");
                self.write(&relative, b"");
                relative
            })
            .collect()
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

/// A `Stop` that fires once a path has gone: the user quitting part-way
/// through a plan, keyed to the plan's own effect.
///
/// Counting `should_stop` polls would be a different stop point on every
/// machine and every git version — the runner polls while it waits on the
/// child — so the stop is tied to something the plan itself does. Point it at
/// a file the plan's *first* invocation removes and the *second* is the one
/// that gets killed, every time.
pub struct StopWhenGone {
    path: PathBuf,
}

impl StopWhenGone {
    pub fn new(path: PathBuf) -> StopWhenGone {
        StopWhenGone { path }
    }
}

impl Stop for StopWhenGone {
    fn should_stop(&self) -> bool {
        !self.path.exists()
    }
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

// ── running an operation, and putting it back ───────────────────────────

/// Apply one operation with a fresh context, and return the entry.
pub fn run(repo: &TempRepo, op: &Operation, at: i64) -> JournalEntry {
    try_run(repo, op, at)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .entry
}

/// The same, handing back the error instead of panicking.
pub fn try_run(repo: &TempRepo, op: &Operation, at: i64) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    apply(&mut ctx, op)
}

/// Apply with a stop of the caller's choosing, for the plans that are meant
/// to be cut short.
pub fn run_with_stop(
    repo: &TempRepo,
    op: &Operation,
    at: i64,
    stop: &dyn Stop,
) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, at, stop);
    apply(&mut ctx, op)
}

/// Undo the newest in-force operation.
pub fn run_undo(repo: &TempRepo, at: i64, force: bool) -> Result<Applied, UndoError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    undo::undo(&mut ctx, force)
}

/// Put back the newest undo still in force.
pub fn run_redo(repo: &TempRepo, at: i64, force: bool) -> Result<Applied, UndoError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    undo::redo(&mut ctx, force)
}

/// Apply, undo, redo, undo again — the state has to land back where each
/// step says it should, every time. Both legs matter: an inverse that only
/// ever runs once is half a test (m3/03 review, M2).
pub fn round_trip(repo: &TempRepo, op: &Operation) {
    let before: RepoState = state_of(repo);
    run(repo, op, NOW);
    let after = state_of(repo);
    assert_ne!(before, after, "the operation changed nothing to undo");

    run_undo(repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(repo), before, "undo did not put it back exactly");

    run_redo(repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(repo), after, "redo did not put it forward exactly");

    run_undo(repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("undo 2: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        state_of(repo),
        before,
        "the second undo did not put it back"
    );
}

/// Everything git can reach only through a reflog, gone; everything an
/// unreferenced object, pruned.
///
/// This is `git gc`'s two-week/thirty-day windows brought forward to now. An
/// object a journal entry names has to survive it, or the entry is a promise
/// that expires quietly (m3/03 review, C1).
pub fn expire_and_prune(repo: &TempRepo) {
    repo.git(&[
        "reflog",
        "expire",
        "--expire=now",
        "--expire-unreachable=now",
        "--all",
    ]);
    repo.git(&["gc", "--prune=now", "-q"]);
}

/// Whether the repository still holds an object.
pub fn holds(repo: &TempRepo, id: &str) -> bool {
    repo.try_git(&["cat-file", "-e", id]).is_ok()
}

/// Today's letters for every key graft-ops' words name (ticket m8/10e): the
/// integration tree's one `KeyNames`, so its assertions keep the shipped
/// spelling. Never production — the app spells from his keymap.
pub struct Letters;

impl graft_ops::keys::KeyNames for Letters {
    fn label(&self, key: graft_ops::keys::OpKey) -> Option<String> {
        use graft_ops::keys::OpKey;
        Some(
            match key {
                OpKey::Commit | OpKey::Continue => "c",
                OpKey::Amend | OpKey::Abort => "a",
                OpKey::Stage => "space",
                OpKey::Stash => "S",
                OpKey::Switch => "C",
                OpKey::Rewrite => "R",
                OpKey::Fetch => "f",
                OpKey::Pull => "p",
                OpKey::Push => "P",
                OpKey::Merge => "m",
                OpKey::Rebase => "r",
                OpKey::Undo => "u",
                OpKey::Redo => "U",
            }
            .to_owned(),
        )
    }
}

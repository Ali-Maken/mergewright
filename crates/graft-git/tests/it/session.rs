//! What the blame and trace tests share: the `session` fixture's commits by
//! name, and a `git` to build on it or to ask for the oracle's answer
//! (m7-power-tools/18).

use std::path::Path;
use std::process::{Command, Output};

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;

pub const FILE: &str = "src/auth/session.rs";
pub const OLD_FILE: &str = "src/session.rs";

pub fn open(path: &Path) -> RepoHandle {
    RepoHandle::open(path).unwrap_or_else(|why| panic!("opening {}: {why}", path.display()))
}

/// `git` as the product runs it — user config kept, so the oracle and the
/// reader see the same settings — with a fixed identity for any commit a
/// test makes, and nothing from the environment pointing elsewhere.
pub fn git(dir: &Path, args: &[&str]) -> Output {
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .output()
        .unwrap_or_else(|why| panic!("running git: {why}"))
}

/// [`git`], asserting success; its stdout.
pub fn git_ok(dir: &Path, args: &[&str]) -> Vec<u8> {
    let output = git(dir, args);
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

pub fn rev(dir: &Path, revision: &str) -> Oid {
    let hex = String::from_utf8(git_ok(dir, &["rev-parse", revision]))
        .unwrap_or_else(|why| panic!("rev-parse printed non-UTF-8: {why}"));
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{hex:?}: {why}"))
}

/// The fixture's commits, oldest first.
pub struct Commits {
    pub ana: Oid,
    pub tom: Oid,
    pub lina: Oid,
    pub rustfmt: Oid,
    pub priya: Oid,
    pub you: Oid,
}

pub fn commits(dir: &Path) -> Commits {
    Commits {
        ana: rev(dir, "main~5"),
        tom: rev(dir, "main~4"),
        lina: rev(dir, "main~3"),
        rustfmt: rev(dir, "main~2"),
        priya: rev(dir, "main~1"),
        you: rev(dir, "main"),
    }
}

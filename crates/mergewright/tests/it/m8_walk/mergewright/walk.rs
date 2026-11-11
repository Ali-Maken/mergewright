//! What every Mergewright walk shares: his repository's place, the launch
//! and quit over the pty, the capture with its "what right looks like"
//! line, and the truth checks read from git.

use std::path::{Path, PathBuf};
use std::process::Command;

use mergewright_core::journal::JournalEntry;
use mergewright_ops::journal::Journal;

use crate::pty::{Isolated, PINNED_NOW, Pty, Screen};

/// His repository under the isolated home, drawn `~/code/api`.
pub fn his_repo(isolated: &Isolated) -> PathBuf {
    isolated.home().join("code").join("api")
}

/// `mwr <repo>` at `size`: the graph drawn, the cursor `▸` on a row,
/// nothing still loading.
pub fn launch(isolated: &Isolated, repo: &Path, size: (u16, u16)) -> (Pty, Screen) {
    let pty = Pty::spawn(isolated, repo, isolated.root.path(), size);
    let screen = pty.wait_for("the graph, the cursor on a row, nothing loading", |s| {
        s.has("\u{25b8}")
            && s.lines()[0].contains("api")
            && !s.has("sorting\u{2026}")
            && !s.has("reading\u{2026}")
    });
    (pty, screen)
}

/// `q`, and the app ends cleanly.
pub fn quit(mut pty: Pty) {
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// `J`, and the journal once nothing in it is still loading and its cursor
/// `▸` has landed on a row.
pub fn journal_screen(pty: &mut Pty, has: &str) -> Screen {
    pty.send(b"J");
    pty.wait_for(&format!("J with {has:?}, nothing loading"), |s| {
        s.has(has)
            && !s.has("finding lost commits\u{2026}")
            && s.lines().iter().any(|line| line.starts_with('\u{25b8}'))
    })
}

/// `Esc` out of J, back on the graph (`q` there is not quit).
pub fn close_journal(pty: &mut Pty) {
    pty.send(b"\x1b");
    pty.wait_for("the graph again", |s| !s.lines()[0].starts_with("JOURNAL"));
}

/// The cursor's row.
pub fn cursor_row(screen: &Screen) -> String {
    screen
        .lines()
        .into_iter()
        .find(|line| line.starts_with('\u{25b8}'))
        .unwrap_or_else(|| panic!("no cursor row:\n{}", screen.text()))
}

/// The capture under `docs/shots/`, with what right looks like on top.
pub fn shot(name: &str, right: &str, screen: &Screen) {
    crate::captures::check_shot(
        name,
        &format!(
            "# m8-polish-and-ship/57, `mwr` over a pty (Unicode tier): what right looks like —\n\
             # {right}\n# Colour unverified (rule 5).\n\n{}\n",
            screen.text()
        ),
    );
}

/// `git` in `repo` dated two days before the pinned clock; stdout, trimmed.
pub fn git(repo: &Path, args: &[&str]) -> String {
    git_at(repo, PINNED_NOW - 2 * 86_400, args)
}

/// `git` in `repo` with its dates at `secs`; stdout, trimmed.
pub fn git_at(repo: &Path, secs: i64, args: &[&str]) -> String {
    let date = format!("{secs} +0000");
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_DATE", &date)
        .env("GIT_COMMITTER_DATE", &date)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `git init` in a new folder `repo` on `main`, with an identity and the
/// local settings the machine's global config would otherwise reach into.
pub fn init(repo: &Path) {
    std::fs::create_dir_all(repo).unwrap_or_else(|why| panic!("{}: {why}", repo.display()));
    git(repo, &["init", "-q", "-b", "main", "."]);
    for (key, value) in [
        ("user.name", "Test Maker"),
        ("user.email", "test@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("core.autocrlf", "false"),
        ("core.hooksPath", "/dev/null"),
    ] {
        git(repo, &["config", key, value]);
    }
}

/// The journal as the app reads it, under the current names.
pub fn journal(repo: &Path) -> Vec<JournalEntry> {
    let handle = mergewright_git::repo::RepoHandle::open(repo)
        .unwrap_or_else(|why| panic!("the repo opens: {why:?}"));
    let path = mergewright_ops::journal::journal_path(&handle)
        .unwrap_or_else(|why| panic!("the journal path: {why:?}"));
    Journal::open(&path)
        .unwrap_or_else(|why| panic!("the journal opens: {why:?}"))
        .entries()
        .to_vec()
}

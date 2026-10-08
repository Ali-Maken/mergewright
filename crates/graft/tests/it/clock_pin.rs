//! `GRAFT_NOW` pins the binary's session clock (m8-polish-and-ship/19d-b):
//! the drawn ages and the journal's stamps, for the whole run. A value that
//! is not whole seconds refuses to launch.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::path::Path;

use crate::pty::{Isolated, Pty, Screen};

const SIZE: (u16, u16) = (120, 30);

/// 2024-01-01 and 2025-01-01, UTC: a year apart. Against the fixtures'
/// `FIXED_DATE` (2021-01-01) the label rounds them to `4y` and `5y`.
const EARLIER: i64 = 1_704_067_200;
const A_YEAR_LATER: i64 = 1_735_689_600;

/// Launch on `repo` with `GRAFT_NOW` = `now`.
fn launch(isolated: &Isolated, repo: &Path, now: &str) -> Pty {
    Pty::spawn_with_env(
        isolated,
        repo,
        repo.parent().expect("a temp repo has a parent"),
        SIZE,
        &[("GRAFT_NOW", now.into())],
    )
}

/// Whether any word on screen is exactly `age` (`3y` but not `13y`).
fn draws_age(screen: &Screen, age: &str) -> bool {
    screen.lines().iter().any(|line| {
        line.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == age)
    })
}

fn age_drawn_at(now: i64, age: &str) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let isolated = Isolated::new();
    let mut pty = launch(&isolated, scratch.path(), &now.to_string());
    let screen = pty.wait_for(&format!("a commit row aged {age}"), |s| draws_age(s, age));
    for other in ["3y", "4y", "5y", "6y"].into_iter().filter(|o| *o != age) {
        assert!(!draws_age(&screen, other), "{other}: {}", screen.text());
    }
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// The same repository, two pins a year apart: two different ages on the
/// same commit row.
#[test]
fn graft_now_pins_the_drawn_age() {
    age_drawn_at(EARLIER, "4y");
    age_drawn_at(A_YEAR_LATER, "5y");
}

/// Not whole seconds: exit 2, the words on stderr, and the terminal never
/// taken (no alternate screen).
#[test]
fn a_bad_graft_now_refuses_to_launch() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let isolated = Isolated::new();
    let pty = launch(&isolated, scratch.path(), "1.5");
    let (status, stderr, output) = pty.wait_exit();
    assert_eq!(status.code(), Some(2), "stderr: {stderr}");
    assert_eq!(
        stderr,
        "graft: GRAFT_NOW must be whole seconds since 1970, got 1.5\n"
    );
    assert_eq!(
        crate::pty::count(&output, b"\x1b[?1049h"),
        0,
        "no screen: {:?}",
        String::from_utf8_lossy(&output)
    );
}

/// Every entry stamped in the same pinned second, and `u` still takes the
/// newest: two commits, then `u` undoes the second, not the first.
#[test]
fn u_under_a_pinned_clock_undoes_the_newest() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let isolated = Isolated::new();
    // Both files before launch: the app reads them at start. Each round
    // stages one in the changes pane (`⏎` on the top row, `space`) and
    // commits it (`c`, a message, `⏎`), as the tour's tasks 1-2 do.
    std::fs::write(repo.join("README.md"), "edited\n").expect("the scratch repo is writable");
    std::fs::write(repo.join("notes.txt"), "edited\n").expect("the scratch repo is writable");
    let mut pty = launch(&isolated, repo, &EARLIER.to_string());
    pty.wait_for("the graph", |s| {
        draws_age(s, "4y") && s.has("Uncommitted changes")
    });

    for (left, message) in [(2, "First pinned"), (1, "Second pinned")] {
        pty.send(b"k");
        pty.wait_for("the cursor on the changes row", |s| {
            s.lines()
                .iter()
                .any(|l| l.contains('\u{25b8}') && l.contains("Uncommitted changes"))
        });
        pty.send(b"\r");
        let unstaged = format!("UNSTAGED ({left})");
        pty.wait_for("the changes pane", |s| {
            s.has("space stage") && s.has(&unstaged)
        });
        pty.send(b" ");
        pty.wait_for("one staged", |s| s.has("STAGED (1)") && !s.has(&unstaged));
        pty.send(b"\x1b");
        pty.wait_for("the graph's own keys", |s| {
            s.has("\u{23ce} changes \u{b7} c commit")
        });
        pty.send(b"c");
        pty.wait_for("the commit editor", |s| s.has("\u{23ce} commit"));
        pty.send(message.as_bytes());
        pty.send(b"\r");
        let tip = format!("main] {message}");
        pty.wait_for(&format!("{message} the tip of main"), |s| s.has(&tip));
    }

    let entries = journal(repo);
    // Stage, commit, stage, commit: four entries in one pinned second.
    assert_eq!(entries.len(), 4, "{entries:?}");
    for entry in &entries {
        assert_eq!(entry.at_unix_ms, EARLIER * 1_000, "{entry:?}");
    }

    pty.send(b"u");
    pty.wait_for("the second commit undone", |s| {
        s.has("main] First pinned") && !s.has("main] Second pinned")
    });
    let head = git_out(repo, &["log", "-1", "--format=%s"]);
    assert_eq!(head.trim(), "First pinned");

    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

fn journal(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    let handle = graft_git::repo::RepoHandle::open(repo).expect("repo opens");
    let path = graft_ops::journal::journal_path(&handle).expect("journal path");
    let bytes = std::fs::read(&path).expect("the commits wrote a journal");
    graft_ops::journal::parse(&bytes)
        .expect("journal parses")
        .entries
}

fn git_out(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).expect("utf-8")
}

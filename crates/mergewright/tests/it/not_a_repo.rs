//! Starting outside a repository (m8-polish-and-ship/08 decision 2): on a
//! terminal, the empty state; anywhere else (piped, scripted), one plain
//! line and no screen, as before.

use std::process::Command;

use crate::pty::{Isolated, Pty};

/// Not a terminal: the repository is opened before the terminal is touched,
/// so a path that is not a repository never enters the alternate screen —
/// not even for the length of the capability probe — and says why on stderr.
#[test]
fn off_a_terminal_a_path_that_is_not_a_repository_is_one_plain_line() {
    let dir = std::env::temp_dir().join(format!("mwr-not-a-repo-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let isolated = Isolated::new();

    let output = Command::new(env!("CARGO_BIN_EXE_mwr"))
        .arg(&dir)
        // A git repository above the temp dir must not count.
        .env(
            "GIT_CEILING_DIRECTORIES",
            dir.parent().expect("temp has a parent"),
        )
        .env("XDG_CONFIG_HOME", isolated.config())
        .env("HOME", isolated.home())
        .output()
        .expect("mwr runs");
    std::fs::remove_dir(&dir).expect("the scratch directory is empty");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("mwr: ") && stderr.contains("not inside a git repository"),
        "{stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\u{1b}[?1049h"), "{stdout:?}");
    assert!(
        !isolated.recent_file().exists(),
        "a headless run remembers nothing"
    );
}

/// On a terminal: the empty state, not an error; `q` leaves cleanly — exit
/// 0, nothing on stderr.
#[test]
fn on_a_terminal_a_path_that_is_not_a_repository_shows_the_empty_state() {
    let root = tempfile::tempdir().expect("scratch");
    let isolated = Isolated::new();
    let mut pty = Pty::spawn(&isolated, root.path(), root.path(), (80, 24));
    let screen = pty.wait_for("the empty state", |s| s.has("no repository here"));
    assert!(screen.has("o  Open"), "{}", screen.text());
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    assert!(stderr.is_empty(), "{stderr:?}");
}

/// On a terminal, a folder that does not exist: the empty state says so and
/// offers no init (the open user question's default, ticket 08 NOTES).
#[test]
fn on_a_terminal_a_missing_folder_says_so_and_offers_no_init() {
    let root = tempfile::tempdir().expect("scratch");
    let isolated = Isolated::new();
    let missing = root.path().join("newthing");
    let mut pty = Pty::spawn(&isolated, &missing, root.path(), (80, 24));
    let screen = pty.wait_for("the empty state", |s| s.has("no such folder"));
    assert!(
        !screen.has("Init") && !screen.has("i init"),
        "{}",
        screen.text()
    );
    pty.send(b"i");
    pty.send(b"q");
    let (status, _, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0));
    assert!(!missing.exists(), "nothing was made");
}

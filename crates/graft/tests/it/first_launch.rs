//! The first-launch line through the real binary (m8-polish-and-ship/18d,
//! rulings R7): `drive` counts a terminal launch; a piped run never does.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::process::Command;

use crate::pty::{Isolated, Pty};

/// Headless `run` (stdout piped) leaves `launches` absent.
#[test]
fn headless_never_counts() {
    let dir = graft_fixtures::borrow(graft_fixtures::Shape::Linear);
    let isolated = Isolated::first_launch();
    let output = Command::new(env!("CARGO_BIN_EXE_mwr"))
        .arg(&dir)
        .env("XDG_CONFIG_HOME", isolated.config())
        .env("HOME", isolated.home())
        .output()
        .expect("graft runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !isolated.launches_file().exists(),
        "a headless run counts nothing"
    );
}

/// A first launch on a terminal: the graph's status row says where help
/// is; `?` opens the palette and writes `done`.
#[test]
fn a_terminal_launch_counts_and_shows_the_line_until_the_palette() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear);
    let isolated = Isolated::first_launch();
    let mut pty = Pty::spawn(&isolated, scratch.path(), scratch.path(), (120, 36));
    let screen = pty.wait_for("the first-launch line", |s| s.has("first time?"));
    // J11: the line is there with no key pressed.
    crate::m8_walk::within_the_note::<&str>("first-launch", &[]);
    assert!(
        screen.has("first time? ? keys here · ⌃P anything"),
        "{}",
        screen.text()
    );
    assert_eq!(
        std::fs::read_to_string(isolated.launches_file()).expect("counted"),
        "1\n"
    );
    pty.send(b"?");
    pty.wait_for("the palette, and the line gone", |s| !s.has("first time?"));
    assert_eq!(
        std::fs::read_to_string(isolated.launches_file()).expect("written"),
        "done\n"
    );
    pty.send(b"\x1b");
    let closed = pty.wait_for("the palette closed", |s| !s.has("esc close"));
    assert!(!closed.has("first time?"), "{}", closed.text());
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

/// Seconds since the epoch, for recents a known number of days old.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
}

fn shot(
    shots: &mut graft_fixtures::Captures,
    name: &str,
    right: &str,
    screen: &crate::pty::Screen,
) {
    shots.check(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots/m8")
            .join(name),
        format!(
            "m8-polish-and-ship/18d, from the real binary on a pty: what right looks like — \
             {right} Muted ink: colour unverified (rule 5).\n\n{}\n",
            screen.text()
        ),
    );
}

/// The two first-launch captures: the graph's status row and the empty
/// state's row above the rule, both at 120x36 on a first launch.
#[test]
fn first_launch_captures() {
    let mut shots = graft_fixtures::Captures::new();

    let isolated = Isolated::first_launch();
    let code = isolated.home().join("code");
    std::fs::create_dir_all(&code).expect("~/code");
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Linear);
    let repo = code.join("api");
    let copied = Command::new("cp")
        .arg("-R")
        .arg(scratch.path())
        .arg(&repo)
        .status()
        .expect("cp runs");
    assert!(copied.success(), "copied");
    let mut pty = Pty::spawn(&isolated, &repo, isolated.root.path(), (120, 36));
    let screen = pty.wait_for("the first-launch line", |s| {
        s.has("first time?") && s.has(" commits") && !s.has("…")
    });
    shot(
        &mut shots,
        "first-launch-120x36.txt",
        "his first launch in a repository: the graph as always, and the last row, where the \
         key hints usually sit, reads `first time? ? keys here · ⌃P anything` until he first \
         opens the palette (then the hints are back, on this and every later launch).",
        &screen,
    );
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");

    let isolated = Isolated::first_launch();
    let code = isolated.home().join("code");
    std::fs::create_dir_all(&code).expect("~/code");
    let day = 86_400;
    let entries: Vec<(u64, std::path::PathBuf)> = ["payments-service", "web-dashboard", "dotfiles"]
        .iter()
        .zip([1, 3, 8])
        .map(|(name, days)| (now() - days * day - day / 2, code.join(name)))
        .collect();
    for (_, path) in &entries {
        std::fs::create_dir_all(path).expect("a folder");
    }
    let borrowed: Vec<(u64, &std::path::Path)> = entries
        .iter()
        .map(|(secs, path)| (*secs, path.as_path()))
        .collect();
    isolated.write_recent(&borrowed);
    let mut pty = Pty::spawn(&isolated, &code, isolated.root.path(), (120, 36));
    let screen = pty.wait_for("the first-launch line", |s| s.has("first time?"));
    shot(
        &mut shots,
        "first-launch-empty-state-120x36.txt",
        "his first launch outside a repository, three recents: the start screen as always, \
         plus `first time? ? keys here · ⌃P anything` on the row just above the `─` rule \
         (the bar row under the rule unchanged).",
        &screen,
    );
    pty.send(b"q");
    let (status, stderr, _) = pty.wait_exit();
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
    shots.finish();
}

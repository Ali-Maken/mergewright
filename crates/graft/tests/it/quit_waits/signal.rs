//! The real binary, a real merge whose `pre-merge-commit` hook hangs, on a
//! pty with stderr on the same pty, so the transcript shows what came after
//! what (m8/13b review fix, M1 items 5, 7, 8).
//!
//! - A signal while the write runs: the terminal restored, **then** the
//!   waiting line; a second signal stops git, prints the stopped line, and
//!   the process dies of the signal.
//! - `q y`, the real 10 s offer, `q y`: restored, then the stopped line,
//!   exit 0.

use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::pty::{Isolated, Pty};

/// The terminal's restore: leaving the alternate screen.
const RESTORED: &[u8] = b"\x1b[?1049l";

/// The hook: git's pid, its own, then ten minutes asleep.
fn hang(repo: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let hook = repo.join(".git/hooks/pre-merge-commit");
    std::fs::create_dir_all(hook.parent().expect("a hooks dir")).expect("writable");
    std::fs::write(
        &hook,
        "#!/bin/sh\necho $PPID > .git/git.pid\necho $$ > .git/hook.pid\nexec sleep 600\n",
    )
    .expect("writable");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// A pid the hook wrote, polled until it is there.
fn pid(repo: &Path, file: &str) -> u32 {
    let path = repo.join(".git").join(file);
    let began = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(&path)
            && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "the hook never ran"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .expect("kill runs")
        .success()
}

/// Kill a leftover by pid (the hook outlives git's stop: the pool signals
/// git's pid, not its group). Never by pattern.
fn reap(pid: u32) {
    if alive(pid) {
        let status = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status()
            .expect("kill runs");
        assert!(status.success(), "could not kill {pid}");
    }
}

/// `graft` on the fork, `j j m` (the preview), `m`: the merge runs, its
/// hook asleep. Returns the pty and the hook's and git's pids.
fn merging(isolated: &Isolated, repo: &Path) -> (Pty, u32, u32) {
    hang(repo);
    let mut pty = Pty::spawn_with_stderr_on_pty(isolated, repo, repo, (100, 30));
    pty.wait_for("the graph", |s| s.has("finish the feature"));
    pty.send(b"jj");
    pty.wait_for("main's row", |s| {
        s.lines()
            .iter()
            .any(|line| line.contains('\u{25b8}') && line.contains("main moves again"))
    });
    pty.send(b"m");
    pty.wait_for("the merge preview's plan", |s| {
        s.has("no conflicts expected")
    });
    pty.send(b"y");
    let git = pid(repo, "git.pid");
    let hook = pid(repo, "hook.pid");
    (pty, git, hook)
}

/// The position of `needle` in `haystack` at or after `from`.
fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + from)
}

fn wait_for_bytes(pty: &Pty, what: &str, needle: &[u8]) {
    let began = Instant::now();
    while find(&pty.output(), needle, 0).is_none() {
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "never saw {what}: {}",
            String::from_utf8_lossy(&pty.output())
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_signal_restores_then_waits_and_a_second_stops_git() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let (pty, git, hook) = merging(&isolated, &repo);

    pty.signal("TERM");
    wait_for_bytes(
        &pty,
        "the waiting line",
        b"send the signal again to stop it",
    );
    let transcript = pty.output();
    let restored = find(&transcript, RESTORED, 0).expect("the terminal restored");
    let waiting = find(&transcript, b"waiting for merge main into feature", 0)
        .expect("the waiting line names the merge");
    assert!(restored < waiting, "restored before the line");
    assert!(
        String::from_utf8_lossy(&transcript).contains("(up to 60 s)"),
        "the bound is named"
    );
    assert!(alive(git), "the first signal leaves git running");

    pty.signal("TERM");
    let (status, _, transcript) = pty.wait_exit();
    let text = String::from_utf8_lossy(&transcript).into_owned();
    let stopped = find(
        &transcript,
        b"stopped merge main into feature half-way",
        restored,
    )
    .unwrap_or_else(|| panic!("the stopped line: {text}"));
    assert!(stopped > waiting, "{text}");
    assert_eq!(status.signal(), Some(15), "dies of SIGTERM: {status:?}");
    assert!(!alive(git), "git stopped");
    reap(hook);
}

#[test]
fn stop_now_after_the_real_offer_restores_then_says_what_it_stopped() {
    let isolated = Isolated::new();
    let scratch = crate::preview::forked();
    let repo = scratch.path().canonicalize().expect("the repo resolves");
    let (mut pty, git, hook) = merging(&isolated, &repo);

    pty.send(b"q");
    pty.wait_for("the quit box", |s| s.has("quit waits for it"));
    pty.send(b"y");
    pty.wait_for("the wait", |s| s.has("quitting when it finishes"));
    // The loop's own clock: no key for 10 s, then the offer.
    let screen = pty.wait_for("the offer, at 10 s", |s| s.has("still running after 10 s"));
    assert!(screen.has("q stop it now"), "{}", screen.text());
    pty.send(b"q");
    pty.wait_for("the stop box", |s| s.has("Quit now?"));
    pty.send(b"y");
    let (status, _, transcript) = pty.wait_exit();
    let text = String::from_utf8_lossy(&transcript).into_owned();
    let restored = find(&transcript, RESTORED, 0).expect("the terminal restored");
    let stopped = find(
        &transcript,
        b"stopped merge main into feature half-way after ",
        0,
    )
    .unwrap_or_else(|| panic!("the stopped line: {text}"));
    assert!(restored < stopped, "restored, then the line");
    assert!(text.contains("here shows where it stopped"), "{text}");
    assert_eq!(status.code(), Some(0), "{text}");
    assert!(!alive(git), "git stopped");
    reap(hook);
}

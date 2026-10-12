//! The practice tour from the real binary on a pty (m8-polish-and-ship/19b):
//! into it from the empty state (`t`) and from his repository (`⌃P tour ⏎`),
//! out of it by `Esc` and by `q`, and what it leaves behind — nothing. The
//! practice repository is built under `TMPDIR`, set per test to a folder
//! under the isolated home, so every folder it makes can be listed.
//!
//! Elsewhere: each task ticked by doing it, `tour_tasks.rs` (19c-b); the
//! captures and `P` under a macOS-length `TMPDIR`, `tour_walk.rs` (19c-c).
//! `MWR_PRACTICE_EPOCH` fixes the practice repository's dates (19c-a), so
//! its ids and order are the same every build.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::app_harness::git;
use crate::outside_repo::{assert_within_the_note, quit_cleanly};
use crate::pty::{Isolated, Pty, Screen};
use crate::repo_truth::git_truth;
use graft_core::on_disk::CURRENT_DIR;

const CARD: &str = "practice 1 of 8";

/// The isolated home's temp folder, made; `TMPDIR` for the child.
fn temp_under(isolated: &Isolated) -> PathBuf {
    let tmp = isolated.home().join("tmp");
    std::fs::create_dir_all(&tmp).expect("tmp");
    tmp.canonicalize().expect("canonical tmp")
}

fn spawn(isolated: &Isolated, arg: &Path, tmp: &Path, size: (u16, u16)) -> Pty {
    Pty::spawn_with_env(
        isolated,
        arg,
        isolated.root.path(),
        size,
        &[("TMPDIR", tmp.as_os_str().to_owned())],
    )
}

/// Every folder in `tmp`, by name.
fn left_in(tmp: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(tmp)
        .expect("tmp readable")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Every file under `root`, by path, with its bytes.
fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut todo = vec![root.to_path_buf()];
    while let Some(dir) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                todo.push(path);
            } else {
                let bytes = std::fs::read(&path).expect("readable");
                out.insert(path.strip_prefix(root).expect("under").to_path_buf(), bytes);
            }
        }
    }
    out
}

fn in_the_tour(pty: &Pty) -> Screen {
    pty.wait_for("the practice tour's card", |s| {
        s.has(CARD) && s.has("a throwaway repo, not yours")
    })
}

/// The practice graph settled: its walk complete (`N commits`) and no
/// topological sort still out (`sorting…`). Only then is `Esc` the tour's:
/// the graph's `Esc` ladder backs out of a running sort or a reading walk
/// first (`update/keys/escape.rs`, `tour::esc_leaves` is its last rung), so
/// an `Esc` sent while the card is drawn but the reads are out gives up the
/// sort — `order stays provisional (date)` — and the tour stays. Under load
/// the reads outlast the card's first frame; that ate 7 of 8 `Esc`s in a
/// loaded loop (`M8/tour-esc-load-flake`).
fn nothing_else_takes_esc(pty: &Pty) {
    pty.wait_for("the practice graph settled", |s| {
        let header = &s.lines()[0];
        header.contains(" commits") && !header.contains("sorting\u{2026}")
    });
}

/// The screen line the cursor is on; `usize::MAX` while none is drawn.
fn cursor_row(screen: &Screen) -> usize {
    screen
        .lines()
        .iter()
        .position(|line| line.contains('\u{25b8}'))
        .unwrap_or(usize::MAX)
}

/// The subject on the cursor's row, of the subjects `subjects`.
fn cursor_subject(screen: &Screen, subjects: &[String]) -> String {
    let row = screen
        .lines()
        .into_iter()
        .find(|line| line.contains('\u{25b8}'))
        .unwrap_or_else(|| panic!("no cursor row:\n{}", screen.text()));
    subjects
        .iter()
        .filter(|subject| row.contains(subject.as_str()))
        .max_by_key(|subject| subject.len())
        .unwrap_or_else(|| panic!("no subject on the cursor row {row:?}"))
        .clone()
}

/// His graph with the cursor landed on `HEAD`'s row; that row. The rows can
/// be drawn a frame before the refs read answers and the landing moves the
/// cursor from the top row to `HEAD` (as `outside_repo::landed` says), so a
/// wait for any `▸` could take the top row: keys sent then counted from the
/// wrong row, and under load the walk waited 20 s for a row the cursor never
/// reached, or remembered a commit it never left from.
fn on_head(pty: &Pty, repo: &Path, name: &str) -> usize {
    let head = git(repo, &["log", "-1", "--format=%s"]).trim().to_owned();
    cursor_row(
        &pty.wait_for(&format!("his graph, the cursor on {head:?}"), |s| {
            s.lines()[0].contains(name)
                && s.lines()
                    .iter()
                    .any(|line| line.contains('\u{25b8}') && line.contains(head.as_str()))
        }),
    )
}

/// His repository: a copy of the feature-branch corpus; its path and every
/// subject on its graph.
fn his_repo() -> (graft_fixtures::Scratch, PathBuf, Vec<String>) {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let path = scratch.path().canonicalize().expect("canonical");
    let subjects = git(&path, &["log", "--all", "--format=%s"])
        .lines()
        .map(str::to_owned)
        .collect();
    (scratch, path, subjects)
}

/// J12 `start-tour-empty-state` = 1 (`t`) and J13 `leave-tour` = 1
/// (`Esc`): into the tour from the empty state and back to it, the practice
/// folder gone after.
#[test]
fn t_on_the_empty_state_starts_the_tour_and_esc_comes_back() {
    let isolated = Isolated::new();
    let tmp = temp_under(&isolated);
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = spawn(&isolated, &plain, &tmp, (120, 36));
    pty.wait_for("the empty state", |s| s.has("no repository here"));

    pty.send(b"t");
    let screen = in_the_tour(&pty);
    assert_within_the_note("start-tour-empty-state", false, &[b"t"]);
    let made = left_in(&tmp);
    assert_eq!(made.len(), 1, "one practice folder: {made:?}");
    assert!(made[0].starts_with("mergewright-practice-"), "{made:?}");
    assert!(screen.has("stage a change"), "{}", screen.text());

    nothing_else_takes_esc(&pty);
    pty.send(b"\x1b");
    pty.wait_for("back on the empty state", |s| s.has("no repository here"));
    assert_within_the_note("leave-tour", false, &[b"\x1b"]);
    assert_eq!(left_in(&tmp), Vec::<String>::new(), "removed after Esc");
    quit_cleanly(pty);
}

/// J12 `start-tour` = 3 (`⌃P tour ⏎`) and J13 `leave-tour` = 1: from his
/// repository with the cursor moved off `HEAD`, into the tour and back,
/// the cursor on the same commit; his repository and the state folder
/// byte-for-byte as they were; the practice folder gone.
#[test]
fn the_palette_starts_the_tour_and_esc_returns_to_his_commit() {
    let isolated = Isolated::new();
    let tmp = temp_under(&isolated);
    let (_scratch, repo, subjects) = his_repo();
    let mut pty = spawn(&isolated, &repo, &tmp, (120, 36));
    let name = repo
        .file_name()
        .expect("named")
        .to_string_lossy()
        .into_owned();
    let landed = on_head(&pty, &repo, &name);
    pty.send(b"j");
    pty.send(b"j");
    let moved = pty.wait_for("the cursor two rows down", |s| cursor_row(s) == landed + 2);
    let was_on = cursor_subject(&moved, &subjects);
    let truth_before = git_truth(&repo).expect("his repo reads");
    let graft_before = bytes_under(&repo.join(".git").join(CURRENT_DIR));
    let state_before = bytes_under(&isolated.config());

    pty.send(b"\x10");
    pty.send(b"tour");
    pty.wait_for("the tour row", |s| s.has("Practice tour"));
    pty.send(b"\r");
    in_the_tour(&pty);
    assert_within_the_note("start-tour", false, &[b"\x10", b"tour", b"\r"]);
    assert_eq!(left_in(&tmp).len(), 1);

    nothing_else_takes_esc(&pty);
    pty.send(b"\x1b");
    pty.wait_for("his graph again", |s| {
        s.lines()[0].contains(&name) && !s.has(CARD)
    });
    let back = pty.wait_for(&format!("the cursor back on {was_on:?}"), |s| {
        s.lines()
            .iter()
            .any(|line| line.contains('\u{25b8}') && line.contains(was_on.as_str()))
            || s.has("the commit you were on is gone")
    });
    assert_eq!(
        cursor_subject(&back, &subjects),
        was_on,
        "back on his commit:\n{}",
        back.text()
    );
    assert_within_the_note("leave-tour", false, &[b"\x1b"]);
    assert_eq!(left_in(&tmp), Vec::<String>::new(), "removed after Esc");
    assert_eq!(git_truth(&repo).expect("his repo reads"), truth_before);
    assert_eq!(
        bytes_under(&repo.join(".git").join(CURRENT_DIR)),
        graft_before
    );
    assert_eq!(
        bytes_under(&isolated.config()),
        state_before,
        "state folder"
    );
    quit_cleanly(pty);
}

/// `q` inside the tour ends Graft, and the practice folder goes with it.
#[test]
fn q_inside_the_tour_removes_the_practice_folder() {
    let isolated = Isolated::new();
    let tmp = temp_under(&isolated);
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = spawn(&isolated, &plain, &tmp, (120, 36));
    pty.wait_for("the empty state", |s| s.has("no repository here"));
    pty.send(b"t");
    in_the_tour(&pty);
    assert_eq!(left_in(&tmp).len(), 1);
    quit_cleanly(pty);
    assert_eq!(left_in(&tmp), Vec::<String>::new(), "removed after q");
}

/// A temp folder that cannot be written: the tour cannot start, so he
/// stays in his repository with his cursor, and the status says why.
#[test]
fn an_unwritable_temp_folder_stays_in_his_repo_and_says_why() {
    use std::os::unix::fs::PermissionsExt;
    let isolated = Isolated::new();
    let tmp = temp_under(&isolated);
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o555)).expect("read-only");
    let (_scratch, repo, subjects) = his_repo();
    let mut pty = spawn(&isolated, &repo, &tmp, (120, 36));
    let name = repo
        .file_name()
        .expect("named")
        .to_string_lossy()
        .into_owned();
    let landed = on_head(&pty, &repo, &name);
    pty.send(b"j");
    let moved = pty.wait_for("the cursor down a row", |s| cursor_row(s) == landed + 1);
    let was_on = cursor_subject(&moved, &subjects);
    pty.send(b"\x10");
    pty.send(b"tour");
    pty.wait_for("the tour row", |s| s.has("Practice tour"));
    pty.send(b"\r");
    let back = pty.wait_for("the reason on his graph", |s| {
        s.has("tour: ") && s.lines()[0].contains(&name)
    });
    assert!(!back.has(CARD), "{}", back.text());
    assert!(back.lines()[0].contains(&name), "{}", back.text());
    let back = pty.wait_for(&format!("the cursor back on {was_on:?}"), |s| {
        s.lines()
            .iter()
            .any(|line| line.contains('\u{25b8}') && line.contains(was_on.as_str()))
    });
    assert_eq!(cursor_subject(&back, &subjects), was_on);
    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).expect("writable");
    assert_eq!(left_in(&tmp), Vec::<String>::new());
    quit_cleanly(pty);
}

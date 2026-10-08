//! The practice tour on a pty by the card's own words: its captures, and
//! `P` with a temp folder as long as macOS's own (m8-polish-and-ship/19c-c).
//!
//! Not here (19c-c NOTES, red): the whole tour walked to the end. Followed in
//! order, task 6's merge is refused by git — `u` (task 3) leaves `notes.txt`
//! staged, task 4 brings it back staged, and `git merge` will not run over a
//! staged change. The walk, the conflict capture and the finished capture
//! wait on that.
//!
//! Reproducible: the practice repository's dates come from
//! `GRAFT_PRACTICE_EPOCH` = `graft_fixtures::git::FIXED_DATE`'s instant, and
//! the commits he makes in the tour from `GIT_AUTHOR_DATE` /
//! `GIT_COMMITTER_DATE` an hour after it (no Graft commit or merge sets
//! those itself: grep, 19c-c NOTES). `TMPDIR` is a fresh folder under
//! `/tmp/graft-capture/` (the capture convention, `FIXED_ROOT_VAR`); its
//! random suffix is the one part of a drawn path that differs per run, and
//! the captures write it as `XXXXXX`.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built leaves nothing to test; the panic names why"
)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::app_harness::git;
use crate::outside_repo::quit_cleanly;
use crate::pty::{Isolated, Pty, Screen};

/// `graft_fixtures::git::FIXED_DATE` (2021-01-01T00:00:00Z), the instant
/// the fixtures' own commits carry.
const EPOCH: i64 = 1_609_459_200;

/// The ticks row's names, in `TourTask::ALL` order (`view::tour`).
const NAMES: [&str; 8] = [
    "stage", "commit", "undo", "stash", "branch", "merge", "conflict", "push",
];

/// What the card says for each task, shipped keymap — the words a walk
/// follows. `(title, line)`. Task 7's line is C+1's: the keys 19c-b's
/// `task_7_ticks_by_resolving_clash_and_finishing` presses.
const CARD: [(&str, &str); 8] = [
    (
        "stage a change",
        "\u{23ce} on the top row opens your changes \u{b7} space stages the change under the cursor",
    ),
    ("commit it", "c writes a commit from what you staged"),
    (
        "undo that commit",
        "u takes back the last thing you did \u{2014} nothing is lost",
    ),
    (
        "stash and back",
        "S opens the stashes \u{b7} s sets your edits aside \u{b7} p brings them back",
    ),
    ("make a branch", "b makes a branch at the cursor"),
    (
        "merge feature",
        "m on feature shows the merge before it happens \u{b7} y does it",
    ),
    (
        "resolve a conflict",
        "m on clash \u{b7} y stops on its conflict \u{b7} t takes theirs \u{b7} \u{23ce} marks it done \u{b7} y finishes",
    ),
    ("push", "P sends main to origin"),
];

/// `echo ${#TMPDIR}` on the Mac this was built on (2026-10-08): the
/// per-user temp folder macOS sets, trailing `/` included.
const MACOS_TMPDIR_LEN: usize = 49;

/// A fresh `/tmp/graft-capture/<scenario>-<random>`, removed on drop.
/// Random so two runs at once (two worktrees) never share one.
fn capture_root(scenario: &str, random: usize) -> (tempfile::TempDir, PathBuf) {
    let parent = Path::new("/tmp/graft-capture");
    std::fs::create_dir_all(parent).expect("/tmp/graft-capture");
    let dir = tempfile::Builder::new()
        .prefix(&format!("{scenario}-"))
        .rand_bytes(random)
        .tempdir_in(parent)
        .expect("a capture folder");
    // Not canonicalised: on macOS that would be `/private/tmp/…`, and the
    // app draws the path it was given.
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// `root/practice-tmp`, made: a capture's `TMPDIR`. The header cuts the
/// practice path from the left, and this fixed name is what the cut leaves
/// in sight of `/tributary-practice-…` — not the root's random suffix.
fn tour_tmp(root: &Path) -> PathBuf {
    let tmp = root.join("practice-tmp");
    std::fs::create_dir(&tmp).expect("practice-tmp");
    tmp
}

/// The screen settled: the working tree read (its row on top) and the
/// graph's sort done, so two runs draw the same frame.
fn settled(screen: &Screen) -> bool {
    screen.has("Uncommitted changes (1 unstaged, 1 untracked)")
        && !screen.has("sorting\u{2026}")
        && screen.has("\u{24d8}")
}

/// The child's environment: practice dates, his own commits' dates, and
/// where the practice repository (and git's askpass socket) is made.
fn env(tmp: &OsString) -> Vec<(&'static str, OsString)> {
    let his = format!("{} +0000", EPOCH + 3_600);
    vec![
        ("TMPDIR", tmp.clone()),
        ("GRAFT_PRACTICE_EPOCH", EPOCH.to_string().into()),
        ("GIT_AUTHOR_DATE", his.clone().into()),
        ("GIT_COMMITTER_DATE", his.into()),
    ]
}

/// The screen as a capture holds it: the run's random folder suffix as
/// `X`s of its width. The suffix alone, because the header cuts the path
/// from the left and may cut the scenario name in front of it; every
/// length in the path is fixed, so the cut falls at the same place each run.
fn steady(screen: &Screen, root: &Path) -> String {
    let name = root
        .file_name()
        .expect("named")
        .to_string_lossy()
        .into_owned();
    let (_, suffix) = name.rsplit_once('-').expect("<scenario>-<suffix>");
    screen.text().replace(suffix, &"X".repeat(suffix.len()))
}

fn shot(shots: &mut graft_fixtures::Captures, name: &str, right: &str, text: &str) {
    shots.check(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/shots")
            .join(name),
        format!(
            "m8-polish-and-ship/19c-c: what right looks like — {right} Colour unverified \
             (rule 5).\n\n{text}\n"
        ),
    );
}

/// His repository: the feature-branch corpus copied into `root/his`.
fn his_repo(root: &Path) -> PathBuf {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::FeatureBranches);
    let path = root.join("his");
    let status = std::process::Command::new("cp")
        .arg("-R")
        .arg(scratch.path())
        .arg(&path)
        .status()
        .expect("cp runs");
    assert!(status.success(), "copied");
    path
}

/// The `tributary-practice-*` folders under `tmp`.
fn practice_folders(tmp: &Path) -> Vec<String> {
    std::fs::read_dir(tmp)
        .expect("tmp readable")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("tributary-practice-"))
        .collect()
}

fn practice_folder_is_gone(tmp: &Path) {
    assert_eq!(practice_folders(tmp), Vec::<String>::new());
}

fn cursor_row(screen: &Screen) -> Option<usize> {
    screen
        .lines()
        .iter()
        .position(|line| line.contains('\u{25b8}'))
}

/// The card's ticks row as bits (`graft_ui::tour_card::ticks_text`: each
/// name, a `✓` before the done ones); `None` where the card dropped them
/// (narrower than its line plus the ticks).
fn drawn_ticks(screen: &Screen) -> Option<[bool; 8]> {
    screen.lines().iter().find_map(|line| ticks_in(line))
}

fn ticks_in(line: &str) -> Option<[bool; 8]> {
    let tokens: Vec<&str> = line
        .split_whitespace()
        .filter(|token| *token != "\u{b7}")
        .collect();
    let end = tokens.iter().rposition(|token| *token == "push")?;
    let mut bits = [false; 8];
    let mut at = end;
    for (index, name) in NAMES.iter().enumerate().rev() {
        if tokens.get(at) != Some(name) {
            return None;
        }
        let before = at.checked_sub(1).and_then(|i| tokens.get(i));
        bits[index] = before == Some(&"\u{2713}");
        if index > 0 {
            at = at.checked_sub(if bits[index] { 2 } else { 1 })?;
        }
    }
    Some(bits)
}

fn with(done: &[usize]) -> [bool; 8] {
    let mut bits = [false; 8];
    for bit in done {
        bits[*bit] = true;
    }
    bits
}

/// The practice repository's work tree under `tmp`.
fn work_under(tmp: &Path) -> PathBuf {
    tmp.join(format!("tributary-practice-{EPOCH}")).join("work")
}

/// The card on task `n` (0-based) with `done` ticked: its title and its
/// line as `CARD` has them, the journal agreeing, and the drawn ticks too
/// where the card has room for them.
fn on_task(pty: &Pty, work: &Path, n: usize, done: [bool; 8]) -> Screen {
    let (title, line) = CARD[n];
    let heading = format!("practice {} of 8 \u{b7} {title}", n + 1);
    let screen = pty.wait_for(&format!("the card on {heading:?}"), |s| {
        // The ticks follow the graph by one progress read: wait for them
        // where the card draws them.
        s.has(&heading)
            && s.has(line)
            && cursor_row(s).is_some()
            && drawn_ticks(s).is_none_or(|ticks| ticks == done)
    });
    let read = graft_ops::tour::tour_progress(work)
        .unwrap_or_else(|error| panic!("tour_progress: {}", error.explain()));
    assert_eq!(read, done, "the practice journal on task {}", n + 1);
    screen
}

/// The tour's captures that do not need the whole walk: step 1 at 120×36
/// from his repository (`⌃P tour ⏎`), and step 1 at 80×24 from the empty
/// state (`t`). Each leaves by `Esc`, the practice folder gone after.
#[test]
fn the_tour_captures_match() {
    let mut shots = graft_fixtures::Captures::new();
    step1_from_his_repo(&mut shots);
    step1_at_80x24(&mut shots);
    shots.finish();
}

fn step1_from_his_repo(shots: &mut graft_fixtures::Captures) {
    let isolated = Isolated::new();
    let (_root, root) = capture_root("tour-step1", 6);
    let tmp = tour_tmp(&root);
    let repo = his_repo(&root);
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &repo,
        &tmp,
        (120, 36),
        &env(&tmp.as_os_str().to_owned()),
    );
    pty.wait_for("his graph", |s| {
        s.lines()[0].contains("his") && cursor_row(s).is_some()
    });
    pty.send(b"\x10");
    pty.send(b"tour");
    pty.wait_for("the tour row", |s| s.has("Practice tour"));
    pty.send(b"\r");
    on_task(&pty, &work_under(&tmp), 0, with(&[]));
    let step1 = pty.wait_for("the practice graph settled", settled);
    shot(
        shots,
        "m8-tour-step1-120x36.txt",
        "`⌃P tour ⏎` from his repo: the header `◇ work`, then the practice path cut from the \
         left (`…-step1-XXXXXX/practice-tmp/tributary-practice-1609459200/work`, the run's random \
         folder written as `X`s), `· a throwaway repo, not yours · main  7 commits`; the card at the graph pane's top, 3 rows, \
         `practice 1 of 8 · stage a change` and `esc leave` on its top border, line 1 `⏎ on the \
         top row opens your changes · space stages the change under the cursor` (at 120 columns \
         the ticks row does not fit beside that line, so it is dropped, never cut); the practice \
         graph under it: `Uncommitted changes` on top, `main ↑1` one ahead of `origin/main`, \
         `feature` and `clash` beside it, the cursor on `main`.",
        &steady(&step1, &root),
    );
    pty.send(b"\x1b");
    pty.wait_for("his graph again", |s| {
        s.lines()[0].contains("his") && !s.has("practice 1 of 8")
    });
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

fn step1_at_80x24(shots: &mut graft_fixtures::Captures) {
    let isolated = Isolated::new();
    let (_root, root) = capture_root("tour-80x24", 6);
    let tmp = tour_tmp(&root);
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &plain,
        isolated.root.path(),
        (80, 24),
        &env(&tmp.as_os_str().to_owned()),
    );
    pty.wait_for("the empty state", |s| s.has("no repository here"));
    pty.send(b"t");
    let screen = pty.wait_for("the card at 80 columns", |s| {
        s.has("practice 1 of 8 \u{b7} stage a change") && s.has("esc leave") && settled(s)
    });
    assert_eq!(drawn_ticks(&screen), None, "no ticks under 100 columns");
    shot(
        shots,
        "m8-tour-80x24.txt",
        "`t` from the empty state at 80×24: the card's top border still holds `practice 1 of 8 · \
         stage a change` and `esc leave`; its line is cut to the width with `…` and no ticks row \
         is drawn (under 100 columns the ticks go); the header `◇ work` with a gap before the \
         path cut from the left (`…ctice-1609459200/work · a throwaway repo, not yours`); the \
         practice graph under the card, `Uncommitted changes` on top, the cursor on `main`.",
        &steady(&screen, &root),
    );
    pty.send(b"\x1b");
    pty.wait_for("back on the empty state", |s| s.has("no repository here"));
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

/// 19c C+2: `P` in the tour with `TMPDIR` as long as the per-user temp
/// folder macOS gives him. git's askpass socket is made under it
/// (`graft-git/src/run/askpass.rs:254`, `temp_dir()` — every push, not the
/// tour alone) and must fit `SUN_LEN` (104 on macOS); a longer path stops
/// `P` before git runs. The push lands and task 8 ticks.
#[test]
fn push_in_the_tour_fits_the_askpass_socket_under_a_default_length_tmpdir() {
    // `/tmp/graft-capture/` + `sun-len-probe-` + 15 random = 48, and the
    // trailing `/` macOS's own value carries makes 49.
    let (_root, tmp) = capture_root("sun-len-probe", 15);
    let mut tmpdir = tmp.as_os_str().to_owned();
    tmpdir.push("/");
    assert_eq!(tmpdir.len(), MACOS_TMPDIR_LEN, "{tmpdir:?}");

    let isolated = Isolated::new();
    let plain = isolated.home().join("code");
    std::fs::create_dir_all(&plain).expect("plain folder");
    let mut pty = Pty::spawn_with_env(
        &isolated,
        &plain,
        isolated.root.path(),
        (300, 50),
        &env(&tmpdir),
    );
    pty.wait_for("the empty state", |s| s.has("no repository here"));
    pty.send(b"t");
    let work = work_under(&tmp);
    on_task(&pty, &work, 0, with(&[]));
    pty.send(b"P");
    pty.wait_for("the push preview", |s| {
        s.has("Push main \u{2192} origin/main")
    });
    pty.send(b"y");
    pty.wait_for("main and origin together", |s| {
        s.has("[\u{25c6} main \u{21c4} origin]")
    });
    let screen = on_task(&pty, &work, 0, with(&[7]));
    assert_eq!(drawn_ticks(&screen), Some(with(&[7])), "{}", screen.text());
    assert!(!screen.has("askpass"), "{}", screen.text());
    assert_eq!(
        git(&work, &["rev-parse", "main"]),
        git(&work, &["rev-parse", "origin/main"]),
        "pushed"
    );
    quit_cleanly(pty);
    practice_folder_is_gone(&tmp);
}

#[test]
fn the_ticks_row_reads_as_bits() {
    let line = "│ c writes a commit   ✓ stage · commit undo stash branch merge conflict push │";
    assert_eq!(ticks_in(line), Some(with(&[0])));
    assert_eq!(ticks_in("│ space stages the change │"), None);
}

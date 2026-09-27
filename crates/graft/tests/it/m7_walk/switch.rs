//! m7-power-tools/10b, the switch rule walked through the app against real
//! repositories: `. ⏎` on a clean tree switches at once; `. ⏎ y` and
//! `b <name> ⇥ ⏎ y` over edits set them aside and put them back; `u` after
//! each puts everything back. Keys counted as they go. The two captures are
//! drawn by `view::view` from the walked state (colour unverified, CLAUDE.md
//! rule 5).
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, settled_frame};
use crate::m6_stash_put_back::{main_and_fix, planned, rev};
use crate::preview::{FORK_NOW, opened};
use crate::stash_clash::{quiet, stash_list};

/// What every walk writes over the tracked files.
const EDIT: &str = "my edit\n";

/// `main_and_fix` with `fix` renamed `feature-b`: on `main`, clean.
fn main_and_feature_b() -> graft_fixtures::Scratch {
    let scratch = main_and_fix();
    git(scratch.path(), &["branch", "-m", "fix", "feature-b"]);
    scratch
}

/// A file `main` and `feature-b` both hold with the same contents, other
/// than `base.txt`: one of the Trivial shape's own.
fn shared_file(path: &Path) -> String {
    let on_b = git(path, &["ls-tree", "--name-only", "feature-b"]);
    git(path, &["ls-tree", "--name-only", "main"])
        .lines()
        .find(|name| *name != "base.txt" && on_b.lines().any(|other| other == *name))
        .unwrap_or_else(|| panic!("main and feature-b share only base.txt"))
        .to_owned()
}

/// `base.txt` edited and staged, a shared file edited unstaged: the staged
/// split is part of what must come back. Returns the shared file's name.
fn edited(path: &Path) -> String {
    let other = shared_file(path);
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    git(path, &["add", "--", "base.txt"]);
    std::fs::write(path.join(&other), EDIT).expect("the scratch repo is writable");
    other
}

fn read(path: &Path, name: &str) -> String {
    std::fs::read_to_string(path.join(name)).unwrap_or_else(|why| panic!("{name}: {why}"))
}

fn head(path: &Path) -> String {
    git(path, &["symbolic-ref", "HEAD"]).trim().to_owned()
}

/// The staged and the unstaged names, and the stash list.
fn split(path: &Path) -> (String, String, Vec<String>) {
    (
        git(path, &["diff", "--cached", "--name-only"]),
        git(path, &["diff", "--name-only"]),
        stash_list(path),
    )
}

/// Opened, the first status read in, and the cursor on `target` (`j`/`k`,
/// not counted: reaching the row is not the journey). The status read is
/// waited for so the walk takes the rule's edits branch, not the stale-clean
/// one (that path: `update::switch::tests::an_instant_switch_refused_for_edits_lands_on_the_preview`).
fn ready_on(app: &App, state: AppState, target: &str) -> AppState {
    let mut state = app.pump(opened(app, state), "the first status read", |state| {
        state.working_tree.is_some() && state.status_read.is_none()
    });
    for code in [KeyCode::Char('k'), KeyCode::Char('j')] {
        for _ in 0..10 {
            let on = state
                .history
                .as_ref()
                .and_then(graft::history::History::selected)
                .map(|row| row.commit.id.to_string());
            if on.as_deref() == Some(target) {
                return state;
            }
            state = app.key(&state, code);
        }
    }
    panic!("the cursor never reached {target}");
}

/// Keys pressed, in order: the walk's count is its length.
struct Walk<'a> {
    app: &'a App,
    keys: Vec<&'static str>,
}

impl Walk<'_> {
    fn press(&mut self, state: &AppState, code: KeyCode, named: &'static str) -> AppState {
        self.keys.push(named);
        self.app.key(state, code)
    }
}

/// `u`, settled: HEAD back on `main`.
fn undo(walk: &mut Walk<'_>, state: &AppState) -> AppState {
    let state = walk.press(state, KeyCode::Char('u'), "u");
    assert!(
        state.confirm.is_none(),
        "`u` asks nothing: {:?}",
        state.status
    );
    walk.app.settle_write(state, "the undo")
}

// ── `.` Checkout, clean ──────────────────────────────────────────────────

/// `checkout-branch` on a clean tree: `. ⏎` on `feature-b`'s row — no
/// preview, on `feature-b`, the line `on feature-b ✓ · u goes back`, the
/// cursor on the new `HEAD`'s row. Then `u`: back on `main`.
#[test]
fn switch_clean_via_menu_is_two_keys() {
    let scratch = main_and_feature_b();
    let path = scratch.path();
    let target = rev(path, "feature-b");
    let (app, state) = App::open(path);
    let state = ready_on(&app, state, &target);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.press(&state, KeyCode::Char('.'), ".");
    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(state.preview.is_none(), "no preview: {:?}", state.status);
    assert!(state.operating.is_some(), "the switch launched at once");
    let state = app.settle_write(state, "the switch");
    let state = settled_frame(&app, quiet(&app, state));

    assert_eq!(walk.keys, [".", "⏎"]);
    assert_eq!(head(path), "refs/heads/feature-b");
    assert_eq!(
        state.status,
        Status::Done {
            what: "on feature-b".to_owned(),
            also: None,
            keys: Some("u goes back".to_owned()),
        }
    );
    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let text = drawn.join("\n");
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("on feature-b \u{2713} \u{b7} u goes back")),
        "{text}"
    );
    assert!(!text.contains("Preview"), "no preview box:\n{text}");
    assert_eq!(
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|row| row.commit.id.to_string()),
        Some(target),
        "the cursor on the new HEAD's row"
    );
    check_shot(
        "m7/switch-menu-clean-landed-120x40.txt",
        &format!("{CLEAN_HEADER}\n{text}\n"),
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    drop(state);
    app.close();
}

const CLEAN_HEADER: &str = "\
# m7/switch-menu-clean-landed-120x40.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m7_walk/switch.rs,
# switch_clean_via_menu_is_two_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# Right: `. ⏎` on feature-b's row, the tree clean — no preview box; the
# cursor on the new HEAD's row (feature-b, `◆`); the status line
# `on feature-b ✓ · u goes back`.";

// ── `.` Checkout, with edits ─────────────────────────────────────────────

/// `switch-with-edits` by the menu: `. ⏎ y` — on `feature-b`, both edits
/// back with their staging, no stash left, the line naming them. Then `u`:
/// on `main`, the split and the stash list as before.
#[test]
fn switch_with_edits_via_menu() {
    let scratch = main_and_feature_b();
    let path = scratch.path();
    let other = edited(path);
    let before = split(path);
    let (app, state) = App::open(path);
    let state = ready_on(&app, state, &rev(path, "feature-b"));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.press(&state, KeyCode::Char('.'), ".");
    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(state.preview.is_some(), "edits: the preview first");
    let state = planned(&app, state);
    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the switch"));

    assert_eq!(walk.keys, [".", "⏎", "y"]);
    assert_eq!(head(path), "refs/heads/feature-b");
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert_eq!(split(path), before, "the staged split kept, no stash left");
    assert_eq!(
        state.status,
        Status::Done {
            what: "on feature-b".to_owned(),
            also: Some("your edits are back".to_owned()),
            keys: Some("u goes back".to_owned()),
        }
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(split(path), before);
    drop(state);
    app.close();
}

// ── `b ⇥`, with edits ────────────────────────────────────────────────────

/// `b`, `name`, `⇥`, `⏎` on the cursor's row: the preview, not a refusal.
fn b_tab(walk: &mut Walk<'_>, state: &AppState, name: &str) -> AppState {
    let mut state = walk.press(state, KeyCode::Char('b'), "b");
    assert!(state.name_prompt.is_some(), "`b` opened the prompt");
    walk.keys.push("<name>");
    for character in name.chars() {
        state = walk.app.key(&state, KeyCode::Char(character));
    }
    let state = walk.press(&state, KeyCode::Tab, "⇥");
    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(state.name_prompt.is_none(), "the prompt closed");
    assert!(
        state.preview.is_some(),
        "the preview opened: {:?}",
        state.status
    );
    planned(walk.app, state)
}

/// `new-branch-with-edits`: `b nb ⇥ ⏎ y` on `feature-b`'s row — on `nb` at
/// `feature-b`'s commit, both edits back with their staging, no stash left.
/// Then `u`: `nb` gone, on `main`, the split as before.
#[test]
fn new_branch_with_edits() {
    let scratch = main_and_feature_b();
    let path = scratch.path();
    let other = edited(path);
    let before = split(path);
    let at = rev(path, "feature-b");
    let (app, state) = App::open(path);
    let state = ready_on(&app, state, &at);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = b_tab(&mut walk, &state, "nb");
    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the new branch"));

    assert_eq!(walk.keys, ["b", "<name>", "⇥", "⏎", "y"]);
    assert_eq!(head(path), "refs/heads/nb");
    assert_eq!(rev(path, "nb"), at);
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert_eq!(split(path), before, "the staged split kept, no stash left");
    assert!(
        matches!(&state.status, Status::Done { what, also: Some(clause), keys: Some(keys) }
            if what == "on nb" && clause == "your edits are back" && keys == "u goes back"),
        "{:?}",
        state.status
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    assert!(
        !git(path, &["branch", "--list", "nb"]).contains("nb"),
        "`u` deleted nb"
    );
    assert_eq!(split(path), before);
    drop(state);
    app.close();
}

/// At `HEAD`'s own row nothing is set aside: the preview names the new
/// branch and says `your edits stay as they are`; `Esc` leaves everything
/// as it was.
#[test]
fn new_branch_at_head_with_edits_previews_edits_stay() {
    let scratch = main_and_feature_b();
    let path = scratch.path();
    edited(path);
    let before = split(path);
    let (app, state) = App::open(path);
    let state = ready_on(&app, state, &rev(path, "main"));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = settled_frame(&app, b_tab(&mut walk, &state, "feature-c"));

    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let text = drawn.join("\n");
    assert!(text.contains("Preview"), "{text}");
    assert!(text.contains("feature-c"), "names the new branch:\n{text}");
    assert!(
        drawn
            .iter()
            .any(|line| line.trim_matches(|c| c == '\u{2502}' || c == ' ')
                == "your edits stay as they are"),
        "{text}"
    );
    assert!(!text.contains("set aside"), "nothing set aside:\n{text}");
    check_shot(
        "m7/new-branch-with-edits-preview-120x40.txt",
        &format!("{EDITS_STAY_HEADER}\n{text}\n"),
    );

    let state = app.key(&state, KeyCode::Escape);
    assert!(state.preview.is_none());
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(split(path), before, "nothing written");
    drop(state);
    app.close();
}

const EDITS_STAY_HEADER: &str = "\
# m7/new-branch-with-edits-preview-120x40.txt: drawn by `view::view` from a
# real AppState (crates/graft/tests/it/m7_walk/switch.rs,
# new_branch_at_head_with_edits_previews_edits_stay, asserted). Not a pty
# capture: a character grid with no colour (CLAUDE.md rule 5, colour
# unverified).
#
# Right: `b feature-c ⇥ ⏎` on HEAD's own row over staged and unstaged
# edits — a `Preview:` box naming the new branch, the line
# `your edits stay as they are` (nothing is set aside at HEAD), `y` on the
# box edge.";

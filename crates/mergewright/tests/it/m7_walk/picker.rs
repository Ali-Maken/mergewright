//! m7-power-tools/10, the branch picker walked through the app against real
//! repositories: `C <type> ⏎`, `C ⏎` and `C <type> ⏎ y`, each counted
//! against the M7 note's fence, each landing on the graph with the cursor on
//! the new `HEAD`, each undone by `u`. The captures are drawn by
//! `view::view` from the walked state (colour unverified, CLAUDE.md rule 5).
//!
//! m7-power-tools/11 adds `o`: `cherry-pick-in-view` (`motion o y`) and
//! `cherry-pick-other-branch` (`C <type> ⇥ motion o y`), a merge refused,
//! and the `.` menu's slot 2 dim on `HEAD`'s row.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use mergewright::session::Clock;
use mergewright::update::{AppState, Status};
use mergewright_core::event::KeyCode;

use super::noted_keys;
use super::switch::{EDIT, edited_against, head, read, split};
use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, settled_frame};
use crate::m6_stash_put_back::{main_and_fix, planned, rev};
use crate::preview::{FORK_NOW, dated_commit, opened};
use crate::stash_clash::quiet;

/// On `main`, clean, with every kind of row the picker draws:
///
/// - `feature-a` the branch before (`↩ before`), built on `main` (its
///   upstream is under `refs/heads`);
/// - `login-page` a local branch, `main_and_fix`'s `fix` renamed;
/// - `origin/sara/login-page` a remote branch with no local copy;
/// - `main` here, tracking `origin/main`.
///
/// `origin` is a remote in config only: nothing here fetches.
fn picker_repo() -> mergewright_fixtures::Scratch {
    let scratch = main_and_fix();
    let path = scratch.path();
    git(path, &["branch", "-m", "fix", "login-page"]);
    git(path, &["branch", "feature-a", "main~1"]);
    git(path, &["config", "branch.feature-a.remote", "."]);
    git(
        path,
        &["config", "branch.feature-a.merge", "refs/heads/main"],
    );
    git(
        path,
        &["remote", "add", "origin", "/nonexistent/origin.git"],
    );
    let login = rev(path, "login-page");
    git(
        path,
        &["update-ref", "refs/remotes/origin/sara/login-page", &login],
    );
    let main = rev(path, "main");
    git(path, &["update-ref", "refs/remotes/origin/main", &main]);
    git(path, &["config", "branch.main.remote", "origin"]);
    git(path, &["config", "branch.main.merge", "refs/heads/main"]);
    git(path, &["switch", "feature-a"]);
    git(path, &["switch", "main"]);
    scratch
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

    /// `C`, and the list landed (waiting for the read is not a key).
    fn open(&mut self, state: &AppState) -> AppState {
        let state = self.press(state, KeyCode::Char('C'), "C");
        assert!(
            mergewright::update::picker::shown(&state),
            "`C` opened the picker: {:?}",
            state.status
        );
        listed(self.app, state)
    }

    /// Typing counts as one step, whatever its length (the note's `<type>`).
    fn type_in(&mut self, state: &AppState, text: &str) -> AppState {
        self.keys.push("<type>");
        let mut state = state.clone();
        for character in text.chars() {
            state = self.app.key(&state, KeyCode::Char(character));
        }
        state
    }
}

/// Pumps until the picker's list has landed.
fn listed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the switch list", |state| {
        state
            .picker
            .as_ref()
            .is_some_and(|picker| !picker.rows().is_empty())
    })
}

/// Opened, and the first status read in: the walks take the rule's real
/// branch (clean or edits), not the stale-clean one.
fn ready(app: &App, state: AppState) -> AppState {
    app.pump(opened(app, state), "the first status read", |state| {
        state.working_tree.is_some() && state.status_read.is_none()
    })
}

fn cursor(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// `u`, settled.
fn undo(walk: &mut Walk<'_>, state: &AppState) -> AppState {
    let state = walk.press(state, KeyCode::Char('u'), "u");
    assert!(
        state.confirm.is_none(),
        "`u` asks nothing: {:?}",
        state.status
    );
    walk.app.settle_write(state, "the undo")
}

fn on(what: &str, also: Option<&str>) -> Status {
    Status::Done {
        what: what.to_owned(),
        also: also.map(str::to_owned),
        keys: Some("u goes back".to_owned()),
    }
}

/// The frame at `columns × rows`, the clock pinned.
fn drawn(state: &AppState, columns: u16, rows: u16) -> String {
    frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n")
}

fn header(name: &str, test: &str, right: &str) -> String {
    format!(
        "# m7/{name}: drawn by `view::view` from a real AppState\n\
         # (crates/mergewright/tests/it/m7_walk/picker.rs, {test}, asserted). Not a pty\n\
         # capture: a character grid with no colour (CLAUDE.md rule 5, colour\n\
         # unverified).\n#\n# Right: {right}"
    )
}

/// `C login` at 100 columns, checked and captured.
fn filtered_box(state: &AppState) {
    let text = drawn(state, 100, 30);
    // The box's rows start under its query line; the graph above it draws
    // the same names in its badges.
    let first_row = text
        .lines()
        .skip_while(|line| !line.contains("> login"))
        .find(|line| line.contains("login-page"))
        .unwrap_or_else(|| panic!("a row names login-page:\n{text}"));
    assert!(
        !first_row.contains("sara/"),
        "the segment-prefix local match on top, above the remote:\n{text}"
    );
    assert!(
        !text.contains("local"),
        "under 120 columns: glyphs, no words:\n{text}"
    );
    check_shot(
        "m7/picker-filtered-100x30.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "picker-filtered-100x30.txt",
                "switch_named_branch",
                "`C login` at 100 columns — each row a glyph, a name and an\n\
                 # age only; `login-page` (the segment-prefix match) above\n\
                 # `origin/sara/login-page`; the footer `2 of 4 branches`."
            )
        ),
    );
}

/// `C` with nothing typed at 187 columns, checked and captured.
fn empty_query_box(state: &AppState) {
    let text = drawn(state, 187, 50);
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| {
            crate::row_reads::PICKER_KINDS
                .iter()
                .any(|kind| line.contains(&format!(" {kind} ")))
        })
        .collect();
    assert!(
        rows.first().is_some_and(|row| row.contains("feature-a")),
        "`↩ before  feature-a` first:\n{text}"
    );
    for words in [
        "built on main",
        "tracks origin/main",
        "no copy of yours yet",
        "you're on it",
        "type to find",
        "esc close",
    ] {
        assert!(text.contains(words), "`{words}` drawn:\n{text}");
    }
    check_shot(
        "m7/picker-empty-query-187x50.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "picker-empty-query-187x50.txt",
                "switch_back",
                "`C` with nothing typed — `↩ before  feature-a` first;\n\
                 # `● local` / `◌ remote` / `· here` in their own column; `built on\n\
                 # main`, `tracks origin/main`, `no copy of yours yet` on the right;\n\
                 # `main` last, `you're on it`; the bar `type to find  ⏎ switch to\n\
                 # it  esc close  ⇥ show it on the graph  ↑↓ row`."
            )
        ),
    );
}

// ── `switch-named-branch`: `C <type> ⏎` ───────────────────────────────────

/// `C login ⏎` on a clean tree: on `login-page` at once, no preview, the
/// cursor on its row, `✓ on login-page  u goes back`. Then `u`: on `main`.
/// Captures the box filtered at 100×30 on the way, and the landing at
/// 120×40.
#[test]
fn switch_named_branch() {
    let scratch = picker_repo();
    let path = scratch.path();
    let target = rev(path, "login-page");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);
    let state = walk.type_in(&state, "login");

    filtered_box(&state);

    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(state.preview.is_none(), "no preview: {:?}", state.status);
    assert!(state.picker.is_none(), "the picker closed");
    let state = app.settle_write(state, "the switch");
    let state = settled_frame(&app, quiet(&app, state));

    assert_eq!(walk.keys, ["C", "<type>", "⏎"]);
    assert!(walk.keys.len() <= noted_keys("switch-named-branch"));
    assert_eq!(head(path), "refs/heads/login-page");
    assert_eq!(state.status, on("on login-page", None));
    assert_eq!(
        cursor(&state),
        Some(target),
        "the cursor on the new HEAD's row"
    );
    let text = drawn(&state, 120, 40);
    assert!(
        text.contains("\u{2713} on login-page  u goes back"),
        "{text}"
    );
    check_shot(
        "m7/switch-landed-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "switch-landed-120x40.txt",
                "switch_named_branch",
                "`C login ⏎` on a clean tree — no preview box; the cursor on\n\
                 # the new HEAD's row (login-page); the status line\n\
                 # `✓ on login-page  u goes back`."
            )
        ),
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    drop(state);
    app.close();
}

// ── `switch-back`: `C ⏎` ──────────────────────────────────────────────────

/// `C ⏎`: the branch before is the first row, so `⏎` alone goes back to it.
/// Captures the empty-query box at 187×50 on the way.
#[test]
fn switch_back() {
    let scratch = picker_repo();
    let path = scratch.path();
    let target = rev(path, "feature-a");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);

    empty_query_box(&state);

    let state = walk.press(&state, KeyCode::Enter, "⏎");
    let state = app.settle_write(state, "the switch");
    let state = quiet(&app, state);

    assert_eq!(walk.keys, ["C", "⏎"]);
    assert!(walk.keys.len() <= noted_keys("switch-back"));
    assert_eq!(head(path), "refs/heads/feature-a");
    assert_eq!(state.status, on("on feature-a", None));
    assert_eq!(
        cursor(&state),
        Some(target),
        "the cursor on the new HEAD's row"
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    drop(state);
    app.close();
}

// ── `switch-with-edits`: `C <type> ⏎ y` ───────────────────────────────────

/// `C login ⏎ y` over staged and unstaged edits: the preview, then on
/// `login-page` with both edits back and their staging kept, no stash left.
/// `Esc` on that preview would bring the picker back (unit-tested:
/// `update::picker::tests::esc_on_that_preview_returns_to_the_picker_with_the_query`).
/// Then `u`: on `main`, the split as before.
#[test]
fn switch_with_edits() {
    let scratch = picker_repo();
    let path = scratch.path();
    let other = edited_against(path, "login-page");
    let before = split(path);
    let target = rev(path, "login-page");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);
    let state = walk.type_in(&state, "login");
    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(state.preview.is_some(), "edits: the preview first");
    assert!(
        !mergewright::update::picker::shown(&state),
        "the picker hides under the preview"
    );
    let state = planned(&app, state);
    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the switch"));

    assert_eq!(walk.keys, ["C", "<type>", "⏎", "y"]);
    assert!(walk.keys.len() <= noted_keys("switch-with-edits"));
    assert!(state.picker.is_none(), "`y` closed the picker");
    assert_eq!(head(path), "refs/heads/login-page");
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(read(path, &other), EDIT);
    assert_eq!(split(path), before, "the staged split kept, no stash left");
    assert_eq!(
        state.status,
        on("on login-page", Some("your edits are back"))
    );
    assert_eq!(
        cursor(&state),
        Some(target),
        "the cursor on the new HEAD's row"
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(read(path, "base.txt"), EDIT);
    assert_eq!(split(path), before);
    drop(state);
    app.close();
}

// ── A remote branch with no local copy ───────────────────────────────────

/// `C sara ⏎` on `origin/sara/login-page`: a local `sara/login-page` is made,
/// tracking it, and checked out; the line says so. Then `u`: on `main`, the
/// new branch gone.
#[test]
fn switch_to_a_remote_only_branch_makes_its_local_copy() {
    let scratch = picker_repo();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);
    let state = walk.type_in(&state, "sara");
    let state = walk.press(&state, KeyCode::Enter, "⏎");
    assert!(
        state.preview.is_none(),
        "clean: no preview: {:?}",
        state.status
    );
    let state = app.settle_write(state, "the switch");
    let state = settled_frame(&app, quiet(&app, state));

    assert_eq!(head(path), "refs/heads/sara/login-page");
    assert_eq!(
        git(
            path,
            &["rev-parse", "--abbrev-ref", "sara/login-page@{upstream}"]
        )
        .trim(),
        "origin/sara/login-page"
    );
    let text = drawn(&state, 120, 40);
    let line = "on sara/login-page (new, tracks origin/sara/login-page)";
    assert!(text.contains(line), "`{line}`:\n{text}");
    assert!(text.contains("u goes back"), "{text}");
    check_shot(
        "m7/picker-remote-landed-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "picker-remote-landed-120x40.txt",
                "switch_to_a_remote_only_branch_makes_its_local_copy",
                "`C sara ⏎` on `origin/sara/login-page` — the status line\n\
                 # `✓ on sara/login-page (new, tracks origin/sara/login-page)  u\n\
                 # goes back`; the cursor on the new HEAD's row."
            )
        ),
    );

    let state = undo(&mut walk, &state);
    assert_eq!(head(path), "refs/heads/main");
    assert!(
        !git(path, &["branch", "--list", "sara/login-page"]).contains("sara"),
        "`u` deleted the branch it made"
    );
    drop(state);
    app.close();
}

// ── m7-power-tools/11: `o` takes a commit onto the branch ────────────────

/// Moves the cursor to `target` with `j`/`k`, however many presses: the
/// note counts reaching a row as one step (`motion`), so the walk records
/// `motion` once — and nothing when the cursor is already there.
fn reach(walk: &mut Walk<'_>, mut state: AppState, target: &str) -> AppState {
    let mut moved = false;
    for _ in 0..20 {
        if cursor(&state).as_deref() == Some(target) {
            if moved {
                walk.keys.push("motion");
            }
            return state;
        }
        state = walk.app.key(&state, KeyCode::Char('j'));
        moved = true;
    }
    panic!("the cursor never reached {target}");
}

/// The line a landed take leaves.
fn took(subject: &str, also: Option<&str>) -> Status {
    Status::Done {
        what: format!("took \"{subject}\" onto main"),
        also: also.map(str::to_owned),
        keys: Some("u undoes".to_owned()),
    }
}

/// `cherry-pick-in-view`: `(motion) o y` over an unsaved edit — `login-page`'s
/// commit lands on `main` (one commit, its file there), the edit back, the
/// cursor on the new commit. Captures the preview at 120×40 on the way. Then
/// `u`: `main` as before, the edit still there.
#[test]
fn cherry_pick_in_view() {
    let scratch = picker_repo();
    let path = scratch.path();
    std::fs::write(path.join("base.txt"), EDIT).expect("the scratch repo is writable");
    let pick = rev(path, "login-page");
    let main_before = rev(path, "main");
    let (app, state) = App::open(path);
    let state = crate::context_menu::settled(&app, app.resize(&ready(&app, state), 120, 40));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = reach(&mut walk, state, &pick);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    let state = planned(&app, state);

    let title = format!("Preview: take {} \"fix the bug\" onto main", &pick[..7]);
    let text = drawn(&state, 120, 40);
    assert!(text.contains(&title), "`{title}`:\n{text}");
    assert!(text.contains("y confirm"), "{text}");
    assert!(text.contains("esc cancel"), "{text}");
    check_shot(
        "m7/take-preview-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "take-preview-120x40.txt",
                "cherry_pick_in_view",
                "`o` on login-page's commit, main checked out, base.txt edited —\n\
                 # the title `Preview: take <sha7> \"fix the bug\" onto main`; the\n\
                 # footer `y confirm … esc cancel`; the set-aside line (the edit is\n\
                 # put aside and back)."
            )
        ),
    );

    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the take"));

    // The graph opens on its newest row, which here is the very commit
    // wanted: reaching it costs nothing, so the walk is `o y` (2), under the
    // note's `motion o y` (3). `cherry_pick_other_branch` walks a real motion.
    assert_eq!(walk.keys, ["o", "y"]);
    assert!(walk.keys.len() <= noted_keys("cherry-pick-in-view"));
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(rev(path, "main~1"), main_before, "one commit on main");
    assert!(
        path.join("fix.txt").exists(),
        "the commit's change is on HEAD"
    );
    assert_eq!(read(path, "base.txt"), EDIT, "the edit is back");
    y_takes_it_and_lands_on_the_new_commit(&state, path);

    let state = undo(&mut walk, &state);
    assert_eq!(rev(path, "main"), main_before, "`u` removed it");
    assert!(!path.join("fix.txt").exists(), "and its change");
    assert_eq!(read(path, "base.txt"), EDIT, "the edit survives the undo");
    drop(state);
    app.close();
}

/// The ticket's `y_takes_it_and_lands_on_the_new_commit`, walked inside
/// [`cherry_pick_in_view`]: the status line exact, the cursor on the new
/// `HEAD`.
fn y_takes_it_and_lands_on_the_new_commit(state: &AppState, path: &std::path::Path) {
    assert_eq!(
        state.status,
        took("fix the bug", Some("your edits are back"))
    );
    assert_eq!(
        cursor(state),
        Some(rev(path, "main")),
        "the cursor on the new commit"
    );
    let text = drawn(state, 120, 40);
    let line = "\u{2713} took \"fix the bug\" onto main \u{b7} your edits are back  u undoes";
    assert!(text.contains(line), "`{line}`:\n{text}");
}

/// `cherry-pick-other-branch`: `C <type> ⇥ motion o y` — the picker puts the
/// cursor on `login-page`'s tip, one row down is the commit wanted (not the
/// tip), `o y` takes it onto `main`. Then `u`.
#[test]
fn cherry_pick_other_branch() {
    let scratch = picker_repo();
    let path = scratch.path();
    git(path, &["switch", "login-page"]);
    dated_commit(path, "typo.txt", "typo\n", "fix the typo", 4);
    git(path, &["switch", "main"]);
    let pick = rev(path, "login-page~1");
    let main_before = rev(path, "main");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = walk.open(&state);
    let state = walk.type_in(&state, "login");
    let state = walk.press(&state, KeyCode::Tab, "⇥");
    assert!(state.picker.is_none(), "⇥ closed the picker");
    assert_eq!(
        cursor(&state),
        Some(rev(path, "login-page")),
        "⇥ put the cursor on the tip"
    );
    let state = reach(&mut walk, state, &pick);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    let state = planned(&app, state);
    let state = walk.press(&state, KeyCode::Char('y'), "y");
    let state = quiet(&app, app.settle_write(state, "the take"));

    assert_eq!(walk.keys, ["C", "<type>", "⇥", "motion", "o", "y"]);
    assert!(walk.keys.len() <= noted_keys("cherry-pick-other-branch"));
    assert_eq!(head(path), "refs/heads/main");
    assert_eq!(rev(path, "main~1"), main_before, "one commit on main");
    assert!(
        path.join("fix.txt").exists(),
        "the commit's change is on HEAD"
    );
    assert!(!path.join("typo.txt").exists(), "and only that commit's");
    assert_eq!(state.status, took("fix the bug", None));
    assert_eq!(cursor(&state), Some(rev(path, "main")));

    let state = undo(&mut walk, &state);
    assert_eq!(rev(path, "main"), main_before, "`u` removed it");
    drop(state);
    app.close();
}

/// `o` on a merge: refused on the status line in `check_commit`'s words, no
/// preview box. Captured at 120×40.
#[test]
fn take_refused_on_a_merge() {
    let scratch = picker_repo();
    let path = scratch.path();
    git(path, &["switch", "-c", "merged", "feature-a"]);
    let when = format!("@{} +0000", crate::preview::FORK_EPOCH + 5 * 60);
    mergewright_fixtures::git::run_at(
        path,
        &["merge", "--no-ff", "-m", "merge login-page", "login-page"],
        &when,
    )
    .unwrap_or_else(|why| panic!("merging: {why}"));
    git(path, &["switch", "main"]);
    let merge = rev(path, "merged");
    let (app, state) = App::open(path);
    let state = crate::context_menu::settled(&app, ready(&app, state));
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = reach(&mut walk, state, &merge);
    let state = walk.press(&state, KeyCode::Char('o'), "o");
    assert!(state.preview.is_none(), "no preview box");
    let words = format!(
        "{} is a merge \u{b7} taking a merge isn't in Mergewright yet",
        &merge[..7]
    );
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: words.clone(),
            keys: None
        }
    );
    let text = drawn(&state, 120, 40);
    assert!(text.contains(&words), "`{words}`:\n{text}");
    check_shot(
        "m7/take-refused-merge-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "take-refused-merge-120x40.txt",
                "take_refused_on_a_merge",
                "`o` on a merge commit — the status line `<sha7> is a merge ·\n\
                 # taking a merge isn't in Mergewright yet`; no preview box."
            )
        ),
    );
    drop(state);
    app.close();
}

/// `.` on `HEAD`'s row, `main` only there: slot 1 `Checkout — already on
/// main` and slot 2 `Cherry-pick (o) — already on main`, both dim, in
/// places 1 and 2. Captured at 120×40 (the dim itself is an attribute a
/// capture cannot carry: unverified by eye, CLAUDE.md rule 5; the cells are
/// asserted in `m6_stash_put_back::the_checkout_slot_on_head_s_row_is_drawn_dim_and_only_it`).
#[test]
fn menu_slot_two_dim_on_head() {
    let scratch = picker_repo();
    let path = scratch.path();
    let main = rev(path, "main");
    let (app, state) = App::open(path);
    let state = ready(&app, state);
    let mut walk = Walk {
        app: &app,
        keys: Vec::new(),
    };
    let state = reach(&mut walk, state, &main);
    let state = crate::context_menu::settled(&app, state);
    let state = walk.press(&state, KeyCode::Char('.'), ".");
    let open = state.context_menu.as_ref().expect("a menu is open");
    let slot = |at: usize| {
        let item = &open.menu.items[at];
        (
            item.label.as_str().to_owned(),
            item.disabled.as_ref().map(|why| why.as_str().to_owned()),
        )
    };
    assert_eq!(
        slot(0),
        ("Checkout".to_owned(), Some("already on main".to_owned()))
    );
    assert_eq!(
        slot(1),
        (
            "Cherry-pick (o)".to_owned(),
            Some("already on main".to_owned())
        )
    );
    let text = drawn(&state, 120, 40);
    assert!(text.contains("Checkout \u{2014} already on main"), "{text}");
    assert!(
        text.contains("Cherry-pick (o) \u{2014} already on main"),
        "{text}"
    );
    check_shot(
        "m7/menu-slot-two-dim-120x40.txt",
        &format!(
            "{}\n{text}\n",
            header(
                "menu-slot-two-dim-120x40.txt",
                "menu_slot_two_dim_on_head",
                "`.` on HEAD's row (main) — slot 1 `Checkout — already on main`\n\
                 # and slot 2 `Cherry-pick (o) — already on main`, both dim (dim\n\
                 # unverified: a capture carries no attributes), in places 1 and 2."
            )
        ),
    );
    drop(state);
    app.close();
}

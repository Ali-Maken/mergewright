//! m6-conflicts/71: the `.` menu on a commit row opens with Checkout, then
//! Cherry-pick, always — walked against a real repository, keys counted
//! against the design note's journeys (`menu-checkout` 3, `menu-cherry-pick`
//! 4; `docs/design-notes/m6-stash-put-back.md` "Journeys").
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;
use graft::update::detail::Loading;
use graft_core::event::{KeyCode, MouseButton};
use graft_ui::context_menu::ContextAction;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at};
use crate::preview::{FORK_NOW, dated_commit, opened};

/// `main` with one commit past the fork; `fix` with one commit of its own,
/// `fix the bug`, adding `fix.txt`. `main` is checked out. Dated commits, so
/// every id is the same on every run.
fn main_and_fix() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, "base.txt", "base\n", "add the base", 1);
    git(path, &["branch", "fix"]);
    dated_commit(path, "main-one.txt", "one\n", "main moves on", 2);
    git(path, &["switch", "fix"]);
    dated_commit(path, "fix.txt", "fixed\n", "fix the bug", 3);
    git(path, &["switch", "main"]);
    scratch
}

fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name]).trim().to_owned()
}

/// Presses `j` (not counted: getting to the row is not the journey) until
/// the cursor rests on `target`.
fn cursor_to(app: &App, mut state: AppState, target: &str) -> AppState {
    for _ in 0..20 {
        let on = state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|row| row.commit.id.to_string());
        if on.as_deref() == Some(target) {
            return state;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("the cursor never reached {target}");
}

/// Keys the journey presses, counted as they go.
struct Keys<'a> {
    app: &'a App,
    count: usize,
}

impl Keys<'_> {
    fn press(&mut self, state: &AppState, code: KeyCode) -> AppState {
        self.count += 1;
        self.app.key(state, code)
    }
}

/// The preview's plan has come back, so `y` has something to confirm.
fn planned(app: &App, state: AppState) -> AppState {
    app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !matches!(preview.plan, Loading::Pending(_) | Loading::Idle))
    })
}

fn labels(state: &AppState) -> Vec<String> {
    state
        .context_menu
        .as_ref()
        .expect("a menu is open")
        .menu
        .items
        .iter()
        .map(|item| item.label.as_str().to_owned())
        .collect()
}

/// `menu-checkout`: cursor on `fix`'s row, `. ⏎ y` — on `fix`, 3 keys.
#[test]
fn menu_checkout_is_three_keys() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let fix = rev(path, "fix");
    let (app, state) = App::open(path);
    let state = cursor_to(&app, opened(&app, state), &fix);

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = keys.press(&state, KeyCode::Char('.'));
    assert_eq!(labels(&state)[0], "Checkout fix", "{:?}", labels(&state));
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(&app, state);
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the checkout");

    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    assert!(keys.count <= 3, "{} keys", keys.count);
    drop(state);
    app.close();
}

/// `menu-cherry-pick`: cursor on `fix`'s commit, `main` checked out,
/// `. j ⏎ y` — the commit's patch lands on `main`, 4 keys. With `topic` also
/// on that commit (N7), the same four keys.
#[test]
fn menu_cherry_pick_is_four_keys() {
    for with_topic in [false, true] {
        let scratch = main_and_fix();
        let path = scratch.path();
        if with_topic {
            git(path, &["branch", "topic", "fix"]);
        }
        let fix = rev(path, "fix");
        let main_before = rev(path, "main");
        let (app, state) = App::open(path);
        let state = cursor_to(&app, opened(&app, state), &fix);

        let mut keys = Keys {
            app: &app,
            count: 0,
        };
        let state = keys.press(&state, KeyCode::Char('.'));
        let state = keys.press(&state, KeyCode::Char('j'));
        let open = state.context_menu.as_ref().expect("a menu is open");
        assert_eq!(
            open.menu.items[open.menu.cursor].label.as_str(),
            format!("Cherry-pick {}", &fix[..7]),
            "topic: {with_topic}"
        );
        let state = keys.press(&state, KeyCode::Enter);
        let state = planned(&app, state);
        let state = keys.press(&state, KeyCode::Char('y'));
        let state = app.settle_write(state, "the cherry-pick");

        assert_eq!(
            git(path, &["symbolic-ref", "HEAD"]).trim(),
            "refs/heads/main"
        );
        assert_eq!(rev(path, "main~1"), main_before, "one commit on main");
        assert!(path.join("fix.txt").exists(), "fix.txt landed on main");
        assert!(keys.count <= 4, "topic: {with_topic}: {} keys", keys.count);
        drop(state);
        app.close();
    }
}

/// A click on the dim slot does what `⏎` does: the menu stays, the status
/// says why, nothing runs. Built in m5/10 (`update/mouse.rs`); this pins
/// that the dim Checkout reaches it.
#[test]
fn a_click_on_the_dim_checkout_slot_keeps_the_menu_and_says_why() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let main = rev(path, "main");
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = cursor_to(&app, opened(&app, state), &main);
    let state = app.key(&state, KeyCode::Char('.'));

    let drawn = frame_at(&state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let (row, column) = drawn
        .iter()
        .enumerate()
        .find_map(|(y, line)| {
            line.find("Checkout — already on main")
                .map(|at| (y, line[..at].chars().count()))
        })
        .unwrap_or_else(|| panic!("no dim Checkout drawn:\n{}", drawn.join("\n")));
    let row = u16::try_from(row).expect("a row on screen");
    let column = u16::try_from(column + 2).expect("a column on screen");
    let clicked = app.mouse_down(&state, MouseButton::Left, column, row);

    let open = clicked.context_menu.as_ref().expect("the menu stays open");
    assert_eq!(open.menu.items[0].action, ContextAction::Unavailable);
    assert!(clicked.preview.is_none(), "nothing previewed");
    assert!(
        format!("{:?}", clicked.status).contains("already on main"),
        "{:?}",
        clicked.status
    );
    app.close();
}

/// Regenerates `docs/shots/m6/menu-fixed-slots-120x40.txt`.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it m6_stash_put_back`.
#[test]
fn capture_menu_fixed_slots() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let fix = rev(path, "fix");
    let main = rev(path, "main");
    let (app, state) = App::open(path);
    let state = app.resize(&state, 120, 40);
    let state = crate::context_menu::settled(&app, opened(&app, state));

    let on_fix = app.key(&cursor_to(&app, state, &fix), KeyCode::Char('.'));
    assert_eq!(
        &labels(&on_fix)[..2],
        &[
            "Checkout fix".to_owned(),
            format!("Cherry-pick {}", &fix[..7])
        ]
    );
    let fix_frame = frame_at(&on_fix, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    let closed = app.key(&on_fix, KeyCode::Escape);
    let home = app.key(&closed, KeyCode::Char('g'));
    let on_main = app.key(&cursor_to(&app, home, &main), KeyCode::Char('.'));
    let open = on_main.context_menu.as_ref().expect("a menu is open");
    assert_eq!(
        open.menu.items[0]
            .disabled
            .as_ref()
            .map(graft_term::SanitizedText::as_str),
        Some("already on main")
    );
    assert_eq!(
        open.menu.items[1].label.as_str(),
        format!("Cherry-pick {}", &main[..7])
    );
    let main_frame = frame_at(&on_main, 120, 40, Some(Clock::Fixed(FORK_NOW)));

    let body = format!(
        "{HEADER}\n== 1. `.` on fix's row ==\n\n{}\n\n== 2. `.` on main's tip (checked out, no other branch) ==\n\n{}\n",
        fix_frame.join("\n"),
        main_frame.join("\n"),
    );
    check_shot("m6/menu-fixed-slots-120x40.txt", &body);
    drop(on_main);
    app.close();
}

const HEADER: &str = "\
# m6/menu-fixed-slots-120x40.txt: 120x40, drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back.rs,
# capture_menu_fixed_slots, asserted). Not a pty capture: a character grid
# with no colour (CLAUDE.md rule 5, colour and dim unverified).
#
# What right looks like:
#  - Frame 1, `.` on fix's row: item 1 `Checkout fix`, item 2
#    `Cherry-pick <sha7>`, item 3 `Explain` (the row carries the radar's
#    ⓘ, so it is marked), then the row's own actions.
#  - Frame 2, `.` on main's tip (checked out, no other branch): item 1
#    `Checkout — already on main` (drawn dim: not provable here), item 2
#    `Cherry-pick <sha7>`.
";

//! m6-conflicts/71: the `.` menu on a commit row opens with Checkout, then
//! Cherry-pick, always — walked against a real repository, keys counted
//! against the design note's journeys (`menu-checkout` 3, `menu-cherry-pick`
//! 4; `docs/design-notes/m6-stash-put-back.md` "Journeys").
//!
//! m6-conflicts/72b: a stash pop or apply from `S` that clashes — `S p o ⏎`
//! and `S ⏎ o ⏎` at 4 keys, and `esc u` putting a clashed pop back.
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
pub(crate) fn main_and_fix() -> graft_fixtures::Scratch {
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

pub(crate) fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name]).trim().to_owned()
}

/// Presses `j` (not counted: getting to the row is not the journey) until
/// the cursor rests on `target`.
pub(crate) fn cursor_to(app: &App, mut state: AppState, target: &str) -> AppState {
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
pub(crate) fn planned(app: &App, state: AppState) -> AppState {
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
    // The `ⓘ` that puts `Explain` third is the "no upstream" mark: fix has
    // none, in a repository with no remote at all (a known false mark).
    let fix_id = graft_core::oid::Oid::from_hex(&fix).expect("a full object id");
    let fix_findings: Vec<_> = on_fix
        .radar
        .ref_findings
        .get(&fix_id)
        .map(|found| found.iter().map(|finding| &finding.evidence).collect())
        .unwrap_or_default();
    assert!(
        matches!(
            fix_findings.as_slice(),
            [graft_ops::mistake_radar::Evidence::Risk(
                graft_ops::mistake_radar::Risk::NoUpstream { .. }
            )]
        ),
        "{fix_findings:?}"
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

/// Pumps until the undo the walk pressed has run and been read back — or
/// the drift box it may raise first is up.
fn undone(app: &App, state: AppState) -> AppState {
    let state = app.pump(state, "the undo, or its box", |state| {
        state.confirm.is_some() || state.operating.is_none()
    });
    if state.confirm.is_some() {
        return state;
    }
    crate::stash_clash::quiet(app, state)
}

/// The landing, at 120x40 and 80x24, against its captures.
fn landing_shots(state: &AppState) {
    for (columns, rows, name) in [
        (120, 40, "m6/stash-clash-landing-120x40.txt"),
        (80, 24, "m6/stash-clash-landing-80x24.txt"),
    ] {
        let drawn = frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n");
        check_shot(name, &format!("{LANDING_HEADER}\n{drawn}\n"));
    }
}

/// The frame after the last file done: its words asserted, then the capture.
fn done_shot(state: &AppState) {
    let done = frame_at(state, 120, 40, Some(Clock::Fixed(FORK_NOW)));
    // The status row sits under the graph, above the pane (the stacked
    // layout at 120x40), not on the frame's last row.
    assert!(
        done.iter().any(|row| row.trim()
            == "every file done \u{b7} stash@{0} dropped \u{2713} \u{b7} u brings it back"),
        "{}",
        done.join("\n")
    );
    assert!(
        done.iter().all(|row| !row.contains("CONFLICTS")),
        "{}",
        done.join("\n")
    );
    check_shot(
        "m6/stash-clash-done-120x40.txt",
        &format!("{DONE_HEADER}\n{}\n", done.join("\n")),
    );
}

/// `stash-pop-clash`: `S p` lands in the resolver on the clashed file; `o ⏎`
/// takes yours and finishes, and the stash is dropped. ≤ 4 keys. Then `u`
/// brings the stash back. Writes the landing and done captures.
#[test]
fn a_stash_pop_that_clashes_is_four_keys() {
    use crate::stash_clash::{NOTES, a_stash_that_clashes, landed, quiet, stash_list, unmerged};
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.pump(state, "the working tree and the refs", |state| {
        state.working_tree.is_some() && state.refs.is_some()
    });

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = keys.press(&state, KeyCode::Char('S'));
    let state = app.pump(state, "the stash list", |state| {
        !matches!(state.stash.entries, Loading::Idle | Loading::Pending(_))
    });
    let state = keys.press(&state, KeyCode::Char('p'));
    let state = crate::captures::settled_frame(&app, landed(&app, state));
    assert_eq!(
        state.resolver.as_ref().map(|open| open.path.as_path()),
        Some(Path::new(NOTES)),
        "`S p` lands in the resolver on the clashed file"
    );
    landing_shots(&state);

    let state = keys.press(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, graft::update::Status::Done { what, .. } if what.contains("dropped"))
    });
    let state = crate::captures::settled_frame(&app, quiet(&app, state));

    assert!(keys.count <= 4, "{} keys", keys.count);
    assert_eq!(
        std::fs::read_to_string(path.join(NOTES)).expect("the file"),
        "committed\n",
        "the file holds yours"
    );
    assert!(stash_list(path).is_empty(), "the stash is dropped");
    assert_eq!(unmerged(path), "");
    done_shot(&state);

    let state = app.key(&state, KeyCode::Char('u'));
    let state = undone(&app, state);
    assert!(state.confirm.is_none(), "undoing the drop asks nothing");
    assert_eq!(stash_list(path).len(), 1, "`u` brought the stash back");
    drop(state);
    app.close();
}

/// `stash-apply-clash`: `S ⏎ o ⏎` — the apply clashes, yours taken, every
/// file done, and the stash is kept. ≤ 4 keys.
#[test]
fn a_stash_apply_that_clashes_is_four_keys() {
    use crate::stash_clash::{NOTES, a_stash_that_clashes, landed, quiet, stash_list, unmerged};
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.pump(state, "the working tree and the refs", |state| {
        state.working_tree.is_some() && state.refs.is_some()
    });

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = keys.press(&state, KeyCode::Char('S'));
    let state = app.pump(state, "the stash list", |state| {
        !matches!(state.stash.entries, Loading::Idle | Loading::Pending(_))
    });
    let state = keys.press(&state, KeyCode::Enter);
    let state = landed(&app, state);
    let state = keys.press(&state, KeyCode::Char('o'));
    let state = crate::resolver::settled(&app, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage", |state| state.resolver.is_none());
    let state = quiet(&app, state);

    assert!(keys.count <= 4, "{} keys", keys.count);
    assert_eq!(
        std::fs::read_to_string(path.join(NOTES)).expect("the file"),
        "committed\n"
    );
    assert_eq!(unmerged(path), "");
    assert_eq!(stash_list(path).len(), 1, "an apply keeps the stash");
    assert!(
        matches!(&state.status, graft::update::Status::Done { what, .. } if what == "every file done \u{b7} stash@{0} kept"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
}

/// `stash-pop-clash-back`: `S p`, then `esc u` (+ `y` if the drift box
/// asks) — the tree as it was before the pop, nothing unmerged, the stash
/// listed. ≤ 2 keys after the clash (+1 for the box).
#[test]
fn esc_u_puts_a_clashed_pop_back() {
    use crate::stash_clash::{NOTES, a_stash_that_clashes, landed, stash_list, unmerged};
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let stash_before = stash_list(path);
    let (app, state) = App::open(path);
    let state = crate::stash_clash::overlay_open(&app, state);
    let state = app.key(&state, KeyCode::Char('p'));
    let state = landed(&app, state);

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = keys.press(&state, KeyCode::Escape);
    let state = keys.press(&state, KeyCode::Char('u'));
    let mut state = undone(&app, state);
    let mut allowed = 2;
    if state.confirm.is_some() {
        allowed += 1;
        state = keys.press(&state, KeyCode::Char('y'));
        state = undone(&app, state);
    }

    assert!(keys.count <= allowed, "{} keys", keys.count);
    assert_eq!(
        std::fs::read_to_string(path.join(NOTES)).expect("the file"),
        "committed\n",
        "the tree is as it was before the pop"
    );
    assert_eq!(git(path, &["status", "--porcelain"]).trim(), "");
    assert_eq!(unmerged(path), "");
    assert_eq!(stash_list(path), stash_before, "the stash is listed again");
    assert!(state.clash.is_none(), "no clash left");
    drop(state);
    app.close();
}

const LANDING_HEADER: &str = "\
# m6/stash-clash-landing-<size>.txt: drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back.rs,
# a_stash_pop_that_clashes_is_four_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `S p` on a stash that clashes in notes.md.
#  - No `S` overlay; the resolver open on notes.md.
#  - Panes named `yours · main` and `stashed · stash@{0}`.
#  - The notice `stash@{0} clashed in 1 file · esc, u puts it all back`.
#  - The bar offers `o yours` and `t stashed`; no git error text anywhere.
";

const DONE_HEADER: &str = "\
# m6/stash-clash-done-120x40.txt: 120x40, drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back.rs,
# a_stash_pop_that_clashes_is_four_keys, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: after `o ⏎` on the last clashed file.
#  - No `CONFLICTS` section anywhere: the tree is clean again (yours was
#    taken, which is what is committed), so the pane shows HEAD's commit.
#  - The status row, under the graph, reads
#    `every file done · stash@{0} dropped ✓ · u brings it back`.
";

const HEADER: &str = "\
# m6/menu-fixed-slots-120x40.txt: 120x40, drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back.rs,
# capture_menu_fixed_slots, asserted). Not a pty capture: a character grid
# with no colour (CLAUDE.md rule 5, colour and dim unverified).
#
# What right looks like:
#  - Frame 1, `.` on fix's row: item 1 `Checkout fix`, item 2
#    `Cherry-pick <sha7>`, item 3 `Explain` (the row carries the radar's
#    \"no upstream\" ⓘ, a known false mark in a repository with no remote,
#    so it is marked), then the row's own actions.
#  - Frame 2, `.` on main's tip (checked out, no other branch): item 1
#    `Checkout — already on main` (drawn dim: not provable here), item 2
#    `Cherry-pick <sha7>`.
";

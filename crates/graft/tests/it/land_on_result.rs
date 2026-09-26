//! Every finished write that moves `HEAD` lands the graph cursor on the new
//! `HEAD`'s row, the detail pane following, whatever row it was on (UX §3.1,
//! m7-power-tools/03). Real repositories, real job pool, real keys; the
//! repository facts are read back with `git`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::jobs::Dispatch;
use graft::update::AppState;
use graft::update::detail::{DetailSpec, Loading};
use graft_core::event::KeyCode;
use graft_core::oid::Oid;

use crate::app_harness::{App, git};
use crate::preview::{forked, on_main, opened};
use crate::remote_by_key::{local, plain, ready, rev};

/// The id of the commit the graph cursor is on.
fn selected(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// `j` until the cursor is on `main`'s row.
fn onto_main(app: &App, state: AppState) -> AppState {
    let mut state = opened(app, state);
    for _ in 0..20 {
        if on_main(&state) {
            return state;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("never found main's row");
}

/// Runs the queued write to its answer and checks the landing was armed by
/// that answer — before any of the refresh's reads could answer — then
/// pumps until it is spent.
fn written_and_landed(app: &App, state: AppState, what: &str) -> AppState {
    let state = app.settle_write(state, what);
    assert!(
        state.land_on_head,
        "{what}: the Done owes a landing on HEAD"
    );
    // Spent, and — when HEAD's row was not laid yet — the rows its
    // `ShowRows` asked for have come.
    app.pump(state, "the landing", |state| {
        !state.land_on_head && selected(state).is_some()
    })
}

fn assert_on_head(state: &AppState, repo: &Path, what: &str) {
    let head = rev(repo, "HEAD");
    assert_eq!(
        selected(state).as_deref(),
        Some(head.as_str()),
        "{what}: the cursor is on HEAD"
    );
    assert!(!state.on_working_tree, "{what}: not the working-tree row");
    assert!(
        state.graph_has_focus(),
        "{what}: the graph has the keyboard"
    );
}

/// `m m` on `main`'s row with `feature` checked out: the cursor was on
/// `main`, and lands on the merge commit (gap row :773).
#[test]
fn after_m_m_the_cursor_is_on_the_merge() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = onto_main(&app, state);
    let before = selected(&state);
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('m'));
    let state = written_and_landed(&app, state, "`git merge`");
    assert_on_head(&state, repo, "after the merge");
    assert_ne!(selected(&state), before, "it moved off main's row");
    let parents = git(repo, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 3, "a merge: {parents}");
    drop(state);
    app.close();
}

/// The detail pane, open on `main`'s row when `m m` is pressed, is about
/// the commit the cursor landed on.
#[test]
fn the_detail_pane_follows_the_landing() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = opened(&app, state);
    let mut state = app.key(&state, KeyCode::Enter);
    assert!(state.detail.shown, "`⏎` opens the detail pane");
    for _ in 0..3 {
        if state.graph_has_focus() {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    assert!(state.graph_has_focus(), "the graph has the keys back");
    let state = onto_main(&app, state);
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('m'));
    let state = written_and_landed(&app, state, "`git merge`");
    let head = Oid::from_hex(&rev(repo, "HEAD")).expect("HEAD is an id");
    let state = app.pump(state, "the detail pane on HEAD", |state| {
        state.detail.spec == Some(DetailSpec::Commit(head))
            && matches!(state.detail.files, Loading::Ready(_))
    });
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(head)));
    drop(state);
    app.close();
}

/// `c` through the commit editor: the cursor leaves the working-tree row
/// for the new commit (the user's decision, 2026-09-26).
#[test]
fn after_a_commit_the_cursor_is_on_it() {
    let remote = local();
    let work = remote.work();
    std::fs::write(work.join("landed.txt"), "landed\n").expect("the work clone is writable");
    git(work, &["add", "landed.txt"]);
    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let before = rev(work, "HEAD");
    let mut state = app.key(&state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    for character in "land here".chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    for _ in 0..4 {
        if state.commit_editor.as_ref().map(|editor| editor.focus)
            == Some(graft_ui::commit_editor::Field::CommitButton)
        {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    let state = app.key(&state, KeyCode::Enter);
    let state = written_and_landed(&app, state, "the commit");
    assert_ne!(rev(work, "HEAD"), before, "a commit was made");
    assert_on_head(&state, work, "after the commit");
    drop(state);
    app.close();
}

/// Cherry-pick from the `.` menu, previewed, `y`: the cursor was on the
/// picked commit and lands on the copy.
#[test]
fn after_a_cherry_pick_the_cursor_is_on_the_new_commit() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = onto_main(&app, state);
    let picked = selected(&state);
    let state = menu_item(&app, &state, "Cherry-pick");
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = written_and_landed(&app, state, "the cherry-pick");
    assert_on_head(&state, repo, "after the cherry-pick");
    assert_ne!(selected(&state), picked, "the copy, not the original");
    drop(state);
    app.close();
}

/// Checkout of `main` from its row: `HEAD` moves to it, and the landing is
/// armed by the finish rather than left to the row the key was pressed on.
#[test]
fn after_a_checkout_the_cursor_is_on_the_new_head() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = onto_main(&app, state);
    let state = menu_item(&app, &state, "Checkout");
    // A clean tree switches at once (ADR-0048); a preview, when one opens,
    // is confirmed with `y`.
    let state = if state.preview.is_some() {
        let state = app.pump(state, "the preview's plan", |state| {
            state
                .preview
                .as_ref()
                .is_some_and(|preview| !preview.rows.is_empty())
        });
        app.key(&state, KeyCode::Char('y'))
    } else {
        state
    };
    let state = written_and_landed(&app, state, "the checkout");
    assert_eq!(
        git(repo, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert_on_head(&state, repo, "after the checkout");
    drop(state);
    app.close();
}

/// `p` with the cursor two rows down: it lands on the pulled tip.
#[test]
fn after_a_pull_the_cursor_is_on_the_new_head() {
    let remote = local();
    let work = remote.work();
    let theirs = remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.press(&state, plain('p'));
    let state = written_and_landed(&app, state, "the pull");
    assert_eq!(rev(work, "HEAD"), theirs, "fast-forwarded");
    assert_on_head(&state, work, "after the pull");
    drop(state);
    app.close();
}

/// `f` moves no `HEAD`: no landing is owed, the cursor stays.
#[test]
fn a_fetch_leaves_the_cursor() {
    let remote = local();
    let work = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let (app, state) = App::open(work);
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));
    let before = selected(&state);
    assert_ne!(before.as_deref(), Some(rev(work, "HEAD").as_str()));
    let state = app.press(&state, plain('f'));
    let state = app.settle_write(state, "the fetch");
    assert!(!state.land_on_head, "a fetch owes no landing");
    let state = ready(&app, state);
    assert_eq!(selected(&state), before, "the cursor stayed");
    drop(state);
    app.close();
}

/// `u` after a landed merge is not an `Operation`: it arms no landing of
/// this ticket's, and keeps whatever its own is.
#[test]
fn undo_keeps_its_own_landing() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = onto_main(&app, state);
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('m'));
    let state = written_and_landed(&app, state, "`git merge`");
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    assert!(!state.land_on_head, "undo arms no HEAD landing");
    drop(state);
    app.close();
}

/// The fresh walk has laid out no rows when the landing is spent, so the
/// cursor reaches `HEAD` by a `ShowRows` the walk answers — the landing
/// never blocks and never lands on a stale row.
#[test]
fn landing_waits_for_an_unlaid_row() {
    let remote = local();
    let repo = remote.work();
    remote
        .commit_on_remote("theirs")
        .expect("a teammate's push");
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.press(&state, plain('p'));
    let mut state = app.settle_write(state, "the pull");
    assert!(state.land_on_head, "armed by the Done");
    // The walk's rows are held back until the landing is spent, so the
    // order is the unlaid one every run: which of refs and rows answers
    // first is otherwise up to the pool.
    let mut asked_for_rows = false;
    let mut held = Vec::new();
    while state.land_on_head {
        let event = app
            .events
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("the refresh answers");
        if matches!(
            event,
            graft::update::AppEvent::Progress(_, graft::update::Progress::History(_))
        ) {
            held.push(event);
            continue;
        }
        let (next, commands) = graft::update::update(&state, event);
        if !next.land_on_head {
            asked_for_rows = commands
                .iter()
                .any(|command| matches!(command, graft::update::Command::ShowRows(..)));
        }
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }
    assert!(asked_for_rows, "the landing asked the walk for HEAD's row");
    for event in held {
        let (next, commands) = graft::update::update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }
    let tip = rev(repo, "HEAD");
    let state = app.pump(state, "HEAD's row laid", |state| {
        selected(state).as_deref() == Some(tip.as_str())
    });
    assert_on_head(&state, repo, "after the rows came");
    drop(state);
    app.close();
}

/// `.` then down the menu to the item starting `prefix`, and `⏎`.
fn menu_item(app: &App, state: &AppState, prefix: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('.'));
    for _ in 0..20 {
        let menu = &state
            .context_menu
            .as_ref()
            .expect("`.` opened the menu")
            .menu;
        if menu.items[menu.cursor].label.starts_with(prefix) {
            return app.key(&state, KeyCode::Enter);
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("no menu item starting {prefix:?}");
}

/// What right looks like (`m7/landed-after-merge-120x40.txt`): `▏` on the
/// merge commit's row (the top row, `Merge branch 'main' into feature`),
/// the detail pane on that commit (`COMMIT 0000000 … parents 8d6cd28,
/// 45fa51d`), the status line `merge main into feature ✓` in today's words.
/// The merge is made at run time, so its id is zeroed and its date redacted
/// (widths kept); its age reads `future` against the fixture's fixed clock.
/// Colour unverified.
#[test]
fn capture_landed_after_merge() {
    let scratch = forked();
    let repo = scratch.path();
    // The merge is committed by whoever runs the test unless the repository
    // says otherwise: the fixture's name keeps a real person's out.
    git(repo, &["config", "user.name", "Graft Fixture"]);
    git(repo, &["config", "user.email", "fixture@example.invalid"]);
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    let mut state = app.key(&state, KeyCode::Enter);
    for _ in 0..3 {
        if state.graph_has_focus() {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    let state = onto_main(&app, state);
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('m'));
    let state = written_and_landed(&app, state, "`git merge`");
    assert_on_head(&state, repo, "the capture");
    let head = rev(repo, "HEAD");
    let head_oid = Oid::from_hex(&head).expect("HEAD is an id");
    let state = app.pump(state, "the detail pane on the merge", |state| {
        state.detail.spec == Some(DetailSpec::Commit(head_oid))
            && matches!(state.detail.files, Loading::Ready(_))
    });
    let state = crate::captures::settled_frame(&app, state);
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(graft::session::Clock::Fixed(crate::preview::FORK_NOW)),
    );
    let mut body = format!("{}\n", frame.join("\n"));
    body = crate::captures::without_invented_id(&body, &head);
    // The detail header draws the merge's date as the wall clock and zone
    // made it: redacted, width kept.
    let made = git(
        repo,
        &[
            "log",
            "-1",
            "--date=format:%Y-%m-%d %H:%M %z",
            "--format=%ad",
        ],
    );
    let made = made.trim();
    assert!(body.contains(made), "the header shows {made}:\n{body}");
    body = body.replace(made, "YYYY-MM-DD hh:mm +zzzz");
    let cursor_row = frame
        .iter()
        .find(|row| row.contains('\u{258f}'))
        .unwrap_or_else(|| panic!("no cursor row:\n{body}"));
    assert!(cursor_row.contains("Merge"), "{cursor_row}");
    crate::captures::check_shot("m7/landed-after-merge-120x40.txt", &body);
    drop(state);
    app.close();
}

//! Journeys at a rewrite's stop (m7-power-tools/07c; note R4, journeys 4
//! and 5): a conflict stop lands on the resolver, an `edit` stop on the
//! graph with the still-to-apply block drawn, `space a c` amends and goes
//! home, and `esc a y` aborts a chain stop with every branch back.

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;
use mergewright_ui::still_to_apply::TITLE;

use super::rebase_line::{commit, cursor_to_main, frame, git, open, r, within};
use crate::app_harness::App;
use crate::rewrite_stop::{rebase_merge, relaunched, stopped_on_a1};

/// `feature-a` (a1 a2) ← `feature-b` (b1 b2) ← `feature-c` (c1), one line;
/// `main` moved on by m2, which writes `shared.txt` as b1 does: rebasing
/// the line onto main stops on b1 — step 3 of 5, branch 2 of 3.
fn chain_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    let shared = |text: &str, subject: &str, hour: u8| {
        std::fs::write(repo.join("shared.txt"), text).unwrap_or_else(|why| panic!("{why}"));
        git(repo, &["add", "-A"], hour);
        git(repo, &["commit", "-q", "-m", subject], hour);
    };
    git(repo, &["init", "-q", "-b", "main"], 0);
    shared("base\n", "m1", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(repo, "a1", 1);
    commit(repo, "a2", 2);
    git(repo, &["switch", "-q", "-c", "feature-b"], 3);
    shared("theirs\n", "b1", 3);
    commit(repo, "b2", 4);
    git(repo, &["switch", "-q", "-c", "feature-c"], 6);
    commit(repo, "c1", 6);
    git(repo, &["switch", "-q", "main"], 5);
    shared("ours\n", "m2", 5);
    git(repo, &["switch", "-q", "feature-a"], 9);
    git(repo, &["commit-graph", "write", "--reachable"], 9);
    dir
}

fn tip(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name], 0).trim().to_owned()
}

/// `y` on the preview; the rewrite stops (not a success).
fn y(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("y");
    let state = app.key(state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "y queued the rewrite");
    app.pump(state, "the stop", |state| {
        state.operating.is_none() && state.stopped.is_some()
    })
}

/// The chain stop through the app: cursor to main, `r`, `y`.
fn chain_stop(columns: u16, rows: u16) -> (tempfile::TempDir, App, AppState) {
    let dir = chain_repo();
    let (app, state) = open(dir.path(), columns, rows);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let state = y(&app, &state, &mut route);
    (dir, app, state)
}

/// `rewrite-stop-to-first-file`: after `y`, no key — the resolver is open on
/// the first conflicted file.
#[test]
fn rewrite_stop_to_first_file() {
    let (dir, app, state) = chain_stop(187, 50);
    assert!(rebase_merge(dir.path()), "git stopped");
    let route: Vec<&str> = Vec::new();
    let state = app.pump(state, "the resolver", |state| {
        state.in_progress.is_some()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some())
    });
    within("rewrite-stop-to-first-file", &route);
    let resolver = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(resolver.path, std::path::PathBuf::from("shared.txt"));
    let banner = mergewright::update::banner::banner_view(&state).expect("a banner");
    let words = mergewright_ui::graph_screen::banner_line(
        &banner,
        u16::MAX,
        mergewright_ui::keymap::Keymap::shipped(),
    );
    assert!(
        words.contains("step 3 of 5") && words.contains("feature-b, branch 2 of 3"),
        "{words}"
    );
    crate::captures::check_shot("m7/rewrite-chain-stop-187x50.txt", &frame(&state, 187, 50));
    drop(state);
    app.close();
}

/// `edit-stop-lands`: no key — the graph has the keyboard, the cursor on
/// the stopped commit, the still-to-apply block drawn.
#[test]
fn edit_stop_lands() {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    let route: Vec<&str> = Vec::new();
    within("edit-stop-lands", &route);
    assert!(!state.banner_focus, "the graph has the keyboard");
    let pending = mergewright::update::banner::pending_view(&state).expect("the block");
    // a2 is his; b1 and c1 are the followers' (07d D2), summarised.
    let rows: Vec<&str> = pending
        .rows
        .iter()
        .map(|row| row.subject.as_str())
        .collect();
    assert_eq!(rows, ["a2"], "his rows still to apply: {pending:?}");
    let followers: Vec<(&str, usize)> = pending
        .followers
        .iter()
        .map(|(branch, _, commits)| (branch.as_str(), *commits))
        .collect();
    assert_eq!(followers, [("feature-b", 1), ("feature-c", 1)]);
    let shot = frame(&state, 120, 40);
    assert!(shot.contains(TITLE), "{shot}");
    let cursor_row = shot
        .lines()
        .find(|line| line.starts_with('▸'))
        .unwrap_or_else(|| panic!("no cursor row: {shot}"));
    assert!(
        cursor_row.contains("stopped here"),
        "the cursor is on the stopped commit: {cursor_row}"
    );
    drop(state);
    app.close();
}

/// Review m7-10-to-07c Major 2: the still-to-apply block takes rows from
/// the graph, and the app's layout mirror knows it — a click on a row drawn
/// under the block selects that row, and `graph_rows` counts only the rows
/// the body really has.
#[test]
fn a_click_under_the_block_selects_the_row_drawn_there() {
    for (columns, rows) in [(120, 40), (80, 24)] {
        click_under_the_block(columns, rows);
    }
}

fn click_under_the_block(columns: u16, rows: u16) {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    let state = app.resize(&state, columns, rows);
    let shot = frame(&state, columns, rows);
    let lines: Vec<&str> = shot.lines().collect();
    let top = lines
        .iter()
        .position(|line| line.contains(TITLE))
        .unwrap_or_else(|| panic!("no block: {shot}"));
    let bottom = top
        + lines[top..]
            .iter()
            .position(|line| line.contains('╰'))
            .unwrap_or_else(|| panic!("the block has no bottom: {shot}"));
    let m1 = bottom
        + lines[bottom..]
            .iter()
            .position(|line| line.contains(" m1 ") || line.ends_with(" m1"))
            .unwrap_or_else(|| panic!("m1 is not drawn under the block: {shot}"));
    let row = u16::try_from(m1).unwrap_or_else(|why| panic!("{why}"));
    let clicked = app.mouse_down(&state, mergewright_core::event::MouseButton::Left, 30, row);
    let selected = clicked
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .trim()
                .to_owned()
        });
    assert_eq!(
        selected.as_deref(),
        Some("m1"),
        "the row drawn at {row}: {shot}"
    );
    // The body is every row from under the block to above the footer — and
    // above the rule, from 30 rows (ADR-0058 §4).
    let rule = usize::from(mergewright_ui::graph_screen::spaced(rows));
    let body = usize::from(rows) - 1 - rule - (bottom + 1);
    assert_eq!(state.graph_rows(), body, "{columns}x{rows}: {shot}");
    drop(clicked);
    app.close();
}

/// His `feature-a` of `his` commits (a1…), Sara's `feature-b` of two on
/// it, one line; Mergewright's rewrite of it from a1 stopped on `edit` at his
/// row 3 — the design note's mock-up (R4): step 3 of `his + 2`.
fn sara_line_stopped_on_a3(his: u8) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"], 0);
    commit(repo, "m1", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    for n in 1..=his {
        commit(repo, &format!("a{n}"), n);
    }
    git(repo, &["switch", "-q", "-c", "feature-b"], his + 1);
    for (hour, name) in [(his + 1, "b1"), (his + 2, "b2")] {
        std::fs::write(repo.join(format!("{name}.txt")), format!("{name}\n"))
            .unwrap_or_else(|why| panic!("write: {why}"));
        git(repo, &["add", "-A"], hour);
        git(
            repo,
            &[
                "commit",
                "-q",
                "--author",
                "Sara <sara@example.com>",
                "-m",
                name,
            ],
            hour,
        );
    }
    git(repo, &["switch", "-q", "feature-a"], his + 3);
    git(repo, &["commit-graph", "write", "--reachable"], his + 3);
    let from = format!("feature-a~{}", his - 1);
    crate::rewrite_stop::mergewright_rewrite_stops(repo, &from, Some("a3"), None);
    dir
}

/// Captures of the `edit` stop at 187×50 (his 5, Sara's 2: the note's
/// mock-up) and 80×24 (his 10, Sara's 2). Rule 6: the user judges them.
#[test]
fn capture_edit_stop_sizes() {
    let dir = sara_line_stopped_on_a3(5);
    let (app, state) = relaunched(dir.path());
    let wide = app.resize(&state, 187, 50);
    let shot = frame(&wide, 187, 50);
    assert!(shot.contains("REBASING  step 3 of 7"), "{shot}");
    let picks: Vec<&str> = shot
        .lines()
        .filter(|line| line.contains("│ pick "))
        .collect();
    assert_eq!(picks.len(), 2, "his two rows only: {shot}");
    assert!(
        picks[0].contains(" a5") && picks[1].contains(" a4"),
        "{shot}"
    );
    assert!(
        shot.contains("feature-b (Sara): 2 commits follow, on top"),
        "{shot}"
    );
    let bar = shot.lines().last().unwrap_or_default();
    for words in ["a amend it", "c continue", "space stage"] {
        assert!(bar.contains(words), "the bar has `{words}`: {bar}");
    }
    crate::captures::check_shot("m7/rewrite-edit-stop-187x50.txt", &shot);
    drop(wide);
    app.close();

    let dir = sara_line_stopped_on_a3(10);
    let (app, state) = relaunched(dir.path());
    let narrow = app.resize(&state, 80, 24);
    let shot = frame(&narrow, 80, 24);
    assert!(shot.contains("REBASING  step 3 of 12"), "{shot}");
    assert!(!shot.contains("change files"), "{shot}");
    let picks = shot.lines().filter(|line| line.contains("│ pick ")).count();
    let more = format!("+{} more", 7 - picks);
    assert!(
        shot.contains(&more),
        "the block ends its picks with `{more}`: {shot}"
    );
    eprintln!(
        "80x24: {picks} picks drawn, `{more}`, follower row drawn: {}",
        shot.contains("feature-b (Sara)")
    );
    crate::captures::check_shot("m7/rewrite-edit-stop-80x24.txt", &shot);
    drop(narrow);
    app.close();
}

/// After `a` at the edit stop on a1: still stopped, HEAD's tree has the
/// edit, the message kept, HEAD an amend of `stopped` (same parent).
fn assert_amended_in_place(repo: &Path, stopped: &str) {
    assert!(rebase_merge(repo), "still stopped");
    assert_eq!(
        git(repo, &["show", "HEAD:a1.txt"], 0),
        "a1, fixed\n",
        "HEAD's tree has the edit"
    );
    assert_eq!(
        git(repo, &["log", "-1", "--format=%s"], 0).trim(),
        "a1",
        "the message is kept"
    );
    assert_eq!(
        tip(repo, "HEAD^"),
        tip(repo, &format!("{stopped}^")),
        "an amend of the stopped commit"
    );
}

/// `edit-stop-amend-continue`: from the landing (the cursor on the stopped
/// row), his edit to a tracked file, then `space a c` (07d D1): `space`
/// stages it, `a` amends the stopped commit keeping its message, `c` takes
/// the rewrite home with the line built on the amended commit.
#[test]
fn edit_stop_amend_continue() {
    let dir = stopped_on_a1();
    let repo = dir.path();
    let (app, state) = relaunched(repo);
    let stopped = tip(repo, "HEAD");
    // At his width (187) the bar has room for the verb; at 120 the ladder
    // draws `space` bare.
    let landing = frame(&state, 187, 50);
    assert!(
        landing.contains("space stage"),
        "the landing's bar offers the stage: {landing}"
    );

    // His edit, in his editor; back in the terminal, Mergewright re-reads the
    // working tree (focus gained: an event, not a key).
    std::fs::write(repo.join("a1.txt"), "a1, fixed\n").unwrap_or_else(|why| panic!("{why}"));
    let state = app.focus_gained(&state);
    let state = app.pump(state, "the edit, read", |state| {
        state.working_tree.as_ref().is_some_and(|tree| {
            tree.entries
                .iter()
                .any(|entry| entry.path == Path::new("a1.txt") && entry.unstaged.is_some())
        })
    });

    let mut route: Vec<&str> = vec!["space"];
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.settle_write(state, "the stage");
    assert_eq!(
        git(repo, &["diff", "--cached", "--name-only"], 0).trim(),
        "a1.txt",
        "space staged the edit"
    );

    route.push("a");
    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let asked = app.key(&state, KeyCode::Char('a'));
    let asked = app.pump(asked, "the amend queued", |state| state.operating.is_some());
    let state = app.settle_write(asked, "the amend");
    assert_amended_in_place(repo, &stopped);

    route.push("c");
    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.pump(state, "the rewrite home", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    within("edit-stop-amend-continue", &route);
    assert!(!rebase_merge(repo), "the rewrite finished");
    assert_eq!(
        git(repo, &["show", "feature-a~1:a1.txt"], 0),
        "a1, fixed\n",
        "his branch's commit has the edit"
    );
    assert_eq!(
        tip(repo, "feature-b~1"),
        tip(repo, "feature-a"),
        "feature-b on top of his new tip"
    );
    drop(state);
    app.close();
}

/// `abort-from-a-chain-stop`: `esc a y` (07d D3) — the stop lands on the
/// resolver, which has no `a` of its own (keymap §2.6); `Esc` hands the
/// keyboard to the banner (§2.6a), whose `a` asks and `y` aborts. Every
/// branch back where it was, HEAD on his branch.
#[test]
fn abort_from_a_chain_stop() {
    let dir = chain_repo();
    let repo = dir.path();
    let before: Vec<String> = ["feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|name| tip(repo, name))
        .collect();
    let (app, state) = open(repo, 187, 50);
    let mut setup = Vec::new();
    let state = cursor_to_main(&app, state, &mut setup);
    let state = r(&app, &state, &mut setup);
    let state = y(&app, &state, &mut setup);
    let state = app.pump(state, "the resolver", |state| {
        state.resolver.is_some() && state.in_progress.is_some()
    });
    let mut route: Vec<&str> = vec!["esc"];
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closed the resolver");
    assert!(state.banner_focus, "Esc landed on the banner");
    route.push("a");
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(
        state.confirm.is_some(),
        "`a` asks first: {:?}",
        state.status
    );
    route.push("y");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(
        state.operating.is_some(),
        "y sent the abort: {:?}",
        state.status
    );
    let state = app.pump(state, "the abort", |state| {
        state.operating.is_none() && state.status_read.is_none() && state.in_progress.is_none()
    });
    within("abort-from-a-chain-stop", &route);
    assert!(!rebase_merge(repo), "aborted");
    let after: Vec<String> = ["feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|name| tip(repo, name))
        .collect();
    assert_eq!(after, before, "every branch back");
    assert_eq!(
        git(repo, &["symbolic-ref", "--short", "HEAD"], 0).trim(),
        "feature-a",
        "HEAD on his branch"
    );
    // The note's words (:642) are `✓ rewrite aborted · every branch is as
    // before`; today's are the resume's own (`banner::closing`). Not
    // changed here: ticket 07d NOTES, an open question for the conductor.
    assert_eq!(
        state.status,
        mergewright::update::Status::done("aborted: rebase feature-a onto main")
    );
    drop(state);
    app.close();
}

//! Recover: the "lost work" fixture (`mergewright_fixtures::lost_work`), with
//! `core.autocrlf=true` set and every working file LF — the byte-exact `u`
//! of 30q/30r is part of "you cannot lose work".
//!
//! 1. `J ⏎ y` (`lost-to-shell-reset`, 3): main has D again.
//! 2. `u` (1): back to the reset state, every working file byte-equal.
//! 3. `J n ⏎ y` (4): D again — after the undo, the undo's `●` row is newer
//!    than the reset, nothing is preselected, `n` finds D's `✗` (26's
//!    ruling). Then `J n ⏎ y` (`deleted-branch-back`, 4): spike is back.
//! 4. `J n ⏎ y` (`edit-in-undo-backup`, 4): his edit to `src/a.rs` is back.
//!
//! Total 3 + 1 + 4 + 4 + 4 = 16 keys.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;
use mergewright_fixtures::lost_work;

use super::super::blame::Route;
use super::super::journal::{
    NOW, confirm, drawn, enter_to_the_box, lost_work_with as build, open_j, opened, rev,
    selected_row, text,
};
use super::tree_bytes;
use crate::app_harness::App;

/// The scenario's keys: 3 + 1 + 4 + 4 + 4.
const TOTAL: usize = 16;

/// Set for [`promise_recover_git_clock_pinned`] by its parent.
const PINNED_CHILD: &str = "MWR_M7_PROMISE_RECOVER_CHILD";

/// Run in a child with git's clock pinned to [`NOW`], for the reason
/// `journal::the_three_lost_work_journeys` gives: the put-back's reflog
/// records must share the app's clock.
#[test]
fn promise_recover_without_leaving() {
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "m7_walk::promise::recover::promise_recover_git_clock_pinned",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PINNED_CHILD, "1")
        .env("GIT_COMMITTER_DATE", format!("@{NOW} +0000"))
        .status()
        .expect("the child runs");
    assert!(status.success(), "the pinned-clock walk failed: {status:?}");
}

/// `J`, `n` if `find`, `⏎`, `y`: one row put back.
fn put_back(app: &App, state: &AppState, route: &mut Route, find: bool, row: &str) -> AppState {
    let state = open_j(route, app, state);
    let state = if find {
        route.press(app, &state, "n", KeyCode::Char('n'))
    } else {
        state
    };
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains(row),
        "the row for {row} is selected:\n{}",
        text(&frame)
    );
    let state = enter_to_the_box(route, app, &state);
    confirm(route, app, &state, row)
}

/// The child half of [`promise_recover_without_leaving`].
#[test]
#[ignore = "the child of promise_recover_without_leaving; runs only under it, git's clock pinned"]
fn promise_recover_git_clock_pinned() {
    if std::env::var_os(PINNED_CHILD).is_none() {
        return;
    }
    let fixture = build(&[("core.autocrlf", "true")]);
    let repo = fixture.work();
    let commits = fixture.commits().clone();
    an_lf_file(repo);
    let reset_state = tree_bytes(repo);
    let (app, state) = opened(repo);
    let mut keys: Vec<&'static str> = Vec::new();

    // 1. `J ⏎ y`: D's ✗ row preselected.
    let mut route = Route::new("lost-to-shell-reset");
    let state = put_back(&app, &state, &mut route, false, lost_work::LOST_ON_MAIN);
    route.within_count();
    keys.extend(route.keys());
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main has D again");

    // 2. `u`: the reset state, byte for byte.
    keys.push("u");
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo of the put-back");
    assert_ne!(rev(repo, "refs/heads/main"), commits.d, "main off D again");
    assert_bytes(repo, &reset_state);

    // 3. `J n ⏎ y`: D again (not journey 13's start, so not held to its
    // count), then `J n ⏎ y`: spike.
    let mut again = Route::new("lost-to-shell-reset");
    let state = put_back(&app, &state, &mut again, true, lost_work::LOST_ON_MAIN);
    keys.extend(again.keys());
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main has D again");
    let mut route = Route::new("deleted-branch-back");
    let state = put_back(&app, &state, &mut route, true, lost_work::LOST_ON_SPIKE);
    route.within_count();
    keys.extend(route.keys());
    assert_eq!(rev(repo, "refs/heads/spike"), commits.e, "spike is back");

    // 4. `J n ⏎ y`: his edit.
    let mut route = Route::new("edit-in-undo-backup");
    let state = put_back(&app, &state, &mut route, true, "your edit to src/a.rs");
    route.within_count();
    keys.extend(route.keys());
    let on_disk = std::fs::read(repo.join(lost_work::EDITED)).expect("src/a.rs");
    assert_eq!(on_disk, lost_work::LOST_EDIT, "his edit is back on disk");

    assert_eq!(keys.len(), TOTAL, "the scenario's keys: {keys:?}");
    drop(state);
    app.close();
}

/// The working files as checked out (LF) under `core.autocrlf=true`, set
/// before the fixture's first commit (m7-power-tools/30w R2: set later, the
/// undo's backup was taken under a config that converted nothing, ADR-0052
/// §2). Status stays clean: git compares them normalised.
fn an_lf_file(repo: &Path) {
    let files = tree_bytes(repo);
    assert!(
        files
            .values()
            .any(|bytes| bytes.contains(&b'\n') && !bytes.windows(2).any(|w| w == b"\r\n")),
        "at least one LF file: {:?}",
        files.keys().collect::<Vec<_>>()
    );
}

/// Every working file byte-equal to `before`, named when one differs.
fn assert_bytes(repo: &Path, before: &std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>) {
    let after = tree_bytes(repo);
    let names: Vec<_> = before.keys().chain(after.keys()).collect();
    for name in names {
        assert!(
            before.get(name) == after.get(name),
            "{} differs after u: {:?} then {:?}",
            name.display(),
            before.get(name).map(|b| String::from_utf8_lossy(b)),
            after.get(name).map(|b| String::from_utf8_lossy(b))
        );
    }
}

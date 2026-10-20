//! One identity per tip (ease-pass fix round 15): a branch level with its
//! remote is the local branch to the lane, column, pane and `C`; and a
//! branch merged *into* the walker is not what the walker was built on.
//! Words only; colour unverified (rule 5).

use mergewright_core::event::KeyCode;

use super::fixture::{self, mock};
use super::screen::{cursor_on, drawn, line_with, open, pane};

/// In-sync `ops/ci` (its tip is level with `origin/ops/ci`): the pane
/// names `ops/ci`, and `C` opens the list on it.
#[test]
fn in_sync_ops_ci_row_is_on_a_branch() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let state = cursor_on(&app, state, fixture::CI_MORE);
    let lines = pane(&drawn(&state), 120);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("ops/ci"),
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("not on a branch")),
        "{lines:#?}"
    );
    let state = app.pump(app.key(&state, KeyCode::Char('C')), "the list", |state| {
        state.picker.as_ref().is_some_and(|picker| {
            matches!(picker.list, mergewright::update::detail::Loading::Ready(_))
        })
    });
    let on = state
        .picker
        .as_ref()
        .and_then(|picker| picker.selected_row())
        .map(|row| String::from_utf8_lossy(row.name.as_bytes()).into_owned());
    assert_eq!(on.as_deref(), Some("refs/heads/ops/ci"));
    app.close();
}

/// `HEAD`'s branch in sync with its remote (`ops/ci` checked out): `ops/ci
/// (you)` in the column and `(you're here)` in the pane.
#[test]
fn head_in_sync_is_you() {
    let scratch = mock();
    crate::app_harness::git(scratch.path(), &["switch", "--quiet", "ops/ci"]);
    let (app, state) = open(scratch.path(), 120);
    let state = cursor_on(&app, state, fixture::CI_MORE);
    let frame = drawn(&state);
    assert!(
        line_with(&frame, fixture::CI_MORE).contains("ops/ci (you)"),
        "{frame:#?}"
    );
    let lines = pane(&frame, 120);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("ops/ci  (you're here)"),
        "{lines:#?}"
    );
    app.close();
}

/// `dana/receipts` was merged into `wallet-release` through a second
/// parent: sara's route is `main ─◆ wallet-release ─◆ sara/fix-toc`, and
/// the pane never says `up to date with dana/receipts`.
#[test]
fn merged_in_branch_is_not_sara_s_base() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let state = cursor_on(&app, state, fixture::TOC_TWO);
    let lines = pane(&drawn(&state), 120);
    let at = lines
        .iter()
        .position(|line| line == "built on")
        .unwrap_or_else(|| panic!("no built on: {lines:#?}"));
    let route: Vec<&str> = lines[at + 1..]
        .iter()
        .take_while(|line| !line.is_empty())
        .map(|line| line.trim())
        .collect();
    assert_eq!(route, ["main", "─◆ wallet-release", "─◆ sara/fix-toc"]);
    assert!(
        !lines.iter().any(|line| line.contains("dana/receipts")),
        "{lines:#?}"
    );
    app.close();
}

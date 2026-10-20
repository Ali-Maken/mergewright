//! ADR-0064 on the main screen's repository, judged by the role a node
//! cell draws by (the mock draws a commit row's own cell as its node and
//! shows the role only in its colour, `graph()` at mock :147; colour cannot
//! be captured, so the role value is asserted, not the glyph). The `│` of a
//! lane passing a row without its commit is `ease_walk::graph::
//! g1_lane_shape_is_the_role_against_the_cursor_branch`'s.

use mergewright::update::AppState;
use mergewright_core::event::MouseButton;
use mergewright_graph::{BranchIdentity, LaneRole, Route, made_on};

use super::{drawn, line_with, main_screen_repo, opened};
use crate::captures::at_rest;

fn name(text: &str) -> BranchIdentity {
    BranchIdentity::of_name(text.as_bytes())
}

/// The role of the node cell of every window row, subject first.
fn node_roles(state: &AppState, route: &Route) -> Vec<(String, LaneRole)> {
    let window = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("a window");
    window
        .rows
        .iter()
        .map(|row| {
            let subject = String::from_utf8_lossy(&row.commit.message)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            let role = made_on(row).map_or(LaneRole::Other, |branch| route.role(branch));
            (subject, role)
        })
        .collect()
}

fn role_of(roles: &[(String, LaneRole)], subject: &str) -> LaneRole {
    roles
        .iter()
        .find(|(text, _)| text.starts_with(subject))
        .unwrap_or_else(|| panic!("no row {subject:?} in {roles:?}"))
        .1
}

/// Cursor on HEAD (`Draw the receipts list`, feature/receipts, built on
/// wallet-release): HEAD's node is `This`, the merge row and the release's
/// first commit, wallet-release's, are `Under`.
#[test]
fn on_head_the_stacked_branchs_own_rows_are_under() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let route = Route::new(
        name("main"),
        &[name("wallet-release")],
        Some(name("feature/receipts")),
    );
    let roles = node_roles(&state, &route);
    assert_eq!(role_of(&roles, "Draw the receipts list"), LaneRole::This);
    assert_eq!(
        role_of(&roles, "Merge branch 'sara/fix-toc'"),
        LaneRole::Under
    );
    assert_eq!(role_of(&roles, "Start the wallet release"), LaneRole::Under);
    drop(state);
    app.close();
}

/// Cursor on `Start the wallet release`: focus is wallet-release (built on
/// nothing but the trunk), so every feature/receipts row is `Other`, and the
/// bar's `C` names wallet-release and `receipts` appears nowhere on it.
#[test]
fn on_the_release_commit_receipts_rows_are_other_and_the_bar_names_the_release() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let frame = drawn(&state, 120);
    let y = frame
        .iter()
        .position(|line| line.contains("Start the wallet release"))
        .expect("drawn");
    let state = app.mouse_down(
        &state,
        MouseButton::Left,
        40,
        u16::try_from(y).expect("short"),
    );
    let state = at_rest(&app, state);

    let route = Route::new(name("main"), &[], Some(name("wallet-release")));
    let window = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("a window");
    let receipts: Vec<_> = window
        .rows
        .iter()
        .filter(|row| made_on(row) == Some(name("feature/receipts")))
        .collect();
    assert!(!receipts.is_empty(), "HEAD's row is feature/receipts");
    for row in receipts {
        assert_eq!(route.role(made_on(row).expect("named")), LaneRole::Other);
    }

    // At 120 the bar's hint gives way to `C switch` (its branch name is the
    // part that is cut first); `m` still names the release. At WIDE `C`
    // carries the name too.
    for (columns, wanted) in [
        (120, "m merge wallet-release into yours"),
        (super::WIDE, "C switch to wallet-release"),
    ] {
        let frame = drawn(&state, columns);
        let bar = line_with(&frame, "C switch");
        assert!(bar.contains(wanted), "{columns}: {bar:?}");
        assert!(!bar.contains("receipts"), "{columns}: {bar:?}");
    }
    drop(state);
    app.close();
}

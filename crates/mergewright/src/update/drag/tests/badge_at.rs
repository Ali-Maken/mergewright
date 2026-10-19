//! Which badges a drag may pick up, over the same scripted state as the
//! drag's other tests.

use mergewright_git::repo::HeadState;

use super::{COLUMNS, badge_cell, name, oid, ref_badges, screen_row_of};
use crate::update::AppState;

/// Only a local branch is draggable: §2.15 says a branch badge, and the
/// other four badge kinds name nothing this repository may move.
#[test]
fn badge_at_ignores_remote_tag_stash_and_head() {
    let refs = ref_badges(
        &[
            (name("refs/heads/main"), oid(1)),
            (name("refs/remotes/origin/feature"), oid(3)),
            (name("refs/tags/v1"), oid(2)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
    );
    let state = AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: crate::update::Size {
            columns: COLUMNS,
            rows: 24,
        },
        ..AppState::default()
    };
    // Rows 0 and 1 carry the remote and the tag; only row 2 has a branch.
    for row in [0_usize, 1] {
        let screen_row = screen_row_of(&state, row);
        assert!(
            (0..state.size.columns).all(|column| crate::update::geometry::badge_at(
                &state, column, screen_row
            )
            .is_none()),
            "row {row} carries no draggable badge"
        );
    }
    let (_, screen_row) = badge_cell(&state, 2);
    let source = (0..state.size.columns)
        .find_map(|column| crate::update::geometry::badge_at(&state, column, screen_row))
        .expect("`main` is draggable");
    assert_eq!(source.short, "main");
    assert!(source.current, "`main` is checked out");
}

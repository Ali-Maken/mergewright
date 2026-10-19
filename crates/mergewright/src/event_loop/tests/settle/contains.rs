//! Settle test 1 of ease-pass/07b: is the "contained in" read asked when
//! the cursor rests, with the panes as on first launch? It decides where
//! the cursor row's second line takes its `also in A, B` from.
//!
//! Found: **not asked**. The read is the detail pane's (`update::detail`'s
//! `load`); with the pane hidden — as `update::opening` starts — a rest
//! asks only the evil probe. So `also in` draws only when `ContainedIn` for
//! the cursor's commit is already held, which is after the pane read it.

use super::*;

#[test]
fn the_contains_read_is_asked_at_rest_only_with_the_detail_pane_open() {
    let (opened, _) = crate::update::opening(
        std::path::PathBuf::from("/nowhere"),
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
    );
    assert!(!opened.detail.shown, "first launch hides the detail pane");
    for shown in [false, true] {
        let jobs = Recorded::default();
        // Three keys a held key's gap apart from row 1: the first follows
        // row 2 at once; the cursor rests on row 4, a merge.
        let mut wakes = Timed::new(&jobs, held_j(3, Duration::from_millis(16)));
        let (state, _) = run(graph(1, shown), &mut wakes);
        assert_eq!(cursor(&state), 4);
        let commands = jobs.commands();
        let (contained, evil) = asked_about(&commands);
        if shown {
            assert!(contained.contains(&commit_on(4)), "{commands:?}");
        } else {
            assert!(contained.is_empty(), "pane hidden: {commands:?}");
            // It did rest: the evil probe ran for the merge it rested on.
            assert_eq!(evil, vec![commit_on(4)], "{commands:?}");
        }
    }
}

//! `show_line` on a commit the default fold hides (ease-pass 06c): the
//! merge opens and the landing goes on to the pane, as for any commit.

use super::*;
use crate::update::tests::reveal::{find_for, on, revealed};

/// A folded merge's second parent — what the detail pane lists since 06d —
/// is hidden behind the merge: following it opens the merge and lands.
#[test]
fn show_line_on_a_folded_merges_second_parent_opens_the_merge_and_lands() {
    let mut state = on_graph();
    let (merge, parent) = (commit(5), commit(210));
    let commands = show_line(
        &mut state,
        DetailSpec::Commit(parent),
        Path::new("src/x.rs"),
        Some(new_line(2)),
        None,
    );
    let find = find_for(&commands, parent);

    let shown: &[u8] = &[1, 2, 3, 4, 5, 6];
    let opened: &[u8] = &[1, 2, 3, 4, 5, 210, 6];
    let (state, commands) = revealed(&state, WALK, find, (merge, parent), (shown, opened));

    assert_eq!(on(&state), Some(parent), "the cursor on parent 2");
    assert_eq!(state.status, Status::Ready, "no miss said");
    list_job(&commands);
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(parent)));
}

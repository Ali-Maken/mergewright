//! Banner `c` in a merge: the review's door (ease-pass/05b). Moved from
//! `banner/tests.rs` (ease-pass/05s), same name.

use super::*;

/// `c` on a resolved merge **begun in the shell** (no live entry of Graft's)
/// opens the finish summary like any merge — no shell check here (m6-20 B1).
/// The `⏎` that follows sends `Resume::Continue`, which `graft_ops` routes
/// to `Operation::FinishMerge` since ticket 20 Part A.
#[test]
fn c_on_a_shell_begun_merge_opens_the_summary() {
    let mut state = stopped(InProgress::Merge, &[]);
    assert!(state.live_entries.is_empty(), "begun in the shell");
    assert_eq!(
        crate::update::commit_editor::live_operation(&state),
        Some((
            InProgress::Merge,
            crate::update::commit_editor::Begun::InShell
        ))
    );
    let (claimed, commands) = press(&mut state, KeyCode::Char('c'));
    assert!(claimed);
    let finish = state.finish.as_ref().expect("the summary is open");
    assert_eq!(finish.return_to, crate::update::finish::ReturnTo::Banner);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Resume(..))),
        "no Resume before ⏎: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFinishSummary(_))),
        "the summary is read: {commands:?}"
    );
}

//! `c` on a rebase or cherry-pick begun in the shell: the terminal goes to
//! `git <kind> --continue`, and the landing waits on one working-tree read
//! (m7-power-tools/07c). The hand-over itself is faked, as the `M` tests
//! do: the loop's answer is fed to `update` directly.

use super::*;

/// `c` pressed, git's answer fed back, then the newest working-tree read
/// answered with `in_progress` and `unmerged`.
fn hand_over_and_return(
    kind: InProgress,
    exit: Result<Option<i32>, String>,
    in_progress: Option<InProgress>,
    unmerged_paths: &[&str],
) -> AppState {
    let mut state = stopped(kind, &[]);
    assert!(state.live_entries.is_empty(), "begun in the shell");
    let (_, commands) = press(&mut state, KeyCode::Char('c'));
    let [Command::ContinueInGit { id, kind: sent }] = commands.as_slice() else {
        panic!("expected one hand-over: {commands:?}");
    };
    assert_eq!(*sent, kind);
    let (returned, _) = crate::update::update(
        &state,
        AppEvent::Produced(*id, Product::ContinuedInGit(exit)),
    );
    let Some(read_id) = returned.status_read else {
        return returned;
    };
    let read = crate::working_tree::WorkingTree {
        rewrite: None,
        status: std::sync::Arc::new(WorkingTreeStatus {
            entries: unmerged_paths.iter().map(|path| unmerged(path)).collect(),
        }),
        in_progress: in_progress.map(|kind| detail(kind, Some((4, 7)))),
        mergetool: None,
    };
    let (landed, _) = crate::update::update(
        &returned,
        AppEvent::Produced(read_id, Product::WorkingTree(Ok(Box::new(read)))),
    );
    landed
}

#[test]
fn a_shell_continue_that_finished_lands_on_the_graph() {
    for kind in [InProgress::Rebase, InProgress::CherryPick] {
        let landed = hand_over_and_return(kind, Ok(Some(0)), None, &[]);
        assert!(landed.in_progress.is_none());
        assert!(!landed.banner_focus, "the graph has the keyboard");
        assert_eq!(
            landed.status,
            Status::done(format!("{} finished", kind.continue_command()))
        );
        assert_eq!(landed.continue_in_git_owed, None, "spent");
    }
}

#[test]
fn a_shell_continue_that_stopped_again_lands_on_the_stop() {
    let landed = hand_over_and_return(
        InProgress::Rebase,
        Ok(Some(1)),
        Some(InProgress::Rebase),
        &["a.rs"],
    );
    let stop = landed.stopped.as_ref().expect("a stop");
    assert!(stop.again, "stopped again");
    assert!(landed.banner_focus, "a conflict stop focuses the banner");
    assert!(
        landed.resolver.is_some(),
        "M6's landing: the resolver on the first file"
    );
}

#[test]
fn a_shell_continue_that_exited_non_zero_keeps_the_stop() {
    let landed = hand_over_and_return(
        InProgress::CherryPick,
        Ok(Some(128)),
        Some(InProgress::CherryPick),
        &[],
    );
    assert!(landed.in_progress.is_some());
    assert_eq!(
        landed.status,
        Status::sticky_keys(
            "git cherry-pick --continue exited 128 · still stopped".to_owned(),
            Some("c continue  a abort".to_owned())
        )
    );
}

#[test]
fn a_hand_over_that_could_not_run_says_why() {
    let landed = hand_over_and_return(
        InProgress::Rebase,
        Err("there is no terminal to hand over".to_owned()),
        Some(InProgress::Rebase),
        &[],
    );
    assert!(
        matches!(&landed.status, Status::Sticky { reason, .. } if reason == "there is no terminal to hand over"),
        "{:?}",
        landed.status
    );
}

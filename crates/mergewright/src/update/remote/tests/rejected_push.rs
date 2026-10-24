//! ease-pass 28 A: a rejected push is a failure he asked for, so it draws `⊗`;
//! at 80 columns the reason is cut first and `p pull · P force` stays whole.

use mergewright_git::remote::RemoteFailure;
use mergewright_ops::operation::Operation;

use super::super::{push_params, remote_failed};
use super::{THEIRS, remote, upstream, with_facts};

#[test]
fn a_rejected_push_is_marked_and_keeps_its_keys() {
    let mut state = with_facts(
        vec![remote("origin")],
        Some("refs/heads/main"),
        Some(upstream(Some(THEIRS))),
    );
    let op = Operation::Push(push_params(&state).expect("an upstream"));
    let rejected = "push rejected: origin/main has commits you don't · p pull, then m or r · P force with lease";
    state.status = remote_failed(
        &mut state.clone(),
        &op,
        &RemoteFailure::NonFastForward,
        rejected,
    );
    for columns in [80, 120] {
        let drawn = crate::view::tests::draw(&state, columns, 24);
        let bar = drawn
            .lines()
            .last()
            .expect("a bottom row")
            .trim_matches(['"', ' ']);
        assert!(bar.starts_with("⊗ push rejected"), "{columns}: {bar}");
        assert!(
            bar.contains("p pull, then m or r · P force with lease"),
            "{columns}: {bar}"
        );
        assert_eq!(drawn.matches('⊗').count(), 1, "{columns}: {drawn}");
    }
}

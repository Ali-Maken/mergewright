//! ease-pass/18: the focus comes from the refs read's own index
//! (`RefBadges::branch_list`), built once per read, not from every
//! candidate rebuilt and sorted per ask. These pin that the index answers
//! what a scan of every candidate answered: the same branch, key and trunk
//! for every branch in focus, and the ask still carries every candidate.
//! The cost itself is the heavy tier's (`frames::fifty_thousand_refs_*`).

use super::*;

/// A refs read with remote-only branches and no local `main`: the trunk is
/// `origin/main`, a remote one.
fn remote_trunk() -> RefBadges {
    let remote = |short: &str| RefName::new(format!("refs/remotes/{short}")).expect("valid");
    crate::refs::badges(
        &[
            (full("feature"), oid(3)),
            (full("sara/fix-toc"), oid(1)),
            (remote("origin/main"), oid(9)),
            (remote("origin/omar/x"), oid(5)),
            (remote("origin/develop"), oid(8)),
            (remote("upstream/main"), oid(7)),
        ],
        &HeadState::Branch(full("feature")),
        &HashMap::<RefName, RefName>::new(),
        &[],
        |_, _| unreachable!("no upstreams to count"),
    )
    .expect("nothing to count")
}

/// What `focused` answered before the index: every candidate built, the
/// first in name order drawn under the focus, its key over all of them, and
/// the first trunk among them.
fn scanned(state: &AppState) -> Option<(Candidate, StatusKey, Option<Candidate>)> {
    let history = state.history.as_ref()?;
    let focus = crate::view::focus(history)?;
    let names = BranchNames::of(history.walk.as_ref()?);
    let every = candidates(state.refs.as_deref()?, &names);
    let branch = every.iter().find(|c| identity_of(c) == focus)?.clone();
    let key = status_key(&branch, &every);
    let trunk = every
        .iter()
        .find(|c| c.kind == CandidateKind::Trunk)
        .cloned();
    Some((branch, key, trunk))
}

/// `state` with the cursor row (row 0) on `identity`'s lane.
fn focus_on(mut state: AppState, identity: BranchIdentity) -> AppState {
    let history = state.history.as_mut().expect("a walk");
    let window = history.window.as_mut().expect("a window");
    let mut rows = window.rows.to_vec();
    rows[0].lanes = vec![Some(identity)];
    window.rows = rows.into();
    state
}

#[test]
fn the_focus_from_the_index_is_the_one_a_scan_of_every_candidate_finds() {
    for refs in [badges(), remote_trunk()] {
        let mut state = on_screen();
        state.refs = Some(Arc::new(refs.clone()));
        let every = candidates(&refs, &BranchNames::of(&walk()));
        assert!(every.len() >= 4, "a list worth checking: {every:?}");
        for candidate in &every {
            let state = focus_on(state.clone(), identity_of(candidate));
            let found = focused(&state).expect("a branch in focus");
            let Some((branch, key, trunk)) = scanned(&state) else {
                panic!("the scan finds {} too", candidate.name);
            };
            assert_eq!(found.branch, branch, "{}", candidate.name);
            assert_eq!(found.key, key, "{}", candidate.name);
            assert_eq!(found.trunk, trunk, "{}", candidate.name);
        }
        let nowhere = focus_on(state, BranchIdentity::of_name(b"deleted/pr"));
        assert_eq!(focused(&nowhere), None, "a lane no branch names");
        assert_eq!(scanned(&nowhere), None);
    }
}

#[test]
fn the_trunk_is_a_remote_one_when_no_local_branch_stands_for_it() {
    let mut state = on_screen();
    state.refs = Some(Arc::new(remote_trunk()));
    let state = focus_on(state, BranchIdentity::of_name(b"sara/fix-toc"));
    let trunk = focused(&state).and_then(|focused| focused.trunk);
    assert_eq!(
        trunk.map(|trunk| (trunk.name, trunk.kind)),
        Some(("refs/remotes/origin/main".to_owned(), CandidateKind::Trunk))
    );
}

#[test]
fn the_ask_carries_every_candidate_with_its_kind() {
    let (_, commands) = rest(&on_screen());
    let Some(Command::AtRest(_, RestRead::BranchStatus(ask))) = commands
        .iter()
        .find(|c| matches!(c, Command::AtRest(_, RestRead::BranchStatus(_))))
    else {
        panic!("asked: {commands:?}");
    };
    assert_eq!(
        ask.candidates,
        candidates(&badges(), &BranchNames::of(&walk()))
    );
    assert_eq!(ask.key, status_key(&ask.branch, &ask.candidates));
}

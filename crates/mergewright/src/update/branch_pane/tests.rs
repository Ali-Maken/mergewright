//! The branch pane's read (ease-pass 09): which branch, the candidates, the
//! ask at rest after the fold count, one job per key, the cache cleared by a
//! refs read, and the route reaching the lane colours.

use std::collections::HashMap;
use std::sync::Arc;

use mergewright_core::event::KeyCode;
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history::Order;
use mergewright_git::repo::HeadState;
use mergewright_graph::LaneRole;
use mergewright_ops::branch_status::{Behind, Merged, Pushed, Who};

use super::*;
use crate::history::{History, WalkFacts};
use crate::update::tests::{key, merge_on_screen, sized};
use crate::update::{AppEvent, Product, update};

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn full(short: &str) -> RefName {
    RefName::new(format!("refs/heads/{short}")).expect("a valid name")
}

/// `main` (off screen) ← `wallet-release` (row 1, `HEAD`) ←
/// `sara/fix-toc` (row 0, the cursor); `develop` an integration branch.
fn badges() -> RefBadges {
    crate::refs::badges(
        &[
            (full("main"), oid(9)),
            (full("develop"), oid(8)),
            (full("wallet-release"), oid(2)),
            (full("sara/fix-toc"), oid(1)),
        ],
        &HeadState::Branch(full("wallet-release")),
        &HashMap::<RefName, RefName>::new(),
        &[],
        |_, _| unreachable!("no upstreams to count"),
    )
    .expect("nothing to count")
}

fn walk() -> WalkFacts {
    WalkFacts {
        requested: Order::Topological,
        order: Order::Topological,
        default_branch: Some(full("main")),
        integration: Arc::from([full("develop")]),
    }
}

/// Two rows on screen, each on its branch's lane; refs and walk facts read.
fn on_screen() -> AppState {
    let (mut state, _) = merge_on_screen(JobId::new(5), 0);
    let history: &mut History = state.history.as_mut().expect("a walk");
    history.walk = Some(walk());
    let window = history.window.as_mut().expect("a window");
    let mut rows = window.rows.to_vec();
    rows[0].lanes = vec![Some(BranchIdentity::of_name(b"sara/fix-toc"))];
    rows[1].lanes = vec![Some(BranchIdentity::of_name(b"wallet-release"))];
    window.rows = rows.into();
    state.refs = Some(Arc::new(badges()));
    state
}

fn rest(state: &AppState) -> (AppState, Vec<Command>) {
    update(state, sized(120, 30))
}

/// The branches asked, in order.
fn asked(commands: &[Command]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::AtRest(_, RestRead::BranchStatus(ask)) => Some(ask.branch.name.clone()),
            _ => None,
        })
        .collect()
}

fn candidate(name: &str, tip: u8, kind: CandidateKind) -> Candidate {
    Candidate {
        name: format!("refs/heads/{name}"),
        tip: oid(tip),
        kind,
    }
}

fn status(route: Vec<Candidate>) -> Arc<BranchStatus> {
    Arc::new(BranchStatus {
        route,
        pushed: Pushed::NoUpstream,
        merged: Merged::NotYet { into: None },
        behind: Behind::NoBase,
        who: Who::NoOwnCommits,
    })
}

#[test]
fn candidates_are_every_local_branch_with_its_kind_by_name() {
    let candidates = candidates(&badges(), &BranchNames::of(&walk()));
    assert_eq!(
        candidates,
        [
            candidate("develop", 8, CandidateKind::Integration),
            candidate("main", 9, CandidateKind::Trunk),
            candidate("sara/fix-toc", 1, CandidateKind::Other),
            candidate("wallet-release", 2, CandidateKind::Other),
        ]
    );
}

#[test]
fn a_candidates_identity_is_its_short_names() {
    let sara = candidate("sara/fix-toc", 1, CandidateKind::Other);
    assert_eq!(identity_of(&sara), BranchIdentity::of_name(b"sara/fix-toc"));
}

#[test]
fn the_key_moves_when_any_candidate_tip_moves() {
    let facts = walk();
    let names = BranchNames::of(&facts);
    let before = candidates(&badges(), &names);
    let mut after = before.clone();
    after[0].tip = oid(7);
    let sara = before[2].clone();
    assert_eq!(status_key(&sara, &before), status_key(&sara, &before));
    assert_ne!(status_key(&sara, &before), status_key(&sara, &after));
}

#[test]
fn the_branch_ask_comes_from_settle_after_the_fold_count_ask() {
    let mut state = on_screen();
    state.fold_counts.clear();
    let (_, commands) = rest(&state);
    let count = commands
        .iter()
        .position(|c| matches!(c, Command::AtRest(_, RestRead::CountFolded { .. })));
    let branch = commands
        .iter()
        .position(|c| matches!(c, Command::AtRest(_, RestRead::BranchStatus(_))));
    let (Some(count), Some(branch)) = (count, branch) else {
        panic!("both asked at rest: {commands:?}");
    };
    assert!(count < branch, "fold count first: {commands:?}");
    assert_eq!(asked(&commands), ["refs/heads/sara/fix-toc"]);
}

#[test]
fn one_job_per_branch_another_branch_and_back_asks_one_each() {
    let (state, first) = rest(&on_screen());
    assert_eq!(asked(&first), ["refs/heads/sara/fix-toc"]);
    let (state, again) = rest(&state);
    assert!(
        asked(&again).is_empty(),
        "same branch, no second job: {again:?}"
    );
    let (state, down) = update(&state, key(KeyCode::Char('j')));
    assert_eq!(asked(&down), ["refs/heads/wallet-release"]);
    let (_, up) = update(&state, key(KeyCode::Char('k')));
    assert!(asked(&up).is_empty(), "back to sara, already asked: {up:?}");
}

#[test]
fn a_new_refs_read_clears_the_reads_and_the_next_rest_asks_again() {
    let (state, _) = rest(&on_screen());
    let (state, commands) = update(
        &state,
        AppEvent::Produced(JobId::new(77), Product::Refs(Ok(badges()))),
    );
    assert_eq!(
        asked(&commands),
        ["refs/heads/sara/fix-toc"],
        "{commands:?}"
    );
    assert_eq!(state.look.reads.by_key.len(), 1);
}

#[test]
fn the_focused_branchs_read_sets_the_route_its_bases_draw_under() {
    let (state, commands) = rest(&on_screen());
    let Some(Command::AtRest(_, RestRead::BranchStatus(ask))) = commands
        .iter()
        .find(|c| matches!(c, Command::AtRest(_, RestRead::BranchStatus(_))))
    else {
        panic!("asked: {commands:?}");
    };
    let route = vec![
        candidate("wallet-release", 2, CandidateKind::Other),
        candidate("main", 9, CandidateKind::Trunk),
    ];
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(1),
            Product::BranchStatusRead {
                key: ask.key.clone(),
                result: Ok(status(route)),
            },
        ),
    );
    let read = state.look.route.as_ref().expect("the route landed");
    assert_eq!(
        read.role(BranchIdentity::of_name(b"wallet-release")),
        LaneRole::Under
    );
    assert_eq!(read.role(BranchIdentity::of_name(b"main")), LaneRole::Trunk);
    assert_eq!(
        read.role(BranchIdentity::of_name(b"sara/fix-toc")),
        LaneRole::This
    );
    assert_eq!(
        read.role(BranchIdentity::of_name(b"develop")),
        LaneRole::Other
    );
}

#[test]
fn a_read_for_a_branch_out_of_focus_is_kept_but_colours_nothing() {
    let (state, _) = rest(&on_screen());
    let elsewhere = StatusKey {
        branch: "refs/heads/wallet-release".to_owned(),
        tips: 0,
    };
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(1),
            Product::BranchStatusRead {
                key: elsewhere.clone(),
                result: Err("stopped".to_owned()),
            },
        ),
    );
    assert_eq!(state.look.route, None);
    assert_eq!(
        state.look.reads.get(&elsewhere),
        Some(&StatusRead::Failed("stopped".to_owned()))
    );
}

/// A burst defers the ask, and the loop wakes for rest only while
/// `rest_owed` holds: an unasked focus must keep it true, or the pane reads
/// `reading…` until the next key. Asked, it is owed no more; `settled`
/// holds once the answer lands.
#[test]
fn an_unasked_focus_is_owed_to_rest_and_settles_when_its_read_lands() {
    let state = on_screen();
    assert!(owed(&state) && crate::update::rest_owed(&state));
    assert!(!settled(&state));
    let (state, commands) = rest(&state);
    assert!(!owed(&state), "asked at rest");
    assert!(!settled(&state), "asked, not landed");
    let Some(Command::AtRest(_, RestRead::BranchStatus(ask))) = commands
        .iter()
        .find(|c| matches!(c, Command::AtRest(_, RestRead::BranchStatus(_))))
    else {
        panic!("asked: {commands:?}");
    };
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            JobId::new(1),
            Product::BranchStatusRead {
                key: ask.key.clone(),
                result: Ok(status(Vec::new())),
            },
        ),
    );
    assert!(settled(&state) && !owed(&state));
}

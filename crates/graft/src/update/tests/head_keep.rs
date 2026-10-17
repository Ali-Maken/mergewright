//! The walk starts with `HEAD`'s commit kept out of the fold (ease-pass 06
//! D2): a launch lays out once, and a refs read re-lays out only when
//! `HEAD` really moved.

use std::collections::{BTreeSet, HashMap};

use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

/// The refs: `HEAD` on `main` at `tip`.
fn refs_on_main(tip: Oid) -> crate::refs::RefBadges {
    let main = RefName::new("refs/heads/main").expect("a valid name");
    crate::refs::badges(
        &[(main.clone(), tip)],
        &HeadState::Branch(main),
        &HashMap::<RefName, RefName>::new(),
        &[],
        |_, _| unreachable!("no upstreams to count"),
    )
    .expect("nothing to count")
}

fn opened(head: Option<Oid>) -> AppEvent {
    let mut facts = facts("/code/api", "main");
    facts.head_commit = head;
    AppEvent::Produced(JobId::new(1), Product::Repository(Ok(facts)))
}

fn requests(commands: &[Command]) -> Vec<RowRequest> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ShowRows(_, request) => Some(request.clone()),
            _ => None,
        })
        .collect()
}

fn kept(request: &RowRequest) -> BTreeSet<Oid> {
    request.view.inclusion.keep().expect("folding by default").clone()
}

/// A launch whose walk has started before the opening answered: the walk
/// id, the refs read's id, and the state.
fn launched() -> (AppState, JobId, JobId) {
    let (state, commands) = opening(
        PathBuf::from("/code/api"),
        Size {
            columns: 80,
            rows: 24,
        },
    );
    let walk = state.awaiting.expect("a walk is asked");
    let refs = commands
        .iter()
        .find_map(|command| match command {
            Command::ReadRefs(id) => Some(*id),
            _ => None,
        })
        .expect("the refs are asked");
    let (state, commands) = update(&state, AppEvent::Started(walk, Task::ReadHistory));
    assert_eq!(requests(&commands), Vec::new(), "the first request waits for HEAD");
    assert!(state.history.as_ref().is_some_and(|history| history.head_pending));
    (state, walk, refs)
}

#[test]
fn the_first_request_waits_for_the_opening_and_keeps_head() {
    let (state, _, _) = launched();
    let (state, commands) = update(&state, opened(Some(oid(7))));
    let asked = requests(&commands);
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(kept(&asked[0]), BTreeSet::from([oid(7)]));
    assert!(state.history.as_ref().is_some_and(|history| !history.head_pending));
}

/// An unborn branch (or an unreadable `HEAD`, which the facts say) keeps
/// nothing, and the walk still asks.
#[test]
fn an_opening_with_no_head_commit_still_releases_the_first_request() {
    let (state, _, _) = launched();
    let (_, commands) = update(&state, opened(None));
    let asked = requests(&commands);
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(kept(&asked[0]), BTreeSet::new());
}

#[test]
fn a_failed_opening_still_releases_the_first_request() {
    let (mut state, _, _) = launched();
    // Not the launch's own failure, which quits: a refresh's.
    state.repository = Some(facts("/code/api", "main"));
    let event = AppEvent::Produced(JobId::new(1), Product::Repository(Err("gone".to_owned())));
    let (_, commands) = update(&state, event);
    assert_eq!(requests(&commands).len(), 1, "{commands:?}");
}

/// The refs read lands with `HEAD` where the opening said: no request,
/// the phase stays — the launch laid out once.
#[test]
fn refs_arriving_with_head_unchanged_keeps_the_view() {
    let (state, walk, refs) = launched();
    let (state, _) = update(&state, opened(Some(oid(7))));
    let (state, _) = update(&state, rows(walk, 3));
    let before = state.history.clone().expect("a walk");
    let (state, commands) = update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_on_main(oid(7))))),
    );
    assert_eq!(requests(&commands), Vec::new(), "{commands:?}");
    let after = state.history.as_ref().expect("a walk");
    assert_eq!(after.phase, before.phase);
    assert_eq!(after.view, before.view);
}

/// The refs read lands before the opening: its `HEAD` rides in the first
/// request, still one request.
#[test]
fn refs_before_the_opening_carry_head_into_the_one_first_request() {
    let (state, _, refs) = launched();
    let (state, commands) = update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_on_main(oid(7))))),
    );
    assert_eq!(requests(&commands), Vec::new(), "{commands:?}");
    let (_, commands) = update(&state, opened(Some(oid(7))));
    let asked = requests(&commands);
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(kept(&asked[0]), BTreeSet::from([oid(7)]));
}

/// `HEAD` moved onto a merged branch's tip (a switch): the refs read keeps
/// the new tip, so the fold shows it — one re-layout, from the change.
#[test]
fn switching_to_a_merged_branch_shows_its_tip() {
    let (state, walk, refs) = launched();
    let (state, _) = update(&state, opened(Some(oid(7))));
    let (state, _) = update(&state, rows(walk, 3));
    let (state, commands) = update(
        &state,
        AppEvent::Produced(refs, Product::Refs(Ok(refs_on_main(oid(9))))),
    );
    let asked = requests(&commands);
    assert_eq!(asked.len(), 1, "{commands:?}");
    assert_eq!(kept(&asked[0]), BTreeSet::from([oid(9)]));
    assert_eq!(state.history.as_ref().map(|history| history.phase.clone()), Some(Phase::Streaming));
}

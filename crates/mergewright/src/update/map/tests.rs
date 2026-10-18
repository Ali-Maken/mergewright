//! `M`'s map over scripted state: opening builds, a stale build is
//! dropped, `⏎` closes and seeks, a resize rebuilds. Moved out of
//! `map.rs` (ease-pass 06c, move-only) so it stays under 400 lines.

use mergewright_ops::mistake_radar::Policy;
use mergewright_ui::detail::DetailFocus;

use super::*;
use crate::map::{MapBranch, MapData};
use mergewright_core::ref_name::RefName;
use mergewright_graph::identity::BranchIdentity;

fn key(code: KeyCode) -> KeyPress {
    KeyPress::plain(code)
}

fn state_with_columns(columns: u16) -> AppState {
    let mut state = AppState::default();
    state.size.columns = columns;
    state.size.rows = 40;
    state
}

fn sample_data(rows: usize) -> Arc<MapData> {
    let branches = (0..rows)
        .map(|i| MapBranch {
            name: RefName::new(format!("refs/heads/b{i}").into_bytes()).unwrap(),
            identity: BranchIdentity::of_name(format!("b{i}").as_bytes()),
            tip: Oid::from_hex(&"1".repeat(40)).unwrap(),
            tip_date: 0,
            counts: vec![0; 10],
            is_current: i == 0,
            is_default: i == 0,
            merged_into: None,
            marks: Vec::new(),
            complete: true,
        })
        .collect();
    Arc::new(MapData {
        branches,
        more: 0,
        from: 0,
        to: 1,
        buckets: 10,
        complete: true,
    })
}

#[test]
fn m_opens_the_view_with_one_build_map_command() {
    let mut state = state_with_columns(100);
    let mut commands = Vec::new();
    assert!(map_key(&mut state, &mut commands, key(KeyCode::Char('M'))));
    assert!(state.map.shown);
    match commands.as_slice() {
        [Command::BuildMap(_, req)] => {
            assert_eq!(req.buckets, 100 - 24);
            assert_eq!(req.policy, Policy::default());
            assert!(req.previous_default_tips.is_empty());
        }
        other => panic!("expected one BuildMap command, got {other:?}"),
    }
}

#[test]
fn narrow_terminals_never_ask_for_fewer_than_ten_buckets() {
    let state = state_with_columns(5);
    assert_eq!(request(&state).buckets, MIN_BUCKETS);
}

#[test]
fn m_again_or_escape_closes_and_cancels_a_pending_build() {
    let mut state = state_with_columns(100);
    let mut commands = Vec::new();
    map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
    let Loading::Pending(id) = state.map.data else {
        panic!("expected a pending build")
    };
    commands.clear();
    assert!(map_key(&mut state, &mut commands, key(KeyCode::Escape)));
    assert!(!state.map.shown);
    assert_eq!(state.map.data, Loading::Idle);
    assert_eq!(commands, vec![Command::CancelJob(id)]);
}

#[test]
fn a_product_lands_as_ready_and_a_stale_one_is_dropped() {
    let mut state = state_with_columns(100);
    let mut commands = Vec::new();
    map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
    let Loading::Pending(id) = state.map.data else {
        panic!("expected a pending build")
    };
    let data = sample_data(2);
    map_event(
        &mut state,
        super::super::events::AppEvent::Produced(
            id,
            super::super::events::Product::Map(Ok(data.clone())),
        ),
    );
    assert_eq!(state.map.data, Loading::Ready(data.clone()));

    // A stale id (from a build already superseded) changes nothing.
    let other = super::super::state::AppState::default()
        .local_jobs
        .next_id();
    map_event(
        &mut state,
        super::super::events::AppEvent::Produced(
            other,
            super::super::events::Product::Map(Err("stale".to_owned())),
        ),
    );
    assert_eq!(state.map.data, Loading::Ready(data));
}

#[test]
fn enter_on_a_row_closes_the_view_and_seeks_its_tip() {
    let mut state = state_with_columns(100);
    state.history = Some(crate::history::History::started());
    let mut commands = Vec::new();
    map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
    let Loading::Pending(id) = state.map.data else {
        panic!("expected a pending build")
    };
    state.map.data = Loading::Ready(sample_data(2));
    state.awaiting = Some(id);
    commands.clear();
    assert!(map_key(&mut state, &mut commands, key(KeyCode::Enter)));
    assert!(!state.map.shown);
    assert_eq!(state.map.data, Loading::Idle);
}

/// A merged branch's tip is hidden behind its merge by the default fold
/// (ease-pass 06c): `⏎` opens the merge and lands on the tip.
#[test]
fn enter_on_a_merged_branch_opens_its_merge_and_lands_on_its_tip() {
    use crate::update::tests::reveal::{find_for, oid_of, on, revealed};
    let walk = JobId::new(4242);
    let mut state = state_with_columns(100);
    state.history = Some(crate::history::History::started());
    let mut commands = Vec::new();
    map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
    state.map.data = Loading::Ready(sample_data(2));
    state.awaiting = Some(walk);
    commands.clear();
    map_key(&mut state, &mut commands, key(KeyCode::Enter));
    // Every sample branch's tip is 0x11.
    let (merge, tip) = (oid_of(0x20), oid_of(0x11));
    let find = find_for(&commands, tip);

    let shown: &[u8] = &[0x30, 0x20, 0x10];
    let opened: &[u8] = &[0x30, 0x20, 0x11, 0x10];
    let (state, _) = revealed(&state, walk, find, (merge, tip), (shown, opened));

    assert_eq!(on(&state), Some(tip), "the cursor on the branch's tip");
}

#[test]
fn resize_while_shown_cancels_and_rebuilds_at_the_new_width() {
    let mut state = state_with_columns(100);
    let mut commands = Vec::new();
    map_key(&mut state, &mut commands, key(KeyCode::Char('M')));
    let Loading::Pending(old) = state.map.data else {
        panic!("expected a pending build")
    };
    state.size.columns = 60;
    commands.clear();
    resized(&mut state, &mut commands);
    match commands.as_slice() {
        [Command::CancelJob(cancelled), Command::BuildMap(_, req)] => {
            assert_eq!(*cancelled, old);
            assert_eq!(req.buckets, 60 - 24);
        }
        other => panic!("expected cancel + rebuild, got {other:?}"),
    }
}

#[test]
fn nothing_happens_while_the_detail_pane_is_not_focused_on_the_graph() {
    let mut state = state_with_columns(100);
    state.detail.focus = DetailFocus::Files;
    let mut commands = Vec::new();
    assert!(!map_key(&mut state, &mut commands, key(KeyCode::Char('M'))));
    assert!(!state.map.shown);
}

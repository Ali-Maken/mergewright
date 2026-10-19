//! The two graph modes (ease-pass 06h): Folded merges (the default) and
//! Every branch — the switch between them, what Every branch turns off, and
//! the search's folded count asked again on every view change (gap
//! `ease/06c-suffix-stale`).

use super::*;
use crate::config::MergedBranches;
use crate::history::{Inside, RowFold, RowRequest, Sought};
use crate::update::keys::view_change::switch_graph_mode;
use crate::update::tests::reveal::on;

pub(super) const WALK: JobId = JobId::new(4242);

pub(super) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid")
}

/// Every row request in `commands`.
pub(super) fn row_requests(commands: &[Command]) -> Vec<&RowRequest> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::ShowRows(_, request) => Some(request),
            _ => None,
        })
        .collect()
}

/// The one row request in `commands`.
pub(super) fn one_request(commands: &[Command]) -> RowRequest {
    let requests = row_requests(commands);
    assert_eq!(requests.len(), 1, "one row request: {commands:?}");
    requests[0].clone()
}

/// A copy of `like` as commit `byte`, with no parents.
fn as_commit(like: &Row, byte: u8) -> Row {
    let mut row = like.clone();
    row.commit.id = oid(byte);
    row.commit.parents = Vec::new();
    row.drawn_parents = Vec::new();
    row
}

/// The walk's answer to `request`: `rows` from row `first`, their fold
/// facts, and what the walk found.
fn answered(
    request: u64,
    first: usize,
    (rows, fold): (Vec<Row>, Vec<RowFold>),
    sought: Option<Sought>,
) -> AppEvent {
    let window = Window {
        request,
        first,
        laid_out: first + rows.len(),
        rows: rows.into(),
        done: true,
        promoted: false,
        fold: fold.into(),
        sought: sought.map(Box::new),
    };
    AppEvent::Progress(WALK, Progress::History(HistoryUpdate::Window(window)))
}

fn rows_on_hand(state: &AppState) -> Vec<Row> {
    let history = state.history.as_ref().expect("a walk");
    let window = history.window.as_ref().expect("rows");
    window.rows.to_vec()
}

fn view_of(state: &AppState) -> View {
    state.history.as_ref().expect("a walk").view.clone()
}

/// `merge_on_screen` switched to Every branch and answered: rows plain 1,
/// merge 2 and its side 4, nothing folding; the cursor on `cursor`.
fn shown_on_screen(cursor: usize) -> (AppState, Oid) {
    let (mut state, merge) = merge_on_screen(WALK, 1);
    let request = one_request(&switch_graph_mode(&mut state, MergedBranches::Shown));
    let mut rows = rows_on_hand(&state);
    rows.push(as_commit(&rows[1], 4));
    let fold = vec![RowFold::default(); 3];
    let (mut state, _) = update(&state, answered(request.id, 0, (rows, fold), None));
    state.history.as_mut().expect("a walk").cursor = cursor;
    (state, merge)
}

#[test]
fn folded_to_every_branch_keeps_the_cursors_commit_and_is_not_an_operation() {
    let (mut state, merge) = merge_on_screen(WALK, 1);
    let journal = state.journal.clone();
    let request = one_request(&switch_graph_mode(&mut state, MergedBranches::Shown));
    assert_eq!(request.view.inclusion.fold, None, "nothing folds");
    assert_eq!(request.keep, Some(merge));
    assert!(!matches!(request.wanted, Wanted::Find { .. }), "no seek");
    assert_eq!(state.graph_mode, MergedBranches::Shown);
    assert_eq!(state.journal, journal, "no journal entry");

    let mut rows = rows_on_hand(&state);
    rows.insert(0, as_commit(&rows[0], 7));
    let fold = vec![RowFold::default(); 3];
    let (state, _) = update(&state, answered(request.id, 0, (rows, fold), None));
    assert_eq!(on(&state), Some(merge));
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.merged_branches(), MergedBranches::Shown);
}

#[test]
fn the_same_mode_again_or_no_walk_sends_nothing() {
    let (mut state, _) = shown_on_screen(1);
    assert!(switch_graph_mode(&mut state, MergedBranches::Shown).is_empty());

    let mut empty = AppState::default();
    assert!(switch_graph_mode(&mut empty, MergedBranches::Shown).is_empty());
    assert_eq!(
        empty.graph_mode,
        MergedBranches::Shown,
        "the next walk starts in it"
    );
    let (started, _) = super::screens::walk_started(&empty, WALK);
    let history = started.history.as_ref().expect("a walk");
    assert_eq!(history.merged_branches(), MergedBranches::Shown);
    let (again, _) = super::screens::walk_started(&started, JobId::new(4243));
    let history = again.history.as_ref().expect("a walk");
    assert_eq!(
        history.merged_branches(),
        MergedBranches::Shown,
        "a re-walk keeps it"
    );
}

/// Every branch → folded with the cursor on the side's 4, which folds
/// behind merge 2: one row request keeping 4, no find; the answer lands the
/// cursor on the merge with nothing opened and no reveal owed.
#[test]
fn every_branch_to_folded_lands_a_folded_away_commit_on_its_merge() {
    let (mut state, merge) = shown_on_screen(2);
    let request = one_request(&switch_graph_mode(&mut state, MergedBranches::Folded));
    assert_eq!(request.keep, Some(oid(4)));
    assert!(!matches!(request.wanted, Wanted::Find { .. }), "no seek");
    assert_eq!(request.view, View::folded());

    let rows = rows_on_hand(&state)[..2].to_vec();
    let fold = vec![
        RowFold::default(),
        RowFold {
            folds: Some(true),
            inside: None,
        },
    ];
    let sought = Sought {
        anchor_behind: Some((oid(4), merge)),
        found: None,
    };
    let (mut state, _) = update(&state, answered(request.id, 0, (rows, fold), Some(sought)));
    assert_eq!(on(&state), Some(merge));
    let history = state.history.as_mut().expect("a walk");
    assert_eq!(history.view, View::folded(), "no merge opened");
    assert_eq!(history.take_reveal(), None);
    assert_eq!(state.graph_mode, MergedBranches::Folded);
}

#[test]
fn a_switch_inside_first_parent_stays_in_it_and_closes_every_opened_merge() {
    let (state, _) = merge_on_screen(WALK, 1);
    let (mut first_parent, _) = update(&state, key(KeyCode::Char('1')));
    let request = one_request(&switch_graph_mode(&mut first_parent, MergedBranches::Shown));
    assert_eq!(request.view.follow, Follow::FirstParent);
    assert_eq!(request.view.inclusion.fold, None);

    let (mut opened, _) = update(&state, key(KeyCode::Char(' ')));
    assert_ne!(view_of(&opened), View::folded(), "a merge opened");
    switch_graph_mode(&mut opened, MergedBranches::Shown);
    let request = one_request(&switch_graph_mode(&mut opened, MergedBranches::Folded));
    assert_eq!(request.view, View::folded(), "none opened after");
}

#[test]
fn every_branch_space_does_nothing_on_any_row() {
    for cursor in 0..3 {
        let (state, _) = shown_on_screen(cursor);
        let (next, commands) = update(&state, key(KeyCode::Char(' ')));
        assert!(
            row_requests(&commands).is_empty(),
            "row {cursor}: {commands:?}"
        );
        assert_eq!(next.history, state.history, "row {cursor}");
    }
}

#[test]
fn every_branch_first_parent_and_stash_toggles_stay_unfolded() {
    let (mut state, _) = shown_on_screen(1);
    for _ in 0..2 {
        let (next, commands) = update(&state, key(KeyCode::Char('1')));
        assert_eq!(one_request(&commands).view.inclusion.fold, None);
        state = next;
    }
    state.refs = Some(Arc::new(refs_with_stash_at(oid(9))));
    for _ in 0..2 {
        let (next, commands) = update(&state, key(KeyCode::Char('H')));
        assert_eq!(one_request(&commands).view.inclusion.fold, None);
        state = next;
    }
}

/// Refs: `HEAD` on `main` at 1, and one stash entry at `stash`.
pub(super) fn refs_with_stash_at(stash: Oid) -> crate::refs::RefBadges {
    let main = mergewright_core::ref_name::RefName::new("refs/heads/main").expect("valid");
    crate::refs::badges(
        &[(main.clone(), oid(1))],
        &mergewright_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[(stash, 0)],
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges")
}

/// Refs: `HEAD` on `main` at 1, no stash.
fn refs_at_head() -> crate::refs::RefBadges {
    let main = mergewright_core::ref_name::RefName::new("refs/heads/main").expect("valid");
    crate::refs::badges(
        &[(main.clone(), oid(1))],
        &mergewright_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[],
        |_, _| Err("no upstreams here".to_owned()),
    )
    .expect("badges")
}

/// No fold, no `HEAD` keep: a refs read leaves the view and the walk as
/// they are.
#[test]
fn every_branch_a_refs_read_relays_nothing() {
    let (state, _) = shown_on_screen(1);
    let refs = AppEvent::Produced(JobId::new(3), Product::Refs(Ok(refs_at_head())));
    let (next, commands) = update(&state, refs);
    assert!(row_requests(&commands).is_empty(), "{commands:?}");
    let history = next.history.as_ref().expect("a walk");
    assert_eq!(history.phase, Phase::Complete);
    assert_eq!(history.view, view_of(&state));
}

/// At rest nothing is counted (no folding merge), so no count is owed and
/// a merge just made never gets `space show the N commits inside` (M18):
/// its bar needs a count. Folded asks one for the same merge.
#[test]
fn every_branch_asks_no_fold_count_at_rest_and_offers_no_space_after_a_merge() {
    let tall = Size {
        columns: 100,
        rows: 30,
    };
    let (mut state, merge) = shown_on_screen(1);
    state.size = tall;
    state.fold_counts.clear();
    assert!(!crate::update::fold_count::owed(&state));
    let mut commands = Vec::new();
    let mut ids = 100;
    let mut next_id = || {
        ids += 1;
        JobId::new(ids)
    };
    crate::update::fold_count::at_rest(&mut state, &mut commands, &mut next_id);
    assert!(commands.is_empty(), "{commands:?}");
    state.last_merge = Some(merge);
    assert_eq!(
        crate::update::finish::just_merged_bar(&state, Some(merge)),
        None
    );

    let (mut folded, _) = merge_on_screen(WALK, 1);
    folded.size = tall;
    folded.fold_counts.clear();
    assert!(crate::update::fold_count::owed(&folded));
    crate::update::fold_count::at_rest(&mut folded, &mut commands, &mut next_id);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::AtRest(_, RestRead::CountFolded { merge: m }) if *m == merge)),
        "{commands:?}"
    );
}

/// `space` on the last row of a long opened merge whose own row is off the
/// rows on hand: the seek to the merge finds it shown, so the cursor lands
/// on it with the merge closed and no reveal owed — one row request.
#[test]
fn space_closing_a_merge_off_the_rows_on_hand_owes_no_reveal() {
    let (state, merge) = merge_on_screen(WALK, 1);
    let (opened, commands) = update(&state, key(KeyCode::Char(' ')));
    let request = one_request(&commands);
    let like = rows_on_hand(&opened)[0].clone();
    let side: Vec<Row> = (20..23).map(|byte| as_commit(&like, byte)).collect();
    let inside = RowFold {
        folds: None,
        inside: Some(Inside { merge, label: None }),
    };
    let (mut opened, _) = update(
        &opened,
        answered(request.id, 5, (side, vec![inside; 3]), None),
    );
    let history = opened.history.as_mut().expect("a walk");
    history.top = 5;
    history.cursor = 7;

    let (closed, commands) = update(&opened, key(KeyCode::Char(' ')));
    let find = one_request(&commands);
    assert!(
        matches!(find.wanted, Wanted::Find { commit, .. } if commit == merge),
        "{find:?}"
    );
    let rows = rows_on_hand(&state);
    let fold = vec![
        RowFold::default(),
        RowFold {
            folds: Some(true),
            inside: None,
        },
    ];
    let found = Sought {
        anchor_behind: None,
        found: Some(crate::history::Found::Row),
    };
    let (mut landed, commands) = update(&closed, answered(find.id, 0, (rows, fold), Some(found)));
    assert!(row_requests(&commands).is_empty(), "{commands:?}");
    assert_eq!(on(&landed), Some(merge));
    let history = landed.history.as_mut().expect("a walk");
    assert_eq!(history.view, View::folded(), "the merge closed");
    assert_eq!(history.take_reveal(), None);
}

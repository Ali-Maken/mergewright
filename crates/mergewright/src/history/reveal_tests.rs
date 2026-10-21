//! `History` going to a commit the default fold hides (ease-pass 06c): a
//! find's answer lands, and owes a reveal only when the fold hides it; the
//! promotion's anchor lands on its merge and opens nothing; the search's
//! folded ask rides on every request until answered.

use std::sync::Arc;

use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::oid::Oid;
use mergewright_graph::Row;

use super::*;

pub(super) fn oid(byte: u8) -> Oid {
    Oid::from_sha1([byte; Oid::SHA1_BYTES])
}

fn row(byte: u8) -> Row {
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    Row {
        drawn_parents: vec![oid(byte + 1)],
        commit: Commit {
            id: oid(byte),
            parents: vec![oid(byte + 1)],
            author: who.clone(),
            committer: who,
            message: b"row".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    }
}

/// The answer to request `request`: rows `bytes` from row 0, and `sought`.
pub(super) fn answer(request: u64, bytes: &[u8], sought: Option<Sought>) -> HistoryUpdate {
    HistoryUpdate::Window(Window {
        request,
        first: 0,
        rows: bytes.iter().map(|&byte| row(byte)).collect(),
        laid_out: bytes.len(),
        walked: bytes.len(),
        hidden: 0,
        done: false,
        promoted: false,
        fold: Arc::from([]),
        sought: sought.map(Box::new),
    })
}

fn found(found: Found) -> Sought {
    Sought {
        anchor_behind: None,
        found: Some(found),
    }
}

/// A walk whose first window holds rows 1..=5, the cursor on row 0 —
/// then a seek to 9, which no row on hand holds: the find it asks.
fn seeking_9() -> (History, RowRequest) {
    let mut history = History::started(crate::config::MergedBranches::Folded);
    let first = history.show(5).expect("asks the first rows");
    history.apply(answer(first.id, &[1, 2, 3, 4, 5], None), 5);
    let find = history.seek(oid(9), 5).expect("not on hand: a find");
    assert!(matches!(find.wanted, Wanted::Find { commit, .. } if commit == oid(9)));
    (history, find)
}

#[test]
fn a_find_answered_folded_lands_on_the_merge_and_owes_one_reveal() {
    let (mut history, find) = seeking_9();
    let folded = Some(found(Found::Folded { behind: oid(3) }));
    history.apply(answer(find.id, &[1, 2, 3, 4, 5], folded), 5);

    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(3)));
    assert_eq!(history.seeking, None, "this find has ended");
    assert_eq!(
        history.take_reveal(),
        Some(Reveal {
            merge: oid(3),
            target: oid(9)
        })
    );
    assert_eq!(history.take_reveal(), None, "taken once");
}

#[test]
fn a_find_answered_row_or_not_in_view_owes_nothing() {
    let (mut history, find) = seeking_9();
    history.apply(answer(find.id, &[7, 8, 9, 10], Some(found(Found::Row))), 5);
    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(9)));
    assert_eq!(history.take_reveal(), None);

    let (mut history, find) = seeking_9();
    let cursor = history.cursor;
    history.apply(
        answer(find.id, &[1, 2, 3, 4, 5], Some(found(Found::NotInView))),
        5,
    );
    assert_eq!(history.cursor, cursor, "not in this view: the cursor stays");
    assert_eq!(history.seeking, None);
    assert_eq!(history.take_reveal(), None);
}

#[test]
fn a_promoted_anchor_the_new_order_folds_lands_on_its_merge_and_opens_nothing() {
    let mut history = History::started(crate::config::MergedBranches::Folded);
    let first = history.show(5).expect("asks the first rows");
    history.apply(answer(first.id, &[1, 2, 3, 4, 5], None), 5);
    history.cursor = 3;
    let view = history.view.clone();
    let order: Arc<[CommitHandle]> = Arc::from([]);
    let request = history.promote(order, 5).expect("carries the order");
    let anchor = request.promote.as_ref().and_then(|promote| promote.anchor);
    assert_eq!(anchor.map(|anchor| anchor.commit), Some(oid(4)));

    // The new order folds 4 behind merge 2.
    let HistoryUpdate::Window(mut promoted) = answer(request.id, &[1, 2, 5], None) else {
        unreachable!("answer builds a window");
    };
    promoted.promoted = true;
    promoted.sought = Some(Box::new(Sought {
        anchor_behind: Some((oid(4), oid(2))),
        found: None,
    }));
    history.apply(HistoryUpdate::Window(promoted), 5);

    assert_eq!(history.selected().map(|row| row.commit.id), Some(oid(2)));
    assert_eq!(history.view, view, "no merge opened");
    assert_eq!(history.take_reveal(), None);
}

#[test]
fn a_folded_ask_rides_on_every_request_until_its_answer() {
    let (mut history, _) = seeking_9();
    let ids: Arc<[Oid]> = Arc::from([oid(9), oid(12)]);
    let older = history.ask_folded(Arc::clone(&ids));
    let newer = history.ask_folded(Arc::clone(&ids));
    assert_ne!(older, newer);

    // A find still out is asked again, carrying the ask, and stays a jump.
    let again = history.ask_again(5).expect("a request to carry it");
    assert!(matches!(again.wanted, Wanted::Find { commit, .. } if commit == oid(9)));
    assert_eq!(history.jump, Some(again.id));
    let carried = again.folded.as_ref().map(|ask| ask.search);
    assert_eq!(carried, Some(newer), "the newest ask rides");

    history.apply(
        HistoryUpdate::FoldedHits {
            search: older,
            folded: 1,
            unwalked: 0,
        },
        5,
    );
    let request = history.move_cursor(scroll::Motion::Down(40), 5);
    let request = request.expect("rows beyond those on hand");
    assert!(request.folded.is_some(), "an older answer leaves it riding");

    history.apply(
        HistoryUpdate::FoldedHits {
            search: newer,
            folded: 1,
            unwalked: 0,
        },
        5,
    );
    assert!(!history.folded_ask_out());
    let request = history.ask_again(5).expect("rows");
    assert_eq!(request.folded, None, "answered: nothing rides");
}

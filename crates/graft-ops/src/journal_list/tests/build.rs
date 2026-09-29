//! `build`: dedupe, pairing, folding, placement, order, stages.

use graft_core::journal::OperationKind;

use super::{entry, list, list_with_edits, lost, oid, record, run, with_back};
use crate::journal_list::{HeldBy, JournalList, ListRow, LostEdit, LostScan, OutsideStep, build};

fn outside_rows(list: &JournalList) -> Vec<&OutsideStep> {
    list.rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Outside(step) => Some(step),
            _ => None,
        })
        .collect()
}

/// What each row is, in one word per row, for order assertions.
fn kinds(list: &JournalList) -> Vec<String> {
    list.rows
        .iter()
        .map(|row| match row {
            ListRow::Graft(id) => format!("graft {}", id.as_str()),
            ListRow::Outside(step) => format!("outside {}", step.words),
            ListRow::Lost { event, commit } => {
                format!("lost {} under {event}", commit.run.tip.as_bytes()[0])
            }
            ListRow::LostEdit { event, edit } => {
                format!("edit {} under {event}", edit.paths[0].display())
            }
        })
        .collect()
}

/// A Graft reset writes a record on `main` and its twin on `HEAD`, with the
/// entry's own before and after: one `●`, no `$` twin (ADR-0046 §2).
#[test]
fn a_graft_op_is_one_row_not_two() {
    let reset = entry(
        1000,
        OperationKind::Reset,
        "reset main",
        &[("refs/heads/main", 4, 3)],
    );
    let outside = [
        record("HEAD", 4, 3, 1001, "reset: moving to 0303030"),
        record("refs/heads/main", 4, 3, 1001, "reset: moving to 0303030"),
    ];

    let built = list(std::slice::from_ref(&reset), &outside, None, &[]);

    assert_eq!(built.rows, vec![ListRow::Graft(reset.id)]);
}

#[test]
fn head_and_branch_records_pair_within_the_window() {
    let paired = [
        with_back(record("HEAD", 4, 3, 1002, "reset: moving to HEAD~1"), 1),
        with_back(
            record("refs/heads/main", 4, 3, 1000, "reset: moving to HEAD~1"),
            1,
        ),
    ];
    let built = list(&[], &paired, None, &[]);
    let rows = outside_rows(&built);
    assert_eq!(rows.len(), 1, "{built:?}");
    assert_eq!(
        rows[0].record.refname, "refs/heads/main",
        "the branch's record stands"
    );
    assert_eq!(rows[0].words, "reset main 1 commit back");

    // Three seconds apart is two acts.
    let apart = [
        record("HEAD", 4, 3, 1003, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1000, "reset: moving to HEAD~1"),
    ];
    assert_eq!(outside_rows(&list(&[], &apart, None, &[])).len(), 2);
}

/// The oracle's shell rebase: four `HEAD` records and the branch's finish,
/// one second (git 2.54 writes them all in it).
#[test]
fn a_shell_rebase_folds_into_one_row() {
    let onto = "02".repeat(20);
    let outside = [
        record(
            "HEAD",
            5,
            5,
            1016,
            "rebase (finish): returning to refs/heads/feat",
        ),
        record("HEAD", 4, 5, 1016, "rebase (pick): G"),
        record("HEAD", 2, 4, 1016, "rebase (pick): F"),
        record("HEAD", 9, 2, 1016, "rebase (start): checkout main"),
        record(
            "refs/heads/feat",
            9,
            5,
            1016,
            &format!("rebase (finish): refs/heads/feat onto {onto}"),
        ),
        record("HEAD", 2, 9, 1015, "checkout: moving from main to feat"),
    ];

    let built = list(&[], &outside, None, &[]);

    let rows = outside_rows(&built);
    assert_eq!(rows.len(), 2, "{built:?}");
    assert_eq!(rows[0].words, "rebased feat onto main");
    assert_eq!(rows[0].folded, 4);
    assert_eq!(rows[0].record.refname, "refs/heads/feat");
    assert_eq!(rows[1].words, "switch from main to feat");
}

/// The oracle's shell `reset --hard HEAD~1` on `main`: D (4) was main's tip,
/// nothing holds it, and it sits under the reset, held by `main`.
#[test]
fn a_lost_run_sits_under_the_event_that_lost_it() {
    let outside = [
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("HEAD", 3, 4, 1004, "commit: D"),
        record("refs/heads/main", 3, 4, 1004, "commit: D"),
    ];
    let found = lost(&[run(4, 0)]);

    let built = list(&[], &outside, Some(&found), &[]);

    assert_eq!(
        kinds(&built),
        vec![
            "outside moved main to 0303030",
            "lost 4 under 0",
            r#"outside committed "D" on main"#,
        ]
    );
    let ListRow::Lost { commit, .. } = &built.rows[1] else {
        panic!("a lost row");
    };
    assert_eq!(
        commit.held_by,
        HeldBy::Branch {
            name: "main".to_owned(),
            gone: false
        }
    );
    assert_eq!(commit.subject.as_deref(), Some("subject 4"));
}

/// m7/30o Minor 5: a tip whose subject could not be read carries `None`,
/// not a placeholder sentence the screen would quote as if git had said it.
#[test]
fn an_unreadable_subject_is_none() {
    let outside = [
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
    ];
    let found = lost(&[run(4, 0)]);
    let unread = |_: graft_core::oid::Oid| None;
    let exists = |_: &str| true;
    let built = build(
        &[],
        &outside,
        Some(&found),
        &unread,
        &exists,
        &[],
        LostScan::Done,
    );
    let Some(ListRow::Lost { commit, .. }) = built
        .rows
        .iter()
        .find(|row| matches!(row, ListRow::Lost { .. }))
    else {
        panic!("a lost row: {built:?}");
    };
    assert_eq!(commit.subject, None);
}

#[test]
fn lost_ancestors_are_one_row_with_under() {
    let outside = [
        record("HEAD", 6, 3, 1005, "reset: moving to HEAD~3"),
        record("refs/heads/main", 6, 3, 1005, "reset: moving to HEAD~3"),
    ];
    let found = lost(&[run(6, 2)]);

    let built = list(&[], &outside, Some(&found), &[]);

    let lost_rows: Vec<_> = built
        .rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } => Some(commit),
            _ => None,
        })
        .collect();
    assert_eq!(lost_rows.len(), 1);
    assert_eq!(lost_rows[0].run.tip, oid(6));
    assert_eq!(lost_rows[0].run.under, 2);
}

/// A deleted branch's own log is gone; `HEAD`'s `checkout: moving from
/// spike to main` still names it, and the branch reads `gone`.
#[test]
fn a_deleted_branch_is_named_from_heads_switch() {
    let outside = [
        record("HEAD", 5, 3, 1008, "checkout: moving from spike to main"),
        record("HEAD", 3, 5, 1007, "commit: E"),
        record("HEAD", 3, 3, 1006, "checkout: moving from main to spike"),
    ];
    let found = lost(&[run(5, 0)]);

    let built = list(&[], &outside, Some(&found), &["spike"]);

    assert_eq!(
        kinds(&built),
        vec![
            "outside switch from spike to main",
            "lost 5 under 0",
            // HEAD was on spike: the logs' own switch says so.
            r#"outside committed "E" on spike"#,
            "outside switch from main to spike",
        ]
    );
    let ListRow::Lost { commit, .. } = &built.rows[1] else {
        panic!("a lost row");
    };
    assert_eq!(
        commit.held_by,
        HeldBy::Branch {
            name: "spike".to_owned(),
            gone: true
        }
    );
}

#[test]
fn rows_are_newest_first_and_lost_rows_follow_their_event() {
    let fetch = entry(1010, OperationKind::Fetch, "fetch origin", &[]);
    let commit = entry(
        1001,
        OperationKind::Commit,
        "commit",
        &[("refs/heads/main", 2, 3)],
    );
    let outside = [
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("HEAD", 3, 4, 1003, "commit: D"),
        record("refs/heads/main", 3, 4, 1003, "commit: D"),
        record("HEAD", 2, 3, 1001, "commit: C"),
        record("refs/heads/main", 2, 3, 1001, "commit: C"),
    ];
    let found = lost(&[run(4, 0)]);

    let built = list(
        &[fetch.clone(), commit.clone()],
        &outside,
        Some(&found),
        &[],
    );

    assert_eq!(
        kinds(&built),
        vec![
            format!("graft {}", fetch.id.as_str()),
            "outside moved main to 0303030".to_owned(),
            "lost 4 under 1".to_owned(),
            r#"outside committed "D" on main"#.to_owned(),
            // Graft's commit C is its entry, not a `$` row.
            format!("graft {}", commit.id.as_str()),
        ]
    );
}

#[test]
fn a_lost_edit_sits_under_its_undo() {
    let discard = entry(1000, OperationKind::Discard, "discard src/a.rs", &[]);
    let mut undo = entry(1005, OperationKind::Undo, "undo: discard src/a.rs", &[]);
    undo.reverses = Some(discard.id.clone());
    let edit = LostEdit {
        entry: undo.id.clone(),
        backup_tree: oid(8),
        paths: vec!["src/a.rs".into()],
    };

    let built = list_with_edits(&[discard.clone(), undo.clone()], &[], None, &[], &[edit]);

    assert_eq!(
        kinds(&built),
        vec![
            format!("graft {}", undo.id.as_str()),
            "edit src/a.rs under 0".to_owned(),
            format!("graft {}", discard.id.as_str()),
        ]
    );
}

#[test]
fn later_stages_keep_earlier_rows_in_order() {
    let fetch = entry(1010, OperationKind::Fetch, "fetch origin", &[]);
    let commit = entry(
        1001,
        OperationKind::Commit,
        "commit",
        &[("refs/heads/main", 2, 3)],
    );
    let entries = [fetch, commit];
    let outside = [
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 3, 4, 1003, "commit: D"),
        record("refs/heads/main", 2, 3, 1001, "commit: C"),
    ];
    let found = lost(&[run(4, 0)]);

    let one = list(&entries, &[], None, &[]);
    let two = list(&entries, &outside, None, &[]);
    let three = list(&entries, &outside, Some(&found), &[]);

    let keeps = |earlier: &JournalList, later: &JournalList| {
        let mut at = later.rows.iter();
        earlier
            .rows
            .iter()
            .all(|row| at.any(|candidate| candidate == row))
    };
    assert!(keeps(&one, &two), "{one:?}\n{two:?}");
    assert!(keeps(&two, &three), "{two:?}\n{three:?}");
    assert_eq!(two.rows.len(), 4);
    assert_eq!(three.rows.len(), 5);
}

#[test]
fn scan_state_is_carried() {
    for scan in [
        LostScan::Running,
        LostScan::Done,
        LostScan::Capped { checked: 500 },
        LostScan::Failed("git exited 128".to_owned()),
    ] {
        let built = build(&[], &[], None, &|_| None, &|_| true, &[], scan.clone());
        assert_eq!(built.scan, scan);
    }
}

/// `git stash` resets `HEAD` to itself; that moved nothing and is no row.
#[test]
fn a_record_that_moved_nothing_is_no_row() {
    let outside = [
        record("HEAD", 3, 3, 1003, "reset: moving to HEAD"),
        record("HEAD", 3, 3, 1002, "checkout: moving from main to spike"),
    ];
    let built = list(&[], &outside, None, &[]);
    assert_eq!(
        kinds(&built),
        vec!["outside switch from main to spike"],
        "a switch between branches on one commit still moved HEAD"
    );
}

/// A Graft switch records `HEAD`'s branches, not a commit; its `checkout:`
/// record is matched by the names and the time.
#[test]
fn a_graft_switch_is_one_row() {
    let mut switch = entry(1000, OperationKind::Checkout, "switch to spike", &[]);
    switch.head_after = Some(graft_core::journal::HeadRef::Branch(
        "refs/heads/spike".to_owned(),
    ));
    let outside = [record(
        "HEAD",
        3,
        5,
        1001,
        "checkout: moving from main to spike",
    )];

    let built = list(&[switch.clone()], &outside, None, &[]);

    assert_eq!(built.rows, vec![ListRow::Graft(switch.id)]);
}

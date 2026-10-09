use std::path::PathBuf;

use graft_ops::undo::Replaced;

use super::*;

fn one(path: &str) -> Replaced {
    Replaced {
        paths: vec![PathBuf::from(path)],
        ..Replaced::default()
    }
}

fn drawn(status: &Status) -> String {
    match status {
        Status::Done { what, also, keys } => {
            crate::view::status::done_line(what, also.as_deref(), keys.as_deref())
        }
        other => panic!("a landing is Done: {other:?}"),
    }
}

/// Review m5: the time machine's landing, 93 cells with the path, fits
/// the 80-column footer with its key whole.
#[test]
fn the_time_machine_landing_fits_80_columns_and_keeps_its_key() {
    let what = "reverted to before \u{201c}checkout fix\u{201d} (2 steps)";
    let status = landing(what.to_owned(), &one("fix.txt"), false, 2);
    assert_eq!(
        status,
        Status::done_with_keys(what, "U (2 times) brings it back")
    );
    assert!(drawn(&status).chars().count() <= STATUS_COLUMNS);

    // A deep path is cut from the left first, as the box cuts it.
    let status = landing(
        "undo: checkout fix".to_owned(),
        &one("src/app/very/deep/directory/tree/for/the/file/under/fix.txt"),
        false,
        1,
    );
    assert_eq!(
        status,
        Status::done_with_keys(
            "undo: checkout fix",
            "U brings back your edit to \u{2026}/for/the/file/under/fix.txt"
        )
    );
    assert!(drawn(&status).chars().count() <= STATUS_COLUMNS);

    // A description so long nothing else fits: `what` is cut, the key
    // is not.
    let long = format!(
        "reverted to before \u{201c}{}\u{201d} (2 steps)",
        "x".repeat(90)
    );
    let status = landing(long, &one("fix.txt"), false, 2);
    let line = drawn(&status);
    assert!(
        line.ends_with(" \u{b7} U (2 times) brings it back"),
        "{line}"
    );
    assert_eq!(line.chars().count(), STATUS_COLUMNS, "{line}");
}

/// Review m7: a redo's backup words are in `what`, never in `keys`.
#[test]
fn a_redo_landing_says_the_backup_in_what_with_no_keys() {
    assert_eq!(
        landing("redo: checkout fix".to_owned(), &one("fix.txt"), true, 1),
        Status::done("redo: checkout fix; a backup keeps your edit to fix.txt")
    );
    let long = format!("redo: commit \"{}\"", "y".repeat(90));
    let status = landing(long, &one("fix.txt"), true, 1);
    let Status::Done {
        what,
        also: None,
        keys: None,
    } = &status
    else {
        panic!("no keys on a redo: {status:?}");
    };
    assert!(what.ends_with("\u{2026}; a backup keeps it"), "{what}");
    assert!(drawn(&status).chars().count() <= STATUS_COLUMNS);
}

/// Nothing replaced: the landing is `what` alone.
#[test]
fn a_clean_run_lands_on_what_alone() {
    assert_eq!(
        landing(
            "undo: stage a.rs".to_owned(),
            &Replaced::default(),
            false,
            1
        ),
        Status::done("undo: stage a.rs")
    );
}

/// Review m8, decided in `graft-ops/tests/it/undo_replaced_whole.rs`: a
/// whole restore is named by what it restores.
#[test]
fn a_whole_restore_is_named_by_what_it_restores() {
    let whole = |whole_tree, whole_index| Replaced {
        whole_tree,
        whole_index,
        ..Replaced::default()
    };
    for (replaced, named) in [
        (whole(false, true), "the whole index"),
        (whole(true, false), "the whole working tree"),
        (whole(true, true), "the whole working tree and index"),
    ] {
        assert_eq!(
            landing("undo: stage a.rs".to_owned(), &replaced, false, 1),
            Status::done_with_keys("undo: stage a.rs", format!("U brings back {named}")),
        );
        let drift = graft_ops::undo::Drift {
            unknown: vec!["the index"],
            replaced,
            ..graft_ops::undo::Drift::default()
        };
        assert_eq!(
            drift_consequence(&drift, false, 1, CONFIRM_COLUMNS, 4),
            format!("Undo replaces {named}; a backup keeps it, and U brings it back.")
        );
    }
}

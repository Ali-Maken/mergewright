#![allow(clippy::unwrap_used, clippy::expect_used)]

use graft_core::journal::OperationKind;

use super::*;
use crate::journal::tests::committed;

fn exited(said: &str) -> RunError {
    RunError::Exited {
        status: 1,
        stderr_first_line: said.to_owned(),
        command: "git something".to_owned(),
    }
}

fn stash() -> Oid {
    Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").unwrap()
}

fn f_txt() -> Vec<PathBuf> {
    vec![PathBuf::from("f.txt")]
}

/// B1 (review m7-30-06d Minor 1): the operation failed, the edits came
/// back through the fallback. The failure line says the staging went.
#[test]
fn a_failed_operation_after_the_fallback_names_the_staging() {
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let error = answer(
        Outcome::Back {
            unstaged: f_txt(),
            not_as_they_were: None,
        },
        entry,
        stash(),
        Some(exited("error: your local changes would be overwritten")),
        Vec::new(),
    )
    .expect_err("the operation failed");
    assert!(matches!(error, ApplyError::Git { .. }), "{error:?}");
    let line = error.explain();
    assert!(line.contains("staging not kept: f.txt"), "{line}");
    assert!(line.contains("would be overwritten"), "{line}");
}

/// B2: the drop failed after the fallback. The line says where the
/// edits are **and** that their staging went.
#[test]
fn a_failed_drop_after_the_fallback_names_the_staging() {
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let kept = Kept::Git {
        error: exited("error: could not drop"),
        back: true,
        unstaged: f_txt(),
        not_as_they_were: None,
    };
    let outcome = Outcome::kept(kept, &Ok(Some(0)), stash());
    let error = answer(outcome, entry, stash(), None, Vec::new()).expect_err("S kept");
    let line = error.explain();
    assert!(line.contains("staging not kept: f.txt"), "{line}");
    assert!(
        line.contains("your edits are back · a copy stays in stash@{0}"),
        "{line}"
    );
}

/// A raw step that failed on `src/a.rs`, one path to write.
fn sample_not_as_they_were() -> NotAsTheyWere {
    NotAsTheyWere {
        error: crate::restore::RestoreError::WriteRaw {
            path: PathBuf::from("src/a.rs"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        },
        files: 1,
    }
}

const RAW_WORDS: &str = "your edits are back · src/a.rs not written back as it was: denied · \
                         its exact bytes are in the backup";

/// Ruling 2026-10-01 §1, at the seam: a clean put-back (`S` dropped,
/// `Returned::Back`) whose raw step failed. The entry closes as the
/// operation did — `Committed` — and the answer is an after-state
/// failure with `NotAsTheyWere` first, `StagingNotKept` after it when
/// the fallback ran.
#[test]
fn a_clean_put_back_whose_raw_step_failed_is_committed_not_as_they_were_first() {
    let outcome = Outcome::Back {
        unstaged: f_txt(),
        not_as_they_were: Some(sample_not_as_they_were()),
    };
    assert_eq!(
        status_for(&outcome, None, EntryStatus::Committed),
        EntryStatus::Committed
    );
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let Err(ApplyError::AfterState { applied, unread }) =
        answer(outcome, entry, stash(), None, Vec::new())
    else {
        panic!("not an after-state failure");
    };
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert!(
        matches!(
            unread.as_slice(),
            [
                AfterState::NotAsTheyWere { files: 1, .. },
                AfterState::StagingNotKept { .. }
            ]
        ),
        "{unread:?}"
    );
    assert_eq!(unread[0].to_string(), RAW_WORDS);

    // Without the fallback, it is the one thing reported.
    let whole = Outcome::Back {
        unstaged: Vec::new(),
        not_as_they_were: Some(sample_not_as_they_were()),
    };
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let Err(ApplyError::AfterState { unread, .. }) =
        answer(whole, entry, stash(), None, Vec::new())
    else {
        panic!("not an after-state failure");
    };
    assert!(
        matches!(unread.as_slice(), [AfterState::NotAsTheyWere { .. }]),
        "{unread:?}"
    );
}

/// The drop could not find `S` after a raw step failed: `NotAsTheyWere`
/// first, `SetAsideKept` after it (Ruling 2026-10-01 §1).
#[test]
fn an_unlisted_drop_after_a_failed_raw_step_reports_both_raw_first() {
    let kept = Kept::Unlisted {
        why: FactsError::State(std::io::Error::other("the stash list is unreadable")),
        unstaged: Vec::new(),
        not_as_they_were: Some(sample_not_as_they_were()),
    };
    let outcome = Outcome::kept(kept, &Ok(None), stash());
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let Err(ApplyError::AfterState { unread, .. }) =
        answer(outcome, entry, stash(), None, Vec::new())
    else {
        panic!("not an after-state failure");
    };
    assert!(
        matches!(
            unread.as_slice(),
            [
                AfterState::NotAsTheyWere { .. },
                AfterState::SetAsideKept { .. }
            ]
        ),
        "{unread:?}"
    );
}

/// A git failure has no `unread` to carry a failed raw step: its words
/// ride on git's line, after the rest (builder's call, rule 3).
#[test]
fn a_git_failure_after_a_failed_raw_step_carries_its_words() {
    let kept = Kept::Git {
        error: exited("error: could not drop"),
        back: true,
        unstaged: Vec::new(),
        not_as_they_were: Some(sample_not_as_they_were()),
    };
    let outcome = Outcome::kept(kept, &Ok(Some(0)), stash());
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let error = answer(outcome, entry, stash(), None, Vec::new()).expect_err("S kept");
    assert!(matches!(error, ApplyError::Git { .. }), "{error:?}");
    let line = error.explain();
    assert!(
        line.contains(&format!("error: could not drop; {RAW_WORDS}")),
        "{line}"
    );

    let back = Outcome::Back {
        unstaged: Vec::new(),
        not_as_they_were: Some(sample_not_as_they_were()),
    };
    let entry = committed(1, OperationKind::Checkout, 1, 2);
    let error = answer(
        back,
        entry,
        stash(),
        Some(exited("error: your local changes would be overwritten")),
        Vec::new(),
    )
    .expect_err("the operation failed");
    let line = error.explain();
    assert!(
        line.contains(&format!("would be overwritten; {RAW_WORDS}")),
        "{line}"
    );
}

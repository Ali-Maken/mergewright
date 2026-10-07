use super::*;

fn denied(path: &str) -> RestoreError {
    RestoreError::WriteRaw {
        path: PathBuf::from(path),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
    }
}

/// Ruling 2026-10-01 §1: the three wordings, exact. One path names it;
/// more than one names the first that failed; a failed check carries
/// its own words. No `u` clause in any.
#[test]
fn not_as_they_were_says_the_edits_are_back_and_where_the_bytes_are() {
    let one = AfterState::NotAsTheyWere {
        error: denied("src/a.rs"),
        files: 1,
    };
    assert_eq!(
        one.to_string(),
        "your edits are back · src/a.rs not written back as it was: denied · its exact bytes \
         are in the backup"
    );
    let more = AfterState::NotAsTheyWere {
        error: denied("src/b.rs"),
        files: 3,
    };
    assert_eq!(
        more.to_string(),
        "your edits are back · not all written back as they were: src/b.rs: denied · their \
         exact bytes are in the backup"
    );
    let check = AfterState::NotAsTheyWere {
        error: RestoreError::Run(RunError::Unparsable("hash-object printed \"?\"".to_owned())),
        files: 0,
    };
    assert_eq!(
        check.to_string(),
        "your edits are back · not written back as they were: git's output did not parse as \
         expected: hash-object printed \"?\" · their exact bytes are in the backup"
    );
}

/// m8-polish-and-ship/13c: a paused write explains itself in the conflict's
/// terms, and only a stop's errors say where git was left.
#[test]
fn a_paused_write_says_so_and_only_a_stop_has_a_stopped_at() {
    use graft_core::journal::{EntryStatus, OperationKind};

    use crate::journal::tests::committed;

    let mut pending = committed(1, OperationKind::Merge, 1, 2);
    pending.description = "merge feature into main".to_owned();
    pending.status = EntryStatus::Pending;
    let paused = ApplyError::Paused {
        entry: Box::new(pending.clone()),
        kind: InProgress::Merge,
        paths: Vec::new(),
    };
    assert_eq!(
        paused.explain(),
        "merge feature into main: stopped · the merge is paused · c continues, a aborts"
    );
    assert_eq!(
        paused.stopped_at(),
        Some(StoppedAt::Paused(InProgress::Merge))
    );

    let conflicted = ApplyError::Conflicted {
        entry: Box::new(pending.clone()),
        kind: InProgress::Merge,
        paths: Vec::new(),
    };
    assert_eq!(conflicted.stopped_at(), None, "a conflict is not a stop");
    assert_eq!(
        ApplyError::Stopped.stopped_at(),
        None,
        "stopped before git ran"
    );
    let failed = ApplyError::Git {
        entry: Box::new(pending),
        error: RunError::Exited {
            status: 1,
            stderr_first_line: "fatal: no".to_owned(),
            command: "git merge".to_owned(),
        },
        recoverable: true,
        edits: None,
    };
    assert_eq!(failed.stopped_at(), None, "git's own failure is not a stop");
}

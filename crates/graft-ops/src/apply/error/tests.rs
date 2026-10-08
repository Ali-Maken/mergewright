use super::*;

fn denied(path: &str) -> RestoreError {
    RestoreError::WriteRaw {
        path: PathBuf::from(path),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        tried: 1,
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
        paused.explain(&crate::keys::Letters),
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
        paused: None,
    };
    assert_eq!(failed.stopped_at(), None, "git's own failure is not a stop");
}

fn described(kind: graft_core::journal::OperationKind, description: &str) -> JournalEntry {
    let mut entry = crate::journal::tests::committed(1, kind, 1, 2);
    description.clone_into(&mut entry.description);
    entry
}

/// 13d Q4b: a merge's `pre-merge-commit` hook said no, with no stop. git
/// paused the merge (`c`/`a` are git's own offer) and nothing is unmerged,
/// so there is nothing to resolve and the words do not say there is.
#[test]
fn a_hook_rejection_with_nothing_unmerged_says_a_hook_stopped_it() {
    use graft_core::journal::{EntryStatus, OperationKind};

    let mut entry = described(OperationKind::Merge, "merge feature into main");
    entry.status = EntryStatus::Pending;
    let rejected = ApplyError::Conflicted {
        entry: Box::new(entry.clone()),
        kind: InProgress::Merge,
        paths: Vec::new(),
    };
    assert_eq!(
        rejected.explain(&crate::keys::Letters),
        "merge feature into main: a git hook stopped it · c continues, a aborts"
    );
    let conflicted = ApplyError::Conflicted {
        entry: Box::new(entry),
        kind: InProgress::Merge,
        paths: vec![PathBuf::from("a.txt")],
    };
    assert_eq!(
        conflicted.explain(&crate::keys::Letters),
        "merge feature into main: 1 unmerged · resolve, then c continues, a aborts",
        "a real conflict keeps its words"
    );
}

/// Nothing unmerged is not a hook outside a merge: a rewrite stops on an
/// `edit` row (`apply::stopped_on_conflict`'s `edit_stop`) and an empty
/// pick stops with nothing unmerged too. Those keep today's words.
#[test]
fn nothing_unmerged_outside_a_merge_does_not_blame_a_hook() {
    use graft_core::journal::{EntryStatus, OperationKind};

    for (kind, op, state) in [
        (OperationKind::Rewrite, "reword a1b2c3d", InProgress::Rebase),
        (
            OperationKind::CherryPick,
            "pick feature work",
            InProgress::CherryPick,
        ),
    ] {
        let mut entry = described(kind, op);
        entry.status = EntryStatus::Pending;
        let stopped = ApplyError::Conflicted {
            entry: Box::new(entry),
            kind: state,
            paths: Vec::new(),
        };
        assert_eq!(
            stopped.explain(&crate::keys::Letters),
            format!("{op}: 0 unmerged · resolve, then c continues, a aborts")
        );
    }
}

/// 13d Q3 and Q4: a local stop git did not leave paused says so by whether
/// the operation could have been paused at all, and a finish of a merge
/// begun in the shell that leaves the merge paused says that.
#[test]
fn a_stop_git_did_not_pause_says_whether_it_could_have() {
    use graft_core::journal::OperationKind;

    let stopped = |kind, paused| ApplyError::Git {
        entry: Box::new(described(kind, "x")),
        error: RunError::Stopped,
        recoverable: true,
        edits: None,
        paused,
    };
    assert_eq!(
        stopped(OperationKind::Merge, None).stopped_at(),
        Some(StoppedAt::BeforePause { changed: true }),
        "a merge opens an in-progress state: it could have been paused"
    );
    assert_eq!(
        stopped(OperationKind::Discard, None).stopped_at(),
        Some(StoppedAt::Stopped { changed: true }),
        "a discard can never be paused"
    );
    assert_eq!(
        stopped(OperationKind::FinishMerge, Some(InProgress::Merge)).stopped_at(),
        Some(StoppedAt::Paused(InProgress::Merge)),
        "the merge it finishes is still paused"
    );
}

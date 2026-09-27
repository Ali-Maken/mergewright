use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_git::run::RunError;

use super::*;

/// **Pin (m6-conflicts/68, rule 7).** A resume git failed part-way
/// closes its entry `Aborted` exactly as a user's abort does
/// (`graft_ops::apply::resume::close_resume`). It must answer `Failed`,
/// never `Done`: `banner::resumed_words` reads an `Aborted` entry on a
/// `Done` answer to an abort as `aborted: … ✓`.
#[test]
fn a_resume_git_fails_answers_failed() {
    let entry = JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 1,
        kind: OperationKind::Rebase,
        description: "rebase main onto fix".to_owned(),
        status: EntryStatus::Aborted {
            reason: "git is not installed".to_owned(),
        },
        touches: Touches::default(),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: None,
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        put_back: None,
    };
    let answered = answer(Err(ApplyError::Git {
        entry: Box::new(entry),
        error: RunError::GitNotFound,
        recoverable: true,
        edits: None,
    }));
    assert!(
        matches!(answered, Operated::Failed { .. }),
        "a failed resume reached {answered:?}"
    );
}

/// A failed checkout's entry: no ref named (`touches_checkout`), `HEAD`
/// read after git stopped as `head_after`.
fn failed_checkout(head_after: Option<HeadRef>) -> ApplyError {
    let main = HeadRef::Branch("refs/heads/main".to_owned());
    ApplyError::Git {
        entry: Box::new(JournalEntry {
            id: EntryId::new(1, [0; 10]),
            at_unix_ms: 1,
            kind: OperationKind::Checkout,
            description: "checkout feature".to_owned(),
            status: EntryStatus::Aborted {
                reason: "post-checkout hook exited 1".to_owned(),
            },
            touches: Touches {
                refs: Vec::new(),
                index: true,
                worktree: true,
            },
            head_before: main,
            head_after,
            refs_before: std::collections::BTreeMap::new(),
            refs_after: std::collections::BTreeMap::new(),
            index_before: None,
            index_after: None,
            worktree_before: None,
            worktree_after: None,
            backup_ref: None,
            pins: Vec::new(),
            commands: Vec::new(),
            reverses: None,
            merge_before: None,
            merge_after: None,
            put_back: None,
        }),
        error: RunError::GitNotFound,
        recoverable: true,
        edits: None,
    }
}

/// m6-conflicts/84 (ADR-0045 §5 amended): where the edits are is
/// `graft_ops`' answer, carried as it is — the job reads no stash list. A
/// failure that set nothing aside carries none.
#[test]
fn a_failure_carries_where_graft_ops_says_the_edits_are() {
    let at = graft_ops::EditsAt::Back { copy: Some(1) };
    let mut failed = failed_checkout(None);
    if let ApplyError::Git { edits, .. } = &mut failed {
        *edits = Some(at);
    }
    assert!(
        matches!(answer(Err(failed)), Operated::Failed { edits: Some(found), .. } if found == at)
    );
    assert!(matches!(
        answer(Err(failed_checkout(None))),
        Operated::Failed { edits: None, .. }
    ));
}

/// Review m10 (m6-conflicts/80): refs are not `HEAD`. `git switch` moves
/// `HEAD` and then fails on its `post-checkout` hook
/// (`graft-ops/tests/it/checkout_hook_fails.rs` shows it on a real
/// repository), so a failed checkout that names no ref can still stand on a
/// new `HEAD`: the graph is re-read. `HEAD` where it was is the working
/// tree's alone; `HEAD` unread is re-read whole.
#[test]
fn a_failed_checkout_that_moved_head_rereads_everything() {
    let feature = HeadRef::Branch("refs/heads/feature".to_owned());
    let main = HeadRef::Branch("refs/heads/main".to_owned());
    assert_eq!(
        refresh_after_failure(&failed_checkout(Some(feature))),
        Some(Refresh::Everything)
    );
    assert_eq!(
        refresh_after_failure(&failed_checkout(None)),
        Some(Refresh::Everything)
    );
    assert_eq!(
        refresh_after_failure(&failed_checkout(Some(main))),
        Some(Refresh::WorkingTree)
    );
}

/// A checkout that ran and committed, as `apply` hands it back inside an
/// `AfterState` failure.
fn committed_checkout() -> Box<Applied> {
    let ApplyError::Git { mut entry, .. } = failed_checkout(None) else {
        panic!("failed_checkout builds a Git failure");
    };
    entry.status = EntryStatus::Committed;
    Box::new(Applied {
        entry: *entry,
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: graft_ops::undo::Replaced::default(),
    })
}

fn staging_not_kept() -> graft_ops::apply::AfterState {
    graft_ops::apply::AfterState::StagingNotKept {
        paths: vec![PathBuf::from("f.txt")],
    }
}

/// Ticket 30b, Decided 4: the fallback **alone** is its own outcome; a drop
/// that failed after it (`SetAsideKept`) or any second unread item is
/// really unconfirmed, and stays `Incomplete`.
#[test]
fn answer_maps_staging_not_kept_alone_to_its_own_outcome() {
    let alone = answer(Err(ApplyError::AfterState {
        applied: committed_checkout(),
        unread: vec![staging_not_kept()],
    }));
    assert!(
        matches!(&alone, Operated::StagingNotKept { paths, applied }
            if paths == &[PathBuf::from("f.txt")] && applied.entry.description == "checkout feature"),
        "{alone:?}"
    );

    let kept = graft_ops::apply::AfterState::SetAsideKept {
        error: graft_ops::facts::FactsError::State(std::io::Error::other("no stash list")),
        words: "your edits are in stash@{0} · staging not kept: f.txt".to_owned(),
    };
    let drop_failed = answer(Err(ApplyError::AfterState {
        applied: committed_checkout(),
        unread: vec![kept],
    }));
    assert!(
        matches!(drop_failed, Operated::Incomplete { .. }),
        "{drop_failed:?}"
    );

    let two = answer(Err(ApplyError::AfterState {
        applied: committed_checkout(),
        unread: vec![
            staging_not_kept(),
            graft_ops::apply::AfterState::BackupGone {
                name: "refs/graft/backup/x".to_owned(),
            },
        ],
    }));
    assert!(matches!(two, Operated::Incomplete { .. }), "{two:?}");
}

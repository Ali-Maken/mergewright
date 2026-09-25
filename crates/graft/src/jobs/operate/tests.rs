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
    }
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

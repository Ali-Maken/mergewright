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

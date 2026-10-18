//! Abort refuses while git holds a merge's autostash.
//! Moved out of `finish_merge.rs` to keep that file under its size row.

use super::*;

/// So `a` refuses while `MERGE_AUTOSTASH` waits, by name, before anything
/// is written: the stash is never at risk (m6-conflicts/57).
#[test]
fn abort_refuses_while_git_holds_an_autostash() {
    let repo = autostashed_merge_resolved();
    let head = oid(&repo, "HEAD");
    let files = merge_files(&repo);

    let refused = run_resume(&repo, Resume::Abort, NOW).expect_err("refused");
    assert!(
        matches!(
            refused,
            ApplyError::Refused(Refusal::AutostashWaits {
                end: MergeEnd::Abort
            })
        ),
        "{refused:?}"
    );
    assert_eq!(
        refused.explain(&crate::support::Letters),
        "git set your changes aside for this merge (autostash); abort it in the shell: git merge \
         --abort"
    );
    assert!(journal_ids(&repo).is_empty(), "no entry");
    assert_eq!(oid(&repo, "HEAD"), head, "HEAD unmoved");
    assert!(merging(&repo), "still merging");
    assert_eq!(merge_files(&repo), files, "the merge files untouched");
    assert!(
        git_path(&repo, "MERGE_AUTOSTASH").exists(),
        "the stash waits"
    );
    assert_eq!(repo.read("notes.txt"), b"committed\n", "not applied");
}

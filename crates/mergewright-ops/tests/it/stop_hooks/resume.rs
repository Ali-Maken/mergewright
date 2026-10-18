//! A resume stopped in its hook stays paused.
//! Moved out of `stop_hooks.rs` to keep that file under its size row.

use super::*;

/// Paused, then `c`, and the commit's own hook is stuck and stopped too: git
/// fails the commit, `MERGE_HEAD` stays, and the entry is held again — his
/// edit still waiting in `S` (19d). Then `c` with the hook gone: the merge
/// commits, the edit is back with its split, `S` dropped.
#[test]
fn a_resume_stopped_in_its_hook_stays_paused() {
    let repo = diverged();
    let split = porcelain(&repo);
    let hooked = Hooked::stuck(&repo, "pre-merge-commit");
    let Err(ApplyError::Paused { entry, .. }) = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    ) else {
        panic!("paused");
    };
    hooked.remove("pre-merge-commit");
    hooked.remove("hook");
    let again = Hooked::stuck(&repo, "pre-commit");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let stop = StopOnceRunning(again.running());
    let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
    let resumed = resume(&mut ctx, Resume::Continue { solved: Vec::new() });

    let Err(error) = resumed else {
        panic!("a stopped resume does not commit");
    };
    assert_eq!(
        error.stopped_at(),
        Some(StoppedAt::Paused(InProgress::Merge))
    );
    let ApplyError::Paused { entry: held, .. } = error else {
        panic!("paused again, not {error:?}");
    };
    assert_eq!(held.id, entry.id);
    assert_eq!(entry_now(&repo, &entry.id).status, EntryStatus::Pending);
    assert!(
        held.commands.iter().any(|line| line.contains("--continue")),
        "the resume's line is held with the entry: {:?}",
        held.commands
    );
    assert_his_edit_waits_in_s(&repo, &held);
    again.assert_gone();

    again.remove("pre-commit");
    let continued = resume_with(&repo, Resume::Continue { solved: Vec::new() }, NOW + 2)
        .unwrap_or_else(|why| panic!("c continues: {why}"));
    assert_eq!(continued.entry.id, entry.id, "the paused entry is closed");
    assert_eq!(continued.entry.status, EntryStatus::Committed);
    assert_eq!(
        id(&repo, "HEAD^2"),
        id(&repo, "feature"),
        "the merge is made"
    );
    assert_eq!(repo.read("notes.txt"), HIS_EDIT, "his edit is back");
    assert_eq!(porcelain(&repo), split, "with its split");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

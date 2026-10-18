//! A relaunch after a kill keeps put-back on the entry.
//! Moved out of `set_aside.rs` to keep that file under its size row.

use super::*;

/// A relaunch after a kill: `recover_interrupted` closes the entry as it
/// always has, and the closed entry still carries `put_back`, so the app can
/// name the stash (ADR-0045 §5, 74).
#[test]
fn relaunch_after_a_kill_keeps_put_back_on_the_entry() {
    let (repo, op) = stopping_rebase();
    let Err(ApplyError::Conflicted { entry, .. }) = try_run(&repo, &op, NOW) else {
        panic!("the rebase should stop");
    };
    let stash = set_aside_of(&entry);
    // The process died and the rebase was abandoned in the shell: the
    // Pending line is all that is left.
    repo.git(&["rebase", "--abort"]);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert_eq!(recovered.closed, vec![entry.id.clone()]);
    let closed = last_entry(&repo);
    assert_eq!(closed.id, entry.id);
    assert!(matches!(closed.status, EntryStatus::Aborted { .. }));
    assert_eq!(
        set_aside_of(&closed),
        stash,
        "put_back survives the relaunch"
    );
    assert_eq!(stash_commits(&repo), vec![stash], "S is still listed");
}

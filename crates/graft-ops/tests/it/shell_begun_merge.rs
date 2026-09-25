//! A merge **begun in the shell** — a `git pull` that conflicted, the user's
//! likely daily case — against the M3 promise, "you cannot lose work"
//! (m6 g04/g08 review, M1).
//!
//! Graft wrote no entry for such a merge, so a step inside it has no span to
//! fold into and no backup ref to chain a backup onto. A step that can write
//! the working tree — a whole-file take, a restart — is therefore **its own
//! entry, with its own backup**: the conflicted stash pop's rule
//! (m6-conflicts/03 scope 2, "with no live entry it is its own entry"), and
//! `u` puts back a hand edit the step overwrote. A step that writes only the
//! index still folds into nothing: it has nothing on disk to lose.
//!
//! Every fixture here is a merge run with plain `git`, never through Graft.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, OperationKind};
use graft_git::status::UnmergedHow;
use graft_ops::operation::{
    Operation, ResolveWholeParams, RestartKind, RestartParams, Side, Target,
};

use crate::conflict_ops::{journal_ids, stopped_merge, unmerged_lines};
use crate::support::{NOW, TempRepo, run, run_undo};

/// A hand edit to `theirs-deleted.txt`, made in `$EDITOR` before the take.
const EDITED_UD: &[u8] = b"td OURS alpha\nmy hand edit, made in $EDITOR\n";

/// A hand edit to `both.txt` that has already dropped git's markers.
const EDITED_UU: &[u8] = b"both ours one\nboth theirs two\nmy own third line\n";

/// The fixture is what this file claims it is: an operation git has in
/// progress, with no entry of Graft's behind it.
fn a_shell_begun_merge() -> TempRepo {
    let repo = stopped_merge();
    assert!(
        repo.exists(".git/MERGE_HEAD"),
        "git has a merge in progress"
    );
    assert!(
        journal_ids(&repo).is_empty(),
        "and Graft journaled nothing for it"
    );
    repo
}

/// Hand-edit a `deleted by them` file, then take theirs: `git rm --quiet`,
/// no confirm, and the edit is off the disk. The take is its own entry with
/// its own backup, and `u` brings back both the edit and the conflict.
#[test]
fn a_hand_edit_survives_taking_the_other_side() {
    let repo = a_shell_begun_merge();
    repo.write("theirs-deleted.txt", EDITED_UD);
    let stages = unmerged_lines(&repo, "theirs-deleted.txt");
    assert_eq!(stages.lines().count(), 2, "deleted by them: base and ours");

    let taken = run(
        &repo,
        &Operation::ResolveWholeFile(ResolveWholeParams {
            path: PathBuf::from("theirs-deleted.txt"),
            side: Side::Theirs,
            how: UnmergedHow::DeletedByThem,
        }),
        NOW,
    );
    assert!(
        !repo.exists("theirs-deleted.txt"),
        "theirs deleted it, and the edit went with it"
    );
    assert_eq!(taken.kind, OperationKind::ResolveConflict);
    assert_eq!(taken.status, EntryStatus::Committed);
    assert_eq!(
        journal_ids(&repo),
        vec![taken.id.to_string()],
        "the take is an entry of its own"
    );
    assert!(taken.backup_ref.is_some(), "with a backup of its own");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        repo.read("theirs-deleted.txt"),
        EDITED_UD,
        "u gives the hand edit back, byte for byte"
    );
    assert_eq!(
        unmerged_lines(&repo, "theirs-deleted.txt"),
        stages,
        "and the conflict: the same stage oids and modes"
    );
    assert!(repo.exists(".git/MERGE_HEAD"), "the merge is still git's");
}

/// Hand-edit a `both modified` file, then `d`: `checkout --merge` writes the
/// markers over the edit. The restart is its own entry, and `u` brings the
/// edit back.
#[test]
fn a_hand_edit_survives_starting_the_file_over() {
    let repo = a_shell_begun_merge();
    repo.write("both.txt", EDITED_UU);
    let stages = unmerged_lines(&repo, "both.txt");
    assert_eq!(stages.lines().count(), 3, "both modified: three stages");

    let restarted = run(
        &repo,
        &Operation::RestartConflict(RestartParams {
            path: PathBuf::from("both.txt"),
            kind: RestartKind::Markers,
        }),
        NOW,
    );
    let over = String::from_utf8_lossy(&repo.read("both.txt")).into_owned();
    assert!(
        over.contains("<<<<<<<") && !over.contains("my own third line"),
        "the markers are back over the edit: {over}"
    );
    assert_eq!(restarted.kind, OperationKind::RestartConflict);
    assert_eq!(
        journal_ids(&repo),
        vec![restarted.id.to_string()],
        "the restart is an entry of its own"
    );
    assert!(restarted.backup_ref.is_some(), "with a backup of its own");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        repo.read("both.txt"),
        EDITED_UU,
        "u gives the hand edit back, byte for byte"
    );
    assert_eq!(
        unmerged_lines(&repo, "both.txt"),
        stages,
        "and the stages are the ones git left"
    );
}

/// A step that writes only the index still folds into nothing: `git add` on
/// a resolved file loses no byte on disk, and git's own state is the whole
/// record of an operation Graft did not begin.
///
/// This replaces `conflict_ops::whole_file_folds_into_live_entry`, whose
/// fixture was this same shell-begun merge and which pinned the data-loss
/// path as "no line": a whole-file take here is now its own entry (above).
/// The fold into an entry Graft *did* write is
/// `chained_backup::folded_step_chains_a_backup`.
#[test]
fn a_stage_writes_no_line() {
    let repo = a_shell_begun_merge();
    repo.write("both.txt", EDITED_UU);

    let staged = run(
        &repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from("both.txt")])),
        NOW,
    );
    assert_eq!(staged.kind, OperationKind::Stage);
    assert!(
        journal_ids(&repo).is_empty(),
        "no line for an index-only step"
    );
    assert_eq!(unmerged_lines(&repo, "both.txt"), String::new());
    assert_eq!(repo.read("both.txt"), EDITED_UU, "nothing on disk moved");
}

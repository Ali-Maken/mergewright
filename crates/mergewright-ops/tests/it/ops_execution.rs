//! The execution half, judged by reading the repository back.
//!
//! Every assertion here goes through `git` or the filesystem, never through
//! the value a function returned (PROMPT.md §7.2 item 4). The promise under
//! test is "you cannot lose work": after `apply` then `undo`, the repository
//! is byte-for-byte what it was — refs, the index, and the contents of every
//! file including untracked ones.

use std::path::PathBuf;

use mergewright_core::journal::{EntryStatus, OperationKind};
use mergewright_ops::apply::{ApplyError, Refresh, apply};
use mergewright_ops::operation::{
    CommitParams, FactsScope, Operation, StashParams, StashScope, Target,
};
use mergewright_ops::snapshot;
use mergewright_ops::undo::UndoError;

use crate::support::{
    NOW, Never, TempRepo, context, journal_for, round_trip, run, run_redo, run_undo,
};
use mergewright_core::on_disk::CURRENT_DIR;

fn paths(of: &[&str]) -> Target {
    Target::Paths(of.iter().map(PathBuf::from).collect())
}

/// A repository with one commit, a tracked file edited but not staged, a
/// staged change and an untracked file with bytes no editor would keep.
fn mixed() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.rs", b"fn a() {}\n");
    repo.write("b.rs", b"fn b() {}\n");
    repo.commit_all("one");
    repo.write("a.rs", b"fn a() { changed }\n");
    repo.write("untracked.bin", &[0, 255, 10, 13, 0, 7]);
    repo
}

// ── the journal file ────────────────────────────────────────────────────

#[test]
fn the_journal_lands_under_the_git_dir_and_holds_two_lines_per_entry() {
    let repo = mixed();
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);

    let path = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    let text =
        String::from_utf8(std::fs::read(&path).expect("the journal is there")).expect("it is text");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "one pending line and one outcome line");
    assert!(lines[0].contains("\"status\":\"pending\""), "{}", lines[0]);
    assert!(
        lines[1].contains("\"status\":\"committed\""),
        "{}",
        lines[1]
    );
    assert!(text.ends_with('\n'), "every line is terminated");
}

#[test]
fn a_torn_last_line_is_cut_off_so_the_next_append_is_not_spliced_onto_it() {
    let repo = mixed();
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let path = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    let mut bytes = std::fs::read(&path).expect("the journal reads");
    bytes.extend_from_slice(br#"{"id":"01","stat"#);
    std::fs::write(&path, &bytes).expect("the torn tail is written");

    let handle = repo.open();
    let journal = journal_for(&handle);
    assert_eq!(journal.torn_tail(), Some(3), "the torn line is reported");
    assert_eq!(journal.entries().len(), 1, "the finished entry survives");

    // And the file is usable again: a second open finds nothing torn.
    let again = journal_for(&handle);
    assert_eq!(again.torn_tail(), None);
    assert_eq!(again.entries().len(), 1);
}

#[test]
fn a_pending_entry_with_no_outcome_is_found_and_closed_as_interrupted() {
    let repo = mixed();
    // A crash between the two lines: keep only the pending line.
    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    let path = repo
        .path()
        .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
    let text = std::fs::read_to_string(&path).expect("the journal reads");
    let pending = text.lines().next().expect("there is a pending line");
    std::fs::write(&path, format!("{pending}\n")).expect("the truncated journal is written");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    assert_eq!(journal.unresolved().len(), 1, "the crash is visible");

    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = mergewright_ops::apply::recover_interrupted(&mut ctx).expect("recovery runs");
    assert_eq!(recovered.closed.len(), 1);
    assert!(
        recovered.unreadable.is_empty(),
        "the repository read fine: {:?}",
        recovered.unreadable
    );
    assert!(journal.unresolved().is_empty(), "nothing is left pending");
    let entry = &journal.entries()[0];
    assert!(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason == "interrupted"),
        "{:?}",
        entry.status
    );
    assert!(
        entry.worktree_after.is_some(),
        "recovery records the after-state it could read"
    );
}

// ── the snapshot ────────────────────────────────────────────────────────

#[test]
fn the_backup_commit_holds_exactly_an_index_tree_and_a_worktree_tree() {
    let repo = mixed();
    let entry = run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let name = entry.backup_ref.expect("a staging operation is backed up");
    assert_eq!(
        name,
        format!(
            "{}backup/main/{}",
            mergewright_core::on_disk::REF_PREFIX,
            entry.id
        )
    );

    let listing = repo.git(&["ls-tree", "--name-only", &name]);
    assert_eq!(listing, "index\nworktree");
    let worktree_tree = repo.git(&["rev-parse", &format!("{name}^{{tree}}:worktree")]);
    assert_eq!(
        worktree_tree,
        entry
            .worktree_before
            .expect("the before tree is recorded")
            .to_string(),
        "the ref's worktree tree is the one the entry names"
    );
}

#[test]
fn the_backup_tree_carries_untracked_files_and_unstaged_edits() {
    let repo = mixed();
    let entry = run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let tree = entry.worktree_before.expect("the tree is recorded");

    let untracked = repo.git(&["cat-file", "-p", &format!("{tree}:untracked.bin")]);
    assert!(!untracked.is_empty(), "the untracked file is in the tree");
    let edited = repo.git(&["cat-file", "-p", &format!("{tree}:a.rs")]);
    assert_eq!(
        edited, "fn a() { changed }",
        "the unstaged edit is in the tree, not HEAD's version"
    );
}

#[test]
fn a_worktree_touching_operation_ends_with_an_after_commit_over_the_before_one() {
    let repo = mixed();
    let entry = run(&repo, &Operation::Discard(paths(&["untracked.bin"])), NOW);
    let name = entry.backup_ref.expect("a discard is backed up");
    let after = repo.git(&["rev-parse", &name]);
    let parent = repo.git(&["rev-parse", &format!("{name}^")]);
    assert_ne!(after, parent);
    assert_eq!(
        repo.git(&["rev-parse", &format!("{name}^{{tree}}:worktree")]),
        entry.worktree_after.expect("after is recorded").to_string()
    );
    assert_eq!(
        repo.git(&["rev-parse", &format!("{name}^^{{tree}}:worktree")]),
        entry
            .worktree_before
            .expect("before is recorded")
            .to_string()
    );
}

#[test]
fn reading_the_repository_never_rewrites_the_real_index() {
    let repo = mixed();
    let before = std::fs::read(repo.path().join(".git/index")).expect("the index reads");

    let handle = repo.open();
    let never = Never;
    let scratch = repo.path().join(format!(".git/{CURRENT_DIR}"));
    std::fs::create_dir_all(&scratch).expect("the scratch is made");
    snapshot::index_state(&handle, &scratch, &never).expect("the index state is read");
    snapshot::worktree_tree(&handle, &scratch, &never).expect("the worktree tree is written");
    let version = crate::support::git_version();
    mergewright_ops::facts::read_facts(&handle, version, &never, &FactsScope::default())
        .expect("the facts read");

    let after = std::fs::read(repo.path().join(".git/index")).expect("the index reads");
    assert_eq!(before, after, "a read left the index byte-identical");
}

#[test]
fn staging_is_the_only_thing_that_changes_the_index() {
    let repo = mixed();
    let before = std::fs::read(repo.path().join(".git/index")).expect("the index reads");
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    let after = std::fs::read(repo.path().join(".git/index")).expect("the index reads");
    assert_ne!(before, after, "staging did change the index");
    assert_eq!(repo.git(&["diff", "--cached", "--name-only"]), "a.rs");
}

// ── round trips, one per kind ───────────────────────────────────────────

#[test]
fn stage_round_trips() {
    let repo = mixed();
    round_trip(&repo, &Operation::Stage(paths(&["a.rs"])));
}

#[test]
fn unstage_round_trips() {
    let repo = mixed();
    repo.git(&["add", "a.rs"]);
    round_trip(&repo, &Operation::Unstage(paths(&["a.rs"])));
}

#[test]
fn discard_of_an_unstaged_edit_round_trips() {
    let repo = mixed();
    round_trip(&repo, &Operation::Discard(paths(&["a.rs"])));
}

#[test]
fn discard_of_an_untracked_file_round_trips() {
    let repo = mixed();
    round_trip(&repo, &Operation::Discard(paths(&["untracked.bin"])));
}

#[test]
fn commit_round_trips() {
    let repo = mixed();
    repo.git(&["add", "a.rs"]);
    round_trip(
        &repo,
        &Operation::Commit(CommitParams {
            message: "a change\n".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: mergewright_ops::operation::Include::Staged,
        }),
    );
}

#[test]
fn amend_round_trips() {
    let repo = mixed();
    repo.git(&["add", "a.rs"]);
    round_trip(
        &repo,
        &Operation::Amend(CommitParams {
            message: "one, amended\n".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: mergewright_ops::operation::Include::Staged,
        }),
    );
}

#[test]
fn stash_push_round_trips() {
    let repo = mixed();
    round_trip(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: true,
            },
        }),
    );
}

// ── PROMPT §3's own test, verbatim ──────────────────────────────────────

#[test]
fn an_untracked_binary_file_comes_back_byte_for_byte_after_discard_and_undo() {
    let repo = TempRepo::init();
    repo.write("kept.txt", b"kept\n");
    repo.commit_all("one");
    let bytes: Vec<u8> = vec![0, 255, 10, 13, 0, 0, 255, 200, 1, 10];
    repo.write("new.bin", &bytes);

    run(&repo, &Operation::Discard(paths(&["new.bin"])), NOW);
    assert!(!repo.exists("new.bin"), "the discard removed it");

    run_undo(&repo, NOW + 1, false).expect("undo runs");
    assert_eq!(repo.read("new.bin"), bytes, "the bytes came back exactly");
}

#[test]
fn an_unstaged_edit_to_a_tracked_file_comes_back_byte_for_byte() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"original\n");
    repo.commit_all("one");
    let edited: Vec<u8> = vec![0, 255, 10, 13, b'x'];
    repo.write("a.rs", &edited);

    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    assert_eq!(repo.read("a.rs"), b"original\n", "the discard reverted it");

    run_undo(&repo, NOW + 1, false).expect("undo runs");
    assert_eq!(repo.read("a.rs"), edited, "the edit came back exactly");
}

#[test]
fn a_staged_and_unstaged_file_comes_back_on_both_sides() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"original\n");
    repo.commit_all("one");
    repo.write("a.rs", b"staged\n");
    repo.git(&["add", "a.rs"]);
    repo.write("a.rs", b"staged, then edited\n");

    let staged_before = repo.git(&["ls-files", "-s", "a.rs"]);
    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    run_undo(&repo, NOW + 1, false).expect("undo runs");

    assert_eq!(
        repo.read("a.rs"),
        b"staged, then edited\n",
        "worktree bytes"
    );
    assert_eq!(
        repo.git(&["ls-files", "-s", "a.rs"]),
        staged_before,
        "index bytes"
    );
}

#[test]
fn undo_touches_only_the_paths_the_operation_changed() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"original\n");
    repo.commit_all("one");
    repo.write("a.rs", b"edited\n");

    run(&repo, &Operation::Discard(paths(&["a.rs"])), NOW);
    repo.write("new.txt", b"written after the discard\n");

    // The new file is drift: §6.7 stops and explains before overwriting
    // anything it did not expect.
    let stopped = run_undo(&repo, NOW + 1, false);
    match stopped {
        Err(UndoError::Drift { drift: moved, .. }) => assert!(moved.worktree, "{moved:?}"),
        other => panic!("the rail should have fired: {other:?}"),
    }

    run_undo(&repo, NOW + 2, true).expect("the forced undo runs");
    assert_eq!(repo.read("a.rs"), b"edited\n");
    assert_eq!(
        repo.read("new.txt"),
        b"written after the discard\n",
        "a file the operation never touched is left alone"
    );
}

// ── refusal and failure ─────────────────────────────────────────────────

#[test]
fn a_refused_operation_writes_nothing_at_all() {
    let repo = TempRepo::init();
    repo.write("a.rs", b"a\n");
    repo.commit_all("one");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    let refused = apply(
        &mut ctx,
        &Operation::Commit(CommitParams {
            message: "nothing is staged\n".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: mergewright_ops::operation::Include::Staged,
        }),
    );
    assert!(matches!(refused, Err(ApplyError::Refused(_))));
    assert!(journal.entries().is_empty(), "nothing was journaled");
    assert_eq!(
        repo.git(&[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/{CURRENT_DIR}")
        ]),
        "",
        "no backup ref was made"
    );
}

#[test]
fn refresh_says_everything_only_when_a_ref_moved() {
    let repo = mixed();
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    {
        let mut ctx = context(&handle, &mut journal, NOW, &never);
        let applied = apply(&mut ctx, &Operation::Stage(paths(&["a.rs"]))).expect("stage runs");
        assert_eq!(applied.refresh, Refresh::WorkingTree);
    }
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let applied = apply(
        &mut ctx,
        &Operation::Commit(CommitParams {
            message: "a change\n".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
            include: mergewright_ops::operation::Include::Staged,
        }),
    )
    .expect("commit runs");
    assert_eq!(applied.refresh, Refresh::Everything);
}

// ── the undo/redo cursor over real entries ──────────────────────────────

#[test]
fn a_new_operation_after_an_undo_empties_redo() {
    let repo = mixed();
    run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    run_undo(&repo, NOW + 1, false).expect("undo runs");
    repo.write("b.rs", b"fn b() { changed }\n");
    run(&repo, &Operation::Stage(paths(&["b.rs"])), NOW + 2);
    let nothing = run_redo(&repo, NOW + 3, false);
    assert!(
        matches!(nothing, Err(UndoError::Nothing(_))),
        "{nothing:?} should be nothing to redo"
    );
}

#[test]
fn undo_with_an_empty_journal_says_nothing_to_undo() {
    let repo = mixed();
    let nothing = run_undo(&repo, NOW, false);
    assert!(matches!(nothing, Err(UndoError::Nothing(_))), "{nothing:?}");
}

#[test]
fn the_undo_entry_names_what_it_reverses_and_is_itself_journaled() {
    let repo = mixed();
    let entry = run(&repo, &Operation::Stage(paths(&["a.rs"])), NOW);
    run_undo(&repo, NOW + 1, false).expect("undo runs");

    let handle = repo.open();
    let journal = journal_for(&handle);
    let entries = journal.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[1].kind, OperationKind::Undo);
    assert_eq!(entries[1].reverses.as_ref(), Some(&entry.id));
    assert!(
        !entries[1].commands.is_empty(),
        "the undo's own commands are logged"
    );
}

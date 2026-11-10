use std::sync::Arc;

use mergewright_core::journal::{EntryId, HeadRef, IndexState, OperationKind, Touches};
use mergewright_term::sanitize::SanitizedText;

use super::*;
use crate::jobs::undo::JournalRead;

/// A distinct, valid oid: the journal only ever compares them.
fn oid(byte: u8) -> mergewright_core::oid::Oid {
    mergewright_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn entry(id: i64, kind: OperationKind, description: &str) -> JournalEntry {
    JournalEntry {
        id: EntryId::new(id, [0; 10]),
        at_unix_ms: id,
        kind,
        description: description.to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: Vec::new(),
            index: true,
            worktree: false,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: Some(IndexState::Merged(oid(1))),
        index_after: Some(IndexState::Merged(oid(2))),
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: vec!["git add -- src/a.rs".to_owned()],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    }
}

fn state_with(entries: Vec<JournalEntry>) -> AppState {
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries,
        unreadable: Vec::new(),
        held: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    crate::update::journal::rebuild(&mut state);
    state
}

#[test]
fn rows_are_newest_first_and_the_next_undo_is_marked() {
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        entry(2, OperationKind::Commit, "commit \"x\""),
    ]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows.len(), 2);
    assert_eq!(view.rows[0].description.as_str(), "commit \"x\"");
    assert_eq!(view.rows[0].mark, RowMark::NextUndo);
    assert_eq!(view.rows[1].mark, RowMark::None);
}

#[test]
fn an_undo_entry_is_the_redo_target_and_what_it_reversed_reads_undone() {
    let mut undo = entry(3, OperationKind::Undo, "undo: commit \"x\"");
    undo.reverses = Some(EntryId::new(2, [0; 10]));
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        entry(2, OperationKind::Commit, "commit \"x\""),
        undo,
    ]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::NextRedo, "the undo itself");
    assert_eq!(view.rows[1].mark, RowMark::Undone, "what it took back");
    assert_eq!(view.rows[2].mark, RowMark::NextUndo, "what `u` reaches now");
}

#[test]
fn an_interrupted_entry_says_so_in_the_row_and_in_full_when_expanded() {
    let mut interrupted = entry(2, OperationKind::Commit, "commit \"x\"");
    interrupted.status = EntryStatus::Pending;
    let state = state_with(vec![interrupted]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::Interrupted);
    assert!(view.rows[0].summary.as_str().starts_with("interrupted"));
    assert!(
        view.rows[0]
            .failure
            .as_ref()
            .is_some_and(|line| line.as_str().starts_with("interrupted:")),
    );
}

/// `J` after a stash clash (m6-conflicts/72, 72b): the pop that clashed
/// is `Committed` — git kept the stash and the tree is unmerged, which
/// is the pop's own result, not a failure — so its row reads as any
/// other, never `failed:`. The drop the last file done launched is a
/// `StashDrop` row named by its position, as every stash is, and it is what `u`
/// reaches next.
#[test]
fn a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop() {
    let mut pop = entry(1, OperationKind::StashPop, "pop stash@{0}");
    pop.touches.worktree = true;
    pop.put_back = Some(mergewright_core::journal::PutBack {
        stash: oid(7),
        why: mergewright_core::journal::PutBackWhy::Pop,
    });
    let stage = entry(2, OperationKind::Stage, "stage notes.md");
    let mut drop = entry(3, OperationKind::StashDrop, "drop stash@{0}");
    drop.touches = Touches {
        refs: vec!["refs/stash".to_owned()],
        index: false,
        worktree: false,
    };
    let app = state_with(vec![pop, stage, drop]);
    let view = journal_view(&app, 10);

    let drop_row = &view.rows[0];
    assert_eq!(drop_row.description.as_str(), "drop stash@{0}");
    assert_eq!(
        drop_row.mark,
        RowMark::NextUndo,
        "`u` brings the stash back"
    );
    assert!(drop_row.failure.is_none());

    let pop_row = &view.rows[2];
    assert_eq!(pop_row.description.as_str(), "pop stash@{0}");
    assert_eq!(pop_row.mark, RowMark::None);
    assert_eq!(
        pop_row.summary.as_str(),
        "the index · the working tree · backup saved"
    );
    assert!(pop_row.failure.is_none(), "a clash is not a failure");
}

/// A remote entry: `refs` touched, no index, no tree, no backup.
fn remote_entry(n: i64, kind: OperationKind, description: &str, refs: Vec<String>) -> JournalEntry {
    let mut entry = entry(n, kind, description);
    entry.touches = Touches {
        refs,
        index: false,
        worktree: false,
    };
    entry.backup_ref = None;
    entry
}

/// Push, Fetch and Pull rows in the design note's words (New UI,
/// "Journal browser rows"). `← u` falls on the pull — the newest
/// undoable entry — never on the push or fetch newer than it; that mark
/// is `mergewright_ops::journal::next_undo`'s, asserted here, not restated.
#[test]
fn remote_rows() {
    let tracking = "refs/remotes/origin/main".to_owned();
    let mut pull = entry(1, OperationKind::Pull, "pull origin/main");
    pull.touches = Touches {
        refs: vec!["refs/heads/main".to_owned()],
        index: true,
        worktree: true,
    };
    let mut push = remote_entry(
        2,
        OperationKind::Push,
        "push main → origin/main",
        vec![tracking.clone()],
    );
    let a1 = mergewright_core::oid::Oid::from_hex("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678")
        .expect("40 hex digits");
    let nine = mergewright_core::oid::Oid::from_hex("9f8e7d6c5b4a39281706f5e4d3c2b1a098765432")
        .expect("40 hex digits");
    push.refs_before.insert(tracking.clone(), Some(a1));
    push.refs_after.insert(tracking, Some(nine));
    let unrecorded = remote_entry(
        3,
        OperationKind::Push,
        "push topic → origin/topic (set upstream)",
        Vec::new(),
    );
    let fetch = remote_entry(4, OperationKind::Fetch, "fetch all remotes", Vec::new());
    let state = state_with(vec![pull, push, unrecorded, fetch]);
    let view = journal_view(&state, 10);
    let rows: Vec<(&str, &str, RowMark)> = view
        .rows
        .iter()
        .map(|row| (row.description.as_str(), row.summary.as_str(), row.mark))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "fetch all remotes",
                "remote-tracking refs · not undoable",
                RowMark::None
            ),
            (
                "push topic → origin/topic (set upstream)",
                "not undoable — origin has it",
                RowMark::None
            ),
            (
                "push main → origin/main",
                "not undoable — origin has it · a1b2c3d → 9f8e7d6",
                RowMark::None
            ),
            (
                "pull origin/main",
                "1 ref · the index · the working tree · backup saved",
                RowMark::NextUndo
            ),
        ]
    );
}

/// m8-polish-and-ship/58r: the read ran while an operation was writing its
/// entry, so the `Pending` entry is held, not judged. It is drawn as
/// running: no `interrupted` mark, no "a previous run stopped" line, and
/// `⏎` promises nothing on it.
#[test]
fn an_entry_a_busy_read_held_is_drawn_running_not_interrupted() {
    let mut running = entry(2, OperationKind::Commit, "commit \"x\"");
    running.status = EntryStatus::Pending;
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries: vec![running],
        unreadable: Vec::new(),
        held: vec![EntryId::new(2, [0; 10])],
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    crate::update::journal::rebuild(&mut state);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::None);
    assert_eq!(
        view.rows[0].summary.as_str(),
        "running · the index · backup saved"
    );
    assert_eq!(view.rows[0].failure, None);
    assert_eq!(view.enter_words, None);
}

/// What recovery actually leaves behind, and therefore the only shape
/// the browser ever really sees: `Aborted { reason: "interrupted" }`.
/// Keyed on `Pending` alone this row was unreachable and read
/// `failed: interrupted` (m3/06 review, Major 2).
#[test]
fn an_entry_recovery_closed_is_marked_interrupted_not_failed() {
    let mut recovered = entry(2, OperationKind::Commit, "commit \"x\"");
    // No before-tree: the process died before one was taken, so `u`
    // cannot reach it and the row is not the `← u` target. That is the
    // shape the warning mark exists for.
    recovered.worktree_before = None;
    recovered.status = EntryStatus::Aborted {
        reason: mergewright_core::journal::INTERRUPTED.to_owned(),
    };
    let state = state_with(vec![recovered]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::Interrupted);
    let summary = view.rows[0].summary.as_str();
    assert!(summary.starts_with("interrupted"), "{summary}");
    assert!(
        !summary.contains("failed"),
        "a process that died: {summary}"
    );
    assert!(
        view.rows[0]
            .failure
            .as_ref()
            .is_some_and(|line| line.as_str().starts_with("interrupted:")),
    );
}

#[test]
fn an_aborted_entry_leads_with_gits_first_line_and_keeps_the_rest_for_space() {
    let mut aborted = entry(2, OperationKind::Commit, "commit \"x\"");
    // `in_force` counts an Aborted entry only with a backup of the tree
    // it changed; without one there is nothing for `u` to restore.
    aborted.worktree_before = Some(oid(3));
    aborted.status = EntryStatus::Aborted {
        reason: "hook rejected the commit\nsee .git/hooks/pre-commit".to_owned(),
    };
    let state = state_with(vec![aborted]);
    let view = journal_view(&state, 10);
    let summary = view.rows[0].summary.as_str();
    assert!(
        summary.starts_with("failed: hook rejected the commit"),
        "{summary}"
    );
    assert!(!summary.contains("pre-commit"), "{summary}");
    let failure = view.rows[0]
        .failure
        .as_ref()
        .expect("an aborted entry has a reason");
    assert!(failure.as_str().contains("pre-commit"), "{failure:?}");
    // It kept its backup and changed the index, so it is still what `u`
    // reaches: a write that failed part-way is undoable, and the row
    // must say so rather than look inert.
    assert!(view.rows[0].summary.as_str().ends_with("backup saved"));
    assert_eq!(view.rows[0].mark, RowMark::NextUndo);
}

#[test]
fn an_empty_journal_is_empty_not_ready_and_a_failed_read_carries_its_reason() {
    let state = state_with(Vec::new());
    assert_eq!(journal_view(&state, 10).status, JournalStatus::Empty);

    let mut state = AppState::default();
    state.journal.entries = Loading::Failed("line 3: torn".to_owned());
    match journal_view(&state, 10).status {
        JournalStatus::Failed(why) => assert_eq!(why.as_str(), "line 3: torn"),
        other => panic!("a failed read is not {other:?}"),
    }

    let mut state = AppState::default();
    state.journal.entries = Loading::Idle;
    assert_eq!(journal_view(&state, 10).status, JournalStatus::Loading);
}
/// M6/68: an entry the user aborted (`--abort`, `Aborted("aborted")`)
/// is not a failure: the row says who stopped it and what it left.
#[test]
fn an_aborted_entry_reads_aborted_by_you() {
    let mut aborted = entry(2, OperationKind::Merge, "merge feature");
    aborted.status = EntryStatus::Aborted {
        reason: "aborted".to_owned(),
    };
    let state = state_with(vec![aborted]);
    let view = journal_view(&state, 10);
    let summary = view.rows[0].summary.as_str();
    assert_eq!(summary, "aborted by you · nothing changed");
    assert!(!summary.contains("failed"), "{summary}");
}

/// m8/59 Major A: J's bar is told `Esc` stops the running write exactly
/// when `cancel_operating` would take the key — a write runs and no text
/// field, confirm box or menu has it first.
#[test]
fn j_is_told_esc_cancels_exactly_when_a_write_would_take_it() {
    let mut state = state_with(vec![entry(1, OperationKind::Push, "push main to origin")]);
    assert!(!journal_view(&state, 10).esc_cancels, "no write runs");

    state.operating = Some(mergewright_core::job::JobId::new(1));
    assert!(journal_view(&state, 10).esc_cancels, "a write runs");
    let mut commands = Vec::new();
    let mut probe = state.clone();
    assert!(
        crate::update::remote::cancel_operating(&mut probe, &mut commands),
        "and Esc does go to it"
    );

    // J's own `/` field keeps its `Esc`: the bar is not told otherwise.
    state.journal.shown = true;
    state.journal.find.typing = Some(String::new());
    assert!(
        !journal_view(&state, 10).esc_cancels,
        "a text field has Esc"
    );
}

/// m8/59 m3: a write he stopped with `Esc` (`Aborted("cancelled")`) is his
/// choice, not a failure — on the row and in its expanded reason, for a
/// push and for an undo alike. A cancel whose roll-back failed still is one.
#[test]
fn a_cancelled_entry_reads_cancelled_by_you() {
    let cancelled = |kind, words: &str| {
        let mut entry = entry(2, kind, words);
        entry.status = EntryStatus::Aborted {
            reason: "cancelled".to_owned(),
        };
        entry
    };
    for kind in [OperationKind::Push, OperationKind::Undo] {
        let state = state_with(vec![cancelled(kind, "push main to origin")]);
        let view = journal_view(&state, 10);
        let row = &view.rows[0];
        assert!(
            row.summary.as_str().starts_with("cancelled by you"),
            "{kind:?}: {:?}",
            row.summary
        );
        let failure = row.failure.as_ref().map(|why| why.as_str().to_owned());
        assert_eq!(failure.as_deref(), Some("cancelled by you"), "{kind:?}");
    }

    let mut torn = entry(2, OperationKind::Undo, "undo: reset");
    torn.status = EntryStatus::Aborted {
        reason: "cancelled — rollback failed: index".to_owned(),
    };
    let view = journal_view(&state_with(vec![torn]), 10);
    assert!(
        view.rows[0].summary.as_str().starts_with("failed: "),
        "{:?}",
        view.rows[0].summary
    );
}

/// The 21b-3 probe's reason: the second remote is not there.
const GONE: &str = "no repository at '/nonexistent/gone.git', or no access to it";

/// `refs/remotes/origin/<name>` moved from `before` to `after` (`None` =
/// absent: new / gone).
fn moves(
    list: &[(&str, Option<u8>, Option<u8>)],
) -> std::collections::BTreeMap<String, mergewright_core::journal::RefMove> {
    list.iter()
        .map(|(name, before, after)| {
            (
                (*name).to_owned(),
                mergewright_core::journal::RefMove {
                    before: before.map(oid),
                    after: after.map(oid),
                },
            )
        })
        .collect()
}

/// `fetch all remotes` as `apply` closes it: no refs touched, no backup,
/// `fetched_refs` as given, `Aborted` with `reason` when there is one.
fn fetch_entry(
    n: i64,
    reason: Option<&str>,
    fetched: Option<std::collections::BTreeMap<String, mergewright_core::journal::RefMove>>,
) -> JournalEntry {
    let mut fetch = remote_entry(n, OperationKind::Fetch, "fetch all remotes", Vec::new());
    fetch.index_after = fetch.index_before;
    fetch.commands = vec!["git fetch --progress --all".to_owned()];
    if let Some(reason) = reason {
        fetch.status = EntryStatus::Aborted {
            reason: reason.to_owned(),
        };
    }
    fetch.fetched_refs = fetched;
    fetch
}

/// The 21b-3 probe (ADR-0062): `origin/main` moved, then the second remote
/// failed. The row says what failed and what moved, and is not `refused ·
/// nothing written` — a ref moved — nor `← u`: `u` steps over a fetch.
#[test]
fn a_partial_fetch_row_names_what_moved_and_is_not_refused() {
    let fetch = fetch_entry(
        2,
        Some(GONE),
        Some(moves(&[(
            "refs/remotes/origin/main",
            Some(0xa1),
            Some(0x9f),
        )])),
    );
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        fetch,
    ]);
    let view = journal_view(&state, 10);
    let row = &view.rows[0];
    assert_eq!(
        row.mark,
        RowMark::None,
        "{:?}",
        row.mark.label(&state.keymap)
    );
    assert_eq!(
        row.summary.as_str(),
        format!("failed: {GONE} · origin/main moved before it stopped · not undoable"),
    );
    assert_eq!(
        view.rows[1].mark,
        RowMark::NextUndo,
        "`u` reaches the stage"
    );
}

/// A fetch whose remotes all failed before any ref moved (`Some({})`) wrote
/// nothing: its row, mark and words are exactly what they were before
/// ADR-0062 — `failed: <git's first line>`, `nothing written` (m8/20b R8).
#[test]
fn a_fetch_that_moved_nothing_reads_as_today() {
    let state = state_with(vec![fetch_entry(
        2,
        Some(GONE),
        Some(std::collections::BTreeMap::new()),
    )]);
    let view = journal_view(&state, 10);
    let row = &view.rows[0];
    assert_eq!(row.mark, RowMark::NothingWritten);
    assert_eq!(row.summary.as_str(), format!("failed: {GONE}"));
    assert_eq!(
        row.failure.as_ref().map(SanitizedText::as_str),
        Some(format!("failed: {GONE}").as_str())
    );
    assert!(row.moved.is_empty(), "{:?}", row.moved);
}

/// A finished fetch says what it moved, or that it brought nothing new.
#[test]
fn a_committed_fetch_reads_nothing_new_not_undoable() {
    let nothing = fetch_entry(1, None, Some(std::collections::BTreeMap::new()));
    let some = fetch_entry(
        2,
        None,
        Some(moves(&[
            ("refs/remotes/origin/feat", None, Some(0x9f)),
            ("refs/remotes/origin/main", Some(0xa1), Some(0x9f)),
            ("refs/tags/v1.2", None, Some(0x9f)),
        ])),
    );
    let state = state_with(vec![nothing, some]);
    let view = journal_view(&state, 10);
    let rows: Vec<(&str, RowMark)> = view
        .rows
        .iter()
        .map(|row| (row.summary.as_str(), row.mark))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "origin/feat new, origin/main moved +1 more · not undoable",
                RowMark::None
            ),
            ("nothing new · not undoable", RowMark::None),
        ]
    );
}

/// A line written before ADR-0062 has no `fetched_refs`: what its fetch
/// moved is unknown, and unknown is not "nothing", so an `Aborted` one is
/// no longer `nothing written`. Its words are today's: `failed:
/// …` alone when it failed (`with_failure` keeps only the failure), and
/// `remote-tracking refs · not undoable` when it finished.
#[test]
fn an_old_aborted_fetch_line_is_not_refused() {
    let state = state_with(vec![
        fetch_entry(1, None, None),
        fetch_entry(2, Some(GONE), None),
    ]);
    let view = journal_view(&state, 10);
    let failed = &view.rows[0];
    assert_eq!(failed.mark, RowMark::None);
    assert_eq!(failed.summary.as_str(), format!("failed: {GONE}"));
    assert!(failed.moved.is_empty(), "{:?}", failed.moved);
    let finished = &view.rows[1];
    assert_eq!(finished.mark, RowMark::None);
    assert_eq!(
        finished.summary.as_str(),
        "remote-tracking refs · not undoable"
    );
}

/// A pull's fetch moved its tracking ref: the row says so after today's
/// words, `before it stopped` when the pull failed. Nothing when its fetch
/// moved nothing.
#[test]
fn a_pull_row_ends_with_its_moves() {
    let pull = |n: i64, reason: Option<&str>, fetched| {
        let mut pull = remote_entry(
            n,
            OperationKind::Pull,
            "pull origin/main",
            vec!["refs/heads/main".to_owned()],
        );
        if let Some(reason) = reason {
            pull.status = EntryStatus::Aborted {
                reason: reason.to_owned(),
            };
        }
        pull.fetched_refs = fetched;
        pull
    };
    let main = || moves(&[("refs/remotes/origin/main", Some(0xa1), Some(0x9f))]);
    let state = state_with(vec![
        pull(1, None, Some(main())),
        pull(2, None, Some(std::collections::BTreeMap::new())),
        pull(
            3,
            Some("fatal: Not possible to fast-forward, aborting."),
            Some(main()),
        ),
    ]);
    let view = journal_view(&state, 10);
    let rows: Vec<&str> = view.rows.iter().map(|row| row.summary.as_str()).collect();
    assert_eq!(
        rows,
        [
            "failed: fatal: Not possible to fast-forward, aborting. · 1 ref · origin/main moved before it stopped",
            "1 ref",
            "1 ref · origin/main moved",
        ]
    );
    assert_eq!(
        view.rows[0].mark,
        RowMark::None,
        "a failed pull whose fetch moved a ref wrote something"
    );
}

/// The expanded row's list (box 10's other half): one line per moved ref,
/// at most twenty, then `+N more` — `mergewright_ops::journal::moved_lines`'
/// words, which `mergewright-ui` draws as it gets them.
#[test]
fn an_expanded_fetch_row_carries_at_most_twenty_moves() {
    let names: Vec<String> = (0..25)
        .map(|n| format!("refs/remotes/origin/b{n:02}"))
        .collect();
    let many = moves(
        &names
            .iter()
            .map(|name| (name.as_str(), Some(0xa1), Some(0x9f)))
            .collect::<Vec<_>>(),
    );
    let state = state_with(vec![fetch_entry(1, Some(GONE), Some(many))]);
    let view = journal_view(&state, 10);
    let moved: Vec<&str> = view.rows[0]
        .moved
        .iter()
        .map(SanitizedText::as_str)
        .collect();
    assert_eq!(moved.len(), 21, "{moved:?}");
    assert_eq!(moved[0], "origin/b00  a1a1a1a → 9f9f9f9");
    assert_eq!(moved[19], "origin/b19  a1a1a1a → 9f9f9f9");
    assert_eq!(moved[20], "+5 more");
}

/// m8-polish-and-ship/20 item 11, m8/20b R8: a refused pull or rejected
/// push wrote nothing, so its row reads `nothing written`, never `undone`
/// and never `refused` — the entry does not record why it stopped.
#[test]
fn a_refused_push_reads_nothing_written_not_undone() {
    let mut push = entry(2, OperationKind::Push, "push main to origin");
    push.status = EntryStatus::Aborted {
        reason: "rejected".to_owned(),
    };
    push.index_after = push.index_before;
    let state = state_with(vec![entry(1, OperationKind::Stage, "stage src/a.rs"), push]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::NothingWritten);
    assert_eq!(view.rows[0].mark.label(&state.keymap), "nothing written");
}

/// m8/20b R8: a git failure that wrote nothing (not a refusal) reads the
/// same `nothing written`: the one fact both rows share.
#[test]
fn a_git_failure_that_wrote_nothing_reads_nothing_written() {
    let mut push = entry(2, OperationKind::Push, "push main to origin");
    push.status = EntryStatus::Aborted {
        reason: "fatal: unable to access 'https://h/r.git/': could not resolve host".to_owned(),
    };
    push.index_after = push.index_before;
    let state = state_with(vec![entry(1, OperationKind::Stage, "stage src/a.rs"), push]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::NothingWritten);
    let label = view.rows[0].mark.label(&state.keymap);
    assert_eq!(label, "nothing written");
    assert!(!label.contains("refused"), "{label}");
}

/// The one-time move to the new names (ticket 26) is a row of its own,
/// worded by what moved, and `u` passes over it to the stage before.
#[test]
fn migrate_row_words() {
    let mut migrate = entry(
        2,
        OperationKind::Migrate,
        // The words a move's line stored (journal lines are read as written).
        "rename to Mergewright (3 refs, 12 files)",
    );
    migrate.touches.index = false;
    migrate.index_before = None;
    migrate.index_after = None;
    migrate.backup_ref = None;
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        migrate,
    ]);
    let view = journal_view(&state, 10);
    assert_eq!(
        view.rows[0].description.as_str(),
        "rename to Mergewright (3 refs, 12 files)"
    );
    assert_eq!(view.rows[0].mark, RowMark::None, "not the next undo");
    assert_eq!(view.rows[1].mark, RowMark::NextUndo, "the stage before is");
    assert_eq!(view.rows[0].summary.as_str(), "", "the words are the row");
}

/// Ticket 48 Part B: a move written by an earlier build keeps the name it
/// moved to. The words are stored in the journal line, not rebuilt from
/// the generation in force, so a Tributary-era row still reads Tributary.
/// The two lines are a golden in the journal's on-disk shape.
#[test]
fn an_old_migrate_row_keeps_its_target() {
    let lines = concat!(
        r#"{"id":"01HF7YAT000000000000000000","status":"pending","at_unix_ms":1700000000000,"kind":"migrate","description":"rename to Tributary (3 refs, 12 files)","touches":{"refs":[],"index":false,"worktree":false},"head_before":{"branch":"refs/heads/main"},"refs_before":{},"index_before":null,"worktree_before":null,"backup_ref":null,"pins":[],"reverses":null}"#,
        "\n",
        r#"{"id":"01HF7YAT000000000000000000","status":"committed","head_after":{"branch":"refs/heads/main"},"refs_after":{},"index_after":null,"worktree_after":null,"commands":[]}"#,
        "\n",
    );
    let parsed = mergewright_ops::journal::parse(lines.as_bytes()).expect("the golden parses");
    let mut entries = parsed.entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, OperationKind::Migrate);
    entries.insert(0, entry(1, OperationKind::Stage, "stage src/a.rs"));
    let state = state_with(entries);
    let view = journal_view(&state, 10);
    assert_eq!(
        view.rows[0].description.as_str(),
        "rename to Tributary (3 refs, 12 files)"
    );
}

mod replaced;

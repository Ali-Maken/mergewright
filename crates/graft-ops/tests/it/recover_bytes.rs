//! m7-power-tools/30w: a lost edit put back from the journal's list (`J n ⏎ y`)
//! comes back in his own bytes, where the backup kept them (ADR-0052).
//!
//! Each test: the repository's config set before its first commit; `src/a.rs`
//! committed; he edits it (`E1`); Graft discards it; he edits it again
//! (`LOST`); a forced `u` of the discard writes `E1` over `LOST` — a lost edit,
//! kept only in the undo's backup; `lost_edits` lists it and the put-back runs
//! from that row, as `⏎` on it does. Every expectation is read off the disk,
//! byte for byte.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry};
use graft_ops::apply::{AfterState, ApplyError, RawStepOf};
use graft_ops::journal_list::lost_edits;
use graft_ops::keys::{KeyNames, NoKeys, OpKey};
use graft_ops::operation::{Operation, RecoverParams, RecoverTarget, Target};
use graft_ops::restore::RestoreError;

use crate::support::{NOW, Never, TempRepo, journal_for, run, run_redo, run_undo, try_run};
use graft_core::on_disk::CURRENT_DIR;

const PATH: &str = "src/a.rs";
const COMMITTED: &[u8] = b"pub fn a() -> u32 {\n    1\n}\n";
/// His first edit, the one Graft discards.
const E1: &[u8] = b"pub fn a() -> u32 {\n    2\n}\n";
/// His second edit, the one the forced `u` writes over.
const LOST: &[u8] = b"pub fn a() -> u32 {\n    3\n}\n";
/// `LOST` as git's checkout writes it under `core.autocrlf=true`.
const LOST_CRLF: &[u8] = b"pub fn a() -> u32 {\r\n    3\r\n}\r\n";

const AUTOCRLF: &[(&str, &str)] = &[("core.autocrlf", "true")];

/// One file's three versions: committed, `E1`, `LOST`.
struct Versions {
    path: &'static str,
    committed: &'static [u8],
    e1: &'static [u8],
    lost: &'static [u8],
}

const A_RS: Versions = Versions {
    path: PATH,
    committed: COMMITTED,
    e1: E1,
    lost: LOST,
};

/// A repository with `config` set before its first commit, `versions`'
/// file committed.
fn committed(config: &[(&str, &str)], versions: &Versions) -> TempRepo {
    let repo = TempRepo::init();
    for (key, value) in config {
        repo.git(&["config", key, value]);
    }
    repo.write(versions.path, versions.committed);
    repo.commit_all("one");
    repo
}

/// `E1`, discarded by Graft; `LOST`; the forced `u` of the discard.
fn lose_an_edit(repo: &TempRepo, versions: &Versions) {
    repo.write(versions.path, versions.e1);
    run(
        repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from(versions.path)])),
        NOW,
    );
    repo.write(versions.path, versions.lost);
    run_undo(repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.read(versions.path),
        versions.e1,
        "the forced u wrote E1 over LOST"
    );
}

/// The put-back `⏎` runs on the one lost-edit row, and that row's entry.
fn the_put_back(repo: &TempRepo, versions: &Versions) -> (Operation, JournalEntry) {
    let handle = repo.open();
    let entries = journal_for(&handle).entries().to_vec();
    let edits = lost_edits(&handle, &entries, &Never).expect("lost edits read");
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].paths, vec![PathBuf::from(versions.path)]);
    let entry = entries
        .iter()
        .find(|recorded| recorded.id == edits[0].entry)
        .expect("the row's entry is journaled")
        .clone();
    let op = Operation::Recover(RecoverParams {
        target: RecoverTarget::Files {
            entry: edits[0].entry.clone(),
            backup_tree: edits[0].backup_tree,
            paths: edits[0].paths.clone(),
        },
        because: "undo of \"discard\"".to_owned(),
    });
    (op, entry)
}

/// Under `core.autocrlf=true` from the start: the lost edit, the put-back
/// run. Returns the put-back's entry.
fn put_back_under_autocrlf(repo: &TempRepo) -> JournalEntry {
    lose_an_edit(repo, &A_RS);
    let (op, _) = the_put_back(repo, &A_RS);
    run(repo, &op, NOW + 2)
}

#[test]
fn a_lost_edit_put_back_writes_his_lf_bytes_under_autocrlf() {
    let repo = committed(AUTOCRLF, &A_RS);

    let entry = put_back_under_autocrlf(&repo);

    assert_eq!(
        repo.read(PATH),
        LOST,
        "his LF bytes, not git's CRLF checkout"
    );
    // The raw step's two reads follow the restore on the entry's log.
    assert_eq!(entry.commands.len(), 3, "{:#?}", entry.commands);
    assert!(
        entry.commands[0].contains(" restore --source="),
        "{:#?}",
        entry.commands
    );
    assert!(
        entry.commands[1].contains(" ls-tree "),
        "{:#?}",
        entry.commands
    );
    assert!(
        entry.commands[2].contains(" cat-file --batch"),
        "{:#?}",
        entry.commands
    );
}

#[test]
fn u_after_a_lost_edit_put_back_gives_back_what_it_replaced_byte_for_byte() {
    let repo = committed(AUTOCRLF, &A_RS);
    put_back_under_autocrlf(&repo);

    run_undo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));

    assert_eq!(repo.read(PATH), E1, "u gives back E1, byte for byte");
}

#[test]
fn redo_after_undoing_a_lost_edit_put_back_writes_his_lf_bytes() {
    let repo = committed(AUTOCRLF, &A_RS);
    put_back_under_autocrlf(&repo);
    run_undo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));

    run_redo(&repo, NOW + 4, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));

    assert_eq!(repo.read(PATH), LOST, "U gives back his LF bytes");
}

/// 30q-a's one-way clean filter (nbstripout's shape: clean strips `#out`
/// lines, smudge is `cat`).
#[test]
fn a_lost_edit_put_back_under_a_one_way_clean_filter_keeps_the_stripped_lines() {
    let notebook = Versions {
        path: "nb.txt",
        committed: b"cell 1\n",
        e1: b"cell 2\n",
        lost: b"cell 3\n#out 42\n",
    };
    let repo = TempRepo::init();
    repo.write(".git/info/attributes", b"nb.txt filter=strip\n");
    repo.git(&["config", "filter.strip.clean", "sed '/^#out/d'"]);
    repo.git(&["config", "filter.strip.smudge", "cat"]);
    repo.write(notebook.path, notebook.committed);
    repo.commit_all("one");
    lose_an_edit(&repo, &notebook);
    let (op, _) = the_put_back(&repo, &notebook);

    run(&repo, &op, NOW + 2);

    assert_eq!(
        repo.read(notebook.path),
        b"cell 3\n#out 42\n",
        "the stripped line is back"
    );
}

/// Nothing converted at snapshot time, so the backup kept nothing raw: the
/// put-back's log is the restore alone, as before 30w.
#[test]
fn a_lost_edit_put_back_with_nothing_kept_raw_runs_no_raw_step() {
    let repo = committed(&[], &A_RS);
    lose_an_edit(&repo, &A_RS);
    let (op, lost_entry) = the_put_back(&repo, &A_RS);
    assert_eq!(lost_entry.worktree_raw_before, None, "nothing kept raw");
    let backup_tree = lost_entry.worktree_before.expect("the undo took a backup");

    let entry = run(&repo, &op, NOW + 2);

    assert_eq!(repo.read(PATH), LOST);
    assert_eq!(
        entry.commands,
        vec![format!(
            "git --literal-pathspecs restore --source={backup_tree} --worktree -- {PATH}"
        )],
        "no ls-tree, no cat-file"
    );
}

/// A file already at every `write_raw` temporary name (30s's setup): the
/// restore went through, the raw step cannot land. `Committed`, and the
/// answer says so first, in 30s's words.
#[test]
fn a_lost_edit_put_back_whose_raw_step_cannot_land_says_so() {
    let repo = committed(AUTOCRLF, &A_RS);
    lose_an_edit(&repo, &A_RS);
    let (op, _) = the_put_back(&repo, &A_RS);
    let in_the_way = repo.block_raw_writes_in("src");

    let Err(ApplyError::AfterState { applied, unread }) = try_run(&repo, &op, NOW + 2) else {
        panic!("the raw step's failure was not reported");
    };

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    let [
        AfterState::NotAsTheyWere {
            error: RestoreError::WriteRaw { path, source, .. },
            files: 1,
            ..
        },
    ] = unread.as_slice()
    else {
        panic!("{unread:?}");
    };
    assert_eq!(path, &PathBuf::from(PATH));
    assert_eq!(
        unread[0].to_string(),
        format!(
            "your edits are back · {PATH} not written back as it was: {source} · its exact bytes \
             are in the backup"
        )
    );
    assert_eq!(repo.read(PATH), LOST_CRLF, "LOST's content, in git's CRLF");
    for path in &in_the_way {
        assert_eq!(repo.read(path), b"", "{path}, in the way, is untouched");
    }
}

/// R2's bound (ADR-0052): `core.autocrlf` switched on after the edit was
/// lost. The backup kept nothing raw (git converted nothing then), so the
/// put-back is git's checkout under today's setting: CRLF. Pinned, not
/// endorsed: the gap row `M7/30w-autocrlf-later`.
/// 20b R2: a lost edit's put-back whose raw step is blocked names the route
/// that `after_a_failed_raw_step_u_then_the_put_back_again_restores_his_bytes`
/// walks: clear the way, `u` (as he has it bound), then this put-back again.
#[test]
fn a_failed_raw_step_names_the_route() {
    struct Undo;
    impl KeyNames for Undo {
        fn label(&self, key: OpKey) -> Option<String> {
            (key == OpKey::Undo).then(|| "u".to_owned())
        }
    }

    let repo = committed(AUTOCRLF, &A_RS);
    lose_an_edit(&repo, &A_RS);
    let (op, _) = the_put_back(&repo, &A_RS);
    let _in_the_way = repo.block_raw_writes_in("src");

    let Err(ApplyError::AfterState { unread, .. }) = try_run(&repo, &op, NOW + 2) else {
        panic!("the raw step's failure was not reported");
    };
    let [state @ AfterState::NotAsTheyWere { of, .. }] = unread.as_slice() else {
        panic!("{unread:?}");
    };
    assert_eq!(*of, RawStepOf::LostEdit);
    let words = state.explain(&Undo);
    assert!(
        words
            .ends_with("its exact bytes are in the backup: fix that, then u and put it back again"),
        "{words}"
    );
    assert_eq!(
        state.explain(&NoKeys),
        state.to_string(),
        "undo unbound: today's words, no clause"
    );
}

#[test]
fn a_lost_edit_lost_before_autocrlf_was_set_comes_back_as_git_checks_it_out_now() {
    let repo = committed(&[], &A_RS);
    lose_an_edit(&repo, &A_RS);
    repo.git(&["config", "core.autocrlf", "true"]);
    let (op, lost_entry) = the_put_back(&repo, &A_RS);
    assert_eq!(lost_entry.worktree_raw_before, None, "nothing kept raw");

    run(&repo, &op, NOW + 2);

    assert_eq!(repo.read(PATH), LOST_CRLF, "git's checkout of today");
}

/// m8-polish-and-ship/20 item 12: the raw step's count is the files it
/// tried — the ones `raw/` holds — not every put-back path. `docs/b.txt` is
/// CRLF on disk, which `core.autocrlf=true` gives back, so it is not kept
/// raw and never tried: one file, not two.
#[test]
fn a_failed_raw_step_counts_only_the_files_it_tried() {
    const B: &str = "docs/b.txt";
    let repo = committed(AUTOCRLF, &A_RS);
    repo.write(B, b"one\r\n");
    repo.commit_all("two");
    repo.write(PATH, E1);
    repo.write(B, b"two\r\n");
    run(
        &repo,
        &Operation::Discard(Target::Paths(vec![PathBuf::from(PATH), PathBuf::from(B)])),
        NOW,
    );
    repo.write(PATH, LOST);
    repo.write(B, b"three\r\n");
    run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));

    let handle = repo.open();
    let entries = journal_for(&handle).entries().to_vec();
    let edits = lost_edits(&handle, &entries, &Never).expect("lost edits read");
    assert_eq!(edits.len(), 1, "{edits:?}");
    assert_eq!(edits[0].paths.len(), 2, "{edits:?}");
    let op = Operation::Recover(RecoverParams {
        target: RecoverTarget::Files {
            entry: edits[0].entry.clone(),
            backup_tree: edits[0].backup_tree,
            paths: edits[0].paths.clone(),
        },
        because: "undo of \"discard\"".to_owned(),
    });
    let _in_the_way = repo.block_raw_writes_in("src");

    let Err(ApplyError::AfterState { unread, .. }) = try_run(&repo, &op, NOW + 2) else {
        panic!("the raw step's failure was not reported");
    };
    assert!(
        matches!(
            unread.as_slice(),
            [AfterState::NotAsTheyWere {
                error: RestoreError::WriteRaw { .. },
                files: 1,
                ..
            }]
        ),
        "{unread:?}"
    );
}

/// m8-polish-and-ship/20 item 13: `ApplyError::Journal.kept` carries the
/// words of a raw step that failed (`apply/close.rs`, the `finish` error
/// arm), so the missing outcome line does not take them with it. A smudge
/// filter, run by the put-back's `git restore` after the pending line is
/// written, replaces the journal file with a directory; the raw step is
/// blocked as above.
#[test]
fn a_journal_failure_after_a_failed_raw_step_keeps_its_words() {
    let repo = committed(AUTOCRLF, &A_RS);
    lose_an_edit(&repo, &A_RS);
    let (op, _) = the_put_back(&repo, &A_RS);
    let _in_the_way = repo.block_raw_writes_in("src");
    repo.write(".gitattributes", b"src/a.rs filter=breaks-journal\n");
    repo.git(&[
        "config",
        "filter.breaks-journal.smudge",
        &format!(
            "j=$(git rev-parse --git-path {CURRENT_DIR}/undo-journal.jsonl); \
             if grep -q 'put back your edit' \"$j\" 2>/dev/null; then rm -f \"$j\" && mkdir \"$j\"; fi; cat"
        ),
    ]);

    let error = match try_run(&repo, &op, NOW + 2) {
        Err(error @ ApplyError::Journal { .. }) => error,
        other => panic!("the outcome line should fail: {other:?}"),
    };
    let ApplyError::Journal { kept, .. } = &error else {
        unreachable!()
    };
    let kept = kept.as_deref().expect("the raw step's words are kept");
    assert!(
        kept.starts_with(&format!(
            "your edits are back · {PATH} not written back as it was:"
        )),
        "{kept}"
    );
    let words = error.explain(&crate::support::Letters);
    assert!(words.ends_with(kept), "the words carry it: {words}");
}

/// m8-polish-and-ship/20 item 14: after a raw step that could not land, the
/// route the words point at — `u` the put-back, then `J`, the lost-edit row,
/// `⏎`, `y` — brings his exact bytes back once the way is clear.
#[test]
fn after_a_failed_raw_step_u_then_the_put_back_again_restores_his_bytes() {
    let repo = committed(AUTOCRLF, &A_RS);
    lose_an_edit(&repo, &A_RS);
    let (op, _) = the_put_back(&repo, &A_RS);
    let in_the_way = repo.block_raw_writes_in("src");
    let Err(ApplyError::AfterState { .. }) = try_run(&repo, &op, NOW + 2) else {
        panic!("the raw step's failure was not reported");
    };
    // The undo's own raw write needs the way clear too; clearing it changes
    // the tree, so `u` asks once more ("u again to go ahead").
    for path in &in_the_way {
        std::fs::remove_file(repo.path().join(path)).expect("the way is cleared");
    }
    run_undo(&repo, NOW + 3, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    let handle = repo.open();
    let entries = journal_for(&handle).entries().to_vec();
    let edits = lost_edits(&handle, &entries, &Never).expect("lost edits read");
    assert_eq!(edits.len(), 1, "the row is still there for J: {edits:?}");
    assert_eq!(repo.read(PATH), E1, "u put the tree back as it was");
    let (op, _) = the_put_back(&repo, &A_RS);
    run(&repo, &op, NOW + 4);

    assert_eq!(
        repo.read(PATH),
        LOST,
        "his exact bytes, LF as he wrote them"
    );
}

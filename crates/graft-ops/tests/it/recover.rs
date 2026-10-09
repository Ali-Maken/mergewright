//! m7-power-tools/23: the put-back from the journal's list, as one
//! journaled operation — each target on a real repository, its refusals,
//! `u` over each, `lost_edits` and the preview's `your files:` line.
//!
//! Every expectation is read back out of the repository with `git` and the
//! files on disk, never from the value the run returned.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, JournalEntry, Touches};
use graft_core::oid::Oid;
use graft_fixtures::lost_work::{self, Graft, LostWork};
use graft_git::repo::RepoHandle;
use graft_ops::apply::{ApplyError, apply};
use graft_ops::facts::read_facts;
use graft_ops::journal_list::lost_edits;
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, FetchParams, FilesEffect, Operation, RecoverParams,
    RecoverTarget, Refusal, Target,
};
use graft_ops::preview::{self, PreviewPlan};
use graft_ops::undo;

use crate::support::{
    NOW, Never, RepoState, TempRepo, context, git_version, journal_for, run, run_redo, run_undo,
    stash_list, state_of, try_run,
};

// ── the repository ──────────────────────────────────────────────────────

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("an object id")
}

/// `main`: one (`base.txt`, `a.txt`), two (`b.txt`), three (`c.txt`).
fn three_commits() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.write("b.txt", b"b\n");
    repo.commit_all("two");
    repo.write("c.txt", b"c\n");
    repo.commit_all("three");
    repo
}

fn put_back(target: RecoverTarget, because: &str) -> Operation {
    Operation::Recover(RecoverParams {
        target,
        because: because.to_owned(),
    })
}

fn move_ref(name: &str, to: Oid, expected: Oid) -> Operation {
    put_back(
        RecoverTarget::MoveRef {
            name: name.to_owned(),
            to,
            expected,
        },
        "the shell's reset",
    )
}

fn recreate(name: &str, at: Oid) -> Operation {
    put_back(
        RecoverTarget::Recreate {
            name: name.to_owned(),
            at,
        },
        "the shell's branch -D",
    )
}

fn refused(repo: &TempRepo, op: &Operation) -> Refusal {
    match try_run(repo, op, NOW) {
        Err(ApplyError::Refused(refusal)) => refusal,
        Err(other) => panic!("not a refusal: {}", other.explain(&crate::support::Letters)),
        Ok(_) => panic!("ran; expected a refusal"),
    }
}

/// Everything `u` must put back, and git's own word on the status.
fn whole_state(repo: &TempRepo) -> (RepoState, String, String) {
    (
        state_of(repo),
        repo.git(&["status", "--porcelain=v2", "--untracked-files=all"]),
        stash_list(repo),
    )
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    journal_for(&repo.open()).entries().to_vec()
}

fn preview_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("preview: {}", why.explain(&crate::support::Letters)))
}

/// `feature` at two, `main` checked out at three.
fn with_feature() -> TempRepo {
    let repo = three_commits();
    repo.git(&["branch", "feature", "HEAD~1"]);
    repo
}

// ── a lost edit, made the way M6/77 makes one ───────────────────────────

/// `main` holds `a.txt` = `a`; `other` changes `a.txt` and adds `o.txt`.
/// Checked out on `main`.
fn main_and_other() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("a.txt", b"a on other\n");
    repo.write("o.txt", b"o\n");
    repo.commit_all("two on other");
    repo.git(&["switch", "-q", "main"]);
    repo
}

fn checkout_other() -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/other".to_owned()),
        head_before: graft_core::journal::HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    })
}

const HIS_EDIT: &[u8] = b"a on other, and his edit\n";

/// Check out `other`, edit `a.txt`, then undo forced over the drift: the
/// edit survives only in the undo's backup. Returns the undo's entry.
fn undo_over_drift(repo: &TempRepo) -> JournalEntry {
    run(repo, &checkout_other(), NOW);
    repo.write("a.txt", HIS_EDIT);
    run_undo(repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)))
        .entry
}

fn lost(repo: &TempRepo) -> Vec<graft_ops::journal_list::LostEdit> {
    lost_edits(&repo.open(), &entries(repo), &Never).expect("lost edits read")
}

fn files_of(entry: &JournalEntry, paths: &[&str]) -> Operation {
    put_back(
        RecoverTarget::Files {
            entry: entry.id.clone(),
            backup_tree: entry.worktree_before.expect("the undo took a backup"),
            paths: paths.iter().map(PathBuf::from).collect(),
        },
        "undo of \"checkout other\"",
    )
}

// ── targets ─────────────────────────────────────────────────────────────

#[test]
fn move_ref_not_checked_out_moves_only_the_ref() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    repo.git(&["update-ref", "refs/heads/feature", &one.to_string()]);
    let (before, status, stashes) = whole_state(&repo);

    run(&repo, &move_ref("feature", two, one), NOW);

    assert_eq!(id(&repo, "refs/heads/feature"), two);
    let (after, status_after, stashes_after) = whole_state(&repo);
    assert_eq!(after.index, before.index, "the index is not touched");
    assert_eq!(after.files, before.files, "no file on disk changes");
    assert_eq!(after.head, before.head);
    assert_eq!(status_after, status);
    assert_eq!(stashes_after, stashes);
}

#[test]
fn move_ref_checked_out_resets_and_puts_unsaved_edits_back() {
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write("a.txt", b"a, his unsaved edit\n");

    run(&repo, &move_ref("main", three, two), NOW);

    assert_eq!(id(&repo, "refs/heads/main"), three);
    assert_eq!(repo.read("c.txt"), b"c\n", "the lost commit's file is back");
    assert_eq!(
        repo.read("a.txt"),
        b"a, his unsaved edit\n",
        "his edit is put back after"
    );
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        " M a.txt",
        "unstaged, as it was"
    );
    assert_eq!(stash_list(&repo), "", "the set-aside stash is dropped");
}

/// `reset --hard` would write `c.txt` over his untracked `c.txt`: refused
/// in the preview and again at `y`, and his file's bytes stay (conductor
/// ruling on open point 1; `git checkout` refuses the same case).
#[test]
fn move_ref_checked_out_refuses_over_an_untracked_file_it_would_overwrite() {
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write("c.txt", b"his own c, never added\n");
    let op = move_ref("main", three, two);
    let before = whole_state(&repo);
    // ADR-0049: `InTheWay`, no longer `UnsavedEditsTo` (m7-power-tools/28a).
    let in_the_way = Refusal::InTheWay {
        doing: format!("putting main back to {}", &three.to_string()[..7]),
        ignored: Vec::new(),
        untracked: vec![PathBuf::from("c.txt")],
    };

    let applied = try_run(&repo, &op, NOW);
    assert_eq!(
        repo.read("c.txt"),
        b"his own c, never added\n",
        "his file was overwritten"
    );
    match applied {
        Err(ApplyError::Refused(refusal)) => assert_eq!(refusal, in_the_way),
        Err(other) => panic!("not a refusal: {}", other.explain(&crate::support::Letters)),
        Ok(_) => panic!("ran; expected a refusal"),
    }

    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    match preview::plan(&handle, &git_version(), &op, &facts, &Never) {
        Err(preview::PreviewError::Refused(refusal)) => assert_eq!(refusal, in_the_way),
        other => panic!("preview not refused: {other:?}"),
    }
    assert_eq!(whole_state(&repo), before, "nothing written");
    assert!(entries(&repo).is_empty(), "nothing journaled");
}

/// Review m7-22 C1: an **ignored** file where `to` tracks one is his work
/// too. `git status` never lists it, but `reset --hard` writes over it and
/// the backup (`add -A`) cannot hold it, so `u` could not bring it back.
/// Decided from the disk: refused in the preview and again at `y`.
#[test]
fn move_ref_checked_out_refuses_over_an_ignored_file_it_would_overwrite() {
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", b"HIS IGNORED FILE\n");
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
    let op = move_ref("main", three, two);
    let in_the_way = Refusal::InTheWay {
        doing: format!("putting main back to {}", &three.to_string()[..7]),
        ignored: vec![PathBuf::from("c.txt")],
        untracked: Vec::new(),
    };

    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    match preview::plan(&handle, &git_version(), &op, &facts, &Never) {
        Err(preview::PreviewError::Refused(refusal)) => assert_eq!(refusal, in_the_way),
        other => panic!("preview not refused: {other:?}"),
    }

    let applied = try_run(&repo, &op, NOW);
    assert_eq!(
        repo.read("c.txt"),
        b"HIS IGNORED FILE\n",
        "his ignored file was overwritten"
    );
    match applied {
        Err(ApplyError::Refused(refusal)) => assert_eq!(refusal, in_the_way),
        Err(other) => panic!("not a refusal: {}", other.explain(&crate::support::Letters)),
        Ok(_) => panic!("ran; expected a refusal"),
    }
    assert_eq!(id(&repo, "refs/heads/main"), two, "the branch did not move");
    assert!(entries(&repo).is_empty(), "nothing journaled");
}

/// A tracked file `to` changes is the set-aside's, not "in the way": the
/// disk check skips every path the index tracks.
#[test]
fn a_tracked_file_the_put_back_changes_is_not_in_the_way() {
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.write("a.txt", b"a, changed later\n");
    repo.commit_all("four");
    let four = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", &three.to_string()]);
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    // `a.txt` is tracked at `two` and changes at `four`: not in the way.
    run(&repo, &move_ref("main", four, two), NOW);

    assert_eq!(id(&repo, "refs/heads/main"), four);
    assert_eq!(repo.read("a.txt"), b"a, changed later\n");
}

/// Open point 2: a `Files` put-back of a path the backup does not hold and
/// the index does not track. git refuses the pathspec; the entry closes
/// `Aborted` and nothing on disk or in the repository changes.
#[test]
fn files_over_a_path_the_backup_lacks_changes_nothing() {
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    let before = whole_state(&repo);

    let result = try_run(&repo, &files_of(&undo, &["never.txt"]), NOW + 2);

    assert!(
        result.is_err(),
        "git refuses the path: {:?}",
        result.is_ok()
    );
    assert_eq!(
        whole_state(&repo),
        before,
        "nothing on disk or in git changed"
    );
    let last = entries(&repo).pop().expect("an entry");
    assert!(
        matches!(
            last.status,
            graft_core::journal::EntryStatus::Aborted { .. }
        ),
        "{:?}",
        last.status
    );
}

#[test]
fn move_ref_refuses_when_moved_on_since() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    let three = id(&repo, "main");
    // The list saw `feature` at one; it has moved on to three since.
    repo.git(&["update-ref", "refs/heads/feature", &three.to_string()]);
    let before = whole_state(&repo);

    let refusal = refused(&repo, &move_ref("feature", two, one));

    assert_eq!(
        refusal,
        Refusal::MovedOnSince {
            name: "feature".to_owned(),
            now: three
        }
    );
    assert_eq!(whole_state(&repo), before, "nothing written");
    assert!(entries(&repo).is_empty(), "nothing journaled");
}

#[test]
fn recreate_makes_the_branch() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    repo.git(&["branch", "-q", "-D", "feature"]);
    let (before, status, _) = whole_state(&repo);

    run(&repo, &recreate("feature", two), NOW);

    assert_eq!(id(&repo, "refs/heads/feature"), two);
    let (after, status_after, _) = whole_state(&repo);
    assert_eq!(after.files, before.files);
    assert_eq!(after.index, before.index);
    assert_eq!(after.head, before.head);
    assert_eq!(status_after, status);
}

#[test]
fn recreate_refuses_an_existing_name() {
    let repo = with_feature();
    let one = id(&repo, "HEAD~2");

    let refusal = refused(&repo, &recreate("feature", one));

    assert_eq!(
        refusal,
        Refusal::RefExists {
            name: "feature".to_owned()
        }
    );
}

#[test]
fn files_restores_the_edit_and_leaves_the_index() {
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    assert_eq!(repo.read("a.txt"), b"a\n", "the undo replaced his edit");
    let index_before = repo.git(&["ls-files", "-s"]);

    run(&repo, &files_of(&undo, &["a.txt"]), NOW + 2);

    assert_eq!(repo.read("a.txt"), HIS_EDIT, "his edit is back on disk");
    assert_eq!(
        repo.git(&["ls-files", "-s"]),
        index_before,
        "the index is untouched"
    );
    assert!(!repo.exists("o.txt"), "only the named path is written");
}

// ── a put-back over the file as it is now (m7-power-tools/30o R1) ──────
//
// The put-back no longer refuses a changed path: it writes the lost edit
// over it, and its own entry's before-snapshot keeps what it replaced, so
// `u` brings that back. Proved first (the premise), on the fixture as the
// acceptance walk builds it.

/// Graft's own operations for `LostWork::build`, as `lost_work.rs` runs
/// them.
struct GraftOps;

fn apply_in(work: &Path, op: &Operation, at_ms: i64) -> Result<(), String> {
    let repo = RepoHandle::open(work).map_err(|why| why.to_string())?;
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at_ms, &never);
    apply(&mut ctx, op)
        .map(|_| ())
        .map_err(|why| why.explain(&crate::support::Letters))
}

fn undo_in(work: &Path, at_ms: i64, force: bool) -> Result<(), String> {
    let repo = RepoHandle::open(work).map_err(|why| why.to_string())?;
    let mut journal = journal_for(&repo);
    let never = Never;
    let mut ctx = context(&repo, &mut journal, at_ms, &never);
    undo::undo(&mut ctx, force)
        .map(|_| ())
        .map_err(|why| why.explain(&crate::support::Letters))
}

impl Graft for GraftOps {
    fn fetch(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        let op = Operation::Fetch(FetchParams {
            remote: Some("origin".to_owned()),
            prune: false,
        });
        apply_in(work, &op, at_ms)
    }

    fn discard(&mut self, work: &Path, path: &str, at_ms: i64) -> Result<(), String> {
        let op = Operation::Discard(Target::Paths(vec![PathBuf::from(path)]));
        apply_in(work, &op, at_ms)
    }

    fn undo_forced(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        undo_in(work, at_ms, true)
    }
}

/// After the fixture's last step: later than every time it writes.
const AFTER_FIXTURE: i64 = 1_900_000_000_000;

/// The fixture's one lost edit (`src/a.rs`) as the put-back `⏎` runs. The
/// file on disk is as the forced undo left it: `DISCARDED_EDIT`, unstaged.
fn the_lost_edit_put_back(work: &Path) -> Operation {
    let handle = RepoHandle::open(work).expect("the fixture opens");
    let entries = journal_for(&handle).entries().to_vec();
    let edits = lost_edits(&handle, &entries, &Never).expect("lost edits read");
    assert_eq!(edits.len(), 1, "{edits:?}");
    put_back(
        RecoverTarget::Files {
            entry: edits[0].entry.clone(),
            backup_tree: edits[0].backup_tree,
            paths: edits[0].paths.clone(),
        },
        "undo of \"discard\"",
    )
}

fn disk(work: &Path, path: &str) -> Vec<u8> {
    std::fs::read(work.join(path)).expect("the file reads")
}

#[test]
fn u_after_a_file_put_back_brings_back_what_it_replaced() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    assert_eq!(disk(work, lost_work::EDITED), lost_work::DISCARDED_EDIT);
    let op = the_lost_edit_put_back(work);

    apply_in(work, &op, AFTER_FIXTURE).expect("the put-back runs");
    assert_eq!(disk(work, lost_work::EDITED), lost_work::LOST_EDIT);

    undo_in(work, AFTER_FIXTURE + 1, false).expect("u runs, no drift");
    assert_eq!(
        disk(work, lost_work::EDITED),
        lost_work::DISCARDED_EDIT,
        "u brings back byte-exact what the put-back replaced"
    );
}

/// `o.txt` is `other`'s: his edit to it is lost when the undo goes back to
/// `main`, and he then makes an `o.txt` of his own, untracked. The put-back
/// writes the lost edit over it; `u` brings his untracked file back.
#[test]
fn u_brings_back_an_untracked_file_a_put_back_replaced() {
    let repo = main_and_other();
    run(&repo, &checkout_other(), NOW);
    repo.write("o.txt", b"his edit to o\n");
    let undo = run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)))
        .entry;
    assert!(!repo.exists("o.txt"), "the undo removed it");
    repo.write("o.txt", b"his new o, untracked\n");

    run(&repo, &files_of(&undo, &["o.txt"]), NOW + 2);
    assert_eq!(repo.read("o.txt"), b"his edit to o\n");

    run_undo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("o.txt"), b"his new o, untracked\n");
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", "o.txt"]),
        "?? o.txt",
        "untracked, as it was"
    );
}

#[test]
fn a_file_put_back_over_a_changed_file_runs() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let op = the_lost_edit_put_back(work);

    apply_in(work, &op, AFTER_FIXTURE).expect("the put-back runs over the undo's copy");

    assert_eq!(disk(work, lost_work::EDITED), lost_work::LOST_EDIT);
}

#[test]
fn the_file_put_back_names_what_it_replaces() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let op = the_lost_edit_put_back(work);
    let handle = RepoHandle::open(work).expect("the fixture opens");
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");

    let plan = preview::plan(&handle, &git_version(), &op, &facts, &Never)
        .unwrap_or_else(|why| panic!("preview: {}", why.explain(&crate::support::Letters)));

    assert_eq!(
        plan.files,
        Some(FilesEffect::Changes {
            come_back: vec![PathBuf::from(lost_work::EDITED)],
            go: vec![],
            set_aside: 0,
            replaced: vec![PathBuf::from(lost_work::EDITED)],
            still_staged: vec![],
        })
    );
}

/// An ignored file at the path: the snapshot's `add -A` skips it, so `u`
/// could not bring it back. Refused, as a `MoveRef` is (28a).
#[test]
fn a_file_put_back_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    run(&repo, &checkout_other(), NOW);
    repo.write("o.txt", b"his edit to o\n");
    let undo = run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)))
        .entry;
    repo.write(".git/info/exclude", b"o.txt\n");
    repo.write("o.txt", b"his ignored o\n");

    let refusal = refused(&repo, &files_of(&undo, &["o.txt"]));

    assert_eq!(
        refusal,
        Refusal::InTheWay {
            doing: "putting back your edit".to_owned(),
            ignored: vec![PathBuf::from("o.txt")],
            untracked: vec![],
        }
    );
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "o.txt (ignored) is in the way of putting back your edit · move it aside first"
    );
    assert_eq!(repo.read("o.txt"), b"his ignored o\n", "nothing written");
}

// ── `u` after a put-back, for every path kind it writes over (30p R1) ───
//
// The mechanism is the entry's before-snapshot (`add -A` on an index copy)
// and `u` restoring the worktree from it. Each test observes it on real git
// for one kind of path, byte-exact, never from a returned value.

/// Run `git` in `work`; stdout as bytes, untrimmed (for `git show`).
fn git_bytes(work: &Path, args: &[&str]) -> Vec<u8> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(work)
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git runs: {why}"));
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn git_text(work: &Path, args: &[&str]) -> String {
    String::from_utf8(git_bytes(work, args)).expect("git's output is UTF-8")
}

/// What he staged at `src/a.rs` (test 1's `S`): differs from `HEAD`.
const STAGED: &[u8] = b"pub fn a() -> u32 {\n    3\n}\n";
/// What is on disk at `src/a.rs` after staging (`W`): differs from `S`.
const WORKING: &[u8] = b"pub fn a() -> u32 {\n    4\n}\n";

/// The lost-work fixture, then `src/a.rs` staged as `S` and edited to `W`:
/// the put-back's op, read while the lost edit is the fixture's.
fn staged_and_edited() -> (LostWork, Operation) {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let op = the_lost_edit_put_back(work);
    std::fs::write(work.join(lost_work::EDITED), STAGED).expect("S is written");
    git_bytes(work, &["add", "--", lost_work::EDITED]);
    std::fs::write(work.join(lost_work::EDITED), WORKING).expect("W is written");
    (fixture, op)
}

/// Test 1: Index `S` ≠ `HEAD`, worktree `W` ≠ `S`: the put-back writes the
/// worktree only, and `u` brings back `W` over the index it never touched.
#[test]
fn u_after_a_put_back_over_a_staged_file_brings_back_both() {
    let (fixture, op) = staged_and_edited();
    let work = fixture.work();
    let staged = |at: &str| {
        assert_eq!(
            git_bytes(work, &["show", ":src/a.rs"]),
            STAGED,
            "the index holds S {at}"
        );
    };
    staged("before");

    apply_in(work, &op, AFTER_FIXTURE).expect("the put-back runs");
    assert_eq!(disk(work, lost_work::EDITED), lost_work::LOST_EDIT);
    staged("after the put-back");

    undo_in(work, AFTER_FIXTURE + 1, false).expect("u runs, no drift");
    assert_eq!(disk(work, lost_work::EDITED), WORKING, "W is back");
    staged("after u");
}

/// Test 2: The `go` direction: his drift was a deletion of `a.txt`, which the
/// forced undo re-created, so the backup lacks it. He then writes `W` there.
/// The put-back removes it; `u` brings `W` back.
#[test]
fn u_brings_back_a_file_a_put_back_deleted() {
    let repo = main_and_other();
    run(&repo, &checkout_other(), NOW);
    std::fs::remove_file(repo.path().join("a.txt")).expect("his drift: a.txt deleted");
    let undo = run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)))
        .entry;
    assert_eq!(repo.read("a.txt"), b"a\n", "the undo re-created it");
    repo.write("a.txt", WORKING);
    let op = files_of(&undo, &["a.txt"]);
    let index = repo.git(&["ls-files", "-s"]);

    let plan = preview_of(&repo, &op);
    assert_eq!(
        plan.files,
        Some(FilesEffect::Changes {
            come_back: vec![],
            go: vec![PathBuf::from("a.txt")],
            set_aside: 0,
            replaced: vec![PathBuf::from("a.txt")],
            still_staged: vec![],
        }),
        "the preview's go holds the path"
    );

    run(&repo, &op, NOW + 2);
    assert!(
        std::fs::symlink_metadata(repo.path().join("a.txt")).is_err(),
        "the put-back removed it"
    );

    run_undo(&repo, NOW + 3, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("a.txt"), WORKING, "W is back");
    assert_eq!(
        repo.git(&["ls-files", "-s"]),
        index,
        "the index is unchanged"
    );
}

/// The fixture's put-back over `src/a.rs` as `set_up` leaves it, then `u`.
/// The index is the same before, between and after.
fn put_back_then_u(set_up: impl FnOnce(&Path)) -> LostWork {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let op = the_lost_edit_put_back(work);
    set_up(work);
    let index = git_text(work, &["ls-files", "-s"]);

    apply_in(work, &op, AFTER_FIXTURE).expect("the put-back runs");
    assert_eq!(
        git_text(work, &["ls-files", "-s"]),
        index,
        "the put-back leaves the index"
    );

    undo_in(work, AFTER_FIXTURE + 1, false).expect("u runs, no drift");
    assert_eq!(
        git_text(work, &["ls-files", "-s"]),
        index,
        "u leaves the index"
    );
    fixture
}

/// Test 3: His file is `W`, `0o755`; the backup's is `0o644`.
#[test]
fn u_brings_back_the_exec_bit_a_put_back_replaced() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = put_back_then_u(|work| {
        let path = work.join(lost_work::EDITED);
        std::fs::write(&path, WORKING).expect("W is written");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
            .expect("the exec bit is set");
    });
    let path = fixture.work().join(lost_work::EDITED);

    assert_eq!(std::fs::read(&path).expect("the file reads"), WORKING);
    let mode = std::fs::symlink_metadata(&path)
        .expect("the file is there")
        .permissions()
        .mode();
    assert_eq!(mode & 0o111, 0o111, "the exec bit is back: {mode:o}");
}

/// Test 4: His path is a symlink to `b.rs`.
#[test]
fn u_brings_back_a_symlink_a_put_back_replaced() {
    let fixture = put_back_then_u(|work| {
        let path = work.join(lost_work::EDITED);
        std::fs::remove_file(&path).expect("the file goes");
        std::os::unix::fs::symlink("b.rs", &path).expect("the symlink is made");
    });
    let path = fixture.work().join(lost_work::EDITED);

    let meta = std::fs::symlink_metadata(&path).expect("the path is there");
    assert!(meta.file_type().is_symlink(), "a symlink again: {meta:?}");
    assert_eq!(
        std::fs::read_link(&path).expect("the link reads"),
        PathBuf::from("b.rs")
    );
}

/// `core.autocrlf=true`, then his file `bytes`: after the put-back and `u`,
/// the same bytes.
fn u_under_autocrlf_brings_back(bytes: &'static [u8]) {
    let fixture = put_back_then_u(|work| {
        git_bytes(work, &["config", "core.autocrlf", "true"]);
        std::fs::write(work.join(lost_work::EDITED), bytes).expect("W is written");
    });

    assert_eq!(disk(fixture.work(), lost_work::EDITED), bytes);
}

/// Test 5: `W` with CRLF endings under `core.autocrlf=true`.
#[test]
fn u_brings_back_a_crlf_file_under_autocrlf() {
    u_under_autocrlf_brings_back(b"pub fn a() -> u32 {\r\n    4\r\n}\r\n");
}

/// 6. `W` with LF endings only under `core.autocrlf=true`.
#[test]
fn u_brings_back_an_lf_file_under_autocrlf() {
    u_under_autocrlf_brings_back(WORKING);
}

/// Test 7: `src/a.rs filter=rot`, clean and smudge both rot13.
#[test]
fn u_brings_back_a_file_under_a_clean_smudge_filter() {
    let fixture = put_back_then_u(|work| {
        std::fs::write(work.join(".git/info/attributes"), b"src/a.rs filter=rot\n")
            .expect("the attributes are written");
        let rot = "tr 'A-Za-z' 'N-ZA-Mn-za-m'";
        git_bytes(work, &["config", "filter.rot.clean", rot]);
        git_bytes(work, &["config", "filter.rot.smudge", rot]);
        std::fs::write(work.join(lost_work::EDITED), WORKING).expect("W is written");
    });

    assert_eq!(disk(fixture.work(), lost_work::EDITED), WORKING);
}

/// Test 8: A tracked path deleted on disk: the put-back re-creates it, `u`
/// removes it again, and the index never changes.
#[test]
fn u_after_a_put_back_that_recreates_a_deleted_file_removes_it() {
    let fixture = put_back_then_u(|work| {
        std::fs::remove_file(work.join(lost_work::EDITED)).expect("the file goes");
    });

    assert!(
        std::fs::symlink_metadata(fixture.work().join(lost_work::EDITED)).is_err(),
        "absent again"
    );
}

/// 30p R2: the preview names the replaced paths his index holds apart from
/// `HEAD`, so the box can say they stay staged.
#[test]
fn still_staged_names_the_staged_paths() {
    let edited = || vec![PathBuf::from(lost_work::EDITED)];
    let files_line = |work: &Path, op: &Operation| {
        let handle = RepoHandle::open(work).expect("the fixture opens");
        let facts =
            read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
        preview::plan(&handle, &git_version(), op, &facts, &Never)
            .unwrap_or_else(|why| panic!("preview: {}", why.explain(&crate::support::Letters)))
            .files
    };

    let (fixture, op) = staged_and_edited();
    assert_eq!(
        files_line(fixture.work(), &op),
        Some(FilesEffect::Changes {
            come_back: edited(),
            go: vec![],
            set_aside: 0,
            replaced: edited(),
            still_staged: edited(),
        })
    );

    // The fixture as built: his copy is unstaged, nothing stays staged.
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let op = the_lost_edit_put_back(fixture.work());
    assert_eq!(
        files_line(fixture.work(), &op),
        Some(FilesEffect::Changes {
            come_back: edited(),
            go: vec![],
            set_aside: 0,
            replaced: edited(),
            still_staged: vec![],
        })
    );
}

/// 30p R3: the copy a put-back replaces is never a lost-edit row — he chose
/// it in the box. The lost edit's own row goes once it is back on disk, and
/// comes back with `u`.
#[test]
fn a_put_back_leaves_no_lost_edit_row_and_u_brings_it_back() {
    let fixture = LostWork::build(&mut GraftOps).expect("the fixture builds");
    let work = fixture.work();
    let lost_now = || {
        let handle = RepoHandle::open(work).expect("the fixture opens");
        let entries = journal_for(&handle).entries().to_vec();
        lost_edits(&handle, &entries, &Never).expect("lost edits read")
    };
    let op = the_lost_edit_put_back(work);
    let before = lost_now();

    apply_in(work, &op, AFTER_FIXTURE).expect("the put-back runs");
    assert_eq!(
        lost_now(),
        vec![],
        "no row: not his edit, not the replaced copy"
    );

    undo_in(work, AFTER_FIXTURE + 1, false).expect("u runs, no drift");
    assert_eq!(lost_now(), before, "the one lost edit is back");
    assert_eq!(before.len(), 1);
}

#[test]
fn held_again_is_refused() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");

    // `feature` is back at two already: nothing to move.
    let refusal = refused(&repo, &move_ref("feature", two, one));
    assert_eq!(
        refusal,
        Refusal::HeldAgain {
            commit: two,
            by: "feature".to_owned()
        }
    );
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!(
            "{} is on feature again · nothing to put back",
            &two.to_string()[..7]
        )
    );

    // Made again already, at the same commit.
    let refusal = refused(&repo, &recreate("feature", two));
    assert_eq!(
        refusal,
        Refusal::HeldAgain {
            commit: two,
            by: "feature".to_owned()
        }
    );
}

// ── `u` ─────────────────────────────────────────────────────────────────

/// `y` then `u`: the repository equals its pre-`y` self — refs, index,
/// every file's bytes, `HEAD`, `status --porcelain=v2`, the stash list.
fn assert_u_puts_back(repo: &TempRepo, op: &Operation, at: i64) {
    let before = whole_state(repo);
    run(repo, op, at);
    assert_ne!(whole_state(repo), before, "the put-back changed nothing");
    run_undo(repo, at + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole_state(repo), before, "u did not put it back exactly");
}

#[test]
fn u_undoes_each_target() {
    // MoveRef, not checked out.
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    repo.git(&["update-ref", "refs/heads/feature", &one.to_string()]);
    assert_u_puts_back(&repo, &move_ref("feature", two, one), NOW);

    // MoveRef, checked out, with an unsaved edit set aside around it.
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write("a.txt", b"a, his unsaved edit\n");
    assert_u_puts_back(&repo, &move_ref("main", three, two), NOW);

    // Recreate.
    let repo = with_feature();
    let two = id(&repo, "feature");
    repo.git(&["branch", "-q", "-D", "feature"]);
    assert_u_puts_back(&repo, &recreate("feature", two), NOW);

    // Files.
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    assert_u_puts_back(&repo, &files_of(&undo, &["a.txt"]), NOW + 2);
}

/// The tip a `MoveRef` moves off is held by nothing else; `u` still puts
/// the branch back on it after `gc`, because the entry pins it.
#[test]
fn u_after_gc_puts_the_branch_back_on_a_tip_only_the_entry_holds() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("f.txt", b"f\n");
    repo.commit_all("on feature only");
    let only = id(&repo, "HEAD");
    repo.git(&["switch", "-q", "main"]);

    run(&repo, &move_ref("feature", two, only), NOW);
    crate::support::expire_and_prune(&repo);
    assert!(crate::support::holds(&repo, &only.to_string()), "pinned");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(id(&repo, "refs/heads/feature"), only);
}

// ── lost edits ──────────────────────────────────────────────────────────

#[test]
fn lost_edits_finds_the_edit_an_undo_over_drift_replaced() {
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);

    let found = lost(&repo);

    // `a.txt` only: `o.txt` is the checkout's own file, which the undo
    // removed and a commit still holds — not his edit.
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].entry, undo.id);
    assert_eq!(found[0].paths, vec![PathBuf::from("a.txt")]);
    assert_eq!(Some(found[0].backup_tree), undo.worktree_before);
    let held = repo.git(&["cat-file", "-p", &format!("{}:a.txt", found[0].backup_tree)]);
    assert_eq!(
        format!("{held}\n").as_bytes(),
        HIS_EDIT,
        "the backup holds it"
    );
}

#[test]
fn lost_edits_finds_the_same_after_a_redo_over_drift() {
    let repo = main_and_other();
    run(&repo, &checkout_other(), NOW);
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert!(lost(&repo).is_empty(), "a clean undo loses nothing");
    repo.write("a.txt", b"his edit on main\n");
    let forced = run_redo(&repo, NOW + 2, true)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)))
        .entry;
    assert_eq!(repo.read("a.txt"), b"a on other\n", "the redo replaced it");

    let found = lost(&repo);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].entry, forced.id);
    assert_eq!(found[0].paths, vec![PathBuf::from("a.txt")]);
}

#[test]
fn an_edit_already_back_on_disk_is_not_lost() {
    let repo = main_and_other();
    undo_over_drift(&repo);
    assert_eq!(lost(&repo).len(), 1);
    repo.write("a.txt", HIS_EDIT);

    assert!(lost(&repo).is_empty(), "{:?}", lost(&repo));
}

/// Review m7-22 M2: a path is lost only where the undo **wrote**. He edits
/// `b.txt`, which neither the checkout nor its undo touches, the undo is
/// forced over his drift, and he edits `b.txt` again: that edit is on disk,
/// not lost. Only `a.txt`, which the undo wrote over, is.
#[test]
fn an_edit_to_a_file_the_undo_never_wrote_is_not_lost() {
    let repo = main_and_other();
    run(&repo, &checkout_other(), NOW);
    repo.write("a.txt", HIS_EDIT);
    repo.write("b.txt", b"his b\n");
    run_undo(&repo, NOW + 1, true)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.read("b.txt"), b"his b\n", "the undo left b.txt");
    repo.write("b.txt", b"his b, edited again\n");

    let found = lost(&repo);

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].paths, vec![PathBuf::from("a.txt")]);
}

/// The one-side-unknown arm: the reversed entry recorded no worktree, so
/// nothing tells his drift from the checkout's own files. Unknown is not
/// unchanged: every path the undo wrote over that is not on disk now is
/// claimed, the checkout's `o.txt` included.
#[test]
fn a_reversed_entry_with_no_recorded_worktree_claims_every_path_the_undo_wrote() {
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    let mut entries = entries(&repo);
    let reversed = undo
        .reverses
        .clone()
        .expect("the undo reverses the checkout");
    for entry in &mut entries {
        if entry.id == reversed {
            entry.worktree_after = None;
        }
    }

    let found = lost_edits(&repo.open(), &entries, &Never).expect("lost edits read");

    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].entry, undo.id);
    assert_eq!(
        found[0].paths,
        vec![PathBuf::from("a.txt"), PathBuf::from("o.txt")]
    );
}

// ── preview ─────────────────────────────────────────────────────────────

#[test]
fn recover_preview_lists_your_files() {
    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~2"]);
    repo.git(&["rm", "-q", "a.txt"]);
    repo.commit_all("drop a");
    let dropped = id(&repo, "HEAD");
    repo.write("base.txt", b"base, his unsaved edit\n");

    let plan = preview_of(&repo, &move_ref("main", three, dropped));

    assert_eq!(
        plan.files,
        Some(FilesEffect::Changes {
            come_back: vec![
                PathBuf::from("a.txt"),
                PathBuf::from("b.txt"),
                PathBuf::from("c.txt")
            ],
            go: vec![],
            set_aside: 1,
            replaced: vec![],
            still_staged: vec![],
        })
    );
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].name, "refs/heads/main");
    assert_eq!(plan.ref_moves[0].from, Some(dropped));
    assert_eq!(
        plan.commands,
        vec![format!("git reset --hard {three}")],
        "the plan's own line"
    );

    // Files: what comes back, from the backup.
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    let plan = preview_of(&repo, &files_of(&undo, &["a.txt"]));
    assert_eq!(
        plan.files,
        Some(FilesEffect::Changes {
            come_back: vec![PathBuf::from("a.txt")],
            go: vec![],
            set_aside: 0,
            replaced: vec![],
            still_staged: vec![],
        })
    );
}

#[test]
fn recover_preview_says_unchanged_when_not_checked_out() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    repo.git(&["update-ref", "refs/heads/feature", &one.to_string()]);

    let plan = preview_of(&repo, &move_ref("feature", two, one));

    assert_eq!(
        plan.files,
        Some(FilesEffect::Unchanged {
            not_checked_out: "feature isn't checked out".to_owned()
        })
    );
    assert_eq!(plan.set_aside, None);

    // Any other operation carries no files line.
    let other = preview_of(&repo, &recreate("spike", one));
    assert_eq!(
        other.files,
        Some(FilesEffect::Unchanged {
            not_checked_out: "spike isn't checked out".to_owned()
        })
    );
    let checkout = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: graft_core::journal::HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    assert_eq!(preview_of(&repo, &checkout).files, None);
}

// ── the journal ─────────────────────────────────────────────────────────

#[test]
fn a_recover_entry_describes_itself_in_words() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    let short = |oid: Oid| oid.to_string()[..7].to_owned();

    repo.git(&["update-ref", "refs/heads/feature", &one.to_string()]);
    let moved = run(&repo, &move_ref("feature", two, one), NOW);
    assert_eq!(
        moved.description,
        format!("put feature back to {} (the shell's reset)", short(two))
    );

    repo.git(&["branch", "-q", "-D", "feature"]);
    let made = run(&repo, &recreate("feature", two), NOW + 1);
    assert_eq!(
        made.description,
        format!(
            "made feature again at {} (the shell's branch -D)",
            short(two)
        )
    );

    let one_file = files_of_id(&["src/a.rs"]);
    assert_eq!(one_file.describe(), "put back your edit to src/a.rs");
    let three_files = files_of_id(&["src/a.rs", "src/b.rs", "src/c.rs"]);
    assert_eq!(
        three_files.describe(),
        "put back your edit to src/a.rs +2 more"
    );

    // And through a real run.
    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    let restored = run(&repo, &files_of(&undo, &["a.txt"]), NOW + 2);
    assert_eq!(restored.description, "put back your edit to a.txt");
    assert_eq!(restored.kind, graft_core::journal::OperationKind::Recover);
}

fn files_of_id(paths: &[&str]) -> Operation {
    put_back(
        RecoverTarget::Files {
            entry: EntryId::new(NOW, [7; 10]),
            backup_tree: Oid::from_hex(&"ab".repeat(20)).expect("an id"),
            paths: paths.iter().map(PathBuf::from).collect(),
        },
        "undo of \"x\"",
    )
}

/// The refs whose value differs between two `for-each-ref` listings.
fn changed_refs(before: &str, after: &str) -> Vec<String> {
    let parse = |listing: &str| -> std::collections::BTreeMap<String, String> {
        listing
            .lines()
            .filter_map(|line| line.split_once(' '))
            .map(|(name, oid)| (name.to_owned(), oid.to_owned()))
            .collect()
    };
    let (before, after) = (parse(before), parse(after));
    let mut names: Vec<String> = before
        .keys()
        .chain(after.keys())
        .filter(|name| before.get(*name) != after.get(*name))
        .cloned()
        .collect();
    names.sort();
    names.dedup();
    names
}

/// Run `op`: the entry's touches are `expected`, and cover everything git
/// says moved — every ref, the index, the files.
fn check(repo: &TempRepo, op: &Operation, at: i64, expected: &Touches) {
    let before = whole_state(repo);
    let entry = run(repo, op, at);
    let after = whole_state(repo);
    assert_eq!(&entry.touches, expected, "{}", entry.description);
    for name in changed_refs(&before.0.refs, &after.0.refs) {
        assert!(
            entry.touches.refs.contains(&name),
            "{name} moved but is not touched"
        );
    }
    if before.0.index != after.0.index {
        assert!(entry.touches.index, "the index changed but is not touched");
    }
    if before.0.files != after.0.files {
        assert!(entry.touches.worktree, "files changed but are not touched");
    }
}

#[test]
fn touches_match_what_changed() {
    let repo = with_feature();
    let two = id(&repo, "feature");
    let one = id(&repo, "HEAD~2");
    repo.git(&["update-ref", "refs/heads/feature", &one.to_string()]);
    check(
        &repo,
        &move_ref("feature", two, one),
        NOW,
        &Touches {
            refs: vec!["refs/heads/feature".to_owned()],
            index: false,
            worktree: false,
        },
    );
    repo.git(&["branch", "-q", "-D", "feature"]);
    check(
        &repo,
        &recreate("feature", two),
        NOW + 1,
        &Touches {
            refs: vec!["refs/heads/feature".to_owned()],
            index: false,
            worktree: false,
        },
    );

    let repo = three_commits();
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write("a.txt", b"a, his unsaved edit\n");
    check(
        &repo,
        &move_ref("main", three, two),
        NOW,
        &Touches {
            refs: vec!["refs/heads/main".to_owned(), "refs/stash".to_owned()],
            index: true,
            worktree: true,
        },
    );

    let repo = main_and_other();
    let undo = undo_over_drift(&repo);
    check(
        &repo,
        &files_of(&undo, &["a.txt"]),
        NOW + 2,
        &Touches {
            refs: vec![],
            index: false,
            worktree: true,
        },
    );
}

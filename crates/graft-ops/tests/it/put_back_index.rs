//! Every set-aside put-back keeps what he staged (m7-power-tools/30,
//! ADR-0045 amendment 2026-09-26 b): `git stash apply --index S`, and when
//! git cannot reinstate the index on the landed tree (it refuses and writes
//! nothing), a plain `stash apply S` with words naming the files.
//!
//! Every claim is read back from git or the files: the index with
//! `git show :<path>` and `git status --porcelain`, the stash list by commit
//! id, the Command Log from the journal entry.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::Ordering;

use graft_core::journal::{EntryStatus, HeadRef, PutBackWhy};
use graft_ops::apply::{AfterState, ApplyError};
use graft_ops::clash::live_clash;
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, FactsScope, Operation, RebaseParams, RecoverParams,
    RecoverTarget, Source,
};
use graft_ops::rewrite::{RewriteParams, RewriteRequest, TodoAction, plan};

use crate::set_aside::{
    StopWhen, hold_git_until_killed, id, journal_entries, set_aside_of, stash_commits, step_names,
    whole,
};
use crate::support::{
    NOW, Never, TempRepo, git_version, run_redo, run_undo, run_with_stop, state_of, try_run,
};

// ── repositories ────────────────────────────────────────────────────────

/// `f.txt` and `o.txt` on `main`; `feature` adds two commits to `feat.txt`;
/// `main` then changes `o.txt`. Nothing touches `f.txt` after the fork, so
/// a staged change to it applies anywhere.
fn forked() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("f.txt", b"v1\n");
    repo.write("o.txt", b"o\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("feat.txt", b"feat\n");
    repo.commit_all("feat one");
    repo.write("feat.txt", b"feat\nfeat two\n");
    repo.commit_all("feat two");
    repo.git(&["switch", "-q", "main"]);
    repo.write("o.txt", b"o on main\n");
    repo.commit_all("two on main");
    repo
}

fn checkout_to(repo: &TempRepo, to: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(to.to_owned()),
        head_before: HeadRef::Branch(repo.git(&["symbolic-ref", "HEAD"])),
    })
}

fn rebase_onto_main(repo: &TempRepo) -> Operation {
    let name = repo.git(&["symbolic-ref", "HEAD"]);
    Operation::Rebase(RebaseParams::new(
        graft_ops::operation::Tip {
            oid: id(repo, &name),
            name,
        },
        Source::Branch("refs/heads/main".to_owned()),
        id(repo, "main"),
    ))
}

fn rewrite_planned(repo: &TempRepo, request: &RewriteRequest) -> RewriteParams {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(&handle, &git_version(), request, &facts, None, &Never)
        .unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

/// `R` from `feat one`, dropping `feat two`.
fn rewrite_dropping(repo: &TempRepo) -> Operation {
    let mut params = rewrite_planned(
        repo,
        &RewriteRequest::Open {
            from: id(repo, "HEAD~1"),
        },
    );
    let at = params
        .todo
        .iter()
        .position(|line| line.subject == "feat two")
        .expect("feat two is a row");
    params.todo[at].action = TodoAction::Drop;
    Operation::Rewrite(params)
}

/// `u` on the time machine's `MoveRef`: `main`, checked out, back one.
fn move_main_back(repo: &TempRepo) -> Operation {
    Operation::Recover(RecoverParams {
        target: RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: id(repo, "HEAD~1"),
            expected: id(repo, "HEAD"),
        },
        because: "the shell's commit".to_owned(),
    })
}

/// Every kind `Operation::sets_aside` returns `Some` for that keeps an index
/// of its own after it (`a`'s index goes into its commit: see
/// [`amend_into_commits_the_index_and_puts_the_rest_back`]).
#[derive(Clone, Copy, Debug)]
enum Kind {
    Checkout,
    /// `r`.
    Rebase,
    /// `R`.
    Rewrite,
    /// The time machine putting a checked-out branch back.
    MoveRef,
}

impl Kind {
    const ALL: [Kind; 4] = [Kind::Checkout, Kind::Rebase, Kind::Rewrite, Kind::MoveRef];

    fn start(self, repo: &TempRepo) {
        if let Kind::Rebase | Kind::Rewrite = self {
            repo.git(&["switch", "-q", "feature"]);
        }
    }

    fn op(self, repo: &TempRepo) -> Operation {
        match self {
            Kind::Checkout => checkout_to(repo, "refs/heads/feature"),
            Kind::Rebase => rebase_onto_main(repo),
            Kind::Rewrite => rewrite_dropping(repo),
            Kind::MoveRef => move_main_back(repo),
        }
    }
}

/// T2's shape: `f.txt` staged at v2, then edited on to v3. v2 is only in the
/// index.
fn staged_then_edited(repo: &TempRepo) {
    repo.write("f.txt", b"v2\n");
    repo.git(&["add", "f.txt"]);
    repo.write("f.txt", b"v3\n");
}

/// T2's whole fixture: [`forked`] on `main`, [`staged_then_edited`]. A
/// switch to `feature` puts the split back whole with `--index`.
pub(crate) fn staged_then_edited_forked() -> TempRepo {
    let repo = forked();
    staged_then_edited(&repo);
    repo
}

fn assert_split_kept(repo: &TempRepo, what: &str) {
    assert_eq!(
        repo.git(&["show", ":f.txt"]),
        "v2",
        "{what}: index keeps v2"
    );
    assert_eq!(repo.read("f.txt"), b"v3\n", "{what}: worktree keeps v3");
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", "f.txt"]),
        "MM f.txt",
        "{what}"
    );
}

// ── T1, T2, T3: the index comes back ─────────────────────────────────────

/// The Q1 fixture (review m7-28): `main` tracks `.env` and ignores it, he
/// ran `git rm --cached .env` and holds his own bytes in it; `other` tracks
/// the same `.env` blob and has no `.gitignore`.
fn rm_cached() -> TempRepo {
    let repo = TempRepo::init();
    repo.write(".env", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "other"]);
    repo.write(".gitignore", b".env\n");
    repo.commit_all("ignore .env");
    repo.git(&["rm", "-q", "--cached", ".env"]);
    repo.write(".env", b"MINE\n");
    repo
}

/// T1. What `git switch` gives: the deletion stays staged, his bytes on
/// disk.
#[test]
fn switch_keeps_a_staged_rm_cached() {
    let repo = rm_cached();
    let applied = try_run(&repo, &checkout_to(&repo, "refs/heads/other"), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/other");
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", ".env"]),
        "D  .env\n?? .env"
    );
    assert_eq!(repo.read(".env"), b"MINE\n");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

/// T2. Content only the index holds comes back in the index.
#[test]
fn switch_keeps_index_only_content() {
    let repo = forked();
    staged_then_edited(&repo);
    try_run(&repo, &checkout_to(&repo, "refs/heads/feature"), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_split_kept(&repo, "checkout");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
}

/// T3. Every kind that sets aside, over T2's shape: the split comes back,
/// `S` dropped, undo exact, redo exact.
#[test]
fn every_set_aside_kind_keeps_the_index() {
    for kind in Kind::ALL {
        let repo = forked();
        kind.start(&repo);
        staged_then_edited(&repo);
        let what = format!("{kind:?}");
        let before = whole(&repo);
        let applied = try_run(&repo, &kind.op(&repo), NOW)
            .unwrap_or_else(|why| panic!("{what}: {}", why.explain()));
        assert_eq!(applied.entry.status, EntryStatus::Committed, "{what}");
        set_aside_of(&applied.entry);
        assert_split_kept(&repo, &what);
        assert!(stash_commits(&repo).is_empty(), "{what}: S dropped");
        let after = whole(&repo);
        run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{what}: {}", why.explain()));
        assert_eq!(whole(&repo), before, "{what}: undo exact");
        run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{what}: {}", why.explain()));
        assert_eq!(whole(&repo), after, "{what}: redo exact");
    }
}

/// T3's `a` row. `a` puts the whole index into its commit (review m7-06b,
/// `r2`), so the index after equals the new `HEAD` there; what the put-back
/// keeps is his unstaged edit elsewhere. Not red first: plain apply gave
/// the same.
#[test]
fn amend_into_commits_the_index_and_puts_the_rest_back() {
    let repo = forked();
    repo.git(&["switch", "-q", "feature"]);
    repo.write("f.txt", b"v2\n");
    repo.git(&["add", "f.txt"]);
    repo.write("o.txt", b"o, unstaged\n");
    let before = whole(&repo);
    let params = rewrite_planned(
        &repo,
        &RewriteRequest::AmendInto {
            target: id(&repo, "HEAD~1"),
        },
    );
    try_run(&repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(repo.git(&["show", "HEAD~1:f.txt"]), "v2", "into the target");
    assert_eq!(repo.git(&["status", "--porcelain"]), " M o.txt");
    assert_eq!(repo.read("o.txt"), b"o, unstaged\n");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), before, "undo exact");
}

// ── T5, T6: git cannot reinstate the index ──────────────────────────────

/// `1\n2\n…20\n`.
fn twenty_lines() -> String {
    let mut lines = String::new();
    for n in 1..=20 {
        lines.push_str(&n.to_string());
        lines.push('\n');
    }
    lines
}

/// P3's fixture: a 20-line `f.txt`; `other` changes line 7; he stages an
/// edit to line 5. The staged hunk's context covers line 7, so `stash apply
/// --index` refuses on `other` and writes nothing; a plain apply lands.
pub(crate) fn staged_near_their_change() -> TempRepo {
    let repo = TempRepo::init();
    let lines = twenty_lines();
    repo.write("f.txt", lines.as_bytes());
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("f.txt", lines.replace("\n7\n", "\nseven\n").as_bytes());
    repo.commit_all("other changes line 7");
    repo.git(&["switch", "-q", "main"]);
    repo.write("f.txt", lines.replace("\n5\n", "\nfive\n").as_bytes());
    repo.git(&["add", "f.txt"]);
    repo
}

/// T5. The fallback: edits back, entry `Committed`, `S` dropped, and the
/// words name the file.
#[test]
fn put_back_falls_back_when_the_index_cannot_go_back() {
    let repo = staged_near_their_change();
    let (applied, unread) = match try_run(&repo, &checkout_to(&repo, "refs/heads/other"), NOW) {
        Err(ApplyError::AfterState { applied, unread }) => (*applied, unread),
        other => panic!("the staging should not be kept: {other:?}"),
    };
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    let [AfterState::StagingNotKept { paths }] = unread.as_slice() else {
        panic!("one unread, the staging: {unread:?}");
    };
    assert_eq!(paths, &[PathBuf::from("f.txt")]);
    assert_eq!(
        unread[0].to_string(),
        "your edits are back · staging not kept: f.txt · u undoes it, staging included"
    );
    let text = String::from_utf8(repo.read("f.txt")).expect("utf-8");
    assert!(
        text.contains("\nfive\n") && text.contains("\nseven\n"),
        "{text}"
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), " M f.txt");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
    assert_eq!(
        step_names(&applied.entry),
        [
            "stash create",
            "stash store",
            "reset --hard",
            "the operation",
            "stash apply",
            "stash apply",
            "stash drop"
        ],
        "{:?}",
        applied.entry.commands
    );
    assert!(applied.entry.commands[4].contains("stash apply --index"));
    assert!(!applied.entry.commands[5].contains("--index"));
    // No `A` path: the second read (`diff-tree S^2 S`, 30c) is not on the
    // Log. Reads are never logged, so this guards that rule too.
    assert!(
        !applied
            .entry
            .commands
            .iter()
            .any(|line| line.contains("diff-tree")),
        "{:?}",
        applied.entry.commands
    );
}

/// T5's words when more than one file loses its staging.
#[test]
fn staging_not_kept_counts_the_rest() {
    let words = AfterState::StagingNotKept {
        paths: vec!["a".into(), "b".into(), "c".into()],
    }
    .to_string();
    assert_eq!(
        words,
        "your edits are back · staging not kept: a +2 more · u undoes it, staging included"
    );
}

/// T6. P3 plus a line only the index holds: `u` gives the index and the
/// worktree exactly as they were.
#[test]
fn undo_after_a_fallback_restores_the_split() {
    let repo = staged_near_their_change();
    // Line 5 staged as `five`, the worktree taken back to `5`: only the
    // index holds `five`.
    let lines = twenty_lines();
    repo.write("f.txt", lines.as_bytes());
    let before = whole(&repo);
    assert_eq!(repo.git(&["status", "--porcelain"]), "MM f.txt");
    let result = try_run(&repo, &checkout_to(&repo, "refs/heads/other"), NOW);
    assert!(
        matches!(&result, Err(ApplyError::AfterState { unread, .. })
            if matches!(unread.as_slice(), [AfterState::StagingNotKept { .. }])),
        "{result:?}"
    );
    let after = whole(&repo);
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), before, "undo exact, the split included");
    // Minor 6 (review m7-30-06d): redo puts back the post-fallback state.
    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), after, "redo exact");
}

/// B3 (review m7-30-06d Minor 1): the drop fails after the fallback. The
/// failure line says where the edits are **and** that their staging went;
/// the `u` it names puts back the state before, stash list included.
#[cfg(unix)]
#[test]
fn a_drop_that_fails_after_the_fallback_names_the_staging() {
    use crate::set_aside_kill::{Shim, run_child};
    let repo = staged_near_their_change();
    let before = whole(&repo);
    let (killed, line) = run_child(&repo, "refs/heads/other", Shim::FailDrop);
    assert!(!killed);
    let line = line.expect("the child wrote its line");
    eprintln!("B3 line: {line}");
    assert!(line.contains("staging not kept: f.txt"), "{line}");
    assert!(
        line.contains("your edits are back · a copy stays in stash@{0}"),
        "{line}"
    );
    assert!(line.contains("u restores the backup"), "{line}");
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), before, "undo exact, stash list included");
}

// ── A1–A3: a staged new file on the fallback (review m7-30-06d Major 1) ──

/// P3 through the fallback: the entry's unread staging, or a panic.
fn fallback_paths(repo: &TempRepo) -> Vec<PathBuf> {
    match try_run(repo, &checkout_to(repo, "refs/heads/other"), NOW) {
        Err(ApplyError::AfterState { unread, .. }) => match unread.as_slice() {
            [AfterState::StagingNotKept { paths }] => {
                assert_eq!(
                    unread[0].to_string(),
                    format!(
                        "your edits are back · staging not kept: {} · u undoes it, staging included",
                        if paths.len() == 1 {
                            "f.txt".to_owned()
                        } else {
                            format!("f.txt +{} more", paths.len() - 1)
                        }
                    )
                );
                paths.clone()
            }
            other => panic!("one unread, the staging: {other:?}"),
        },
        other => panic!("the staging should not be kept: {other:?}"),
    }
}

/// A1. A staged new file edited on (`AM`): a plain apply re-adds it from
/// the tree, so its staged v1 is gone, and the words name it. `u` brings
/// v1 back in the index; redo puts the after-state back.
#[test]
fn put_back_names_a_staged_new_file_edited_on() {
    let repo = staged_near_their_change();
    repo.write("n.txt", b"v1\n");
    repo.git(&["add", "n.txt"]);
    repo.write("n.txt", b"v2\n");
    assert_eq!(repo.git(&["status", "--porcelain"]), "M  f.txt\nAM n.txt");
    let before = whole(&repo);
    assert_eq!(fallback_paths(&repo), ["f.txt", "n.txt"].map(PathBuf::from));
    // What git leaves: `n.txt` re-added from the tree, at v2.
    assert_eq!(repo.git(&["status", "--porcelain"]), " M f.txt\nA  n.txt");
    assert_eq!(repo.git(&["show", ":n.txt"]), "v2");
    let after = whole(&repo);
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(repo.git(&["show", ":n.txt"]), "v1", "undo: v1 staged again");
    assert_eq!(repo.read("n.txt"), b"v2\n");
    assert_eq!(whole(&repo), before, "undo exact");
    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), after, "redo exact");
}

/// A2. A staged new file then deleted from the tree (`AD`). `git stash
/// create` does not record that deletion: `S`'s tree holds `n.txt` at its
/// staged blob, so `S^2:n.txt` = `S:n.txt` and the plain apply re-adds it
/// staged at v1, **and back on disk**. Its staging came back, so it is not
/// named (30c NOTES: the ticket expected it named; git's `S` says
/// otherwise). The deletion is lost by the stash itself, not the fallback.
/// `u` restores `AD` exactly.
#[test]
fn put_back_leaves_a_staged_new_file_deleted_since_unnamed() {
    let repo = staged_near_their_change();
    repo.write("n.txt", b"v1\n");
    repo.git(&["add", "n.txt"]);
    std::fs::remove_file(repo.path().join("n.txt")).expect("n.txt removes");
    assert_eq!(repo.git(&["status", "--porcelain"]), "M  f.txt\nAD n.txt");
    let before = whole(&repo);
    assert_eq!(fallback_paths(&repo), [PathBuf::from("f.txt")]);
    // What git leaves: `n.txt` staged at v1 and on disk again.
    assert_eq!(repo.git(&["status", "--porcelain"]), " M f.txt\nA  n.txt");
    assert_eq!(repo.git(&["show", ":n.txt"]), "v1");
    assert_eq!(repo.read("n.txt"), b"v1\n");
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(whole(&repo), before, "undo exact: AD again");
}

/// A3. A staged new file unchanged since (`A `): git re-adds it exactly,
/// so naming it would be false.
#[test]
fn put_back_leaves_an_unchanged_staged_new_file_unnamed() {
    let repo = staged_near_their_change();
    repo.write("n.txt", b"v1\n");
    repo.git(&["add", "n.txt"]);
    assert_eq!(fallback_paths(&repo), [PathBuf::from("f.txt")]);
    assert_eq!(repo.git(&["status", "--porcelain"]), " M f.txt\nA  n.txt");
    assert_eq!(repo.git(&["show", ":n.txt"]), "v1");
}

// ── T7: a clash under --index ───────────────────────────────────────────

/// P4's fixture: `g.txt` staged, `f.txt` edited unstaged where `other`
/// changed the same line. The put-back clashes on `f.txt`; git resets the
/// index before its merge, as a plain apply does.
#[test]
fn a_clash_with_staging_is_the_same_clash() {
    let repo = TempRepo::init();
    let lines = twenty_lines();
    repo.write("f.txt", lines.as_bytes());
    repo.write("g.txt", lines.as_bytes());
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("f.txt", lines.replace("\n5\n", "\ntheirs5\n").as_bytes());
    repo.commit_all("other changes line 5");
    repo.git(&["switch", "-q", "main"]);
    repo.write("g.txt", lines.replace("\n3\n", "\nthree\n").as_bytes());
    repo.git(&["add", "g.txt"]);
    repo.write("f.txt", lines.replace("\n5\n", "\nmine5\n").as_bytes());

    let (entry, clash) = match try_run(&repo, &checkout_to(&repo, "refs/heads/other"), NOW) {
        Err(ApplyError::Clashed { entry, clash }) => (*entry, clash),
        other => panic!("the put-back should clash: {other:?}"),
    };
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("f.txt")]);
    assert_eq!(entry.status, EntryStatus::Committed);
    let stash = set_aside_of(&entry);
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    assert_eq!(repo.git(&["status", "--porcelain"]), "UU f.txt\nM  g.txt");
    assert!(
        entry
            .commands
            .iter()
            .any(|line| line.contains("stash apply --index")),
        "{:?}",
        entry.commands
    );
    assert!(
        !entry
            .commands
            .iter()
            .any(|line| line.contains("stash apply") && !line.contains("--index")),
        "no second apply over the clash: {:?}",
        entry.commands
    );

    let handle = repo.open();
    let scratch = repo.path().join(".git/graft");
    let index_now =
        graft_ops::snapshot::index_state(&handle, &scratch, &Never).expect("the index reads");
    let found = live_clash(&journal_entries(&repo), None, &index_now, &[(stash, 0)])
        .expect("the clash is live");
    assert_eq!(found.entry, entry.id);
    assert_eq!(found.why, PutBackWhy::SetAside);
}

// ── T8: an other failure never runs a second apply over a changed tree ──

/// Both applies refused with nothing unmerged (an untracked `n.txt` dropped
/// in the way of his staged new one by a hook): `--index` wrote nothing, so
/// the plain apply ran once, refused too, and `S` is kept.
#[test]
fn both_applies_refused_keeps_the_stash() {
    let (repo, op) = crate::set_aside::put_back_refused();
    let error = try_run(&repo, &op, NOW).expect_err("the put-back should fail");
    let ApplyError::Git { entry, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(
        step_names(entry),
        [
            "stash create",
            "stash store",
            "reset --hard",
            "the operation",
            "stash apply",
            "stash apply"
        ],
        "{:?}",
        entry.commands
    );
    assert_eq!(stash_commits(&repo).len(), 1, "S kept");
    assert_eq!(error.edits_at(), Some(graft_ops::EditsAt::Stash { n: 0 }));
}

// ── T10: his bytes are never only on disk ───────────────────────────────

/// ADR-0045 §5 for a `rm --cached` path HEAD tracks: at a stop before steps
/// 4, 5 and 6, `MINE` is on disk or at `<listed S>:.env`.
#[test]
fn the_rm_cached_bytes_are_never_only_on_disk() {
    for step in [4_u8, 5, 6] {
        let repo = rm_cached();
        let git_dir = repo.path().join(".git");
        let root = repo.path().to_path_buf();
        let stop = match step {
            4 => StopWhen::new(move || git_dir.join("refs/stash").exists()),
            5 => StopWhen::new(move || {
                std::fs::read(root.join(".env")).is_ok_and(|bytes| bytes == b"base\n")
            }),
            _ => {
                hold_git_until_killed(&repo);
                let start = std::fs::read(git_dir.join("HEAD")).expect("HEAD reads");
                StopWhen::new(move || {
                    std::fs::read(git_dir.join("HEAD")).ok() != Some(start.clone())
                })
            }
        };
        let result = run_with_stop(&repo, &checkout_to(&repo, "refs/heads/other"), NOW, &stop);
        assert!(
            stop.latched.load(Ordering::Relaxed),
            "step {step}: never fired"
        );
        let on_disk = repo.exists(".env") && repo.read(".env") == b"MINE\n";
        let in_stash = stash_commits(&repo).iter().any(|stash| {
            repo.try_git(&["show", &format!("{stash}:.env")])
                .is_ok_and(|shown| shown == "MINE")
        });
        assert!(on_disk || in_stash, "step {step}: MINE lost ({result:?})");
    }
}

/// T10's second half: `u` of the switch puts `MINE` on disk and the
/// deletion staged, as before.
#[test]
fn undo_of_the_switch_keeps_the_rm_cached() {
    let repo = rm_cached();
    let before = state_of(&repo);
    try_run(&repo, &checkout_to(&repo, "refs/heads/other"), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain()));
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(repo.read(".env"), b"MINE\n");
    assert_eq!(
        repo.git(&["status", "--porcelain", "--ignored", "--", ".env"]),
        "D  .env\n!! .env"
    );
    assert_eq!(state_of(&repo), before);
}

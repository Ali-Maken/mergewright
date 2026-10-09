//! Replays (28b): rebase, rewrite, `a` into, pick, revert.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_ops::facts::read_facts;
use graft_ops::operation::{
    FactsScope, Operation, PickParams, RebaseParams, Refusal, RevertParams, Source, Tip,
};
use graft_ops::preview::PreviewError;
use graft_ops::rewrite::{RewriteParams, RewriteRequest, TodoAction};

use super::{
    MINE, TempRepo, applied_words, assert_refused, attempt, id, ignore, main_and_other,
    main_and_other_unignored, main_tip, preview_words, sha7,
};
use crate::support::{Never, git_version};

// ── replays (28b) ───────────────────────────────────────────────────────

fn tip_of(repo: &TempRepo, branch: &str) -> Tip {
    Tip {
        name: format!("refs/heads/{branch}"),
        oid: id(repo, branch),
    }
}

/// `r` of the checked-out `feature` onto `main`.
fn rebase_onto_main(repo: &TempRepo) -> Operation {
    Operation::Rebase(RebaseParams::new(
        tip_of(repo, "feature"),
        Source::Branch("refs/heads/main".to_owned()),
        id(repo, "main"),
    ))
}

/// `main` has `a.txt`; `feature` (checked out) adds `f.txt` from there, and
/// `main` moves on by adding `main_adds`.
fn forked(main_adds: &str) -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("f.txt", b"f\n");
    repo.commit_all("f on feature");
    repo.git(&["switch", "-q", "main"]);
    repo.write(main_adds, b"theirs\n");
    repo.commit_all("main moves");
    repo.git(&["switch", "-q", "feature"]);
    repo
}

#[test]
fn rebase_over_an_ignored_file_is_refused() {
    let repo = forked("c.txt");
    ignore(&repo, "c.txt");
    assert_refused(
        &repo,
        &rebase_onto_main(&repo),
        "c.txt",
        "c.txt (ignored) is in the way of rebasing onto main · move it aside first",
    );
}

/// Commit A adds `c.txt` and B deletes it: neither end of the rebase has
/// it, but replaying A writes it.
#[test]
fn rebase_whose_middle_commit_writes_an_ignored_path_is_refused() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("A adds c");
    repo.git(&["rm", "-q", "c.txt"]);
    repo.git(&["commit", "-q", "-m", "B deletes c"]);
    repo.write("f.txt", b"f\n");
    repo.commit_all("C");
    repo.git(&["switch", "-q", "main"]);
    repo.write("m.txt", b"m\n");
    repo.commit_all("main moves");
    repo.git(&["switch", "-q", "feature"]);
    ignore(&repo, "c.txt");
    assert_refused(
        &repo,
        &rebase_onto_main(&repo),
        "c.txt",
        "c.txt (ignored) is in the way of rebasing onto main · move it aside first",
    );
}

/// `main` tracks `.env`; on `feature`, X takes it out of git (`rm
/// --cached`) and Y follows. `.env` is ignored and holds his bytes.
fn env_dropped() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.write(".env", b"committed secret\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.git(&["rm", "-q", "--cached", ".env"]);
    repo.git(&["commit", "-q", "-m", "X drops .env"]);
    repo.write(".git/info/exclude", b".env\n");
    repo.write(".env", MINE);
    repo.write("y.txt", b"y\n");
    repo.commit_all("Y");
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
    repo
}

fn rewrite_plan(repo: &TempRepo, req: &RewriteRequest) -> RewriteParams {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    graft_ops::rewrite::plan(&handle, &git_version(), req, &facts, None, &Never)
        .unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

#[test]
fn rewrite_over_an_ignored_file_is_refused() {
    let repo = env_dropped();
    let x = id(&repo, "feature~1");
    let mut params = rewrite_plan(&repo, &RewriteRequest::Open { from: x });
    assert_eq!(params.onto, id(&repo, "main"), "onto is below X");
    // A real change, so git can't skip past X without writing `onto`'s tree.
    let row = params
        .todo
        .iter()
        .position(|line| line.commit == x)
        .expect("X is a row");
    params.todo[row].action = TodoAction::Reword;
    params.messages.insert(row, "X, reworded\n".to_owned());
    assert_refused(
        &repo,
        &Operation::Rewrite(params),
        ".env",
        ".env (ignored) is in the way of rewriting feature · move it aside first",
    );
}

/// `a` into T, the first commit on `feature`: git skips T's unchanged pick,
/// so `onto`'s tree is never written; U, replayed above the fixup, adds
/// `c.txt` and V deletes it again.
#[test]
fn amend_into_over_an_ignored_file_is_refused() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("t.txt", b"t\n");
    repo.commit_all("T");
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("U adds c");
    repo.git(&["rm", "-q", "c.txt"]);
    repo.git(&["commit", "-q", "-m", "V deletes c"]);
    ignore(&repo, "c.txt");
    let t = id(&repo, "feature~2");
    repo.write("t.txt", b"t, fixed\n");
    repo.git(&["add", "t.txt"]);
    let params = rewrite_plan(&repo, &RewriteRequest::AmendInto { target: t });
    assert_refused(
        &repo,
        &Operation::Rewrite(params),
        "c.txt",
        "c.txt (ignored) is in the way of rewriting feature · move it aside first",
    );
}

/// Review 28 m6: absorb goes through `Rewrite` with `made` fixups. A
/// staged line of T's is absorbed into T; U, replayed above it, adds
/// `c.txt` and V deletes it again.
#[test]
fn absorb_over_an_ignored_file_is_refused() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("t.txt", b"t1\nt2\nt3\n");
    repo.commit_all("T");
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("U adds c");
    repo.git(&["rm", "-q", "c.txt"]);
    repo.git(&["commit", "-q", "-m", "V deletes c"]);
    ignore(&repo, "c.txt");
    repo.write("t.txt", b"t1\nt2, fixed\nt3\n");
    repo.git(&["add", "t.txt"]);
    let params = rewrite_plan(&repo, &RewriteRequest::Absorb);
    assert_eq!(params.made.len(), 1, "one hunk placed, into T");
    assert_refused(
        &repo,
        &Operation::Rewrite(params),
        "c.txt",
        "c.txt (ignored) is in the way of rewriting feature · move it aside first",
    );
}

/// Not refused, run, and his bytes at `path` still `MINE`. The preview of
/// a rewrite draws nothing (`NotPreviewable`); what matters is that it is
/// not refused as in the way.
fn assert_runs(repo: &TempRepo, op: &Operation, path: &str) {
    let outcome = attempt(repo, op, path);
    assert!(
        !matches!(
            outcome.preview,
            Err(PreviewError::Refused(Refusal::InTheWay { .. }))
        ),
        "the preview refused: {}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, MINE, "his bytes at {path}");
}

/// Plain `git rebase -i`, the todo edited by `sed -e <script>`, git's
/// editor answering with the message as it is.
fn plain_rebase(repo: &TempRepo, script: &str, onto: &str) {
    let editor = format!("sequence.editor=sed -i.bak -e {script}");
    repo.git(&[
        "-c",
        &editor,
        "-c",
        "core.editor=true",
        "rebase",
        "-q",
        "-i",
        "--autosquash",
        onto,
    ]);
}

/// Review 28 m1: `R` from X with only Y reworded. git skips X's unchanged
/// pick (its parent is `onto`), so `onto`'s tree — which tracks `.env` —
/// is never written. Plain git leaves `.env` alone; so must Graft.
#[test]
fn rewrite_that_leaves_the_untracking_commit_unchanged_runs() {
    let plain = env_dropped();
    plain_rebase(&plain, "2s/^pick/reword/", "main");
    assert_eq!(std::fs::read(plain.path().join(".env")).unwrap(), MINE);

    let repo = env_dropped();
    let x = id(&repo, "feature~1");
    let y = id(&repo, "feature");
    let mut params = rewrite_plan(&repo, &RewriteRequest::Open { from: x });
    let row = params
        .todo
        .iter()
        .position(|line| line.commit == y)
        .expect("Y is a row");
    params.todo[row].action = TodoAction::Reword;
    params.messages.insert(row, "Y, reworded\n".to_owned());
    assert_runs(&repo, &Operation::Rewrite(params), ".env");
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "Y, reworded");
}

/// Review 28 m1: `a` into X, the first commit on `feature`: git skips X's
/// pick, stops skipping at the fixup, and replays from X — never writing
/// `onto`'s `.env`. Plain git's `--autosquash` does the same.
#[test]
fn amend_into_the_untracking_commit_runs() {
    let plain = env_dropped();
    plain.write("a.txt", b"a, fixed\n");
    plain.git(&["add", "a.txt"]);
    plain.git(&["commit", "-q", "--fixup", "feature~1"]);
    plain_rebase(&plain, "s/^x/x/", "main");
    assert_eq!(std::fs::read(plain.path().join(".env")).unwrap(), MINE);

    let repo = env_dropped();
    let x = id(&repo, "feature~1");
    repo.write("a.txt", b"a, fixed\n");
    repo.git(&["add", "a.txt"]);
    let params = rewrite_plan(&repo, &RewriteRequest::AmendInto { target: x });
    assert_runs(&repo, &Operation::Rewrite(params), ".env");
    assert_eq!(repo.git(&["show", "feature~1:a.txt"]), "a, fixed");
}

fn pick(repo: &TempRepo, revs: &[&str]) -> Operation {
    Operation::CherryPick(PickParams {
        commits: revs.iter().map(|rev| id(repo, rev)).collect(),
        subject: repo.git(&["log", "-1", "--format=%s", revs[0]]),
        head: main_tip(repo),
    })
}

#[test]
fn cherry_pick_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let other = id(&repo, "other");
    assert_refused(
        &repo,
        &pick(&repo, &["other"]),
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of picking {} · move it aside first",
            sha7(other)
        ),
    );

    let repo = main_and_other_unignored();
    repo.git(&["switch", "-q", "other"]);
    repo.write("d.txt", b"d\n");
    repo.commit_all("three on other");
    repo.git(&["switch", "-q", "main"]);
    ignore(&repo, "c.txt");
    assert_refused(
        &repo,
        &pick(&repo, &["other~1", "other"]),
        "c.txt",
        "c.txt (ignored) is in the way of picking 2 commits · move it aside first",
    );
}

/// The scout's probe: reverting a delete puts the file back.
#[test]
fn revert_of_a_delete_over_an_ignored_file_is_refused() {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("adds c");
    repo.git(&["rm", "-q", "c.txt"]);
    repo.git(&["commit", "-q", "-m", "deletes c"]);
    ignore(&repo, "c.txt");
    let head = id(&repo, "HEAD");
    let op = Operation::Revert(RevertParams {
        commit: head,
        subject: "deletes c".to_owned(),
        mainline: None,
        parents: 1,
        parent: Some(id(&repo, "HEAD^")),
        head: main_tip(&repo),
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of reverting {} · move it aside first",
            sha7(head)
        ),
    );
}

/// No false refusal: an ignored `target/` that no replayed commit writes.
#[test]
fn replay_with_ignored_build_output_elsewhere_runs() {
    let repo = forked("m.txt");
    repo.write(".git/info/exclude", b"target/\n");
    repo.write("target/out.bin", MINE);
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
    let outcome = attempt(&repo, &rebase_onto_main(&repo), "target/out.bin");
    assert!(
        outcome.preview.is_ok(),
        "{}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, MINE, "his build output");
    assert_eq!(id(&repo, "feature^"), id(&repo, "main"), "feature rebased");
}

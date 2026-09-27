//! The chain preview (m7/05, note R3): one plan names every branch that
//! moves, forecasts the replay step by step with each result fed into the
//! next, suggests dropping a squash-merged bottom branch, and names the
//! branches whose upstream holds a commit it rewrites.
//!
//! The forecast's truth is git's own: `the_chained_forecast_agrees_with_git`
//! runs the same operation in a copy and compares where git stops.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use graft_core::journal::{EntryStatus, HeadRef};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::version::GitVersion;
use graft_ops::apply::ApplyError;
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    CheckoutParams, CheckoutTarget, FactsScope, FastForward, MergeParams, Operation, PickParams,
    RebaseParams, Refusal, Source, Tip,
};
use graft_ops::preview::{
    ConflictForecast, DropSuggestion, PreviewError, PreviewPlan, Target, plan, plan_as_run,
    suggest_drops,
};
use graft_ops::rewrite::{BranchLine, RewriteRequest, TodoAction, progress};

use crate::support::{NOW, Never, TempRepo, git_version, try_run};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

fn tip(repo: &TempRepo, branch: &str) -> Tip {
    Tip {
        name: format!("refs/heads/{branch}"),
        oid: id(repo, branch),
    }
}

/// One commit changing its own file, so no two commits conflict.
fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

fn write_commit(repo: &TempRepo, file: &str, bytes: &[u8], message: &str) {
    repo.write(file, bytes);
    repo.commit_all(message);
}

/// `r` of `branch` onto `main`, his branch alone.
fn rebase(repo: &TempRepo, branch: &str) -> Operation {
    Operation::Rebase(RebaseParams::new(
        tip(repo, branch),
        Source::Branch("refs/heads/main".to_owned()),
        id(repo, "main"),
    ))
}

fn plan_with(
    repo: &TempRepo,
    op: &Operation,
    git: &GitVersion,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let handle = repo.open();
    let facts = read_facts(&handle, *git, &Never, &op.facts_scope()).expect("the facts read");
    plan(&handle, git, op, &facts, stop)
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    plan_with(repo, op, &git_version(), &Never).unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

/// `R`'s plan of `feature-a` from its first commit, based on `main`'s tip:
/// the whole line replayed, what `r` runs once it carries the line.
fn line_onto_main(
    repo: &TempRepo,
    edit: impl Fn(&mut Vec<graft_ops::rewrite::TodoLine>),
) -> Operation {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let first = repo.git(&["rev-list", "--reverse", "main..feature-a"]);
    let first = first.lines().next().expect("feature-a has commits");
    let mut params = graft_ops::rewrite::plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open {
            from: Oid::from_hex(first).expect("an id"),
        },
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why:?}"));
    params.onto = id(repo, "main");
    edit(&mut params.todo);
    Operation::Rewrite(params)
}

/// A line of three: `feature-a` (two commits) off `main`'s first commit,
/// `feature-b` one on it, `feature-c` one on that; `main` then moves on.
/// `HEAD` on `feature-a`.
fn line() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "m2");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

const SIX: &[u8] = b"one\ntwo\nthree\nfour\nfive\nsix\n";

/// `feature`: renames `f` to `g`, edits `g`'s first line, adds `x`. `main`
/// edits `f`'s first line. Replayed one by one onto `main`, step 2 stops in
/// `g` — a file that exists only once step 1 is applied. Each commit
/// against `main` alone names `f` instead (verified by hand, git 2.54).
fn renamed_then_edited() -> TempRepo {
    let repo = TempRepo::init();
    write_commit(&repo, "f", SIX, "m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.git(&["mv", "f", "g"]);
    repo.commit_all("rename f to g");
    write_commit(
        &repo,
        "g",
        b"ONE-feature\ntwo\nthree\nfour\nfive\nsix\n",
        "edit g",
    );
    commit(&repo, "x");
    repo.git(&["switch", "-q", "main"]);
    write_commit(
        &repo,
        "f",
        b"ONE-main\ntwo\nthree\nfour\nfive\nsix\n",
        "main edit",
    );
    repo.git(&["switch", "-q", "feature"]);
    repo
}

/// `feature` creates `h`, then edits it; `main` moves on elsewhere. Against
/// `main` alone the edit is a modify/delete of `h`; chained, it is clean.
fn created_then_edited() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    write_commit(&repo, "h", b"1\n", "create h");
    write_commit(&repo, "h", b"2\n", "edit h");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "m2");
    repo.git(&["switch", "-q", "feature"]);
    repo
}

/// Five commits on `feature`; `main` changes `s` as `feature`'s commits
/// numbered in `conflicting` do (1-based), each its own way.
fn stops_at(conflicting: &[usize]) -> TempRepo {
    let repo = TempRepo::init();
    write_commit(&repo, "s", b"base\n", "m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for n in 1..=5 {
        if conflicting.contains(&n) {
            write_commit(
                &repo,
                "s",
                format!("feature {n}\n").as_bytes(),
                &format!("f{n}"),
            );
        } else {
            commit(&repo, &format!("f{n}"));
        }
    }
    repo.git(&["switch", "-q", "main"]);
    write_commit(&repo, "s", b"main\n", "m2");
    repo.git(&["switch", "-q", "feature"]);
    repo
}

// ── the chained forecast ────────────────────────────────────────────────

#[test]
fn the_forecast_chains_through_earlier_picks() {
    let repo = renamed_then_edited();
    let drawn = plan_of(&repo, &rebase(&repo, "feature"));
    assert_eq!(
        drawn.conflicts,
        ConflictForecast::StopsAt {
            step: 2,
            of: 3,
            commit: id(&repo, "feature~1"),
            paths: vec![PathBuf::from("g")],
        }
    );

    let repo = created_then_edited();
    let drawn = plan_of(&repo, &rebase(&repo, "feature"));
    assert_eq!(drawn.conflicts, ConflictForecast::Clean);
}

#[test]
fn the_forecast_stops_at_the_first_conflict() {
    let repo = stops_at(&[3, 5]);
    let drawn = plan_of(&repo, &rebase(&repo, "feature"));
    assert_eq!(
        drawn.conflicts,
        ConflictForecast::StopsAt {
            step: 3,
            of: 5,
            commit: id(&repo, "feature~2"),
            paths: vec![PathBuf::from("s")],
        }
    );
}

#[test]
fn an_old_git_leaves_it_not_checked() {
    let repo = stops_at(&[3]);
    let old = GitVersion {
        major: 2,
        minor: 39,
        patch: 0,
    };
    let drawn = plan_with(&repo, &rebase(&repo, "feature"), &old, &Never).expect("it plans");
    assert!(
        matches!(drawn.conflicts, ConflictForecast::NotChecked(_)),
        "{:?}",
        drawn.conflicts
    );
}

/// Where a real run of `op` in a copy stops: git's step and the paths, or
/// `None` when it finishes.
fn where_git_stops(repo: &TempRepo, op: &Operation) -> Option<(usize, usize, Vec<PathBuf>)> {
    let copy = repo.copy();
    match try_run(&copy, op, NOW) {
        Ok(_) => None,
        Err(ApplyError::Conflicted { mut paths, .. }) => {
            let at = progress(&copy.open(), &Never)
                .expect("progress reads")
                .expect("a rebase is under way");
            paths.sort();
            Some((at.step.0, at.step.1, paths))
        }
        Err(other) => panic!("the run failed: {}", other.explain()),
    }
}

#[test]
fn the_chained_forecast_agrees_with_git() {
    let rows = [
        ("clean", created_then_edited()),
        ("a stop at step 1", stops_at(&[1])),
        ("a stop at step 3", stops_at(&[3, 4])),
        ("a stop only chained", renamed_then_edited()),
    ];
    for (row, repo) in rows {
        let op = rebase(&repo, "feature");
        let forecast = match plan_of(&repo, &op).conflicts {
            ConflictForecast::Clean => None,
            ConflictForecast::StopsAt {
                step,
                of,
                mut paths,
                ..
            } => {
                paths.sort();
                Some((step, of, paths))
            }
            other => panic!("{row}: {other:?}"),
        };
        assert_eq!(forecast, where_git_stops(&repo, &op), "{row}");
    }
    line_rows_agree_with_git();
}

// ── the line ────────────────────────────────────────────────────────────

#[test]
fn a_three_branch_line_moves_three_refs() {
    let repo = line();
    let op = line_onto_main(&repo, |_| {});
    let drawn = plan_of(&repo, &op);
    let moved: Vec<(&str, Option<Oid>)> = drawn
        .ref_moves
        .iter()
        .map(|moved| (moved.name.as_str(), moved.from))
        .collect();
    assert_eq!(
        moved,
        [
            ("refs/heads/feature-a", Some(id(&repo, "feature-a"))),
            ("refs/heads/feature-b", Some(id(&repo, "feature-b"))),
            ("refs/heads/feature-c", Some(id(&repo, "feature-c"))),
        ]
    );
    // Each lands on its own new tip: a2's, b1's and c1's ghosts, in a chain
    // on `main`.
    let landed: Vec<String> = drawn
        .ref_moves
        .iter()
        .map(|moved| match moved.to {
            Some(Target::Ghost(at)) => drawn.ghosts[at].subject.clone(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(landed, ["a2", "b1", "c1"]);
    assert_eq!(drawn.ghosts.len(), 4);
    assert_eq!(drawn.ghosts[0].parents, [Target::Real(id(&repo, "main"))]);
    assert_eq!(
        drawn.head,
        Some(graft_ops::preview::HeadMove::ToBranch(
            "refs/heads/feature-a".to_owned()
        ))
    );
    assert_eq!(drawn.conflicts, ConflictForecast::Clean);
}

#[test]
fn squash_message_is_in_the_plan() {
    let repo = line();
    // a2 squashed into a1: one commit for feature-a, a message Graft writes.
    let op = line_onto_main(&repo, |todo| todo[1].action = TodoAction::Squash);
    let drawn = plan_of(&repo, &op);
    let first_moved = &drawn.ref_moves[0];
    let Some(Target::Ghost(at)) = first_moved.to else {
        panic!("{first_moved:?}");
    };
    assert_eq!(drawn.ghosts[at].subject, "a1");
    assert_eq!(
        drawn.messages,
        [(Target::Ghost(at), "a1\n\na2\n".to_owned())],
        "the kept row's message, a blank line, the melded one's"
    );
    assert_eq!(drawn.ghosts.len(), 3, "a1+a2, b1, c1");
}

#[test]
fn published_names_branches_on_their_upstream() {
    let repo = line();
    // origin/feature-b holds b1; feature-a's and feature-c's upstreams hold
    // nothing the rewrite replays (feature-a's is main's first commit).
    repo.git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/graft.git",
    ]);
    for (branch, at) in [
        ("feature-a", "main~1"),
        ("feature-b", "feature-b"),
        ("feature-c", "main"),
    ] {
        repo.git(&["update-ref", &format!("refs/remotes/origin/{branch}"), at]);
        repo.git(&["config", &format!("branch.{branch}.remote"), "origin"]);
        repo.git(&[
            "config",
            &format!("branch.{branch}.merge"),
            &format!("refs/heads/{branch}"),
        ]);
    }
    let drawn = plan_of(&repo, &line_onto_main(&repo, |_| {}));
    let named: Vec<String> = drawn
        .published
        .iter()
        .map(|name| String::from_utf8_lossy(name.as_bytes()).into_owned())
        .collect();
    assert_eq!(named, ["refs/heads/feature-b"]);

    let plain = plan_of(&repo, &rebase(&repo, "feature-a"));
    assert!(plain.published.is_empty(), "{:?}", plain.published);
}

// ── the squash-merge suggestion ─────────────────────────────────────────

/// `feature-a`'s three commits squash-merged into `main` as one commit
/// (`Add OAuth (#42)`), then `main` moves on; `feature-b` built on it.
fn squash_merged() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    commit(&repo, "a3");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "main"]);
    commit(&repo, "m2");
    repo.git(&["merge", "-q", "--squash", "feature-a"]);
    repo.git(&["commit", "-q", "-m", "Add OAuth (#42)"]);
    commit(&repo, "m3");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

fn line_of(repo: &TempRepo, names: &[&str]) -> BranchLine {
    BranchLine {
        branches: names.iter().map(|name| tip(repo, name)).collect(),
    }
}

#[test]
fn a_squash_merged_bottom_branch_is_suggested_dropped() {
    let repo = squash_merged();
    let found = suggest_drops(
        &repo.open(),
        &git_version(),
        &line_of(&repo, &["feature-a", "feature-b"]),
        id(&repo, "main"),
        &Never,
    )
    .expect("it reads");
    let Some(DropSuggestion::Drop(found)) = found else {
        panic!("{found:?}");
    };
    assert_eq!(found.branch.as_bytes(), b"refs/heads/feature-a");
    assert_eq!(
        found.commits,
        [
            id(&repo, "feature-a~2"),
            id(&repo, "feature-a~1"),
            id(&repo, "feature-a")
        ]
    );
    assert_eq!(found.matches, id(&repo, "main~1"));
    assert_eq!(found.subject, "Add OAuth (#42)");
}

#[test]
fn no_match_suggests_nothing() {
    let repo = line();
    let found = suggest_drops(
        &repo.open(),
        &git_version(),
        &line_of(&repo, &["feature-a", "feature-b"]),
        id(&repo, "main"),
        &Never,
    )
    .expect("it reads");
    assert_eq!(found, None);
}

#[test]
fn past_the_cap_is_not_checked() {
    let repo = squash_merged();
    // 200 more commits on main since feature-a left it: 203 in all.
    let mut parent = id(&repo, "main");
    let tree = repo.git(&["rev-parse", "main^{tree}"]);
    for n in 0..200 {
        let made = repo.git(&[
            "commit-tree",
            &tree,
            "-p",
            &parent.to_string(),
            "-m",
            &format!("empty {n}"),
        ]);
        parent = Oid::from_hex(&made).expect("an id");
    }
    repo.git(&["update-ref", "refs/heads/main", &parent.to_string()]);
    let found = suggest_drops(
        &repo.open(),
        &git_version(),
        &line_of(&repo, &["feature-a"]),
        id(&repo, "main"),
        &Never,
    )
    .expect("it reads");
    assert!(
        matches!(found, Some(DropSuggestion::NotChecked(_))),
        "{found:?}"
    );
}

// ── reads only, and a stop ──────────────────────────────────────────────

/// The index's bytes, every ref, and `status`.
fn written(repo: &TempRepo) -> (Vec<u8>, String, String) {
    (
        std::fs::read(repo.path().join(".git/index")).expect("the index reads"),
        repo.git(&["for-each-ref"]),
        // `--no-optional-locks`: a plain status refreshes `.git/index`.
        repo.git(&["--no-optional-locks", "status", "--porcelain=v2"]),
    )
}

#[test]
fn the_preview_writes_no_ref_index_or_file() {
    let repo = line();
    repo.write("a1.txt", b"an unsaved edit\n");
    let before = written(&repo);
    let _: PreviewPlan = plan_of(&repo, &line_onto_main(&repo, |_| {}));
    let _: PreviewPlan = plan_of(&repo, &rebase(&repo, "feature-a"));
    assert_eq!(written(&repo), before);

    let repo = renamed_then_edited();
    let before = written(&repo);
    let _: PreviewPlan = plan_of(&repo, &rebase(&repo, "feature"));
    assert_eq!(written(&repo), before);
}

/// A `Stop` that lets the first `n` polls through, then stops.
struct StopAfter {
    left: AtomicUsize,
}

impl Stop for StopAfter {
    fn should_stop(&self) -> bool {
        self.left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_err()
    }
}

#[test]
fn the_preview_honours_stop() {
    let repo = stops_at(&[5]);
    let op = rebase(&repo, "feature");
    let mut polls = 1;
    let drawn = loop {
        let stop = StopAfter {
            left: AtomicUsize::new(polls),
        };
        match plan_with(&repo, &op, &git_version(), &stop) {
            Err(PreviewError::Stopped) => polls += 1,
            Ok(drawn) => break drawn,
            Err(other) => panic!("after {polls} polls: {other:?}"),
        }
        assert!(polls < 400, "the plan never completed");
    };
    assert!(polls > 5, "the chain polls between its spawns: {polls}");
    assert!(
        matches!(drawn.conflicts, ConflictForecast::StopsAt { step: 5, .. }),
        "{:?}",
        drawn.conflicts
    );
}

// ── `r` as it runs: the line and the drops (m7/05b) ─────────────────────

/// `plan_as_run` over `op`, with the facts `op` names read fresh.
fn as_run(repo: &TempRepo, op: Operation) -> Result<(Operation, PreviewPlan), PreviewError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    plan_as_run(&handle, &git_version(), op, &facts, &Never)
}

fn completed(repo: &TempRepo, op: Operation) -> (Operation, PreviewPlan) {
    as_run(repo, op).unwrap_or_else(|why| panic!("it plans: {why:?}"))
}

fn rebase_params(op: &Operation) -> &RebaseParams {
    match op {
        Operation::Rebase(params) => params,
        other => panic!("not a rebase: {other:?}"),
    }
}

fn names(line: &BranchLine) -> Vec<&str> {
    line.branches.iter().map(|tip| tip.name.as_str()).collect()
}

/// `line()`, but `experiment` also built on `feature-a`: a fork.
fn forked() -> TempRepo {
    let repo = line();
    repo.git(&["switch", "-q", "-c", "experiment", "feature-a"]);
    commit(&repo, "e1");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

#[test]
fn r_plans_every_branch_of_its_line() {
    let repo = line();
    let (op, drawn) = completed(&repo, rebase(&repo, "feature-a"));

    let params = rebase_params(&op);
    assert_eq!(
        names(&params.line),
        [
            "refs/heads/feature-a",
            "refs/heads/feature-b",
            "refs/heads/feature-c"
        ]
    );
    assert_eq!(params.drops, Vec::<Oid>::new());
    let moved: Vec<(&str, Option<Oid>, String)> = drawn
        .ref_moves
        .iter()
        .map(|moved| {
            let to = match moved.to {
                Some(Target::Ghost(at)) => drawn.ghosts[at].subject.clone(),
                other => panic!("{other:?}"),
            };
            (moved.name.as_str(), moved.from, to)
        })
        .collect();
    assert_eq!(
        moved,
        [
            (
                "refs/heads/feature-a",
                Some(id(&repo, "feature-a")),
                "a2".to_owned()
            ),
            (
                "refs/heads/feature-b",
                Some(id(&repo, "feature-b")),
                "b1".to_owned()
            ),
            (
                "refs/heads/feature-c",
                Some(id(&repo, "feature-c")),
                "c1".to_owned()
            ),
        ],
        "one move per branch, bottom first"
    );
    let subjects: Vec<&str> = drawn.ghosts.iter().map(|g| g.subject.as_str()).collect();
    assert_eq!(subjects, ["a1", "a2", "b1", "c1"], "a ghost for every pick");
    assert_eq!(drawn.ghosts[0].parents, [Target::Real(id(&repo, "main"))]);
    for (at, ghost) in drawn.ghosts.iter().enumerate().skip(1) {
        assert_eq!(ghost.parents, [Target::Ghost(at - 1)], "one chain");
    }
    assert_eq!(drawn.conflicts, ConflictForecast::Clean);
    assert_eq!(drawn.against, op.tips(), "drawn against every branch");
}

#[test]
fn r_over_a_fork_is_refused_before_y() {
    let repo = forked();
    match as_run(&repo, rebase(&repo, "feature-a")) {
        Err(PreviewError::Refused(Refusal::ForkedDependents { a, b })) => {
            assert_eq!(
                (a.as_bytes(), b.as_bytes()),
                (&b"refs/heads/experiment"[..], &b"refs/heads/feature-b"[..])
            );
        }
        other => panic!("expected the fork refused: {other:?}"),
    }
}

#[test]
fn r_with_a_dependent_elsewhere_is_refused() {
    let repo = line();
    repo.git(&["worktree", "add", "-q", "linked", "feature-c"]);
    match as_run(&repo, rebase(&repo, "feature-a")) {
        Err(PreviewError::Refused(Refusal::DependentElsewhere { branch, .. })) => {
            assert_eq!(branch.as_bytes(), b"refs/heads/feature-c");
        }
        other => panic!("expected the worktree refused: {other:?}"),
    }
}

#[test]
fn a_squash_merged_bottom_fills_drops_and_already_in() {
    let repo = squash_merged();
    let (op, drawn) = completed(&repo, rebase(&repo, "feature-a"));

    let params = rebase_params(&op);
    let Some(DropSuggestion::Drop(already_in)) = &drawn.already_in else {
        panic!("{:?}", drawn.already_in);
    };
    assert_eq!(params.drops, already_in.commits);
    assert_eq!(params.drops.len(), 3);
    assert!(
        drawn
            .ghosts
            .iter()
            .all(|ghost| !ghost.replays.is_some_and(|it| params.drops.contains(&it))),
        "no ghost replays a dropped commit: {:?}",
        drawn.ghosts
    );
    let subjects: Vec<&str> = drawn.ghosts.iter().map(|g| g.subject.as_str()).collect();
    assert_eq!(subjects, ["b1"]);
    assert_eq!(
        drawn.ref_moves.first().map(|moved| moved.to),
        Some(Some(Target::Real(id(&repo, "main")))),
        "feature-a, all of it in main, lands on main"
    );
}

/// `squash_merged()`, and `main` then rewrites `a1.txt`: replaying `a1`
/// would stop on it, dropping it does not.
fn squash_merged_then_edited() -> TempRepo {
    let repo = squash_merged();
    repo.git(&["switch", "-q", "main"]);
    write_commit(&repo, "a1.txt", b"main's own a1\n", "m4");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

#[test]
fn the_chained_forecast_skips_drops() {
    let repo = squash_merged_then_edited();
    // The same rebase, nothing dropped, stops on a1: the fixture bites.
    let unfilled = plan_of(&repo, &rebase(&repo, "feature-a"));
    assert!(
        matches!(
            unfilled.conflicts,
            ConflictForecast::StopsAt { step: 1, .. }
        ),
        "{:?}",
        unfilled.conflicts
    );

    let (_, drawn) = completed(&repo, rebase(&repo, "feature-a"));
    assert_eq!(drawn.conflicts, ConflictForecast::Clean);
}

#[test]
fn plan_as_run_leaves_every_other_kind_alone() {
    let repo = line();
    let head = tip(&repo, "feature-a");
    let ops = [
        Operation::Merge(MergeParams {
            source: Source::Branch("refs/heads/main".to_owned()),
            into: head.clone(),
            source_tip: id(&repo, "main"),
            ff: FastForward::Default,
        }),
        Operation::CherryPick(PickParams {
            commits: vec![id(&repo, "main")],
            subject: "m2".to_owned(),
            head: head.clone(),
        }),
        Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Branch("refs/heads/main".to_owned()),
            head_before: HeadRef::Branch("refs/heads/feature-a".to_owned()),
        }),
    ];
    for op in ops {
        let (back, drawn) = completed(&repo, op.clone());
        assert_eq!(back, op);
        assert_eq!(drawn, plan_of(&repo, &op));
    }
}

#[test]
fn a_line_whose_every_row_is_dropped() {
    // git 2.54, a todo of `drop` rows only: "Successfully rebased and
    // updated refs/heads/feature-a", the branch on `onto` (05b NOTES).
    let repo = squash_merged();
    repo.git(&["branch", "-q", "-D", "feature-b"]);
    let (op, drawn) = completed(&repo, rebase(&repo, "feature-a"));
    assert_eq!(rebase_params(&op).drops.len(), 3);
    assert_eq!(drawn.ghosts, []);
    assert_eq!(
        drawn.ref_moves.first().map(|moved| moved.to),
        Some(Some(Target::Real(id(&repo, "main"))))
    );

    let applied = try_run(&repo, &op, NOW).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        id(&repo, "feature-a"),
        id(&repo, "main"),
        "all of it is in main"
    );
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature-a");
    assert_eq!(applied.entry.status, EntryStatus::Committed);
}

/// Every branch `drawn` moves lands as it was drawn: each ref's commit
/// chain, down to the real commit the ghosts stand on, has the ghosts'
/// subjects in order, and a `Real` target is that commit.
fn lands_as_drawn(repo: &TempRepo, drawn: &PreviewPlan) {
    for moved in &drawn.ref_moves {
        let mut at = moved.to;
        let mut commit = id(repo, &moved.name);
        loop {
            match at {
                Some(Target::Real(real)) => {
                    assert_eq!(commit, real, "{} lands on its real commit", moved.name);
                    break;
                }
                Some(Target::Ghost(index)) => {
                    let ghost = &drawn.ghosts[index];
                    let subject = repo.git(&["log", "-1", "--format=%s", &commit.to_string()]);
                    assert_eq!(subject, ghost.subject, "{}: ghost {index}", moved.name);
                    at = ghost.parents.first().copied();
                    commit = id(repo, &format!("{commit}^"));
                }
                None => panic!("{}: deleted", moved.name),
            }
        }
    }
}

/// `line()`, but `feature-b`'s commit and `main`'s second one both change
/// `shared.txt`: `r` of `feature-a` onto `main` stops on `b1`, step 3 of 4.
fn conflicted_line() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    repo.write("shared.txt", b"b\n");
    repo.commit_all("b1");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "main"]);
    repo.write("shared.txt", b"m\n");
    repo.commit_all("m2");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

/// The oracle's two line rows (05b): `plan_as_run`, then a real `y` in a
/// copy. A stop is where git stops; a clean run lands every branch where
/// its ghosts were drawn.
fn line_rows_agree_with_git() {
    let rows = [
        ("a stop inside a dependent's commits", conflicted_line()),
        ("a clean line with drops", squash_merged()),
    ];
    for (row, repo) in rows {
        let (op, drawn) = completed(&repo, rebase(&repo, "feature-a"));
        let forecast = match drawn.conflicts.clone() {
            ConflictForecast::Clean => None,
            ConflictForecast::StopsAt {
                step,
                of,
                mut paths,
                ..
            } => {
                paths.sort();
                Some((step, of, paths))
            }
            other => panic!("{row}: {other:?}"),
        };
        let copy = repo.copy();
        let git = match try_run(&copy, &op, NOW) {
            Ok(_) => {
                lands_as_drawn(&copy, &drawn);
                None
            }
            Err(ApplyError::Conflicted { mut paths, .. }) => {
                let at = progress(&copy.open(), &Never)
                    .expect("progress reads")
                    .expect("a rebase is under way");
                paths.sort();
                Some((at.step.0, at.step.1, paths))
            }
            Err(other) => panic!("{row}: the run failed: {}", other.explain()),
        };
        assert_eq!(forecast, git, "{row}");
    }
    // The stop row does stop, inside feature-b's pick; the drop row is clean.
    let repo = conflicted_line();
    let (_, drawn) = completed(&repo, rebase(&repo, "feature-a"));
    assert!(
        matches!(
            drawn.conflicts,
            ConflictForecast::StopsAt { step: 3, of: 4, .. }
        ),
        "{:?}",
        drawn.conflicts
    );
}

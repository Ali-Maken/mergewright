//! The history operations, the conflict span and Ghost Preview, against
//! real git.
//!
//! Every claim is read back out of the repository — `rev-parse`,
//! `for-each-ref`, `ls-files`, the files on disk — never from the value a
//! function returned (PROMPT.md §7.2 item 4, the M3 group-03 standard). The
//! ghost tests are the sharpest of these: the plan is drawn *first*, the
//! operation then runs, and what git actually did is compared with what the
//! picture promised.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_fixtures::remote::LocalRemote;
use graft_git::repo::RepoHandle;
use graft_git::state::InProgress;
use graft_ops::apply::{Applied, ApplyError, Refresh, Resume, apply, recover_interrupted, resume};
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    Invocation, Lease, MergeParams, Operation, PickParams, PushParams, RebaseParams,
    RefDeleteParams, Refusal, ResetMode, ResetParams, ResetTarget, RevertParams, SetAside, Source,
    StashParams, StashScope, TagCreateParams, Tip,
};
use graft_ops::preview;
use graft_ops::preview::{
    ConflictForecast, HeadMove, PreviewPlan, Target as GhostTarget, UndoNote, WorktreeEffect,
};
use graft_ops::undo::{self, UndoError};

use crate::support::{
    NOW, Never, RepoState, TempRepo, context, git_version, journal_for, round_trip, run, run_redo,
    run_undo, state_of, try_run,
};

// ── building repositories ───────────────────────────────────────────────

/// `main` with two commits, and `feature` branched off the first with one
/// commit of its own, touching a different file: the two diverge and merge
/// cleanly.
fn diverged() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// The same shape, but both branches rewrite the same line of the same
/// file: every merge, rebase or pick between them conflicts.
fn conflicting() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("shared.txt", b"main's line\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("shared.txt", b"feature's line\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// `main` two commits ahead of `feature`, with nothing on `feature` of its
/// own: a merge of `feature` into `main` is already up to date, and a merge
/// the other way fast-forwards.
fn ahead() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo
}

// ── reading the repository ──────────────────────────────────────────────

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn tip(repo: &TempRepo, full_ref: &str) -> Tip {
    Tip {
        name: full_ref.to_owned(),
        oid: id(repo, full_ref),
    }
}

fn head_tip(repo: &TempRepo) -> Tip {
    let branch = repo.git(&["symbolic-ref", "HEAD"]);
    tip(repo, &branch)
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

fn facts_for(repo: &RepoHandle, op: &Operation) -> graft_ops::operation::RepoFacts {
    read_facts(repo, git_version(), &Never, &op.facts_scope()).expect("the facts read")
}

/// Draw the plan for an operation, from a fresh read of the repository.
fn preview_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts = facts_for(&handle, op);
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("preview: {}", why.explain()))
}

fn try_preview(repo: &TempRepo, op: &Operation) -> Result<PreviewPlan, preview::PreviewError> {
    let handle = repo.open();
    let facts = facts_for(&handle, op);
    preview::plan(&handle, &git_version(), op, &facts, &Never)
}

fn run_resume(repo: &TempRepo, how: Resume, at: i64) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    resume(&mut ctx, how)
}

// ── building operations ─────────────────────────────────────────────────

fn merge_op(repo: &TempRepo, source: &str, ff: FastForward) -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch(source.to_owned()),
        into: head_tip(repo),
        source_tip: id(repo, source),
        ff,
    })
}

fn rebase_op(repo: &TempRepo, onto: &str) -> Operation {
    Operation::Rebase(RebaseParams {
        branch: head_tip(repo),
        onto: Source::Branch(onto.to_owned()),
        onto_tip: id(repo, onto),
    })
}

fn pick_op(repo: &TempRepo, rev: &str) -> Operation {
    Operation::CherryPick(PickParams {
        commits: vec![id(repo, rev)],
        subject: repo.git(&["log", "-1", "--format=%s", rev]),
        head: head_tip(repo),
    })
}

fn revert_op(repo: &TempRepo, rev: &str) -> Operation {
    Operation::Revert(RevertParams {
        commit: id(repo, rev),
        subject: repo.git(&["log", "-1", "--format=%s", rev]),
        mainline: None,
        parents: 1,
        parent: Some(id(repo, &format!("{rev}^"))),
        head: head_tip(repo),
    })
}

fn reset_op(repo: &TempRepo, to: &str, mode: ResetMode) -> Operation {
    Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: head_tip(repo),
            mode,
        },
        to: id(repo, to),
    })
}

// ── apply / undo / redo, per kind ───────────────────────────────────────

#[test]
fn merge_applies_undoes_redoes() {
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    round_trip(&repo, &op);
    // And the merge itself did what it said: after re-applying, `main` is a
    // merge commit with both tips as parents.
    run(&repo, &op, NOW + 10);
    assert_eq!(
        repo.git(&["rev-list", "--count", "--merges", "HEAD", "-1"]),
        "1",
        "the tip is a merge commit"
    );
    assert_eq!(entries(&repo).last().unwrap().kind, OperationKind::Merge);
}

#[test]
fn rebase_applies_undoes_redoes() {
    let repo = diverged();
    repo.git(&["switch", "-q", "feature"]);
    round_trip(&repo, &rebase_op(&repo, "refs/heads/main"));
    // Re-applied for real: `main` is now an ancestor of `feature`.
    run(&repo, &rebase_op(&repo, "refs/heads/main"), NOW + 10);
    repo.git(&["merge-base", "--is-ancestor", "main", "feature"]);
    assert_eq!(entries(&repo).last().unwrap().kind, OperationKind::Rebase);
}

#[test]
fn cherry_pick_applies_undoes_redoes() {
    let repo = diverged();
    round_trip(&repo, &pick_op(&repo, "refs/heads/feature"));
    run(&repo, &pick_op(&repo, "refs/heads/feature"), NOW + 10);
    assert_eq!(
        repo.read("feature.txt"),
        b"feature\n".to_vec(),
        "the picked change landed on disk"
    );
}

#[test]
fn revert_applies_undoes_redoes() {
    let repo = diverged();
    round_trip(&repo, &revert_op(&repo, "HEAD"));
    run(&repo, &revert_op(&repo, "HEAD"), NOW + 10);
    assert!(!repo.exists("main.txt"), "the reverted file is gone again");
    assert!(
        repo.git(&["log", "-1", "--format=%s"])
            .starts_with("Revert"),
        "git's own Revert subject"
    );
}

#[test]
fn reset_applies_undoes_redoes() {
    let repo = diverged();
    round_trip(&repo, &reset_op(&repo, "HEAD~1", ResetMode::Hard));
    run(&repo, &reset_op(&repo, "HEAD~1", ResetMode::Hard), NOW + 10);
    assert!(!repo.exists("main.txt"), "--hard took the file with it");
}

#[test]
fn checkout_applies_undoes_redoes() {
    let repo = diverged();
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    });
    round_trip(&repo, &op);
    run(&repo, &op, NOW + 10);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/feature");
    assert!(repo.exists("feature.txt"));
}

/// A checkout's inverse moves only `HEAD` — its `touches.refs` is empty by
/// design (`CheckoutParams`) — so a scope read off `touches.refs` alone left
/// the graph's HEAD badge on the branch the undo just left (m6-conflicts/79).
/// Both legs, both targets: undo and redo of a branch switch and a detach
/// each re-read everything, and `HEAD` really did move each time.
#[test]
fn an_undo_or_redo_that_moves_head_rereads_the_graph() {
    for detach in [false, true] {
        let repo = diverged();
        let to = if detach {
            CheckoutTarget::Detached(id(&repo, "feature"))
        } else {
            CheckoutTarget::Branch("refs/heads/feature".to_owned())
        };
        let op = Operation::Checkout(CheckoutParams {
            to,
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        });
        let forward = try_run(&repo, &op, NOW).unwrap_or_else(|why| panic!("{}", why.explain()));
        assert_eq!(forward.refresh, Refresh::Everything, "the checkout itself");

        let undone =
            run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
        assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
        assert!(undone.entry.touches.refs.is_empty(), "the premise: no ref");
        assert_eq!(undone.refresh, Refresh::Everything, "undo moved HEAD");

        let redone =
            run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("redo: {}", why.explain()));
        assert_ne!(
            repo.git(&["rev-parse", "--symbolic-full-name", "HEAD"]),
            "refs/heads/main"
        );
        assert_eq!(redone.refresh, Refresh::Everything, "redo moved HEAD");
    }
}

/// The other side of the rule: an undo that moves neither a ref nor `HEAD`
/// stays a working-tree re-read, so the fix did not widen every undo.
#[test]
fn an_undo_that_leaves_head_alone_rereads_the_working_tree_only() {
    let repo = diverged();
    repo.write("main.txt", b"edited\n");
    run(
        &repo,
        &Operation::Stage(graft_ops::operation::Target::Paths(vec![PathBuf::from(
            "main.txt",
        )])),
        NOW,
    );
    let undone =
        run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
    assert_eq!(undone.refresh, Refresh::WorkingTree);
}

#[test]
fn branch_create_applies_undoes_redoes() {
    let repo = diverged();
    let op = Operation::BranchCreate(BranchCreateParams {
        name: "release/1.0".to_owned(),
        at: id(&repo, "HEAD"),
        checkout: false,
    });
    round_trip(&repo, &op);
    run(&repo, &op, NOW + 10);
    assert_eq!(
        repo.git(&["rev-parse", "refs/heads/release/1.0"]),
        repo.git(&["rev-parse", "HEAD"])
    );
}

#[test]
fn branch_delete_applies_undoes_redoes() {
    let repo = diverged();
    let op = Operation::BranchDelete(RefDeleteParams {
        name: "refs/heads/feature".to_owned(),
        expected: id(&repo, "refs/heads/feature"),
    });
    round_trip(&repo, &op);
    run(&repo, &op, NOW + 10);
    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/heads/feature"])
            .is_err(),
        "the branch is gone"
    );
}

#[test]
fn tag_create_applies_undoes_redoes() {
    let repo = diverged();
    let op = Operation::TagCreate(TagCreateParams {
        name: "v1.0".to_owned(),
        at: id(&repo, "HEAD"),
        message: Some("first release\n".to_owned()),
    });
    round_trip(&repo, &op);
    run(&repo, &op, NOW + 10);
    assert_eq!(
        repo.git(&["cat-file", "-t", "refs/tags/v1.0"]),
        "tag",
        "an annotated tag object, not a lightweight ref"
    );
}

#[test]
fn tag_delete_applies_undoes_redoes() {
    let repo = diverged();
    repo.git(&["tag", "v0.9"]);
    let op = Operation::TagDelete(RefDeleteParams {
        name: "refs/tags/v0.9".to_owned(),
        expected: id(&repo, "refs/tags/v0.9"),
    });
    round_trip(&repo, &op);
}

#[test]
fn branch_from_stash_applies_undoes_redoes() {
    let repo = diverged();
    repo.write("base.txt", b"edited, not committed\n");
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    let stash = id(&repo, "refs/stash");
    let op = Operation::BranchFromStash(BranchFromStashParams {
        name: "wip-branch".to_owned(),
        index: 0,
        commit: stash,
    });
    let before = state_of(&repo);
    run(&repo, &op, NOW + 1);
    let after = state_of(&repo);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/wip-branch");
    assert_eq!(repo.read("base.txt"), b"edited, not committed\n".to_vec());

    undo_to(&repo, NOW + 2);
    assert_eq!(
        state_of(&repo),
        before,
        "undo put the stash back and removed the branch"
    );

    // The redo leg: a stash drop, the branch re-created, `HEAD` moved and
    // the tree restored — the one inverse in this group that does all four.
    // `round_trip` cannot be used here (the stash push already holds `NOW`).
    run_redo(&repo, NOW + 3, false).unwrap_or_else(|why| panic!("redo: {}", why.explain()));
    assert_eq!(
        state_of(&repo),
        after,
        "redo did not put it forward exactly"
    );
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/wip-branch");
    assert_eq!(repo.read("base.txt"), b"edited, not committed\n".to_vec());

    undo_to(&repo, NOW + 4);
    assert_eq!(
        state_of(&repo),
        before,
        "the second undo did not put it back"
    );
}

fn undo_to(repo: &TempRepo, at: i64) {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    undo::undo(&mut ctx, false).unwrap_or_else(|why| panic!("undo: {}", why.explain()));
}

// ── the ghost equals what git did ───────────────────────────────────────

/// Resolve a plan's target against the ghosts it was drawn with and the
/// commits git actually made: a `Real` target is itself; a `Ghost` target is
/// checked by shape, since its synthetic id is not a real commit.
fn real_of(target: GhostTarget) -> Option<Oid> {
    match target {
        GhostTarget::Real(oid) => Some(oid),
        GhostTarget::Ghost(_) => None,
    }
}

/// Every `RefMove` with a `Real` destination landed exactly there.
fn real_moves_landed(repo: &TempRepo, plan: &PreviewPlan) {
    for moved in &plan.ref_moves {
        let Some(expected) = moved.to.and_then(real_of) else {
            continue;
        };
        assert_eq!(
            id(repo, &moved.name),
            expected,
            "{} did not land where the plan said",
            moved.name
        );
    }
}

#[test]
fn ghost_matches_git_merge() {
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let plan = preview_of(&repo, &op);
    let into = id(&repo, "refs/heads/main");
    let source = id(&repo, "refs/heads/feature");

    assert_eq!(plan.ghosts.len(), 1, "one merge commit");
    assert_eq!(
        plan.ghosts[0].parents,
        vec![GhostTarget::Real(into), GhostTarget::Real(source)]
    );
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Ghost(0)));

    run(&repo, &op, NOW);
    let made = repo.git(&["rev-parse", "HEAD"]);
    let parents: Vec<String> = repo
        .git(&["rev-parse", &format!("{made}^@")])
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        parents,
        vec![into.to_string(), source.to_string()],
        "git's merge has the ghost's parents, in the ghost's order"
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        plan.ghosts[0].subject,
        "git wrote the subject the ghost promised"
    );
}

#[test]
fn ghost_matches_git_merge_fast_forward() {
    let repo = ahead();
    repo.git(&["switch", "-q", "feature"]);
    let op = merge_op(&repo, "refs/heads/main", FastForward::Default);
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty(), "a fast-forward creates no commit");
    assert_eq!(
        plan.ref_moves[0].to,
        Some(GhostTarget::Real(id(&repo, "refs/heads/main")))
    );

    run(&repo, &op, NOW);
    real_moves_landed(&repo, &plan);
    assert!(repo.exists("main.txt"), "the tree came with it");
}

/// The reader's `merge.ff` cannot change what the box promised.
///
/// `merge.ff=false` turns git's default merge of an ff-able branch into a
/// two-parent merge commit, and `merge.ff=only` makes a diverged merge fail
/// — both against a preview that drew the opposite (m5/07 review, M1). The
/// plan names the flag on every arm, so git follows the picture instead of
/// the config. Real git, real config, read back out of the repository.
#[test]
fn a_merge_follows_the_picture_not_the_readers_merge_ff() {
    let repo = ahead();
    repo.git(&["config", "merge.ff", "false"]);
    repo.git(&["switch", "-q", "feature"]);
    let op = merge_op(&repo, "refs/heads/main", FastForward::Default);
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty(), "the box promised a fast-forward");
    let onto = id(&repo, "refs/heads/main");
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Real(onto)));

    run(&repo, &op, NOW);
    assert_eq!(
        id(&repo, "refs/heads/feature"),
        onto,
        "the branch landed exactly where the box said"
    );
    assert_eq!(
        repo.git(&["rev-list", "--parents", "-n1", "HEAD"])
            .split_whitespace()
            .count(),
        2,
        "one parent: a fast-forward, not the merge commit merge.ff=false wants"
    );

    // And the other way: a diverged merge git would refuse under
    // `merge.ff=only` still runs, because the plan says `--no-ff`.
    let other = conflicting();
    other.git(&["config", "merge.ff", "only"]);
    let plan = preview_of(
        &other,
        &merge_op(&other, "refs/heads/feature", FastForward::Never),
    );
    assert_eq!(plan.ghosts.len(), 1, "the box promised a merge commit");
    let op = merge_op(&other, "refs/heads/feature", FastForward::Never);
    let error = try_run(&other, &op, NOW).expect_err("it conflicts, as it would anyway");
    assert!(
        matches!(error, ApplyError::Conflicted { .. }),
        "git got as far as the conflict rather than refusing the merge outright: {error:?}"
    );
}

/// A rebase moves only the branches the entry records.
///
/// `rebase.updateRefs=true` rewrites every branch pointing inside the
/// replayed range. Those refs are in neither `refs_before`/`refs_after` nor
/// the preview's `ref_moves`, so §6.7's rail cannot see them and `u` would
/// leave them rewritten (m5/07 review, M2). The user works on dependent
/// branches, which is exactly who turns this on.
///
/// Since ADR-0047 the branches built **on** his (above his tip) do move and
/// are recorded: `rewrite_run::a_three_branch_rewrite_records_every_ref`.
/// `lower` here is **inside** the range, below his tip: not in the line, so
/// it still stays (ADR-0047 §2), and `rewrite_run::
/// a_resume_never_turns_his_update_refs_config_on` holds it across a resume.
#[test]
fn a_rebase_moves_only_the_branch_the_entry_records() {
    let repo = TempRepo::init();
    repo.git(&["config", "rebase.updateRefs", "true"]);
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.write("main.txt", b"main\n");
    repo.commit_all("on main");
    repo.git(&["switch", "-qc", "lower", "HEAD~1"]);
    repo.write("lower.txt", b"lower\n");
    repo.commit_all("on lower");
    repo.git(&["switch", "-qc", "upper"]);
    repo.write("upper.txt", b"upper\n");
    repo.commit_all("on upper");

    let lower_before = id(&repo, "refs/heads/lower");
    let op = rebase_op(&repo, "refs/heads/main");
    let plan = preview_of(&repo, &op);
    assert!(
        !plan
            .ref_moves
            .iter()
            .any(|moved| moved.name == "refs/heads/lower"),
        "the box does not draw `lower` moving: {:?}",
        plan.ref_moves
    );

    let entry = run(&repo, &op, NOW);
    assert!(
        !entry.refs_before.contains_key("refs/heads/lower"),
        "the entry does not record `lower` either"
    );
    assert_eq!(
        id(&repo, "refs/heads/lower"),
        lower_before,
        "rebase.updateRefs moved a branch neither the box nor the journal knows about"
    );
}

#[test]
fn ghost_matches_git_rebase() {
    let repo = diverged();
    repo.git(&["switch", "-q", "feature"]);
    repo.write("second.txt", b"second\n");
    repo.commit_all("two on feature");
    let op = rebase_op(&repo, "refs/heads/main");
    let plan = preview_of(&repo, &op);
    let onto = id(&repo, "refs/heads/main");

    assert_eq!(plan.ghosts.len(), 2, "two commits replayed");
    assert_eq!(plan.ghosts[0].parents, vec![GhostTarget::Real(onto)]);
    assert_eq!(plan.ghosts[1].parents, vec![GhostTarget::Ghost(0)]);
    let replayed: Vec<Oid> = plan
        .ghosts
        .iter()
        .map(|ghost| ghost.replays.expect("a rebase ghost replays a commit"))
        .collect();

    run(&repo, &op, NOW);
    let landed: Vec<String> = repo
        .git(&["rev-list", "--reverse", "main..feature"])
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(landed.len(), 2);
    for (ghost, made) in plan.ghosts.iter().zip(&landed) {
        assert_eq!(
            repo.git(&["log", "-1", "--format=%s", made]),
            ghost.subject,
            "the replayed commit kept its subject"
        );
    }
    // The replayed commits carry the same patches as the originals.
    for (original, made) in replayed.iter().zip(&landed) {
        assert_eq!(
            patch_id(&repo, &original.to_string()),
            patch_id(&repo, made),
            "the patch git replayed is the one the ghost named"
        );
    }
    real_moves_landed(&repo, &plan);
}

/// The subjects of the commits a ref now reaches that `onto` does not,
/// oldest first: what git actually wrote.
fn subjects_after(repo: &TempRepo, onto: &str, branch: &str) -> Vec<String> {
    repo.git(&["rev-list", "--reverse", branch, &format!("^{onto}")])
        .lines()
        .map(|line| repo.git(&["log", "-1", "--format=%s", line]))
        .collect()
}

/// The subjects the plan drew, in ghost order.
fn ghost_subjects(plan: &PreviewPlan) -> Vec<String> {
    plan.ghosts
        .iter()
        .map(|ghost| ghost.subject.clone())
        .collect()
}

/// A foxtrot merge on `main`: `main`'s tip is a merge whose *first* parent
/// is the feature side, so `main`'s own previous tip is the second parent.
/// `previous` is a branch at that tip — what `Fix: rebase instead` replays
/// onto.
fn foxtrot() -> TempRepo {
    let repo = diverged();
    let previous = id(&repo, "refs/heads/main");
    repo.git(&["switch", "-q", "feature"]);
    repo.git(&[
        "merge",
        "--no-ff",
        "-m",
        "Merge branch 'main' into feature",
        "main",
    ]);
    let merge = id(&repo, "refs/heads/feature").to_string();
    repo.git(&["branch", "-f", "main", &merge]);
    repo.git(&["branch", "previous", &previous.to_string()]);
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// `git rebase` without `--rebase-merges` drops the merge commits in the
/// range, so the preview must not draw a ghost for one — and the `RefMove`
/// must point at the last commit git really writes (m5/43).
#[test]
fn a_rebase_draws_no_ghost_for_a_merge_git_will_drop() {
    let repo = foxtrot();
    let onto = id(&repo, "refs/heads/previous");
    let op = rebase_op(&repo, "refs/heads/previous");
    let plan = preview_of(&repo, &op);

    assert_eq!(
        ghost_subjects(&plan),
        vec!["one on feature".to_owned()],
        "the merge in the range is not a commit git will write"
    );
    assert_eq!(plan.ghosts[0].parents, vec![GhostTarget::Real(onto)]);
    assert_eq!(
        plan.ref_moves[0].to,
        Some(GhostTarget::Ghost(0)),
        "the branch lands on the last commit git writes"
    );

    run(&repo, &op, NOW);
    assert_eq!(
        subjects_after(&repo, "refs/heads/previous", "refs/heads/main"),
        ghost_subjects(&plan),
        "the ghosts are the commits git wrote, in git's order"
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s", "refs/heads/main"]),
        plan.ghosts[0].subject,
        "the tip the box predicted is the tip git left"
    );
}

/// git skips a commit whose change is already on `onto` under another id
/// ("skipped previously applied commit"), so the preview must skip it too.
#[test]
fn a_rebase_draws_no_ghost_for_a_change_already_on_onto() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "topic"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "topic"]);
    repo.write("first.txt", b"first\n");
    repo.commit_all("first on topic");
    repo.write("second.txt", b"second\n");
    repo.commit_all("second on topic");
    let already = id(&repo, "topic~1").to_string();
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["cherry-pick", "--no-edit", &already]);
    repo.git(&["switch", "-q", "topic"]);

    let op = rebase_op(&repo, "refs/heads/main");
    let plan = preview_of(&repo, &op);
    assert_eq!(
        ghost_subjects(&plan),
        vec!["second on topic".to_owned()],
        "`first on topic` is already on main under another id"
    );

    run(&repo, &op, NOW);
    assert_eq!(
        subjects_after(&repo, "refs/heads/main", "refs/heads/topic"),
        ghost_subjects(&plan),
        "the ghosts are the commits git wrote"
    );
}

/// Nothing left to replay is not nothing happening: git checks `onto` out
/// and leaves the branch there. The box has to draw that ref move.
#[test]
fn a_rebase_with_nothing_to_replay_still_draws_the_branch_moving() {
    let repo = diverged();
    repo.git(&["switch", "-q", "-c", "t1"]);
    repo.git(&["merge", "--no-ff", "-m", "Merge feature into t1", "feature"]);
    repo.git(&["switch", "-q", "-c", "t2", "main"]);
    repo.git(&["merge", "--no-ff", "-m", "Merge feature into t2", "feature"]);
    repo.git(&["switch", "-q", "t1"]);
    let onto = id(&repo, "refs/heads/t2");

    let op = rebase_op(&repo, "refs/heads/t2");
    let plan = preview_of(&repo, &op);
    assert!(
        plan.ghosts.is_empty(),
        "the only commit in the range is a merge git drops: {:?}",
        ghost_subjects(&plan)
    );
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].name, "refs/heads/t1");
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Real(onto)));

    run(&repo, &op, NOW);
    real_moves_landed(&repo, &plan);
    assert_eq!(id(&repo, "refs/heads/t1"), onto, "git left t1 on t2");
}

/// The branch already being behind `onto` is the same shape: git
/// fast-forwards it, so the box says so rather than "nothing would be
/// replayed".
#[test]
fn a_rebase_of_a_branch_behind_onto_draws_the_fast_forward() {
    let repo = ahead();
    repo.git(&["switch", "-q", "feature"]);
    let onto = id(&repo, "refs/heads/main");

    let op = rebase_op(&repo, "refs/heads/main");
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty());
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Real(onto)));

    run(&repo, &op, NOW);
    real_moves_landed(&repo, &plan);
    assert_eq!(id(&repo, "refs/heads/feature"), onto);
}

/// Rebasing a branch onto itself really does nothing, and says so.
#[test]
fn a_rebase_onto_the_same_commit_is_not_undoable() {
    let repo = diverged();
    repo.git(&["branch", "here", "main"]);
    let plan = preview_of(&repo, &rebase_op(&repo, "refs/heads/here"));
    assert!(plan.ghosts.is_empty());
    assert!(plan.ref_moves.is_empty(), "no ref moves anywhere");
    assert!(
        matches!(plan.undo, UndoNote::NotUndoable(ref why) if why.contains("nothing")),
        "{:?}",
        plan.undo
    );
}

/// `git patch-id`'s id for one commit: the change, independent of its
/// parents or its own id.
fn patch_id(repo: &TempRepo, rev: &str) -> String {
    let show = repo.git(&["show", "--format=", rev]);
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "cd {} && git show --format= {rev} | git patch-id --stable",
            repo.path().display()
        ))
        .output()
        .expect("patch-id runs");
    assert!(!show.is_empty(), "the commit has a patch to identify");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .expect("patch-id prints an id")
        .to_owned()
}

#[test]
fn ghost_matches_git_cherry_pick() {
    let repo = diverged();
    let op = pick_op(&repo, "refs/heads/feature");
    let plan = preview_of(&repo, &op);
    let head = id(&repo, "HEAD");
    assert_eq!(plan.ghosts.len(), 1);
    assert_eq!(plan.ghosts[0].parents, vec![GhostTarget::Real(head)]);
    assert_eq!(
        plan.ghosts[0].replays,
        Some(id(&repo, "refs/heads/feature"))
    );

    run(&repo, &op, NOW);
    assert_eq!(repo.git(&["rev-parse", "HEAD^"]), head.to_string());
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        plan.ghosts[0].subject
    );
    assert_eq!(
        patch_id(&repo, "HEAD"),
        patch_id(&repo, &plan.ghosts[0].replays.unwrap().to_string()),
        "the same patch landed"
    );
}

#[test]
fn ghost_matches_git_revert() {
    let repo = diverged();
    let op = revert_op(&repo, "HEAD");
    let plan = preview_of(&repo, &op);
    let head = id(&repo, "HEAD");
    assert_eq!(plan.ghosts.len(), 1);
    assert_eq!(plan.ghosts[0].parents, vec![GhostTarget::Real(head)]);

    // The files the revert rewrites are the reverted commit's own change.
    // Reverting `HEAD` is the ordinary case, and it is the one the old
    // `commit..head` pair drew as empty (m5/07 review, M3).
    assert_eq!(
        plan.worktree,
        WorktreeEffect::Updated {
            paths: vec!["main.txt".into()],
            more: false
        },
        "the box names the file the revert takes away"
    );

    run(&repo, &op, NOW);
    assert_eq!(repo.git(&["rev-parse", "HEAD^"]), head.to_string());
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        plan.ghosts[0].subject,
        "git's own `Revert \"…\"` subject is what the ghost drew"
    );
    assert!(
        !repo.exists("main.txt"),
        "git rewrote exactly the path the box named"
    );
}

/// With no recorded parent the list is a named hole, never an empty one:
/// "we do not know which files" must not read as "no files change".
#[test]
fn a_revert_with_no_recorded_parent_says_so_rather_than_drawing_nothing() {
    let repo = diverged();
    let head = id(&repo, "HEAD");
    let op = Operation::Revert(RevertParams {
        commit: head,
        subject: repo.git(&["log", "-1", "--format=%s"]),
        mainline: None,
        parents: 1,
        parent: None,
        head: head_tip(&repo),
    });
    let plan = preview_of(&repo, &op);
    assert!(
        matches!(plan.worktree, WorktreeEffect::NotShown { ref why } if why.contains("parent")),
        "{:?}",
        plan.worktree
    );
}

#[test]
fn ghost_matches_git_reset() {
    let repo = diverged();
    let to = id(&repo, "HEAD~1");
    let op = reset_op(&repo, "HEAD~1", ResetMode::Hard);
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty(), "a reset creates nothing");
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Real(to)));
    assert_eq!(plan.undo, UndoNote::Undoable);

    run(&repo, &op, NOW);
    real_moves_landed(&repo, &plan);
}

#[test]
fn ghost_oids_do_not_change_between_two_previews_of_one_operation() {
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let first = preview_of(&repo, &op);
    let second = preview_of(&repo, &op);
    assert_eq!(
        first
            .ghosts
            .iter()
            .map(|ghost| ghost.oid)
            .collect::<Vec<_>>(),
        second
            .ghosts
            .iter()
            .map(|ghost| ghost.oid)
            .collect::<Vec<_>>(),
        "the same plan draws the same rows"
    );
    assert!(
        repo.try_git(&["cat-file", "-e", &first.ghosts[0].oid.to_string()])
            .is_err(),
        "a ghost is not an object in the repository"
    );
}

// ── the commands are the plan's own ─────────────────────────────────────

/// The one rule that keeps the box honest: what it prints is what `plan`
/// produced, not a second rendering that can drift from it.
fn box_shows_the_lines_that_run(repo: &TempRepo, op: &Operation) {
    let handle = repo.open();
    let facts = facts_for(&handle, op);
    let expected: Vec<String> = op
        .plan(&facts)
        .iter()
        .map(Invocation::command_line)
        .collect();
    let plan = preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}: {}", op.describe(), why.explain()));
    assert!(!plan.commands.is_empty(), "{}", op.describe());
    assert_eq!(
        plan.commands,
        expected,
        "{}: the box must show the lines that run",
        op.describe()
    );
}

#[test]
fn preview_commands_come_from_plan() {
    let repo = diverged();
    repo.git(&["tag", "v0.9"]);
    for op in local_kinds(&repo) {
        box_shows_the_lines_that_run(&repo, &op);
    }

    // A rebase, from the branch it replays.
    let rebasing = diverged();
    rebasing.git(&["switch", "-q", "feature"]);
    box_shows_the_lines_that_run(&rebasing, &rebase_op(&rebasing, "refs/heads/main"));

    // And a branch from a stash, which needs a stash to exist.
    let stashed = diverged();
    stashed.write("base.txt", b"wip\n");
    run(
        &stashed,
        &Operation::StashPush(StashParams {
            message: None,
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    box_shows_the_lines_that_run(
        &stashed,
        &Operation::BranchFromStash(BranchFromStashParams {
            name: "from-stash".to_owned(),
            index: 0,
            commit: id(&stashed, "refs/stash"),
        }),
    );
}

/// Every history kind a `diverged()` repository can run as it stands. The
/// rebase and the branch-from-stash need their own repositories and are
/// built by their callers.
fn local_kinds(repo: &TempRepo) -> Vec<Operation> {
    vec![
        merge_op(repo, "refs/heads/feature", FastForward::Never),
        pick_op(repo, "refs/heads/feature"),
        revert_op(repo, "HEAD"),
        reset_op(repo, "HEAD~1", ResetMode::Mixed),
        Operation::Checkout(CheckoutParams {
            to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
            head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        }),
        Operation::BranchCreate(BranchCreateParams {
            name: "new".to_owned(),
            at: id(repo, "HEAD"),
            checkout: false,
        }),
        Operation::BranchDelete(RefDeleteParams {
            name: "refs/heads/feature".to_owned(),
            expected: id(repo, "refs/heads/feature"),
        }),
        Operation::TagCreate(TagCreateParams {
            name: "v2.0".to_owned(),
            at: id(repo, "HEAD"),
            message: None,
        }),
        Operation::TagDelete(RefDeleteParams {
            name: "refs/tags/v0.9".to_owned(),
            expected: id(repo, "refs/tags/v0.9"),
        }),
    ]
}

// ── the conflict span ───────────────────────────────────────────────────

/// The three things a stop hands back, or a panic naming what came instead.
fn conflict_of(error: ApplyError) -> (JournalEntry, InProgress, Vec<PathBuf>) {
    match error {
        ApplyError::Conflicted { entry, kind, paths } => (*entry, kind, paths),
        other => panic!("expected a conflict, got {other:?}"),
    }
}

/// How many lines the journal file holds: two per finished entry, one for
/// an operation that stopped and has not finished yet (§6.1).
fn journal_lines(repo: &TempRepo) -> usize {
    std::fs::read_to_string(repo.path().join(".git/graft/undo-journal.jsonl"))
        .expect("the journal reads")
        .lines()
        .count()
}

/// The stop itself: one `Pending` entry, one journal line, git's own merge
/// state live, and neither continuing nor staging the unresolved file
/// allowed until the markers are gone.
fn a_merge_that_stops_on_shared_txt(repo: &TempRepo) -> JournalEntry {
    let op = merge_op(repo, "refs/heads/feature", FastForward::Never);
    let (entry, kind, paths) = conflict_of(try_run(repo, &op, NOW).expect_err("it conflicts"));
    assert_eq!(kind, InProgress::Merge);
    assert_eq!(paths, vec![PathBuf::from("shared.txt")]);
    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(
        repo.exists(".git/MERGE_HEAD"),
        "git's own state says the merge is live"
    );
    assert_eq!(
        journal_lines(repo),
        1,
        "a stopped operation writes no outcome line"
    );
    assert_eq!(entries(repo).len(), 1);
    assert_eq!(entries(repo)[0].status, EntryStatus::Pending);

    // Continue before resolving: refused before git is asked.
    let refused = run_resume(repo, Resume::Continue, NOW + 1).expect_err("unmerged");
    assert!(
        matches!(refused, ApplyError::Refused(Refusal::Unmerged { .. })),
        "{refused:?}"
    );
    // Markers still in the file: staging it is refused by name.
    staging_the_markers_is_refused(repo, "shared.txt", NOW + 2);
    entry
}

#[test]
fn conflict_span_is_one_entry_and_undoes_to_before_the_merge() {
    let repo = conflicting();
    let before = state_of(&repo);
    let before_main = id(&repo, "refs/heads/main");
    let entry = a_merge_that_stops_on_shared_txt(&repo);

    // Resolved through Graft's own "mark resolved" — the path M6's resolver
    // takes, and the one that must leave the span one entry. A `Stage` taken
    // inside a live operation is part of that operation: it runs, its command
    // line joins the entry, and it writes no journal line of its own.
    repo.write("shared.txt", b"resolved\n");
    let staged = try_run(
        &repo,
        &Operation::Stage(graft_ops::operation::Target::Paths(vec![
            "shared.txt".into(),
        ])),
        NOW + 3,
    )
    .expect("marking the file resolved is allowed once the markers are gone");
    assert_eq!(
        journal_lines(&repo),
        1,
        "the resolve step is part of the merge's entry, not a second entry"
    );
    assert_eq!(entries(&repo).len(), 1, "still one entry");
    assert_eq!(
        staged.entry.description, "stage shared.txt",
        "the status line says what the user did, not what the merge is"
    );
    assert_eq!(
        staged.refresh,
        Refresh::WorkingTree,
        "nothing moved a ref: the changes pane is what changed"
    );

    // Now it continues, and the entry finally gets its outcome line.
    let applied = run_resume(&repo, Resume::Continue, NOW + 4).expect("the merge finishes");
    assert_eq!(
        applied.entry.id, entry.id,
        "the same entry, start to finish"
    );
    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert!(!repo.exists(".git/MERGE_HEAD"));
    assert_eq!(journal_lines(&repo), 2, "one entry, one outcome");
    assert_eq!(
        entries(&repo)
            .iter()
            .filter(|held| held.kind == OperationKind::Merge)
            .count(),
        1,
        "one entry for the whole merge, not two"
    );

    // And `u` takes the whole span back.
    undo_to(&repo, NOW + 5);
    assert_eq!(id(&repo, "refs/heads/main"), before_main);
    assert_eq!(state_of(&repo), before, "back to before the merge, exactly");
}

/// Two conflicted files, resolved one at a time. The second `Stage` leaves
/// a fully merged index — `write-tree` would succeed and the entry *could*
/// have been journaled — and it still writes no line: the rule is "an
/// operation is in progress", not "the index cannot be snapshotted".
///
/// This is the case a narrower rule would miss, and it is the ordinary one:
/// the last file a user resolves always leaves the index clean.
#[test]
fn resolving_each_file_in_turn_still_leaves_the_span_one_entry() {
    let repo = TempRepo::init();
    repo.write("one.txt", b"base\n");
    repo.write("two.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("one.txt", b"main\n");
    repo.write("two.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("one.txt", b"feature\n");
    repo.write("two.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);

    let before = state_of(&repo);
    let before_main = id(&repo, "refs/heads/main");
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let (entry, _, paths) = conflict_of(try_run(&repo, &op, NOW).expect_err("it conflicts"));
    assert_eq!(paths.len(), 2, "both files conflicted: {paths:?}");

    for (n, path) in [(1_i64, "one.txt"), (2, "two.txt")] {
        repo.write(path, b"resolved\n");
        let stage = Operation::Stage(graft_ops::operation::Target::Paths(vec![path.into()]));
        try_run(&repo, &stage, NOW + n).expect("marking it resolved is allowed");
        assert_eq!(
            journal_lines(&repo),
            1,
            "after resolving {path}: still the merge's one Pending line"
        );
    }
    // The index is fully merged now — the condition that used to decide
    // whether the step got an entry of its own.
    assert_eq!(entries(&repo).len(), 1, "one entry, two resolve steps");

    let applied = run_resume(&repo, Resume::Continue, NOW + 4).expect("the merge finishes");
    assert_eq!(
        applied.entry.id, entry.id,
        "the same entry, start to finish"
    );
    assert_eq!(journal_lines(&repo), 2);

    undo_to(&repo, NOW + 5);
    assert_eq!(id(&repo, "refs/heads/main"), before_main);
    assert_eq!(state_of(&repo), before, "back to before the merge, exactly");
}

/// The resolve step's command line joins the live entry's, so the Command
/// Log shows the whole span rather than the `merge` alone.
///
/// Both run through **one** `Journal`: a `Pending` entry's commands live in
/// memory until its outcome line is written, so that is the only place the
/// merge's own lines are to be found. A caller that opens a fresh journal
/// per job — which the app does — sees only the step it just ran. That is
/// the same limitation the stop itself has and it is not fixed here; it is
/// recorded in the ticket as the one gap the fold inherits.
#[test]
fn a_resolve_step_joins_the_live_entrys_commands() {
    let repo = conflicting();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let stage = Operation::Stage(graft_ops::operation::Target::Paths(vec![
        "shared.txt".into(),
    ]));
    {
        let mut ctx = context(&handle, &mut journal, NOW, &never);
        apply(&mut ctx, &op).expect_err("it conflicts");
    }
    repo.write("shared.txt", b"resolved\n");
    {
        let mut ctx = context(&handle, &mut journal, NOW + 3, &never);
        apply(&mut ctx, &stage).expect("the resolve step runs");
    }
    let held = &journal.entries()[0];
    assert_eq!(held.kind, OperationKind::Merge, "the live entry");
    assert!(
        held.commands.iter().any(|line| line.contains("merge")),
        "the merge's own lines are still there: {:?}",
        held.commands
    );
    assert!(
        held.commands
            .last()
            .is_some_and(|line| line.contains("add")),
        "the resolve step's line is appended, not replacing: {:?}",
        held.commands
    );
}

/// Staging a file that still has `<<<<<<< ` in it is refused, and the
/// refusal names the file rather than reporting a git error.
fn staging_the_markers_is_refused(repo: &TempRepo, path: &str, at: i64) {
    let stage = Operation::Stage(graft_ops::operation::Target::Paths(vec![path.into()]));
    let refused = try_run(repo, &stage, at).expect_err("markers remain");
    let ApplyError::Refused(Refusal::MarkersRemain { path: named }) = refused else {
        panic!("expected MarkersRemain, got {refused:?}");
    };
    assert_eq!(named, PathBuf::from(path));
}

#[test]
fn abort_restores_before_state() {
    let repo = conflicting();
    let before = state_of(&repo);
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    try_run(&repo, &op, NOW).expect_err("the merge conflicts");

    let applied = run_resume(&repo, Resume::Abort, NOW + 1).expect("the abort runs");
    assert_eq!(
        applied.entry.status,
        EntryStatus::Aborted {
            reason: "aborted".to_owned()
        }
    );
    assert!(!repo.exists(".git/MERGE_HEAD"));
    assert_eq!(
        state_of(&repo),
        before,
        "abort put the tree and the refs back, byte for byte"
    );
}

#[test]
fn recovery_leaves_a_live_conflict_alone() {
    let repo = conflicting();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let (entry, _, _) = conflict_of(try_run(&repo, &op, NOW).expect_err("it conflicts"));

    // A relaunch: recovery runs over a journal with a Pending entry whose
    // operation git still says is live.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert_eq!(recovered.live, vec![entry.id.clone()]);
    assert!(recovered.closed.is_empty(), "nothing was closed");
    drop(ctx);
    assert_eq!(
        journal
            .entries()
            .iter()
            .find(|held| held.id == entry.id)
            .expect("the entry is there")
            .status,
        EntryStatus::Pending,
        "the banner takes over; recovery does not close it"
    );

    // Contrast: with git's state cleared, the same entry closes as usual.
    repo.git(&["merge", "--abort"]);
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 2, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert!(recovered.live.is_empty());
    assert_eq!(recovered.closed, vec![entry.id]);
}

/// Graft does not adopt a merge somebody else started.
///
/// A `Pending` entry of Graft's plus a live merge of the same kind used to
/// match on kind alone, so `resume` would have finished the shell's merge
/// and closed Graft's entry with a before-state from a different act
/// (m5/07 review, M7). Two legs: `HEAD` on a different branch, and the same
/// branch moved on since the entry was written.
#[test]
fn a_shell_merge_is_not_adopted_as_grafts_own_entry() {
    // Leg 1: a different branch.
    let repo = conflicting();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let (entry, _, _) = conflict_of(try_run(&repo, &op, NOW).expect_err("it conflicts"));
    repo.git(&["merge", "--abort"]);
    repo.git(&["switch", "-qc", "other"]);
    let _ = repo.try_git(&["merge", "--no-ff", "--no-edit", "-m", "theirs", "feature"]);
    assert!(repo.exists(".git/MERGE_HEAD"), "a merge is live again");
    // Since m6-conflicts/57 `Abort` on a merge with no entry of Graft's
    // aborts it as a shell-begun merge, with an `AbortMerge` entry of its
    // own — never by closing Graft's stale entry.
    let aborted = run_resume(&repo, Resume::Abort, NOW + 1)
        .unwrap_or_else(|why| panic!("the shell's merge aborts: {}", why.explain()));
    assert_eq!(aborted.entry.kind, OperationKind::AbortMerge);
    assert_ne!(aborted.entry.id, entry.id, "a new entry, not Graft's");
    let status = journal_for(&repo.open())
        .entries()
        .iter()
        .find(|held| held.id == entry.id)
        .expect("Graft's entry is there")
        .status
        .clone();
    assert_eq!(status, EntryStatus::Pending, "not closed by the abort");

    // Leg 2: the same branch, but moved on since the entry was written.
    let repo = conflicting();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    let (moved_on, _, _) = conflict_of(try_run(&repo, &op, NOW).expect_err("it conflicts"));
    repo.git(&["merge", "--abort"]);
    repo.write("unrelated.txt", b"later work\n");
    repo.commit_all("main moved on");
    let _ = repo.try_git(&["merge", "--no-ff", "--no-edit", "-m", "theirs", "feature"]);
    assert!(repo.exists(".git/MERGE_HEAD"), "a merge is live again");
    // Since m6-conflicts/20 `Continue` on a merge with no entry of Graft's
    // finishes it as a shell-begun merge of its own, so the refusal here is
    // the unmerged path — and Graft's stale entry is still not adopted.
    let refused = run_resume(&repo, Resume::Continue, NOW + 1).expect_err("not Graft's merge");
    assert!(
        matches!(refused, ApplyError::Refused(Refusal::Unmerged { .. })),
        "the branch moved since the entry was written: {refused:?}"
    );
    let handle = repo.open();
    let status = journal_for(&handle)
        .entries()
        .iter()
        .find(|held| held.id == moved_on.id)
        .expect("Graft's entry is there")
        .status
        .clone();
    assert_eq!(
        status,
        EntryStatus::Pending,
        "and was not closed by the resume"
    );

    // And the entry is not "live" to recovery either: it closes like any
    // other interrupted one instead of waiting for a banner that will never
    // finish it.
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 2, &never);
    let recovered = recover_interrupted(&mut ctx).expect("recovery runs");
    assert!(
        recovered.live.is_empty(),
        "the shell's merge is not Graft's live entry: {:?}",
        recovered.live
    );
    assert_eq!(recovered.closed.len(), 1, "the stale entry was closed");
    drop(entry);
}

#[test]
fn rebase_conflicts_twice_same_entry() {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("shared.txt", b"main\n");
    repo.commit_all("main's change");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("shared.txt", b"feature one\n");
    repo.commit_all("feature one");
    repo.write("shared.txt", b"feature two\n");
    repo.commit_all("feature two");

    let op = rebase_op(&repo, "refs/heads/main");
    let (entry, kind, _) =
        conflict_of(try_run(&repo, &op, NOW).expect_err("the first commit conflicts"));
    assert_eq!(kind, InProgress::Rebase);

    repo.write("shared.txt", b"resolved one\n");
    repo.git(&["add", "shared.txt"]);
    let (again, still, _) =
        conflict_of(run_resume(&repo, Resume::Continue, NOW + 1).expect_err("it stops again"));
    assert_eq!(again.id, entry.id, "one entry across both stops");
    assert_eq!(still, InProgress::Rebase);
    assert_eq!(
        entries(&repo).len(),
        1,
        "still one entry, still with no outcome"
    );

    repo.write("shared.txt", b"resolved two\n");
    repo.git(&["add", "shared.txt"]);
    let done = run_resume(&repo, Resume::Continue, NOW + 2).expect("the rebase finishes");
    assert_eq!(done.entry.id, entry.id);
    assert_eq!(done.entry.status, EntryStatus::Committed);
    assert!(!repo.exists(".git/rebase-merge"));
}

// ── refusals the preview is pinned by ───────────────────────────────────

#[test]
fn stale_preview_is_refused() {
    // A tip that moved between the preview and the confirm.
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    repo.write("late.txt", b"a commit after the preview\n");
    repo.commit_all("later");
    let refused = try_run(&repo, &op, NOW).expect_err("the tip moved");
    assert!(
        matches!(
            refused,
            ApplyError::Refused(Refusal::Moved { ref name }) if name == "refs/heads/main"
        ),
        "{refused:?}"
    );
    assert_eq!(
        entries(&repo).len(),
        0,
        "a refusal journals nothing and runs nothing"
    );

    // A rebase over a dirty tracked file is no longer refused: the edit is
    // set aside and put back (ADR-0045 §4, m6-conflicts/73).
    let dirty = diverged();
    dirty.git(&["switch", "-q", "feature"]);
    let op = rebase_op(&dirty, "refs/heads/main");
    dirty.write("base.txt", b"edited, not committed\n");
    try_run(&dirty, &op, NOW).expect("the edit is set aside");
    assert_eq!(
        dirty.read("base.txt"),
        b"edited, not committed\n".to_vec(),
        "the edit is back"
    );

    // An untracked file never counts as dirty (hard choice 4).
    let dirty = diverged();
    dirty.git(&["switch", "-q", "feature"]);
    dirty.write("scratch.tmp", b"untracked\n");
    try_run(&dirty, &rebase_op(&dirty, "refs/heads/main"), NOW + 1)
        .expect("an untracked file does not block a rebase");
}

#[test]
fn a_fast_forward_only_merge_of_diverged_branches_has_no_preview() {
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Only);
    let refused = try_preview(&repo, &op).expect_err("they have diverged");
    assert!(
        matches!(
            refused,
            preview::PreviewError::Refused(Refusal::NotFastForward { .. })
        ),
        "{refused:?}"
    );
}

#[test]
fn a_merge_that_would_change_nothing_says_so() {
    let repo = ahead();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Default);
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty());
    assert!(plan.ref_moves.is_empty());
    assert_eq!(plan.worktree, WorktreeEffect::Untouched);
    assert!(matches!(plan.undo, UndoNote::NotUndoable(_)));
}

// ── the forecast ────────────────────────────────────────────────────────

#[test]
fn forecast_names_the_conflicting_path() {
    let clean = diverged();
    let plan = preview_of(
        &clean,
        &merge_op(&clean, "refs/heads/feature", FastForward::Never),
    );
    assert_eq!(plan.conflicts, ConflictForecast::Clean);

    let bad = conflicting();
    let plan = preview_of(
        &bad,
        &merge_op(&bad, "refs/heads/feature", FastForward::Never),
    );
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: vec!["shared.txt".into()]
        },
        "the forecast names the file, and says may, not will"
    );
    // And it is a forecast, not a promise: git really does stop there.
    let error = try_run(
        &bad,
        &merge_op(&bad, "refs/heads/feature", FastForward::Never),
        NOW,
    )
    .expect_err("the merge conflicts");
    assert!(matches!(error, ApplyError::Conflicted { .. }));

    // A rebase whose replay conflicts is forecast the same way.
    let rebasing = conflicting();
    rebasing.git(&["switch", "-q", "feature"]);
    let plan = preview_of(&rebasing, &rebase_op(&rebasing, "refs/heads/main"));
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: vec!["shared.txt".into()]
        }
    );
}

#[test]
fn a_capped_range_is_not_checked_rather_than_called_clean() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "long"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("on main");
    repo.git(&["switch", "-q", "long"]);
    for step in 0..preview::FORECAST_CAP + 2 {
        repo.write(&format!("f{step}.txt"), b"x\n");
        repo.commit_all(&format!("commit {step}"));
    }
    let plan = preview_of(&repo, &rebase_op(&repo, "refs/heads/main"));
    assert!(
        matches!(plan.conflicts, ConflictForecast::NotChecked(ref why) if why.contains("50")),
        "{:?}",
        plan.conflicts
    );
    assert_eq!(
        plan.ghosts.len(),
        preview::FORECAST_CAP,
        "the drawn rows stop at the cap too"
    );
}

// ── the time machine ────────────────────────────────────────────────────

#[test]
fn time_machine_reverts_in_order() {
    let repo = diverged();
    let start = state_of(&repo);
    let first = run(&repo, &pick_op(&repo, "refs/heads/feature"), NOW).id;
    run(
        &repo,
        &Operation::BranchCreate(BranchCreateParams {
            name: "second".to_owned(),
            at: id(&repo, "HEAD"),
            checkout: false,
        }),
        NOW + 1,
    );
    run(
        &repo,
        &Operation::TagCreate(TagCreateParams {
            name: "third".to_owned(),
            at: id(&repo, "HEAD"),
            message: None,
        }),
        NOW + 2,
    );
    assert_ne!(state_of(&repo), start);

    // The preview first: three steps, drawn from the plans that will run.
    let plan = preview::plan_revert_to(&entries(&repo), &first).expect("a plan");
    assert!(plan.description.contains("3 steps"), "{}", plan.description);
    assert_eq!(plan.worktree, WorktreeEffect::RestoredFromBackup);
    assert!(plan.ghosts.is_empty(), "nothing is created by going back");

    let done = revert_to(&repo, &first, NOW + 3).expect("the revert runs");
    assert_eq!(done.len(), 3, "one journaled step per operation");
    assert_eq!(
        state_of(&repo),
        start,
        "the repository is back where it started"
    );
    assert_eq!(
        entries(&repo)
            .iter()
            .filter(|entry| entry.kind == OperationKind::Undo)
            .count(),
        3,
        "three Undo entries, so U walks them back one at a time"
    );

    // And `U` ×3 puts each step forward again, one at a time.
    for step in 0..3 {
        let handle = repo.open();
        let mut journal = journal_for(&handle);
        let never = Never;
        let mut ctx = context(&handle, &mut journal, NOW + 10 + step, &never);
        undo::redo(&mut ctx, false).unwrap_or_else(|why| panic!("redo: {}", why.explain()));
    }
    assert_eq!(
        repo.git(&["rev-parse", "refs/tags/third"]),
        repo.git(&["rev-parse", "HEAD"]),
        "the third step is back"
    );
}

/// Travelling back past an `Undo` does not re-apply what the user undid.
///
/// An in-force `Undo` entry is not an operation: inverting it would *redo*
/// its root, so a "revert to before the pick" would hand back a branch the
/// user had deliberately undone. `next_undo` has always filtered on
/// `is_operation`; the time machine did not (m5/07 review, C2).
#[test]
fn the_time_machine_does_not_redo_what_was_undone() {
    let repo = diverged();
    let start = state_of(&repo);
    let first = run(&repo, &pick_op(&repo, "refs/heads/feature"), NOW).id;
    run(
        &repo,
        &Operation::BranchCreate(BranchCreateParams {
            name: "second".to_owned(),
            at: id(&repo, "HEAD"),
            checkout: false,
        }),
        NOW + 1,
    );
    run_undo(&repo, NOW + 2, false).expect("the branch is undone");
    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/heads/second"])
            .is_err(),
        "the branch is gone before the revert"
    );

    // The picture first: it must not promise a branch either.
    let plan = preview::plan_revert_to(&entries(&repo), &first).expect("a plan");
    assert!(
        !plan
            .ref_moves
            .iter()
            .any(|move_| move_.name.contains("second")),
        "the box would show a revert that creates the undone branch: {:?}",
        plan.ref_moves
    );

    let done = revert_to(&repo, &first, NOW + 3).expect("the revert runs");
    assert_eq!(done.len(), 1, "only the pick is still in force");
    assert!(
        repo.try_git(&["rev-parse", "--verify", "refs/heads/second"])
            .is_err(),
        "going back re-created a branch the user had undone"
    );
    assert_eq!(state_of(&repo), start, "exactly the state before the pick");
}

fn revert_to(repo: &TempRepo, target: &EntryId, at: i64) -> Result<Vec<Applied>, UndoError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    undo::revert_to(&mut ctx, target, false)
}

#[test]
fn a_time_machine_step_that_cannot_be_inverted_stops_before_anything_runs() {
    let repo = diverged();
    let state = state_of(&repo);
    let unknown = EntryId::new(NOW, [7; 10]);
    let error = revert_to(&repo, &unknown, NOW + 1).expect_err("no such entry");
    assert!(matches!(error, UndoError::Nothing(_)), "{error:?}");
    assert_eq!(state_of(&repo), state, "nothing ran");
}

// ── the guided re-push ──────────────────────────────────────────────────

/// A work clone whose branch has one commit the remote also has, pushed
/// after `old_tip`: the shape the Command Log offers a re-push from.
fn a_remote_that_moved_past_its_old_tip() -> (LocalRemote, String, Oid, Oid) {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let branch = format!("refs/heads/{}", remote.default_branch());
    let work = remote.work().to_owned();
    let old_tip = rev(&remote, &work, "HEAD");
    remote
        .git(&work, &["commit", "--allow-empty", "-q", "-m", "one more"])
        .expect("a commit");
    remote
        .git(&work, &["push", "-q", "origin", "HEAD"])
        .expect("the push lands");
    let pushed = rev(&remote, remote.bare(), &branch);
    assert_ne!(pushed, old_tip, "the remote moved forward");
    (remote, branch, old_tip, pushed)
}

/// One `rev-parse` in one of the two repositories, as an id.
fn rev(remote: &LocalRemote, at: &Path, what: &str) -> Oid {
    Oid::from_hex(
        remote
            .git(at, &["rev-parse", what])
            .expect("rev-parse")
            .trim(),
    )
    .expect("a full id")
}

#[test]
fn repush_old_tip_with_lease() {
    let (remote, branch, old_tip, pushed) = a_remote_that_moved_past_its_old_tip();
    let work = remote.work().to_owned();
    let handle = RepoHandle::open(&work).expect("the work clone opens");
    let params = PushParams {
        branch: branch.clone(),
        remote: "origin".to_owned(),
        remote_branch: branch.clone(),
        set_upstream: false,
        lease: Some(Lease { expected: pushed }),
        source_commit: Some(old_tip),
    };

    // A re-push without a lease is refused outright (ADR-0038 §4).
    let bare = Operation::Push(PushParams {
        lease: None,
        ..params.clone()
    });
    assert_eq!(
        bare.check(&facts_for(&handle, &bare)),
        Err(Refusal::RepushNeedsLease),
        "never a force without a lease"
    );

    let op = Operation::Push(params);
    the_box_says_it_rewrites_the_remote(&handle, &op, old_tip, remote.default_branch());

    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    apply(&mut ctx, &op).unwrap_or_else(|why| panic!("{}", why.explain()));
    drop(ctx);

    assert_eq!(
        rev(&remote, remote.bare(), &branch),
        old_tip,
        "the remote branch is back at the old tip"
    );
    assert_eq!(
        rev(&remote, &work, &branch),
        pushed,
        "the local branch kept every commit it had"
    );
}

/// ADR-0038 §4's three requirements on the box, in one place: the line
/// names the commit, the command carries a lease and nothing else, and the
/// undo note says plainly that `u` will not bring the remote back.
fn the_box_says_it_rewrites_the_remote(
    handle: &RepoHandle,
    op: &Operation,
    source: Oid,
    branch: &str,
) {
    assert_eq!(
        op.describe(),
        format!(
            "force-push {} → origin/{branch} (with lease)",
            &source.to_string()[..7]
        ),
        "the line names the commit, not the branch"
    );
    let facts = facts_for(handle, op);
    let plan = preview::plan(handle, &git_version(), op, &facts, &Never).expect("a plan");
    assert!(
        matches!(plan.undo, UndoNote::NotUndoable(ref why) if why.contains("everyone")),
        "{:?}",
        plan.undo
    );
    assert_eq!(plan.commands.len(), 1);
    assert!(
        plan.commands[0].contains("--force-with-lease="),
        "{}",
        plan.commands[0]
    );
    assert!(
        !plan.commands[0].contains("--force "),
        "never a bare force: {}",
        plan.commands[0]
    );
}

#[test]
fn a_repush_of_a_commit_that_is_not_on_the_branch_is_refused() {
    let (remote, branch, _, pushed) = a_remote_that_moved_past_its_old_tip();
    let work = remote.work().to_owned();
    let handle = RepoHandle::open(&work).expect("the work clone opens");

    // A commit on a side branch: real, but not an ancestor of the branch.
    remote
        .git(&work, &["switch", "-q", "-c", "side"])
        .expect("a side branch");
    remote
        .git(&work, &["commit", "--allow-empty", "-q", "-m", "elsewhere"])
        .expect("a commit");
    let elsewhere = rev(&remote, &work, "HEAD");
    remote
        .git(&work, &["switch", "-q", "-"])
        .expect("back to the default branch");

    let branch_ref = branch.clone();
    let op = Operation::Push(PushParams {
        branch: branch.clone(),
        remote: "origin".to_owned(),
        remote_branch: branch,
        set_upstream: false,
        lease: Some(Lease { expected: pushed }),
        source_commit: Some(elsewhere),
    });
    let facts = facts_for(&handle, &op);
    let refused =
        preview::plan(&handle, &git_version(), &op, &facts, &Never).expect_err("not on the branch");
    assert!(
        matches!(
            refused,
            preview::PreviewError::Refused(Refusal::NotOnBranch { .. })
        ),
        "{refused:?}"
    );

    // And `apply` refuses it on its own account. The preview is a picture;
    // a caller that never drew one would otherwise have force-pushed a
    // commit off the branch under the lease (m5/07 review, M5).
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    let refused = apply(&mut ctx, &op).expect_err("apply refuses it too");
    assert!(
        matches!(
            refused,
            ApplyError::Refused(Refusal::NotOnBranch { ref commit, .. }) if *commit == elsewhere
        ),
        "{refused:?}"
    );
    drop(ctx);
    assert!(
        journal.entries().is_empty(),
        "refused before the Pending line: nothing was journaled"
    );
    assert_eq!(
        rev(&remote, remote.bare(), &branch_ref),
        pushed,
        "and the remote did not move"
    );
}

// ── the working tree, as the box describes it ───────────────────────────

#[test]
fn a_hard_reset_over_dirty_files_says_loses_and_names_them() {
    let repo = diverged();
    repo.write("base.txt", b"edited\n");
    let op = reset_op(&repo, "HEAD~1", ResetMode::Hard);
    let plan = preview_of(&repo, &op);
    assert_eq!(
        plan.worktree,
        WorktreeEffect::Lost {
            paths: vec!["base.txt".into()]
        }
    );
    assert_eq!(plan.undo, UndoNote::Undoable, "the backup covers it (§6.3)");

    // A soft or mixed reset leaves the tree alone and says so.
    let soft = preview_of(&repo, &reset_op(&repo, "HEAD~1", ResetMode::Soft));
    assert_eq!(soft.worktree, WorktreeEffect::Untouched);

    // And the loss really is recoverable: run it, then `u`.
    let before: RepoState = state_of(&repo);
    run(&repo, &op, NOW);
    assert_eq!(
        repo.read("base.txt"),
        b"base\n".to_vec(),
        "the edit is gone"
    );
    undo_to(&repo, NOW + 1);
    assert_eq!(state_of(&repo), before, "the backup gave it back");
}

#[test]
fn a_checkout_moves_head_and_names_the_files_it_rewrites() {
    let repo = diverged();
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    });
    let plan = preview_of(&repo, &op);
    assert_eq!(
        plan.head,
        Some(HeadMove::ToBranch("refs/heads/feature".to_owned()))
    );
    let WorktreeEffect::Updated { paths, more } = &plan.worktree else {
        panic!("a checkout rewrites files: {:?}", plan.worktree);
    };
    assert!(!more);
    let mut named: Vec<String> = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    named.sort();
    assert_eq!(named, vec!["feature.txt", "main.txt"]);

    run(&repo, &op, NOW);
    for path in &named {
        // Each named path really did change state on disk.
        assert_ne!(
            repo.exists(path),
            path == "main.txt",
            "{path} was rewritten by the checkout"
        );
    }
}

#[test]
fn deleting_a_ref_draws_it_going_away() {
    let repo = diverged();
    let expected = id(&repo, "refs/heads/feature");
    let op = Operation::BranchDelete(RefDeleteParams {
        name: "refs/heads/feature".to_owned(),
        expected,
    });
    let plan = preview_of(&repo, &op);
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].from, Some(expected));
    assert_eq!(plan.ref_moves[0].to, None, "gone, not moved");
}

// ── the three cases the group left untested (m5/07 job 2) ───────────────
//
// The behaviours these pin were recorded against git 2.54 in a throwaway
// repository first: `.scratch/m5-history-and-ghost-preview/plan-behaviours.txt`
// and `plan-behaviours-2.txt`. Two of the ticket's own premises did not
// survive that record, and the tests below assert what git actually does,
// not what the ticket assumed.

/// `main` with a merge commit at its tip: `feature`'s file is in the tree
/// because the merge brought it, `main`'s own file because it was already
/// there. Reverting the merge with mainline 1 must take the first away and
/// leave the second.
fn merged() -> TempRepo {
    let repo = diverged();
    let op = merge_op(&repo, "refs/heads/feature", FastForward::Never);
    run(&repo, &op, NOW - 100);
    repo
}

/// A revert of a merge commit, `-m 1`, run for real and read back out of
/// the tree — the ticket's case (c).
#[test]
fn revert_of_a_merge_with_mainline_one_removes_the_merged_side() {
    let repo = merged();
    let merge = id(&repo, "HEAD");
    assert_eq!(
        repo.git(&["rev-list", "--parents", "-n1", "HEAD"])
            .split_whitespace()
            .count(),
        3,
        "the fixture's tip really is a two-parent merge"
    );
    assert!(repo.exists("feature.txt"), "the merge brought it in");
    assert!(repo.exists("main.txt"));

    // Graft writes the merge's own message, so the subject is read rather
    // than assumed.
    let merge_subject = repo.git(&["log", "-1", "--format=%s", "HEAD"]);
    let op = Operation::Revert(RevertParams {
        commit: merge,
        subject: merge_subject.clone(),
        mainline: Some(1),
        parents: 2,
        parent: Some(id(&repo, "HEAD^1")),
        head: head_tip(&repo),
    });
    assert_eq!(
        op.describe(),
        format!("revert {} \"{merge_subject}\"", &merge.to_string()[..7])
    );

    let entry = run(&repo, &op, NOW);
    assert_eq!(entry.kind, OperationKind::Revert);
    // Read back out of git, not out of the returned value.
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        format!("Revert \"{merge_subject}\"")
    );
    assert!(
        !repo.exists("feature.txt"),
        "mainline 1 keeps main's side and drops feature's"
    );
    assert!(repo.exists("main.txt"), "main's own side stays");
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^"]),
        merge.to_string(),
        "the revert sits on the merge"
    );
}

/// The same operation, all the way round: undo puts the merged side back,
/// redo takes it away again.
#[test]
fn revert_of_a_merge_undoes_and_redoes() {
    let repo = merged();
    let merge = id(&repo, "HEAD");
    let op = Operation::Revert(RevertParams {
        commit: merge,
        subject: repo.git(&["log", "-1", "--format=%s", "HEAD"]),
        mainline: Some(1),
        parents: 2,
        parent: Some(id(&repo, "HEAD^1")),
        head: head_tip(&repo),
    });
    round_trip(&repo, &op);
}

/// The Ghost Preview of a merge revert says the same thing the revert then
/// does: one ghost on `HEAD`, `HEAD`'s branch moving to it, and the commands
/// that actually run.
#[test]
fn the_ghost_of_a_merge_revert_matches_git() {
    let repo = merged();
    let merge = id(&repo, "HEAD");
    let op = Operation::Revert(RevertParams {
        commit: merge,
        subject: repo.git(&["log", "-1", "--format=%s", "HEAD"]),
        mainline: Some(1),
        parents: 2,
        parent: Some(id(&repo, "HEAD^1")),
        head: head_tip(&repo),
    });
    let plan = preview_of(&repo, &op);
    assert_eq!(plan.ghosts.len(), 1, "one commit is made");
    assert_eq!(plan.ghosts[0].parents, vec![GhostTarget::Real(merge)]);
    assert_eq!(plan.ref_moves.len(), 1);
    assert_eq!(plan.ref_moves[0].name, "refs/heads/main");
    assert_eq!(plan.ref_moves[0].to, Some(GhostTarget::Ghost(0)));
    // `-m 1` is in the lines the box shows, and they are the lines that run.
    assert!(
        plan.commands.iter().any(|line| line.contains("-m 1")),
        "{:?}",
        plan.commands
    );

    run(&repo, &op, NOW);
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        plan.ghosts[0].subject,
        "the ghost's subject is the subject git wrote"
    );
    assert_eq!(
        repo.git(&["rev-parse", "HEAD^"]),
        merge.to_string(),
        "the ghost's parent is the commit git built on"
    );
}

/// `check`'s mainline rules, both ways, and the fact behind them: git 2.54
/// **accepts** `-m 1` on a non-merge (older gits refused), so this refusal
/// is Graft's own, not a repeat of git's. A merge with no mainline is
/// refused by git too.
#[test]
fn mainline_is_refused_on_a_non_merge_and_required_on_a_merge() {
    let repo = merged();
    let merge = id(&repo, "HEAD");
    let plain = id(&repo, "refs/heads/feature");
    let handle = repo.open();

    let refuse = |params: RevertParams| {
        let op = Operation::Revert(params);
        let facts = facts_for(&handle, &op);
        op.check(&facts).expect_err("this one must be refused")
    };

    // A mainline on a commit with one parent.
    assert!(matches!(
        refuse(RevertParams {
            commit: plain,
            subject: "one on feature".to_owned(),
            mainline: Some(1),
            parents: 1,
            parent: Some(id(&repo, "refs/heads/feature^")),
            head: head_tip(&repo),
        }),
        Refusal::MainlineOnNonMerge
    ));
    // A mainline a merge has not got.
    assert!(matches!(
        refuse(RevertParams {
            commit: merge,
            subject: repo.git(&["log", "-1", "--format=%s", "HEAD"]),
            mainline: Some(3),
            parents: 2,
            parent: Some(id(&repo, "HEAD^1")),
            head: head_tip(&repo),
        }),
        Refusal::MainlineOnNonMerge
    ));
    // A merge with none at all.
    assert!(matches!(
        refuse(RevertParams {
            commit: merge,
            subject: repo.git(&["log", "-1", "--format=%s", "HEAD"]),
            mainline: None,
            parents: 2,
            parent: Some(id(&repo, "HEAD^1")),
            head: head_tip(&repo),
        }),
        Refusal::MergeNeedsMainline
    ));

    // The fact the first refusal rests on: git itself would have run it.
    // A throwaway clone, so the assertion does not disturb the repository
    // the other assertions read.
    let scratch = TempRepo::init();
    scratch.write("a.txt", b"one\n");
    scratch.commit_all("one");
    scratch.write("a.txt", b"two\n");
    scratch.commit_all("two");
    assert!(
        scratch
            .try_git(&["revert", "--no-edit", "-m", "1", "HEAD"])
            .is_ok(),
        "git 2.54 accepts -m on a non-merge, so Graft's refusal is its own \
         (recorded in .scratch/m5-history-and-ghost-preview/plan-behaviours-2.txt)"
    );
}

/// The ticket's case (d): the compare-and-swap on `update-ref`, against a
/// ref that moved under it.
///
/// The drift rail (§6.7) catches this first and asks the user. This is what
/// happens when the user says go ahead anyway: the swap itself is the last
/// line of defence, it refuses, and the ref is left exactly where the other
/// writer put it.
#[test]
fn a_forced_undo_still_refuses_when_the_ref_moved_under_it() {
    let repo = diverged();
    let op = reset_op(&repo, "refs/heads/main~1", ResetMode::Hard);
    run(&repo, &op, NOW);
    let after_reset = id(&repo, "refs/heads/main");

    // Someone else moves `main` behind Graft's back — a shell, another
    // process — to a commit the entry never saw.
    repo.git(&["update-ref", "refs/heads/main", "refs/heads/feature"]);
    let moved_to = id(&repo, "refs/heads/main");
    assert_ne!(moved_to, after_reset);

    // Unforced: the rail stops it and names the drift.
    let asked = run_undo(&repo, NOW + 1, false).expect_err("drift must stop an undo");
    assert!(matches!(asked, UndoError::Drift(_)), "{}", asked.explain());
    assert_eq!(
        id(&repo, "refs/heads/main"),
        moved_to,
        "nothing was touched"
    );

    // Forced: the rail is past, and `update-ref <name> <to> <expected>`
    // refuses on its own.
    let refused = run_undo(&repo, NOW + 2, true).expect_err("the swap must refuse");
    assert!(
        matches!(
            &refused,
            UndoError::Restore(graft_ops::restore::RestoreError::RefMoved { name })
                if name == "refs/heads/main"
        ),
        "the compare-and-swap is what refused, naming the ref: {}",
        refused.explain()
    );
    assert_eq!(
        id(&repo, "refs/heads/main"),
        moved_to,
        "a refused compare-and-swap leaves the ref where the other writer put it"
    );
}

/// The ticket's case (f) — with its premise corrected.
///
/// `git stash branch` creates the branch **at the stash's own base commit**
/// and applies there, so a branch that moved under the stash cannot make it
/// conflict: verified in
/// `.scratch/m5-history-and-ghost-preview/plan-behaviours-2.txt` (f2), and
/// pinned here.
#[test]
fn branch_from_stash_starts_at_the_stashs_base_so_it_cannot_conflict() {
    let repo = diverged();
    let base = id(&repo, "refs/heads/main");
    repo.write("base.txt", b"my work in progress\n");
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    // `main` moves on, rewriting the very line the stash touches. A `stash
    // apply` here would conflict.
    repo.write("base.txt", b"someone else's line\n");
    repo.commit_all("two on main");
    assert_ne!(id(&repo, "refs/heads/main"), base);

    let op = Operation::BranchFromStash(BranchFromStashParams {
        name: "wip-branch".to_owned(),
        index: 0,
        commit: id(&repo, "refs/stash"),
    });
    run(&repo, &op, NOW + 1);

    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/wip-branch");
    assert_eq!(
        id(&repo, "refs/heads/wip-branch"),
        base,
        "the branch is at the stash's base, not at main"
    );
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "",
        "nothing is unmerged"
    );
    assert_eq!(
        repo.read("base.txt"),
        b"my work in progress\n".to_vec(),
        "the stashed work is on disk"
    );
}

/// The two ways a `stash branch` really can fail, both refused by `check`
/// before git is spawned.
///
/// The dirty-tree one is not a nicety: git creates the branch, moves `HEAD`
/// onto it and *then* fails to apply, leaving a branch that should not
/// exist and the stash still in the list
/// (`plan-behaviours-2.txt` (f4)). Refusing first is what keeps that
/// half-done state from ever being reachable.
#[test]
fn branch_from_stash_is_refused_before_git_can_half_do_it() {
    let repo = diverged();
    repo.write("base.txt", b"edited, not committed\n");
    run(
        &repo,
        &Operation::StashPush(StashParams {
            message: Some("wip".to_owned()),
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    let stash = id(&repo, "refs/stash");
    let handle = repo.open();

    // The name is already a branch.
    let taken = Operation::BranchFromStash(BranchFromStashParams {
        name: "feature".to_owned(),
        index: 0,
        commit: stash,
    });
    let facts = facts_for(&handle, &taken);
    assert!(matches!(
        taken.check(&facts).expect_err("the name is taken"),
        Refusal::RefExists { ref name } if name == "feature"
    ));

    // The working tree is dirty.
    repo.write("main.txt", b"a change made since the stash\n");
    let dirty = Operation::BranchFromStash(BranchFromStashParams {
        name: "wip-branch".to_owned(),
        index: 0,
        commit: stash,
    });
    let facts = facts_for(&handle, &dirty);
    assert!(matches!(
        dirty.check(&facts).expect_err("the tree is dirty"),
        Refusal::DirtyTree { count: 1 }
    ));

    // Neither refusal wrote anything: no branch, and the stash is still
    // there to be taken.
    assert_eq!(repo.git(&["branch", "--list", "wip-branch"]), "");
    assert_eq!(id(&repo, "refs/stash"), stash);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/main");
}

// ── the `working tree:` line's one rule ─────────────────────────────────
//
// **The line names the files that differ between the tree `HEAD` has now
// and the tree it will have once the operation lands.** Nothing else — and
// in particular not the two tips the operation happens to mention, which
// also counts `HEAD`'s own commits: they are already on disk and do not
// move. That pair was what a no-ff merge, a rebase and a cherry-pick all
// drew, and it named a file git never touched in every one of them
// (m5/32, findings from the M5 acceptance walk).
//
// Each test below draws the plan, runs the operation, and reads the bytes
// on disk before and after. The comparison is against the **files**, not
// against another `git diff`, so a mutation that puts the old pair back
// cannot make both sides wrong together.

/// The bytes of every file the `diverged` shape can have, present or not.
fn files_now(repo: &TempRepo) -> Vec<(&'static str, Option<Vec<u8>>)> {
    ["base.txt", "main.txt", "feature.txt"]
        .into_iter()
        .map(|name| {
            let bytes = repo.exists(name).then(|| repo.read(name));
            (name, bytes)
        })
        .collect()
}

/// Which of those files git actually rewrote, in name order.
fn really_changed(
    before: &[(&'static str, Option<Vec<u8>>)],
    after: &[(&'static str, Option<Vec<u8>>)],
) -> Vec<PathBuf> {
    before
        .iter()
        .zip(after)
        .filter(|((_, was), (_, now))| was != now)
        .map(|((name, _), _)| PathBuf::from(name))
        .collect()
}

/// The paths a plan drew as updated, in name order.
fn drawn(plan: &PreviewPlan) -> Vec<PathBuf> {
    let WorktreeEffect::Updated { paths, more } = &plan.worktree else {
        panic!("not an Updated effect: {:?}", plan.worktree);
    };
    assert!(!more, "the shape is too small to cap");
    let mut paths = paths.clone();
    paths.sort();
    paths
}

/// A merge that commits: the files the **source** gained since the two
/// diverged, never `into` against `source`, which would also name
/// `main.txt` — `HEAD`'s own commit, already on disk.
#[test]
fn a_no_ff_merge_names_only_the_files_the_source_brings() {
    let repo = diverged();
    let op = merge_op(&repo, "feature", FastForward::Never);
    let plan = preview_of(&repo, &op);
    assert_eq!(
        drawn(&plan),
        vec![PathBuf::from("feature.txt")],
        "the two tips against each other would add main.txt, which does not move"
    );

    let before = files_now(&repo);
    run(&repo, &op, NOW);
    assert_eq!(really_changed(&before, &files_now(&repo)), drawn(&plan));
}

/// A rebase: the files **`onto`** gained since the two diverged. The
/// branch's own commits are replayed on top, so their files are already
/// here and do not move.
#[test]
fn a_rebase_names_only_the_files_onto_brings() {
    let repo = diverged();
    let op = rebase_op(&repo, "feature");
    let plan = preview_of(&repo, &op);
    assert_eq!(
        drawn(&plan),
        vec![PathBuf::from("feature.txt")],
        "the branch against `onto` would add main.txt, which is replayed and stays"
    );

    let before = files_now(&repo);
    run(&repo, &op, NOW);
    assert_eq!(really_changed(&before, &files_now(&repo)), drawn(&plan));
}

/// A cherry-pick: each picked commit against its **own first parent**,
/// which is what the patch is. `HEAD` against the commit would name
/// everything on the other branch, picked or not.
#[test]
fn a_cherry_pick_names_only_the_picked_patch() {
    let repo = diverged();
    let op = pick_op(&repo, "feature");
    let plan = preview_of(&repo, &op);
    assert_eq!(
        drawn(&plan),
        vec![PathBuf::from("feature.txt")],
        "HEAD against the picked commit would add main.txt, which is not in the patch"
    );

    let before = files_now(&repo);
    run(&repo, &op, NOW);
    assert_eq!(really_changed(&before, &files_now(&repo)), drawn(&plan));
}

/// A fast-forward lands `HEAD` on a real commit, so its tree is the pair
/// directly — and the fork point *is* `into`, so the rule agrees with
/// itself. This is the one case where the two tips were always right, and
/// it is why the wrong pair survived: it looked right here.
#[test]
fn a_fast_forward_merge_and_the_fork_point_rule_agree() {
    let repo = ahead();
    repo.git(&["switch", "-q", "feature"]);
    let op = merge_op(&repo, "main", FastForward::Default);
    let plan = preview_of(&repo, &op);
    assert!(plan.ghosts.is_empty(), "a fast-forward draws no ghost");
    assert_eq!(drawn(&plan), vec![PathBuf::from("main.txt")]);

    let before = files_now(&repo);
    run(&repo, &op, NOW);
    assert_eq!(really_changed(&before, &files_now(&repo)), drawn(&plan));
}

// ── the preview refuses what the write refuses (m6-conflicts/67) ────────

/// One tracked file changed, nothing staged.
fn dirty(repo: &TempRepo) {
    repo.write("base.txt", b"edited, not committed\n");
}

/// A preview of a rebase on a dirty tree is drawn, and says what it would
/// set aside (ADR-0045 §4; flipped by m6-conflicts/73 from 67's refusal).
#[test]
fn a_rebase_preview_on_a_dirty_tree_says_what_is_set_aside() {
    let repo = diverged();
    repo.git(&["switch", "-q", "feature"]);
    let clean = preview_of(&repo, &rebase_op(&repo, "refs/heads/main"));
    assert_eq!(clean.set_aside, None, "a clean tree sets nothing aside");
    dirty(&repo);
    let plan = preview_of(&repo, &rebase_op(&repo, "refs/heads/main"));
    assert_eq!(
        plan.set_aside,
        Some(SetAside {
            files: 1,
            staged: 0
        })
    );
}

/// The same for a checkout (flipped by m6-conflicts/73).
#[test]
fn a_checkout_preview_on_a_dirty_tree_says_what_is_set_aside() {
    let repo = diverged();
    dirty(&repo);
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch("refs/heads/feature".to_owned()),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    });
    let plan = preview_of(&repo, &op);
    assert_eq!(
        plan.set_aside,
        Some(SetAside {
            files: 1,
            staged: 0
        })
    );
}

/// Control: `check` does not refuse a merge on a dirty tree (git refuses
/// only if the merge touches the dirty file), so its picture is still drawn.
#[test]
fn a_merge_preview_on_a_dirty_tree_still_draws() {
    let repo = diverged();
    dirty(&repo);
    let plan = preview_of(
        &repo,
        &merge_op(&repo, "refs/heads/feature", FastForward::Never),
    );
    assert!(!plan.ghosts.is_empty(), "the merge commit is drawn");
}

/// `diverged()` with the `v0.9` tag `local_kinds` deletes.
fn diverged_tagged() -> TempRepo {
    let repo = diverged();
    repo.git(&["tag", "v0.9"]);
    repo
}

/// One repository per operation `plan` draws, each built against its own
/// ids, and built once: the caller judges each case on its clean tree first,
/// then dirties the same tree and judges it again. `check` and `plan` only
/// read.
fn refusal_cases() -> Vec<(TempRepo, Operation)> {
    let mut cases: Vec<(TempRepo, Operation)> = Vec::new();

    let first = diverged_tagged();
    let kinds = local_kinds(&first).len() + 1;
    let mut first = Some(first);
    for at in 0..kinds {
        let fresh = first.take().unwrap_or_else(diverged_tagged);
        let mut ops = local_kinds(&fresh);
        ops.push(Operation::BranchCreate(BranchCreateParams {
            name: "switched".to_owned(),
            at: id(&fresh, "HEAD"),
            checkout: true,
        }));
        let op = ops.swap_remove(at);
        cases.push((fresh, op));
    }

    let rebasing = diverged();
    rebasing.git(&["switch", "-q", "feature"]);
    let rebase = rebase_op(&rebasing, "refs/heads/main");
    cases.push((rebasing, rebase));

    let stashed = diverged();
    stashed.write("base.txt", b"wip\n");
    run(
        &stashed,
        &Operation::StashPush(StashParams {
            message: None,
            scope: StashScope::All {
                include_untracked: false,
            },
        }),
        NOW,
    );
    let from_stash = Operation::BranchFromStash(BranchFromStashParams {
        name: "from-stash".to_owned(),
        index: 0,
        commit: id(&stashed, "refs/stash"),
    });
    cases.push((stashed, from_stash));
    cases
}

/// `plan` is `Err(Refused(r))` exactly when `check` is `Err(r)`, for every
/// kind `plan` draws, on a clean tree and on a dirty one. Push is left out:
/// it needs a remote, and its `check` does not read the tree.
#[test]
fn every_preview_refuses_exactly_what_check_refuses() {
    let cases = refusal_cases();
    for dirty_tree in [false, true] {
        let mut refused = 0;
        for (repo, op) in &cases {
            if dirty_tree {
                dirty(repo);
            }
            let handle = repo.open();
            let facts = facts_for(&handle, op);
            let checked = op.check(&facts);
            let planned = preview::plan(&handle, &git_version(), op, &facts, &Never);
            match (&checked, &planned) {
                (Ok(()), Ok(_)) => {}
                (Err(want), Err(preview::PreviewError::Refused(got))) => {
                    assert_eq!(got, want, "{} (dirty: {dirty_tree})", op.describe());
                    refused += 1;
                }
                _ => panic!(
                    "{} (dirty: {dirty_tree}): check {checked:?}, plan {:?}",
                    op.describe(),
                    planned.as_ref().err()
                ),
            }
        }
        // The table is not vacuous: a dirty tree refuses something.
        assert_eq!(refused > 0, dirty_tree, "refusals: {refused}");
    }
}

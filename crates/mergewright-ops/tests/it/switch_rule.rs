//! The switch rule (m7-power-tools/10b), against real repositories: a new
//! branch switched to over edits, and a remote-only branch checked out as a
//! new tracking branch.
//!
//! Every claim is read back from git or the files: the index with
//! `ls-files -s`, the staged and unstaged diffs, each file's bytes
//! (`support::state_of`), the stash list, and `branch.<b>.*` in the config.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_ops::facts::read_facts;
use mergewright_ops::operation::{
    BranchCreateParams, CheckoutParams, CheckoutTarget, OnEdits, Operation, Tip,
};
use mergewright_ops::preview::{self, ConflictForecast, PreviewPlan, WorktreeEffect};

use crate::set_aside::{last_entry, porcelain, set_aside_of};
use crate::support::{
    NOW, Never, TempRepo, git_version, run_redo, run_undo, stash_list, state_of, try_run,
};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

/// `main` and `feature` both hold `base.txt` and `a.txt`; `feature` adds
/// `feature.txt`. On `main`.
fn two_branches() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// `base.txt` staged one way and edited further unstaged, `a.txt` edited
/// unstaged, and an untracked file: every shape the staged split must keep.
fn edit(repo: &TempRepo) {
    repo.write("base.txt", b"staged\n");
    repo.git(&["add", "base.txt"]);
    repo.write("base.txt", b"staged\nand unstaged\n");
    repo.write("a.txt", b"a, edited\n");
    repo.write("junk.tmp", b"untracked\n");
}

/// The staged and unstaged diffs against `HEAD`, blob ids and modes
/// included.
fn split(repo: &TempRepo) -> (String, String) {
    (
        repo.git(&["diff", "--cached", "--no-renames"]),
        repo.git(&["diff", "--no-renames"]),
    )
}

/// `b nb ⇥` at `at`, as the preview runs it.
fn create_at(at: Oid) -> Operation {
    Operation::BranchCreate(BranchCreateParams {
        name: "nb".to_owned(),
        at,
        checkout: true,
        on_edits: OnEdits::SetAside,
    })
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

// ── a new branch switched to over edits ─────────────────────────────────

/// At `HEAD`'s own commit `switch -c` leaves index and files as they are:
/// nothing is set aside, and every byte of the tree and the index, staged
/// split included, is the same after. The preview says so. `u` takes the
/// branch away and puts `HEAD` back, the edits still untouched.
#[test]
fn switching_create_at_head_over_edits_sets_nothing_aside() {
    let repo = two_branches();
    edit(&repo);
    let op = create_at(id(&repo, "HEAD"));
    let plan = plan_of(&repo, &op);
    assert_eq!(plan.set_aside, None);
    assert_eq!(plan.put_back, None);
    assert_eq!(plan.worktree, WorktreeEffect::EditsStay);

    let before = state_of(&repo);
    let (split_before, status_before) = (split(&repo), porcelain(&repo));
    let stashes_before = stash_list(&repo);
    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.entry.put_back, None, "nothing was set aside");
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/nb");
    let after = state_of(&repo);
    assert_eq!(after.index, before.index, "the index byte-equal");
    assert_eq!(after.files, before.files, "every file byte-equal");
    assert_eq!(split(&repo), split_before, "the staged split as before");
    assert_eq!(porcelain(&repo), status_before);
    assert_eq!(stash_list(&repo), stashes_before, "no stash made");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(state_of(&repo), before, "undo put it back exactly");
}

/// At another commit the edits are set aside and put back (ADR-0045 §4):
/// on `nb` at `feature`, every edit back with its staging, the stash list
/// as before; `u` puts branch, `HEAD`, index and files back exactly.
#[test]
fn switching_create_elsewhere_over_edits_sets_them_aside_and_u_restores() {
    let repo = two_branches();
    edit(&repo);
    let op = create_at(id(&repo, "feature"));
    let before = (state_of(&repo), stash_list(&repo));
    let split_before = split(&repo);

    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    set_aside_of(&applied.entry);
    assert_eq!(repo.git(&["symbolic-ref", "HEAD"]), "refs/heads/nb");
    assert_eq!(repo.read("feature.txt"), b"feature\n", "on feature's tree");
    assert_eq!(repo.read("base.txt"), b"staged\nand unstaged\n");
    assert_eq!(repo.read("a.txt"), b"a, edited\n");
    assert_eq!(repo.read("junk.tmp"), b"untracked\n");
    assert_eq!(split(&repo), split_before, "the staged split as before");
    assert_eq!(stash_list(&repo), before.1, "no stash left");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!((state_of(&repo), stash_list(&repo)), before);
    assert!(
        repo.try_git(&["rev-parse", "--verify", "-q", "refs/heads/nb"])
            .is_err(),
        "u deleted nb"
    );
    assert_eq!(
        last_entry(&repo).description,
        format!("undo: {}", op.describe())
    );
}

/// The preview of a switching create away from `HEAD` forecasts the
/// put-back against `at` (ADR-0045 §8): clean where the edits don't meet
/// `at`'s changes, likely to clash on `base.txt` where they do.
#[test]
fn switching_create_previews_with_the_put_back_forecast() {
    let repo = two_branches();
    repo.write("a.txt", b"a, edited\n");
    let clean = plan_of(&repo, &create_at(id(&repo, "feature")));
    assert!(clean.set_aside.is_some());
    assert_eq!(clean.put_back, Some(ConflictForecast::Clean));

    // `feature` changes `base.txt` too: the edit to it clashes.
    repo.git(&["checkout", "-q", "a.txt"]);
    repo.git(&["switch", "-q", "feature"]);
    repo.write("base.txt", b"feature's line\n");
    repo.commit_all("feature changes base");
    repo.git(&["switch", "-q", "main"]);
    repo.write("base.txt", b"my edit\n");
    let clashing = plan_of(&repo, &create_at(id(&repo, "feature")));
    match clashing.put_back {
        Some(ConflictForecast::Likely { paths }) => {
            assert_eq!(paths, [std::path::PathBuf::from("base.txt")]);
        }
        other => panic!("expected a likely clash on base.txt, got {other:?}"),
    }
}

// ── a remote-only branch ────────────────────────────────────────────────

/// `up` holds `sara/x` one commit past `main`; `down` fetched it, so it has
/// `refs/remotes/origin/sara/x` and no local `sara/x`. On `main`.
fn remote_only() -> (TempRepo, TempRepo) {
    let up = TempRepo::init();
    up.write("base.txt", b"base\n");
    up.commit_all("one");
    up.git(&["switch", "-q", "-c", "sara/x"]);
    up.write("sara.txt", b"sara\n");
    up.commit_all("sara's");
    up.git(&["switch", "-q", "main"]);
    let down = TempRepo::init();
    let path = up.path().to_string_lossy().into_owned();
    down.git(&["remote", "add", "origin", &path]);
    down.git(&["fetch", "-q", "origin"]);
    down.git(&["reset", "-q", "--hard", "origin/main"]);
    // `branch.autoSetupMerge=false`: the explicit `--track` must set the
    // upstream all the same.
    down.git(&["config", "branch.autoSetupMerge", "false"]);
    (up, down)
}

fn track(repo: &TempRepo) -> Operation {
    let name = "refs/remotes/origin/sara/x".to_owned();
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Track {
            remote: Tip {
                oid: id(repo, &name),
                name,
            },
            local: "sara/x".to_owned(),
        },
        head_before: HeadRef::Branch(repo.git(&["symbolic-ref", "HEAD"])),
        on_edits: OnEdits::Refuse,
    })
}

#[test]
fn track_creates_the_local_branch_tracking_the_remote() {
    let (_up, down) = remote_only();
    let op = track(&down);
    try_run(&down, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(down.git(&["symbolic-ref", "HEAD"]), "refs/heads/sara/x");
    assert_eq!(
        id(&down, "refs/heads/sara/x"),
        id(&down, "refs/remotes/origin/sara/x")
    );
    assert_eq!(down.git(&["config", "branch.sara/x.remote"]), "origin");
    assert_eq!(
        down.git(&["config", "branch.sara/x.merge"]),
        "refs/heads/sara/x"
    );
    assert_eq!(
        down.git(&["rev-parse", "--abbrev-ref", "sara/x@{upstream}"]),
        "origin/sara/x"
    );
    assert_eq!(down.read("sara.txt"), b"sara\n");
}

/// `git config --local --get-all <key>`'s values, empty when unset.
fn config_values(repo: &TempRepo, key: &str) -> Vec<String> {
    match repo.try_git(&["config", "--local", "--get-all", key]) {
        Ok(text) => text.lines().map(str::to_owned).collect(),
        Err(_unset) => Vec::new(),
    }
}

/// Review m7-16b..21b, Major 1 (git 2.54): `u` of a Track used to leave
/// `branch.sara/x.remote/.merge`, and a later branch of that name silently
/// tracked `origin/sara/x`. Undo removes what the Track wrote and nothing
/// else; `U` writes it again.
#[test]
fn undo_of_a_track_switch_removes_the_upstream_it_wrote() {
    let (_up, down) = remote_only();
    down.git(&["config", "branch.main.remote", "origin"]);
    down.git(&["config", "branch.main.merge", "refs/heads/main"]);
    try_run(&down, &track(&down), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    run_undo(&down, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        config_values(&down, "branch.sara/x.remote"),
        Vec::<String>::new(),
        "git {:?}",
        git_version()
    );
    assert_eq!(
        config_values(&down, "branch.sara/x.merge"),
        Vec::<String>::new()
    );
    assert_eq!(config_values(&down, "branch.main.remote"), ["origin"]);
    assert_eq!(
        config_values(&down, "branch.main.merge"),
        ["refs/heads/main"]
    );
    // The consequence the review measured: a later plain branch of the same
    // name has no upstream.
    down.git(&["branch", "sara/x", "HEAD"]);
    assert!(
        down.try_git(&["rev-parse", "--abbrev-ref", "sara/x@{upstream}"])
            .is_err()
    );
    down.git(&["branch", "-D", "sara/x"]);

    run_redo(&down, NOW + 2, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(config_values(&down, "branch.sara/x.remote"), ["origin"]);
    assert_eq!(
        config_values(&down, "branch.sara/x.merge"),
        ["refs/heads/sara/x"]
    );
}

/// Config that was there before the Track — a stale upstream left for the
/// same name — is what undo puts back, not a blank.
#[test]
fn a_pre_existing_upstream_for_the_same_name_survives_undo() {
    let (_up, down) = remote_only();
    down.git(&["config", "branch.sara/x.remote", "upstream"]);
    down.git(&["config", "branch.sara/x.merge", "refs/heads/elsewhere"]);
    try_run(&down, &track(&down), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(config_values(&down, "branch.sara/x.remote"), ["origin"]);
    run_undo(&down, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(config_values(&down, "branch.sara/x.remote"), ["upstream"]);
    assert_eq!(
        config_values(&down, "branch.sara/x.merge"),
        ["refs/heads/elsewhere"]
    );
}

/// `u` deletes the branch it made and puts `HEAD` back on `main`; the
/// remote-tracking ref is left alone. The `branch.sara/x.*` lines go too
/// (`undo_of_a_track_switch_removes_the_upstream_it_wrote`).
#[test]
fn track_undo_deletes_the_branch_and_puts_head_back() {
    let (_up, down) = remote_only();
    let before = state_of(&down);
    try_run(&down, &track(&down), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    run_undo(&down, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(
        state_of(&down),
        before,
        "refs, HEAD, index and files as before"
    );
    assert!(
        down.try_git(&["rev-parse", "--verify", "-q", "refs/heads/sara/x"])
            .is_err()
    );
}

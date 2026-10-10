//! The rewrite engine's planning half (ADR-0047), against real repositories:
//! the refusals in the words the status line shows, the line of branches
//! built on his, the range a rewrite opens on, and where a stopped rebase
//! is for the banner.
//!
//! Nothing here runs a rewrite: that is `rewrite_oracle`'s (rewrite-engine-b).

// The scaffold helpers are not `#[test]` functions, so `clippy.toml`'s
// "expect is fine in tests" does not reach them; a scaffold that cannot build
// its repository has to stop the test loudly (as `support` does).
#![allow(clippy::expect_used, reason = "a failing test should panic")]

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::GitVersion;
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Refusal, RepoFacts, Tip};
use graft_ops::rewrite::{
    Dependents, RewriteError, RewriteRequest, TodoAction, dependents, plan, progress,
};

use crate::support::{Never, TempRepo, git_version};

/// A `Stop` that always stops.
struct Always;

impl Stop for Always {
    fn should_stop(&self) -> bool {
        true
    }
}

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

fn tip(repo: &TempRepo, branch: &str) -> Tip {
    Tip {
        name: format!("refs/heads/{branch}"),
        oid: oid(repo, branch),
    }
}

/// One commit changing its own file, so no two commits conflict.
fn commit(repo: &TempRepo, name: &str) {
    repo.write(&format!("{name}.txt"), format!("{name}\n").as_bytes());
    repo.commit_all(name);
}

/// `main` with two commits; `feature-a` two on it; `feature-b` one on
/// `feature-a`; `feature-c` one on `feature-b`. `HEAD` on `feature-a`.
fn stacked() -> TempRepo {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    commit(&repo, "m2");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    commit(&repo, "a2");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "-c", "feature-c"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "feature-a"]);
    repo
}

fn facts(repo: &RepoHandle) -> RepoFacts {
    read_facts(repo, git_version(), &Never, &FactsScope::default()).expect("the facts read")
}

fn plan_of(
    repo: &TempRepo,
    req: &RewriteRequest,
) -> Result<graft_ops::rewrite::RewriteParams, RewriteError> {
    plan_with(repo, &git_version(), req)
}

fn plan_with(
    repo: &TempRepo,
    git: &GitVersion,
    req: &RewriteRequest,
) -> Result<graft_ops::rewrite::RewriteParams, RewriteError> {
    let handle = repo.open();
    plan(&handle, git, req, &facts(&handle), None, &Never)
}

/// The refusal a plan came back with, in the status line's words.
fn refused(result: Result<graft_ops::rewrite::RewriteParams, RewriteError>) -> Refusal {
    match result {
        Err(RewriteError::Refused(refusal)) => refusal,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn names(line: &Dependents) -> Vec<String> {
    match line {
        Dependents::Line(line) => line.branches.iter().map(|tip| tip.name.clone()).collect(),
        Dependents::Fork { .. } => panic!("expected a line, got {line:?}"),
    }
}

// ── Dependents ──────────────────────────────────────────────────────────

#[test]
fn a_teammate_branch_on_top_is_a_dependent() {
    let repo = stacked();
    let found = dependents(
        &repo.open(),
        &git_version(),
        &tip(&repo, "feature-a"),
        None,
        &Never,
    )
    .expect("the line reads");
    assert_eq!(
        names(&found),
        [
            "refs/heads/feature-a",
            "refs/heads/feature-b",
            "refs/heads/feature-c"
        ],
        "his branch first, then each built on the one below"
    );
}

#[test]
fn main_containing_the_branch_is_not_a_dependent() {
    let repo = stacked();
    // main fast-forwarded past feature-a: its first-parent chain reaches
    // feature-a's tip with no merge, and it still is no branch of his line.
    repo.git(&["branch", "-f", "main", "feature-b"]);
    let found = dependents(
        &repo.open(),
        &git_version(),
        &tip(&repo, "feature-a"),
        None,
        &Never,
    )
    .expect("the line reads");
    assert!(
        !names(&found).contains(&"refs/heads/main".to_owned()),
        "{found:?}"
    );
}

#[test]
fn a_branch_merged_long_ago_is_not_a_dependent() {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "side"]);
    commit(&repo, "s1");
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    // `old` took feature-a in once, then merged `side` in, then went on: a
    // merge between it and feature-a's tip.
    repo.git(&["switch", "-q", "-c", "old", "side"]);
    repo.git(&["merge", "-q", "--no-ff", "--no-edit", "feature-a"]);
    commit(&repo, "o1");
    repo.git(&["switch", "-q", "feature-a"]);
    let found = dependents(
        &repo.open(),
        &git_version(),
        &tip(&repo, "feature-a"),
        None,
        &Never,
    )
    .expect("the line reads");
    assert_eq!(names(&found), ["refs/heads/feature-a"]);
}

#[test]
fn a_backup_at_the_same_commit_is_not_a_dependent() {
    let repo = stacked();
    repo.git(&["branch", "feature-a-backup", "feature-a"]);
    let found = dependents(
        &repo.open(),
        &git_version(),
        &tip(&repo, "feature-a"),
        None,
        &Never,
    )
    .expect("the line reads");
    assert!(
        !names(&found).contains(&"refs/heads/feature-a-backup".to_owned()),
        "a second name for the same commit has nothing built on his: {found:?}"
    );
}

#[test]
fn dependents_honour_stop() {
    let repo = stacked();
    let result = dependents(
        &repo.open(),
        &git_version(),
        &tip(&repo, "feature-a"),
        None,
        &Always,
    );
    assert!(
        matches!(result, Err(graft_ops::facts::FactsError::Stopped)),
        "{result:?}"
    );
}

// ── Refusals ────────────────────────────────────────────────────────────

#[test]
fn a_fork_is_refused_naming_both() {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "feature-a"]);
    commit(&repo, "a1");
    repo.git(&["switch", "-q", "-c", "feature-b"]);
    commit(&repo, "b1");
    repo.git(&["switch", "-q", "-c", "feature-c", "feature-a"]);
    commit(&repo, "c1");
    repo.git(&["switch", "-q", "feature-a"]);
    let from = oid(&repo, "feature-a");
    let refusal = refused(plan_of(&repo, &RewriteRequest::Open { from }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "feature-b and feature-c are both built on yours · Tributary moves one line"
    );
}

#[test]
fn a_dependent_in_another_worktree_is_refused_with_its_path() {
    let repo = stacked();
    let elsewhere = repo.path().with_extension("wt-b");
    repo.git(&[
        "worktree",
        "add",
        "-q",
        elsewhere.to_str().expect("a UTF-8 temp path"),
        "feature-b",
    ]);
    let from = oid(&repo, "feature-a");
    let words =
        refused(plan_of(&repo, &RewriteRequest::Open { from })).explain(&crate::support::Letters);
    std::fs::remove_dir_all(&elsewhere).expect("the other worktree clears");
    assert!(
        words.starts_with("feature-b is checked out in another worktree (")
            && words.ends_with(") · it can't follow"),
        "{words}"
    );
    let name = elsewhere
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a name");
    assert!(words.contains(name), "the path is named: {words}");
}

#[test]
fn a_merge_in_range_is_refused() {
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "side"]);
    commit(&repo, "s1");
    repo.git(&["switch", "-q", "-c", "feature-a", "main"]);
    commit(&repo, "a1");
    repo.git(&["merge", "-q", "--no-ff", "--no-edit", "side"]);
    let merge = oid(&repo, "HEAD");
    commit(&repo, "a2");
    let from = oid(&repo, "HEAD");
    let refusal = refused(plan_of(&repo, &RewriteRequest::Open { from }));
    assert_eq!(refusal, Refusal::MergeInRange { merge });
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!("can't rewrite past a merge ({})", &merge.to_string()[..7])
    );
}

#[test]
fn a_commit_off_your_branch_is_refused() {
    let repo = stacked();
    repo.git(&["switch", "-q", "-c", "other", "main"]);
    commit(&repo, "o1");
    let off = oid(&repo, "other");
    repo.git(&["switch", "-q", "feature-a"]);
    let refusal = refused(plan_of(&repo, &RewriteRequest::Open { from: off }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!(
            "R rewrites your branch · {} isn't on it",
            &off.to_string()[..7]
        )
    );
}

#[test]
fn a_into_a_commit_off_your_branch_is_refused_in_as_words() {
    // Note R2: `a` gets its own words, not `R`'s (review 02, Major 2).
    let repo = stacked();
    repo.git(&["switch", "-q", "-c", "other", "main"]);
    commit(&repo, "o1");
    let off = oid(&repo, "other");
    repo.git(&["switch", "-q", "feature-a"]);
    repo.write("staged.txt", b"staged\n");
    repo.git(&["add", "staged.txt"]);
    let refusal = refused(plan_of(&repo, &RewriteRequest::AmendInto { target: off }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!(
            "a puts staged changes into a commit on your branch · {} isn't on it",
            &off.to_string()[..7]
        )
    );
}

#[test]
fn detached_head_is_refused() {
    let repo = stacked();
    repo.git(&["switch", "-q", "--detach", "feature-a"]);
    let from = oid(&repo, "HEAD");
    let refusal = refused(plan_of(&repo, &RewriteRequest::Open { from }));
    assert_eq!(refusal, Refusal::NotOnABranch);
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "you're not on a branch · switch first (C)"
    );
}

#[test]
fn amend_into_with_nothing_staged_is_refused() {
    let repo = stacked();
    let target = oid(&repo, "feature-a~1");
    let refusal = refused(plan_of(&repo, &RewriteRequest::AmendInto { target }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!(
            "nothing staged to put into {} · space stages",
            &target.to_string()[..7]
        )
    );
}

#[test]
fn an_old_git_refuses_only_the_chain() {
    let old = GitVersion {
        major: 2,
        minor: 37,
        patch: 0,
    };
    let repo = stacked();
    let from = oid(&repo, "feature-a");
    let refusal = refused(plan_with(&repo, &old, &RewriteRequest::Open { from }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "branches built on yours can follow only with git 2.38 or newer"
    );
    // The top of the line has nothing built on it: the same git rewrites it.
    repo.git(&["switch", "-q", "feature-c"]);
    let from = oid(&repo, "feature-c");
    let planned =
        plan_with(&repo, &old, &RewriteRequest::Open { from }).expect("no chain, no refusal");
    assert_eq!(planned.line.branches.len(), 1);
}

#[test]
fn amend_into_head_with_nothing_built_on_it_is_a_plain_amend() {
    let repo = stacked();
    repo.git(&["switch", "-q", "feature-c"]);
    repo.write("c1.txt", b"changed\n");
    repo.git(&["add", "c1.txt"]);
    let target = oid(&repo, "HEAD");
    let refusal = refused(plan_of(&repo, &RewriteRequest::AmendInto { target }));
    assert_eq!(refusal, Refusal::PlainAmend);
}

// ── The range ───────────────────────────────────────────────────────────

#[test]
fn open_runs_from_head_down_to_where_the_branch_left_main() {
    let repo = stacked();
    let from = oid(&repo, "feature-a");
    let planned = plan_of(&repo, &RewriteRequest::Open { from }).expect("it plans");
    let subjects: Vec<&str> = planned
        .todo
        .iter()
        .map(|line| line.subject.as_str())
        .collect();
    assert_eq!(
        subjects,
        ["a1", "a2"],
        "oldest first, main's commits not rows"
    );
    assert_eq!(planned.onto, oid(&repo, "main"));
    assert!(
        planned
            .todo
            .iter()
            .all(|line| line.action == TodoAction::Pick)
    );
    assert_eq!(
        planned.line.branches.len(),
        3,
        "feature-b and feature-c follow"
    );
    assert_eq!(planned.head, tip(&repo, "feature-a"));
}

#[test]
fn the_default_branch_override_wins() {
    // `develop` off `main`, his branch off `develop`. Resolved on its own,
    // the default branch is `main`; the app's override says `develop`.
    let repo = TempRepo::init();
    commit(&repo, "m1");
    repo.git(&["switch", "-q", "-c", "develop"]);
    commit(&repo, "d1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    commit(&repo, "f1");
    let handle = repo.open();
    let from = oid(&repo, "feature");
    let subjects = |planned: &graft_ops::rewrite::RewriteParams| -> Vec<String> {
        planned
            .todo
            .iter()
            .map(|line| line.subject.clone())
            .collect()
    };

    let resolved = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open { from },
        &facts(&handle),
        None,
        &Never,
    )
    .expect("it plans");
    assert_eq!(subjects(&resolved), ["d1", "f1"]);
    assert_eq!(resolved.onto, oid(&repo, "main"));

    let develop = RefName::new("refs/heads/develop").expect("a ref name");
    let overridden = plan(
        &handle,
        &git_version(),
        &RewriteRequest::Open { from },
        &facts(&handle),
        Some(&develop),
        &Never,
    )
    .expect("it plans");
    assert_eq!(subjects(&overridden), ["f1"]);
    assert_eq!(overridden.onto, oid(&repo, "develop"));
}

#[test]
fn open_deeper_than_the_base_reaches_from() {
    let repo = stacked();
    let from = oid(&repo, "main~1");
    let planned = plan_of(&repo, &RewriteRequest::Open { from }).expect("it plans");
    let subjects: Vec<&str> = planned
        .todo
        .iter()
        .map(|line| line.subject.as_str())
        .collect();
    // m1 is the first commit: nothing to replay it onto, so it is the base.
    assert_eq!(subjects, ["m2", "a1", "a2"]);
    assert_eq!(planned.onto, from);
}

#[test]
fn open_premarks_autosquash_rows_with_a_default_message() {
    let repo = stacked();
    repo.git(&["switch", "-q", "feature-c"]);
    commit(&repo, "fixup! b1");
    repo.write("x.txt", b"x\n");
    repo.git(&["add", "x.txt"]);
    repo.git(&["commit", "-q", "-m", "squash! c1", "-m", "more about c1"]);
    let from = oid(&repo, "HEAD");
    let planned = plan_of(&repo, &RewriteRequest::Open { from }).expect("it plans");
    let shape: Vec<(TodoAction, &str)> = planned
        .todo
        .iter()
        .map(|line| (line.action, line.subject.as_str()))
        .collect();
    assert_eq!(
        shape,
        [
            (TodoAction::Pick, "a1"),
            (TodoAction::Pick, "a2"),
            (TodoAction::Pick, "b1"),
            (TodoAction::Fixup, "fixup! b1"),
            (TodoAction::Pick, "c1"),
            (TodoAction::Squash, "squash! c1"),
        ]
    );
    assert_eq!(
        planned.messages.get(&4).map(String::as_str),
        Some("c1\n\nsquash! c1\n\nmore about c1\n"),
        "the squash group's default message; the fixup-only group has none"
    );
    assert_eq!(planned.messages.len(), 1);
}

#[test]
fn amend_into_plans_the_index_as_a_fixup_after_its_target() {
    let repo = stacked();
    repo.write("a1.txt", b"a1, fixed\n");
    repo.git(&["add", "a1.txt"]);
    let target = oid(&repo, "feature-a~1");
    let head = oid(&repo, "HEAD");
    let planned = plan_of(&repo, &RewriteRequest::AmendInto { target }).expect("it plans");
    let [made] = planned.made.as_slice() else {
        panic!("one commit made: {:?}", planned.made);
    };
    assert_eq!(planned.onto, oid(&repo, "main"));
    let shape: Vec<(TodoAction, Oid)> = planned
        .todo
        .iter()
        .map(|line| (line.action, line.commit))
        .collect();
    assert_eq!(
        shape,
        [
            (TodoAction::Pick, target),
            (TodoAction::Fixup, *made),
            (TodoAction::Pick, head),
        ]
    );
    // The made commit's parent is HEAD and it holds the index; no ref names
    // it, and HEAD did not move.
    assert_eq!(
        repo.git(&["rev-parse", &format!("{made}^")]),
        head.to_string()
    );
    assert_eq!(repo.git(&["show", &format!("{made}:a1.txt")]), "a1, fixed");
    assert_eq!(oid(&repo, "HEAD"), head);
    assert_eq!(
        repo.git(&["for-each-ref", "--points-at", &made.to_string()]),
        ""
    );
}

// ── Progress ────────────────────────────────────────────────────────────

/// Start `git rebase -i` in the shell with `todo` as the whole todo.
fn rebase_with_todo(repo: &TempRepo, todo: &str, args: &[&str]) {
    let file = repo.path().with_extension("todo");
    std::fs::write(&file, todo).expect("the todo is written");
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(["rebase", "-i", "--no-autostash"])
        .args(args)
        .env(
            "GIT_SEQUENCE_EDITOR",
            format!("cp '{}'", file.to_str().expect("a UTF-8 temp path")),
        )
        .env("GIT_EDITOR", "true")
        .output()
        .expect("git runs");
    std::fs::remove_file(&file).expect("the todo clears");
    // An `edit` stop exits 0; anything else stopping here is the test's
    // mistake.
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn progress_of_a_shell_rebase_has_no_branch() {
    let repo = stacked();
    let (a1, a2) = (oid(&repo, "feature-a~1"), oid(&repo, "feature-a"));
    rebase_with_todo(&repo, &format!("edit {a1} a1\npick {a2} a2\n"), &["main"]);
    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");
    assert_eq!(found.step, (1, 2));
    assert_eq!(found.current.map(|line| line.commit), Some(a1));
    assert_eq!(found.branch, None);
    assert!(!found.begun_in_graft);
}

#[test]
fn progress_lists_pending_oldest_first() {
    let repo = stacked();
    repo.git(&["switch", "-q", "feature-c"]);
    let ids: Vec<Oid> = ["feature-a~1", "feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|rev| oid(&repo, rev))
        .collect();
    // A short id, as `--edit-todo` by hand may leave one.
    let short = &ids[3].to_string()[..9];
    rebase_with_todo(
        &repo,
        &format!(
            "edit {} a1\npick {} a2\npick {} b1\npick {short} c1\n",
            ids[0], ids[1], ids[2]
        ),
        &["main"],
    );
    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");
    let pending: Vec<(TodoAction, Oid)> = found
        .pending
        .iter()
        .map(|line| (line.action, line.commit))
        .collect();
    assert_eq!(
        pending,
        [
            (TodoAction::Pick, ids[1]),
            (TodoAction::Pick, ids[2]),
            (TodoAction::Pick, ids[3]),
        ]
    );
    assert_eq!(found.step, (1, 4));
}

#[test]
fn progress_names_the_branch_of_the_step() {
    let repo = stacked();
    let ids: Vec<Oid> = ["feature-a~1", "feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|rev| oid(&repo, rev))
        .collect();
    // The plan a Graft rewrite saves before its Pending line, on disk as a
    // relaunch reads it (the format is a contract: a newer Graft reads what
    // an older one left mid-rewrite).
    let dir = repo
        .path()
        .join(".git/tributary/rewrite/1700000000000-0000");
    std::fs::create_dir_all(&dir).expect("the plan dir is made");
    let plan_json = format!(
        r#"{{"home":"refs/heads/feature-a","line":[["refs/heads/feature-a","{}"],["refs/heads/feature-b","{}"],["refs/heads/feature-c","{}"]],"onto":"{}"}}"#,
        ids[1],
        ids[2],
        ids[3],
        oid(&repo, "main")
    );
    std::fs::write(dir.join("plan.json"), plan_json).expect("the plan is written");
    rebase_with_todo(
        &repo,
        &format!(
            "pick {} a1\npick {} a2\nupdate-ref refs/heads/feature-a\nedit {} b1\n\
             update-ref refs/heads/feature-b\npick {} c1\n",
            ids[0], ids[1], ids[2], ids[3]
        ),
        &["main", "feature-c"],
    );
    let found = progress(&repo.open(), &Never)
        .expect("it reads")
        .expect("a rebase is stopped");
    let (name, index, of) = found.branch.expect("a line of three");
    assert_eq!(
        (
            String::from_utf8_lossy(name.shorthand()).into_owned(),
            index,
            of
        ),
        ("feature-b".to_owned(), 2, 3)
    );
    assert!(found.begun_in_graft);
    assert_eq!(
        found.step,
        (3, 4),
        "update-ref lines are not steps (git's own msgnum/end say 4/6 here)"
    );
    assert_eq!(
        (
            repo.read(".git/rebase-merge/msgnum"),
            repo.read(".git/rebase-merge/end")
        ),
        (b"4\n".to_vec(), b"6\n".to_vec()),
        "measured: git counts update-ref lines, which is why progress does not read them"
    );
    assert_eq!(found.current.map(|line| line.commit), Some(ids[2]));
}

#[test]
fn no_rebase_no_progress() {
    let repo = stacked();
    assert_eq!(progress(&repo.open(), &Never).expect("it reads"), None);
}

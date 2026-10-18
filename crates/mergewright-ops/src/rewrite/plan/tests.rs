//! `FromRebase` against a real repository: the editor `R` opens from a
//! rebase preview runs, unedited, exactly the rebase's own todo.

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;

use crate::facts::read_facts;
use crate::operation::{FactsScope, RebaseParams, Refusal, RepoFacts, Source, Tip};
use crate::rewrite::{
    BranchLine, RewriteError, RewriteParams, RewriteRequest, TodoAction, TodoLine,
};

use super::plan;

struct Never;

impl Stop for Never {
    fn should_stop(&self) -> bool {
        false
    }
}

/// A scratch repository, removed on drop.
struct Scratch(std::path::PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort: a left-over scratch directory is noise, not a
        // failure, and a panic in `Drop` would hide the real one.
        let _removed = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("graft-from-rebase-{name}-{}", std::process::id()));
        // A directory left by an earlier run of this very test: start
        // clean; absent is the usual case.
        let _stale = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("the scratch directory");
        let scratch = Scratch(dir);
        scratch.git(&["init", "-q", "-b", "main"]);
        scratch.git(&["config", "user.name", "T"]);
        scratch.git(&["config", "user.email", "t@example.com"]);
        scratch.git(&["config", "commit.gpgsign", "false"]);
        scratch
    }

    fn git(&self, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(&self.0)
            .output()
            .expect("git runs");
        assert!(out.status.success(), "git {args:?}: {out:?}");
        String::from_utf8(out.stdout)
            .expect("utf-8")
            .trim()
            .to_owned()
    }

    fn commit(&self, name: &str) {
        std::fs::write(self.0.join(format!("{name}.txt")), name).expect("write");
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "-m", name]);
    }

    fn oid(&self, rev: &str) -> Oid {
        Oid::from_hex(&self.git(&["rev-parse", rev])).expect("an id")
    }

    fn tip(&self, branch: &str) -> Tip {
        Tip {
            name: format!("refs/heads/{branch}"),
            oid: self.oid(branch),
        }
    }

    fn handle(&self) -> RepoHandle {
        RepoHandle::open(&self.0).expect("it opens")
    }
}

fn git_version() -> mergewright_git::version::GitVersion {
    mergewright_git::version::system_git()
        .expect("git is on PATH")
        .version()
}

fn facts(repo: &RepoHandle) -> RepoFacts {
    read_facts(repo, git_version(), &Never, &FactsScope::default()).expect("the facts read")
}

/// `feature` (a1 a2 a3) off m1, `main` moved on by m2; `HEAD` on
/// `feature`. The rebase of `feature` onto `main`, a1 and a3 dropped.
fn rebase_of_feature(name: &str) -> (Scratch, RebaseParams) {
    let repo = Scratch::new(name);
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.commit("a1");
    repo.commit("a2");
    repo.commit("a3");
    repo.git(&["switch", "-q", "main"]);
    repo.commit("m2");
    repo.git(&["switch", "-q", "feature"]);
    let mut params = RebaseParams::new(
        repo.tip("feature"),
        Source::Branch("refs/heads/main".to_owned()),
        repo.oid("main"),
    );
    params.drops = vec![repo.oid("feature~2"), repo.oid("feature")];
    (repo, params)
}

fn plan_from(repo: &Scratch, params: &RebaseParams) -> Result<RewriteParams, RewriteError> {
    let handle = repo.handle();
    plan(
        &handle,
        &git_version(),
        &RewriteRequest::FromRebase(params.clone()),
        &facts(&handle),
        None,
        &Never,
    )
}

#[test]
fn from_rebase_unedited_matches_the_rebase_todo() {
    let (repo, params) = rebase_of_feature("matches");

    let planned = plan_from(&repo, &params).expect("it plans");

    // What `rebase -i main` lists, read here by git itself, oldest
    // first, each drop marked `drop`.
    let listed = repo.git(&["rev-list", "--reverse", "main..feature"]);
    let expected: Vec<TodoLine> = listed
        .lines()
        .zip(["a1", "a2", "a3"])
        .map(|(oid, subject)| {
            let commit = Oid::from_hex(oid).expect("an id");
            TodoLine {
                action: if params.drops.contains(&commit) {
                    TodoAction::Drop
                } else {
                    TodoAction::Pick
                },
                commit,
                subject: subject.to_owned(),
            }
        })
        .collect();
    assert_eq!(planned.todo, expected);
    assert_eq!(
        planned
            .todo
            .iter()
            .map(|row| row.action)
            .collect::<Vec<_>>(),
        [TodoAction::Drop, TodoAction::Pick, TodoAction::Drop]
    );
    assert_eq!(planned.onto, params.onto_tip);
    assert_eq!(planned.line, params.line);
    assert_eq!(planned.head, params.branch);
    assert!(planned.messages.is_empty());
    assert!(planned.made.is_empty());
    assert_eq!(planned.index_base, None);
}

#[test]
fn from_rebase_keeps_the_line_as_given() {
    let (repo, mut params) = rebase_of_feature("line");
    repo.git(&["branch", "on-top", "feature"]);
    params.line = BranchLine {
        branches: vec![repo.tip("feature"), repo.tip("on-top")],
    };

    let planned = plan_from(&repo, &params).expect("it plans");

    assert_eq!(planned.line, params.line);
}

#[test]
fn from_rebase_refuses_when_head_moved() {
    let (repo, params) = rebase_of_feature("moved");
    repo.commit("a4");

    let refused = plan_from(&repo, &params);

    assert!(
        matches!(
            &refused,
            Err(RewriteError::Refused(Refusal::Moved { name })) if name == "refs/heads/feature"
        ),
        "{refused:?}"
    );
}

#[test]
fn from_rebase_refuses_when_head_switched() {
    let (repo, params) = rebase_of_feature("switched");
    repo.git(&["switch", "-q", "-c", "other"]);

    let refused = plan_from(&repo, &params);

    assert!(
        matches!(
            &refused,
            Err(RewriteError::Refused(Refusal::Moved { name })) if name == "refs/heads/feature"
        ),
        "{refused:?}"
    );
}

#[test]
fn from_rebase_refuses_detached() {
    let (repo, params) = rebase_of_feature("detached");
    repo.git(&["switch", "-q", "--detach"]);
    let handle = repo.handle();
    assert!(
        matches!(facts(&handle).head, HeadRef::Detached(_)),
        "HEAD is detached"
    );

    let refused = plan_from(&repo, &params);

    assert!(
        matches!(&refused, Err(RewriteError::Refused(Refusal::NotOnABranch))),
        "{refused:?}"
    );
}

//! Fetch, pull and push: every refusal one fact away from a passing
//! baseline, the exact command lines, `touches`, and `preview == plan`.

use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::remote::{Remote, TrackedUpstream};
use graft_git::state::InProgress;
use graft_git::status::WorkingTreeStatus;
use graft_git::version::GitVersion;

use super::super::{Invocation, Operation, Refusal, RepoFacts};
use super::{FetchParams, Lease, PullParams, PushParams};

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

fn origin() -> Remote {
    Remote {
        name: "origin".to_owned(),
        fetch_url: "file:///srv/repo.git".to_owned(),
        push_url: None,
    }
}

fn upstream() -> TrackedUpstream {
    TrackedUpstream {
        remote: "origin".to_owned(),
        remote_branch: "refs/heads/main".to_owned(),
        tracking_ref: "refs/remotes/origin/main".to_owned(),
        tracking_tip: Some(oid(7)),
    }
}

/// On `main`, tracking `origin/main`, one remote.
fn facts() -> RepoFacts {
    RepoFacts {
        bare: false,
        head: HeadRef::Branch("refs/heads/main".to_owned()),
        in_progress: None,
        status: WorkingTreeStatus { entries: vec![] },
        stashes: vec![],
        git: GitVersion {
            major: 2,
            minor: 40,
            patch: 0,
        },
        remotes: vec![origin()],
        upstream: Some(upstream()),
    }
}

fn fetch(remote: Option<&str>, prune: bool) -> Operation {
    Operation::Fetch(FetchParams {
        remote: remote.map(str::to_owned),
        prune,
    })
}

fn pull() -> Operation {
    Operation::Pull(PullParams {
        branch: "refs/heads/main".to_owned(),
        upstream: upstream(),
    })
}

fn push_params() -> PushParams {
    PushParams {
        branch: "refs/heads/main".to_owned(),
        remote: "origin".to_owned(),
        remote_branch: "refs/heads/main".to_owned(),
        set_upstream: false,
        lease: None,
    }
}

fn lines(op: &Operation) -> Vec<String> {
    op.plan(&facts())
        .iter()
        .map(Invocation::command_line)
        .collect()
}

#[test]
fn kinds_and_is_remote() {
    assert_eq!(fetch(None, true).kind(), OperationKind::Fetch);
    assert_eq!(pull().kind(), OperationKind::Pull);
    assert_eq!(Operation::Push(push_params()).kind(), OperationKind::Push);
    assert!(fetch(None, true).is_remote());
    assert!(pull().is_remote());
    assert!(Operation::Push(push_params()).is_remote());
    let commit = Operation::Commit(crate::operation::CommitParams {
        message: "x".to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
    });
    assert!(!commit.is_remote());
    assert_eq!(commit.preview(), None);
}

#[test]
fn describe_names_each_remote_operation() {
    assert_eq!(fetch(None, true).describe(), "fetch all remotes");
    assert_eq!(fetch(Some("origin"), false).describe(), "fetch origin");
    assert_eq!(pull().describe(), "pull origin/main");
    assert_eq!(
        Operation::Push(push_params()).describe(),
        "push main → origin/main"
    );
    let set = PushParams {
        set_upstream: true,
        ..push_params()
    };
    assert_eq!(
        Operation::Push(set).describe(),
        "push main → origin/main (set upstream)"
    );
    let lease = PushParams {
        lease: Some(Lease { expected: oid(1) }),
        ..push_params()
    };
    assert_eq!(
        Operation::Push(lease).describe(),
        "force-push main → origin/main (with lease)"
    );
}

#[test]
fn fetch_plans() {
    assert_eq!(
        lines(&fetch(None, true)),
        ["git fetch --progress --prune --all"]
    );
    assert_eq!(
        lines(&fetch(Some("origin"), false)),
        ["git fetch --progress origin"]
    );
    assert!(fetch(None, true).plan(&facts()).iter().all(|i| i.remote));
}

#[test]
fn pull_plans_fetch_then_local_fast_forward() {
    assert_eq!(
        lines(&pull()),
        [
            "git fetch --progress origin",
            "git merge --ff-only --no-edit refs/remotes/origin/main"
        ]
    );
    let remote: Vec<bool> = pull().plan(&facts()).iter().map(|i| i.remote).collect();
    assert_eq!(remote, [true, false]);
}

#[test]
fn push_plans() {
    assert_eq!(
        lines(&Operation::Push(push_params())),
        ["git push --progress origin refs/heads/main:refs/heads/main"]
    );
    let both = PushParams {
        set_upstream: true,
        lease: Some(Lease {
            expected: oid(0xab),
        }),
        ..push_params()
    };
    assert_eq!(
        lines(&Operation::Push(both)),
        [format!(
            "git push --progress -u --force-with-lease=refs/heads/main:{} --force-if-includes \
             origin refs/heads/main:refs/heads/main",
            "ab".repeat(20)
        )]
    );
}

#[test]
fn preview_matches_plan() {
    let ops = [
        fetch(None, true),
        fetch(Some("origin"), false),
        pull(),
        Operation::Push(push_params()),
        Operation::Push(PushParams {
            set_upstream: true,
            lease: Some(Lease { expected: oid(3) }),
            ..push_params()
        }),
    ];
    for op in &ops {
        assert_eq!(op.preview(), Some(lines(op)), "{op:?}");
    }
}

#[test]
fn touches() {
    let f = facts();
    assert_eq!(
        fetch(None, true).touches(&f),
        Touches {
            refs: vec![],
            index: false,
            worktree: false
        }
    );
    // The tracking ref is never in a pull's touches: undo must not move it.
    assert_eq!(
        pull().touches(&f),
        Touches {
            refs: vec!["refs/heads/main".to_owned()],
            index: true,
            worktree: true
        }
    );
    assert_eq!(
        Operation::Push(push_params()).touches(&f),
        Touches {
            refs: vec!["refs/remotes/origin/main".to_owned()],
            index: false,
            worktree: false
        }
    );
    // A push to a branch the head's upstream is not: the tracking ref is
    // not known, so none is recorded.
    let elsewhere = PushParams {
        remote_branch: "refs/heads/other".to_owned(),
        ..push_params()
    };
    assert!(Operation::Push(elsewhere).touches(&f).refs.is_empty());
}

#[test]
fn fetch_is_allowed_bare_and_mid_operation() {
    let bare = RepoFacts {
        bare: true,
        head: HeadRef::Detached(oid(1)),
        upstream: None,
        ..facts()
    };
    assert_eq!(fetch(None, true).check(&bare), Ok(()));
    let rebasing = RepoFacts {
        in_progress: Some(InProgress::Rebase),
        ..facts()
    };
    assert_eq!(fetch(None, true).check(&rebasing), Ok(()));
    // Pull and push keep the M3 rules.
    assert_eq!(pull().check(&bare), Err(Refusal::Bare));
    assert!(matches!(
        pull().check(&rebasing),
        Err(Refusal::InProgress(_))
    ));
    assert!(matches!(
        Operation::Push(push_params()).check(&rebasing),
        Err(Refusal::InProgress(_))
    ));
}

#[test]
fn fetch_refusals() {
    let none = RepoFacts {
        remotes: vec![],
        ..facts()
    };
    assert_eq!(fetch(None, true).check(&none), Err(Refusal::NoRemote));
    assert_eq!(
        fetch(Some("origin"), true).check(&none),
        Err(Refusal::NoRemote)
    );
    assert_eq!(
        fetch(Some("fork"), true).check(&facts()),
        Err(Refusal::NoSuchRemote {
            name: "fork".to_owned()
        })
    );
    assert_eq!(fetch(Some("origin"), true).check(&facts()), Ok(()));
}

#[test]
fn pull_refusals() {
    assert_eq!(pull().check(&facts()), Ok(()));
    let detached = RepoFacts {
        head: HeadRef::Detached(oid(1)),
        upstream: None,
        ..facts()
    };
    assert_eq!(pull().check(&detached), Err(Refusal::DetachedHead));
    let untracked = RepoFacts {
        upstream: None,
        ..facts()
    };
    assert_eq!(
        pull().check(&untracked),
        Err(Refusal::NoUpstream {
            branch: "main".to_owned()
        })
    );
    let moved = RepoFacts {
        upstream: Some(TrackedUpstream {
            remote_branch: "refs/heads/trunk".to_owned(),
            ..upstream()
        }),
        ..facts()
    };
    assert_eq!(pull().check(&moved), Err(Refusal::UpstreamChanged));
    let other_branch = RepoFacts {
        head: HeadRef::Branch("refs/heads/topic".to_owned()),
        ..facts()
    };
    assert_eq!(pull().check(&other_branch), Err(Refusal::BranchChanged));
}

#[test]
fn push_refusals() {
    let push = Operation::Push(push_params());
    assert_eq!(push.check(&facts()), Ok(()));
    let unborn = RepoFacts {
        head: HeadRef::Unborn("refs/heads/main".to_owned()),
        ..facts()
    };
    assert_eq!(push.check(&unborn), Err(Refusal::DetachedHead));
    let none = RepoFacts {
        remotes: vec![],
        ..facts()
    };
    assert_eq!(push.check(&none), Err(Refusal::NoRemote));
    let fork = Operation::Push(PushParams {
        remote: "fork".to_owned(),
        ..push_params()
    });
    assert_eq!(
        fork.check(&facts()),
        Err(Refusal::NoSuchRemote {
            name: "fork".to_owned()
        })
    );
    let other_branch = RepoFacts {
        head: HeadRef::Branch("refs/heads/topic".to_owned()),
        ..facts()
    };
    assert_eq!(push.check(&other_branch), Err(Refusal::BranchChanged));
    // No upstream is not a push refusal: `P` sets one.
    let untracked = RepoFacts {
        upstream: None,
        ..facts()
    };
    assert_eq!(push.check(&untracked), Ok(()));
}

#[test]
fn refusal_text_is_the_design_notes() {
    assert_eq!(
        Refusal::DetachedHead.explain(),
        "HEAD is detached — check out a branch first"
    );
    assert_eq!(
        Refusal::NoRemote.explain(),
        "no remote — add one in the shell: git remote add origin <url>"
    );
    assert_eq!(
        Refusal::NoSuchRemote {
            name: "x".to_owned()
        }
        .explain(),
        "no remote named 'x'"
    );
    assert_eq!(
        Refusal::NoUpstream {
            branch: "main".to_owned()
        }
        .explain(),
        "main has no upstream — P pushes and sets one"
    );
    assert!(Refusal::UpstreamChanged.explain().contains("press p again"));
    assert!(!Refusal::BranchChanged.explain().is_empty());
}

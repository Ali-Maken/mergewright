//! Fetch, pull and push: every refusal one fact away from a passing
//! baseline, the exact command lines, `touches`, and `preview == plan`.

use std::path::PathBuf;

use graft_core::journal::{HeadRef, OperationKind, Touches};
use graft_core::oid::Oid;
use graft_git::remote::{Remote, TrackedUpstream};
use graft_git::state::InProgress;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_git::version::GitVersion;

use super::super::{Incoming, Invocation, Operation, Refusal, RepoFacts, SetAside};
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
        refs: std::collections::BTreeMap::new(),
        markers: vec![],
        incoming: None,
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
        source_commit: None,
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
            "git merge --ff-only --no-overwrite-ignore --no-edit refs/remotes/origin/main"
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
            "git push --progress -u --force-with-lease=refs/heads/main:{} \
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

/// The lease's `expected` must be the tracking ref's tip (oid 7 in
/// `facts()`): anything else — fetched since, never fetched, or a target
/// the upstream does not name — is refused before git runs.
#[test]
fn a_lease_must_match_the_fetched_tracking_tip() {
    let leased = |expected: Oid| {
        Operation::Push(PushParams {
            lease: Some(Lease { expected }),
            ..push_params()
        })
    };
    let stale = Err(Refusal::LeaseStale {
        remote_side: "origin/main".to_owned(),
    });
    assert_eq!(leased(oid(7)).check(&facts()), Ok(()));
    assert_eq!(leased(oid(8)).check(&facts()), stale);
    let never_fetched = RepoFacts {
        upstream: Some(TrackedUpstream {
            tracking_tip: None,
            ..upstream()
        }),
        ..facts()
    };
    assert_eq!(leased(oid(7)).check(&never_fetched), stale);
    let no_upstream = RepoFacts {
        upstream: None,
        ..facts()
    };
    assert_eq!(leased(oid(7)).check(&no_upstream), stale);
    let elsewhere = Operation::Push(PushParams {
        remote_branch: "refs/heads/other".to_owned(),
        lease: Some(Lease { expected: oid(7) }),
        ..push_params()
    });
    assert_eq!(
        elsewhere.check(&facts()),
        Err(Refusal::LeaseStale {
            remote_side: "origin/other".to_owned(),
        })
    );
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
    assert_eq!(
        Refusal::LeaseStale {
            remote_side: "origin/main".to_owned()
        }
        .explain(),
        "force-push refused: origin/main is not where the box said \
         (fetched since, or never fetched) · f to fetch, then look"
    );
}

// ── a pull's set-aside rule (m7-power-tools/03b) ────────────────────────

fn status_entry(path: &str) -> StatusEntry {
    StatusEntry {
        path: PathBuf::from(path),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

fn edited(path: &str) -> StatusEntry {
    StatusEntry {
        unstaged: Some(Change::Modified),
        ..status_entry(path)
    }
}

fn with(entries: Vec<StatusEntry>, incoming: Option<Incoming>) -> RepoFacts {
    RepoFacts {
        status: WorkingTreeStatus { entries },
        incoming,
        ..facts()
    }
}

/// A fast-forward whose fetch brought `paths`.
fn incoming(paths: &[&str]) -> Incoming {
    Incoming {
        tip: oid(8),
        fast_forward: true,
        paths: paths.iter().map(PathBuf::from).collect(),
    }
}

/// Overlap: every changed tracked file is set aside, the other one too, and
/// `refs/stash` recorded; a staged rename away from a path the pull changes
/// counts by its old path.
#[test]
fn a_ff_pull_over_an_edit_it_brings_sets_every_edit_aside() {
    let staged = StatusEntry {
        staged: Some(Change::Modified),
        ..status_entry("other.txt")
    };
    let over = with(
        vec![edited("shared.txt"), staged],
        Some(incoming(&["shared.txt"])),
    );
    assert_eq!(pull().sets_aside(&over), Some(SetAside { files: 2 }));
    assert_eq!(
        pull().touches(&over).refs,
        vec!["refs/heads/main".to_owned(), "refs/stash".to_owned()]
    );
    let renamed = StatusEntry {
        original_path: Some(PathBuf::from("shared.txt")),
        staged: Some(Change::Renamed),
        ..status_entry("moved.txt")
    };
    assert!(
        pull()
            .sets_aside(&with(vec![renamed], Some(incoming(&["shared.txt"]))))
            .is_some()
    );
}

/// Not fetched yet, no overlap, diverged, or only an untracked file where
/// the pull brings one: nothing set aside, git's own words.
#[test]
fn a_pull_sets_nothing_aside_otherwise() {
    // Not fetched yet: nothing.
    assert_eq!(
        pull().sets_aside(&with(vec![edited("shared.txt")], None)),
        None
    );
    // No overlap: nothing, and no `refs/stash` recorded.
    let apart = with(vec![edited("other.txt")], Some(incoming(&["shared.txt"])));
    assert_eq!(pull().sets_aside(&apart), None);
    assert_eq!(
        pull().touches(&apart).refs,
        vec!["refs/heads/main".to_owned()]
    );
    // Diverged: nothing.
    let diverged = Incoming {
        fast_forward: false,
        ..incoming(&["shared.txt"])
    };
    assert_eq!(
        pull().sets_aside(&with(vec![edited("shared.txt")], Some(diverged))),
        None
    );
    // Only an untracked file where the pull brings one: git's refusal
    // stands (ADR-0045 §4: untracked files are never set aside).
    let untracked = StatusEntry {
        untracked: true,
        ..status_entry("shared.txt")
    };
    assert_eq!(
        pull().sets_aside(&with(vec![untracked], Some(incoming(&["shared.txt"])))),
        None
    );
}

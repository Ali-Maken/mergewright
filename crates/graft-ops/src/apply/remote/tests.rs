use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{JournalEntry, OperationKind};
use graft_git::remote::{RemoteFailure, TrackedUpstream};
use graft_git::run::askpass::{AskPass, Secret};

use super::{RemoteHooks, explain, remote_invocation, short};
use crate::apply::ApplyError;
use crate::journal::tests::committed;
use crate::operation::{FetchParams, Invocation, Operation, PullParams, PushParams, RemoteNames};

fn entry(kind: OperationKind, description: &str) -> JournalEntry {
    let mut entry = committed(1, kind, 1, 2);
    description.clone_into(&mut entry.description);
    if kind != OperationKind::Pull {
        entry.touches.refs.clear();
    }
    entry
}

fn push_op(branch: &str, remote: &str) -> Operation {
    Operation::Push(PushParams {
        branch: format!("refs/heads/{branch}"),
        remote: remote.to_owned(),
        remote_branch: format!("refs/heads/{branch}"),
        set_upstream: false,
        lease: None,
        source_commit: None,
    })
}

fn pull_op(branch: &str, remote: &str) -> Operation {
    Operation::Pull(PullParams {
        branch: format!("refs/heads/{branch}"),
        upstream: TrackedUpstream {
            remote: remote.to_owned(),
            remote_branch: format!("refs/heads/{branch}"),
            tracking_ref: format!("refs/remotes/{remote}/{branch}"),
            tracking_tip: None,
        },
    })
}

fn fetch_op(remote: &str) -> Operation {
    Operation::Fetch(FetchParams {
        remote: Some(remote.to_owned()),
        prune: false,
    })
}

/// The entry `apply` would journal for `op`, and the names it would carry.
fn journaled(op: &Operation) -> (JournalEntry, RemoteNames) {
    let names = op.remote_names().expect("a remote operation has names");
    (entry(op.kind(), &op.describe()), names)
}

fn every_failure() -> Vec<RemoteFailure> {
    vec![
        RemoteFailure::AuthFailed {
            target: "127.0.0.1:8443".to_owned(),
        },
        RemoteFailure::NonFastForward,
        RemoteFailure::StaleInfo,
        RemoteFailure::RemoteRejected {
            reason: "pre-receive hook declined".to_owned(),
        },
        RemoteFailure::NoSuchRemote {
            name: "upstream".to_owned(),
        },
        RemoteFailure::Unreachable {
            target: "github.com".to_owned(),
        },
        RemoteFailure::NotFastForward,
        RemoteFailure::WouldOverwrite {
            paths: vec!["src/a.rs".to_owned()],
        },
        RemoteFailure::Cancelled,
        RemoteFailure::Other {
            first_line: "fatal: something odd".to_owned(),
        },
    ]
}

/// The operation each failure belongs to, and the words its line must use
/// (design note, "Failure line").
fn case(failure: &RemoteFailure) -> (Operation, &'static [&'static str]) {
    let push = || push_op("main", "origin");
    let pull = || pull_op("main", "origin");
    let fetch = || fetch_op("origin");
    match failure {
        RemoteFailure::AuthFailed { .. } => (
            push(),
            &["authentication failed for 127.0.0.1:8443", "P to try again"],
        ),
        RemoteFailure::NonFastForward => (
            push(),
            &[
                "push rejected: origin/main has moved",
                "p pull first",
                "P force with lease",
            ],
        ),
        RemoteFailure::StaleInfo => (
            push(),
            &[
                "push refused: origin/main moved since your last fetch",
                "stale lease",
                "f to fetch",
            ],
        ),
        RemoteFailure::RemoteRejected { .. } => (
            push(),
            &["rejected by the remote", "pre-receive hook declined"],
        ),
        RemoteFailure::NoSuchRemote { .. } => (fetch(), &["no remote named 'upstream'"]),
        RemoteFailure::Unreachable { .. } => (
            fetch(),
            &["origin unreachable", "Could not resolve host: github.com"],
        ),
        RemoteFailure::NotFastForward => (
            pull(),
            &[
                "pull stopped: main and origin/main diverged",
                "git pull --rebase",
            ],
        ),
        RemoteFailure::WouldOverwrite { .. } => (
            pull(),
            &[
                "pull refused: would overwrite src/a.rs (uncommitted)",
                "commit or stash first",
            ],
        ),
        RemoteFailure::Cancelled => (fetch(), &["fetch origin cancelled"]),
        RemoteFailure::Other { .. } => (fetch(), &["fetch origin failed", "something odd"]),
    }
}

#[test]
fn explain_names_each_failure() {
    let tail = vec![
        "Cloning…".to_owned(),
        "fatal: Could not resolve host: github.com".to_owned(),
    ];
    for failure in every_failure() {
        let (op, words) = case(&failure);
        let (entry, names) = journaled(&op);
        let error = ApplyError::Remote {
            entry: Box::new(entry.clone()),
            failure: failure.clone(),
            stderr_tail: tail.clone(),
            names: Box::new(names.clone()),
        };
        let line = error.explain();
        for word in words {
            assert!(line.contains(word), "{failure:?}: {line:?} lacks {word:?}");
        }
        // `Display` and `explain` are one text.
        assert_eq!(error.to_string(), line);
        assert_eq!(explain(&entry, &failure, &tail, &names), line);
        assert!(
            !short(&failure).is_empty(),
            "{failure:?} has a short reason"
        );
    }
}

#[test]
fn short_reason_for_cancel_is_the_word() {
    assert_eq!(short(&RemoteFailure::Cancelled), "cancelled");
}

/// The names come from the params, not from re-reading the description: a
/// refname may hold `→` (git forbids only space, `~^:?*[\` and control
/// bytes) and a remote name may hold `/` (m4/04 review, M4).
#[test]
fn names_survive_arrows_and_slashes() {
    let (entry, names) = journaled(&push_op("a→b", "team/origin"));
    assert_eq!(entry.description, "push a→b → team/origin/a→b");
    let line = explain(&entry, &RemoteFailure::NonFastForward, &[], &names);
    assert!(
        line.starts_with("push rejected: team/origin/a→b has moved"),
        "{line}"
    );

    let (entry, names) = journaled(&pull_op("x→y", "team/origin"));
    let line = explain(&entry, &RemoteFailure::NotFastForward, &[], &names);
    assert!(
        line.starts_with("pull stopped: x→y and team/origin/x→y diverged"),
        "{line}"
    );

    let (entry, names) = journaled(&fetch_op("team/origin"));
    let line = explain(
        &entry,
        &RemoteFailure::Unreachable {
            target: "h".to_owned(),
        },
        &[],
        &names,
    );
    assert_eq!(line, "team/origin unreachable: h");
}

struct Panics;
impl AskPass for Panics {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        panic!("never asked");
    }
}

#[test]
fn only_a_remote_invocation_with_hooks_gets_them() {
    let hooks = RemoteHooks {
        progress: Box::new(|_| {}),
        askpass: Arc::new(Panics),
        helper: PathBuf::from("/nonexistent/graft"),
    };
    let invocation = |remote| Invocation {
        args: Vec::new(),
        stdin: None,
        remote,
    };
    assert!(remote_invocation(&invocation(true), Some(&hooks)));
    assert!(!remote_invocation(&invocation(false), Some(&hooks)));
    assert!(!remote_invocation(&invocation(true), None));
    assert!(!remote_invocation(&invocation(false), None));
}

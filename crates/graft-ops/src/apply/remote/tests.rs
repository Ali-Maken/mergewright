use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{JournalEntry, OperationKind};
use graft_git::remote::RemoteFailure;
use graft_git::run::askpass::{AskPass, Secret};

use super::{RemoteHooks, explain, remote_invocation, short};
use crate::apply::ApplyError;
use crate::journal::tests::{MAIN, committed};
use crate::operation::Invocation;

fn entry(kind: OperationKind, description: &str) -> JournalEntry {
    let mut entry = committed(1, kind, 1, 2);
    description.clone_into(&mut entry.description);
    if kind != OperationKind::Pull {
        entry.touches.refs.clear();
    }
    entry
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
fn case(failure: &RemoteFailure) -> (JournalEntry, &'static [&'static str]) {
    let push = || entry(OperationKind::Push, "push main → origin/main");
    let pull = || entry(OperationKind::Pull, "pull origin/main");
    let fetch = || entry(OperationKind::Fetch, "fetch origin");
    match failure {
        RemoteFailure::AuthFailed { .. } => (
            push(),
            &["authentication failed for 127.0.0.1:8443", "P to try again"],
        ),
        RemoteFailure::NonFastForward => (
            push(),
            &[
                "push rejected: origin/main has commits you don't",
                "p to pull first",
                "force with lease",
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
        let (entry, words) = case(&failure);
        let error = ApplyError::Remote {
            entry: Box::new(entry.clone()),
            failure: failure.clone(),
            stderr_tail: tail.clone(),
        };
        let line = error.explain();
        for word in words {
            assert!(line.contains(word), "{failure:?}: {line:?} lacks {word:?}");
        }
        // `Display` and `explain` are one text.
        assert_eq!(error.to_string(), line);
        assert_eq!(explain(&entry, &failure, &tail), line);
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

#[test]
fn pull_names_its_branch_from_touches() {
    let pull = entry(OperationKind::Pull, "pull origin/main");
    assert_eq!(pull.touches.refs, [MAIN]);
    let line = explain(&pull, &RemoteFailure::NotFastForward, &[]);
    assert!(
        line.starts_with("pull stopped: main and origin/main"),
        "{line}"
    );
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

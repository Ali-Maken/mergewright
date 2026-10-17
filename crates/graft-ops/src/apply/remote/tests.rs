use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::journal::{JournalEntry, OperationKind, RefMove};
use graft_core::oid::Oid;
use graft_git::remote::{RemoteFailure, TrackedUpstream};
use graft_git::run::askpass::{AskPass, Secret};

use graft_git::run::RunError;

use super::{Refusals, RemoteHooks, cancelled_at_the_prompt, explain, remote_invocation, short};
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
                "push rejected: origin/main has commits you don't",
                "p pull, then m or r",
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
        RemoteFailure::NoSuchRemote { .. } => (
            fetch(),
            &["fetch origin failed: no repository at 'upstream', or no access to it"],
        ),
        RemoteFailure::Unreachable { .. } => (
            fetch(),
            &["origin unreachable", "Could not resolve host: github.com"],
        ),
        RemoteFailure::NotFastForward => (
            pull(),
            &[
                "pull stopped: main and origin/main diverged",
                "merge or rebase",
            ],
        ),
        RemoteFailure::WouldOverwrite { .. } => (
            pull(),
            &["pull refused: your edits to src/a.rs would be overwritten"],
        ),
        RemoteFailure::Cancelled => (fetch(), &["fetch origin cancelled"]),
        RemoteFailure::Other { .. } => (fetch(), &["fetch origin failed", "something odd"]),
    }
}

/// The pull that would overwrite edits names them in his words and names no
/// keys: the app adds the ones that work where he is (`S s stashes them
/// first`), and ops cannot know them. Not git's advice to commit or stash
/// (review m6-usability-2 D; the earlier walk's m7).
#[test]
fn would_overwrite_names_the_paths_and_no_keys() {
    let (entry, names) = journaled(&pull_op("main", "origin"));
    let failure = RemoteFailure::WouldOverwrite {
        paths: vec!["notes.md".to_owned(), "src/a.rs".to_owned()],
    };
    let line = explain(&entry, &failure, &[], &names, None, &crate::keys::Letters);
    assert_eq!(
        line,
        "pull refused: your edits to notes.md, src/a.rs would be overwritten"
    );
    assert!(!line.contains(" \u{b7} "), "no keys part: {line}");
}

/// A pull that cannot fast-forward says so without plan shorthand or a
/// trip to the shell: no milestone number, no git command to run instead
/// (the earlier walk's m11).
#[test]
fn not_fast_forward_says_no_plan_shorthand() {
    let (entry, names) = journaled(&pull_op("main", "origin"));
    let line = explain(
        &entry,
        &RemoteFailure::NotFastForward,
        &[],
        &names,
        None,
        &crate::keys::Letters,
    );
    assert_eq!(
        line,
        "pull stopped: main and origin/main diverged \u{2014} merge or rebase"
    );
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
            moves_unread: None,
        };
        let line = error.explain(&crate::keys::Letters);
        for word in words {
            assert!(line.contains(word), "{failure:?}: {line:?} lacks {word:?}");
        }
        // `Display` is `explain` with no keys.
        assert_eq!(error.to_string(), error.explain(&crate::keys::NoKeys));
        assert_eq!(
            explain(&entry, &failure, &tail, &names, None, &crate::keys::Letters),
            line
        );
        assert!(
            !short(&failure).is_empty(),
            "{failure:?} has a short reason"
        );
    }
}

/// 21c (ADR-0062): the probe — `fetch --all`, one remote gone, `origin/main`
/// moved first. The failure line ends with what the fetch moved; a fetch
/// that moved nothing reads as before; one whose refs could not be listed
/// says so instead.
#[test]
fn a_failed_fetch_that_moved_refs_ends_with_them() {
    let op = Operation::Fetch(FetchParams {
        remote: None,
        prune: false,
    });
    let (mut entry, names) = journaled(&op);
    let failure = RemoteFailure::NoSuchRemote {
        name: "/nonexistent/graft-21c.git".to_owned(),
    };
    let today = explain(&entry, &failure, &[], &names, None, &crate::keys::Letters);
    let oid = |byte: u8| Oid::from_bytes(&[byte; 20]).unwrap();

    entry.fetched_refs = Some(BTreeMap::from([(
        "refs/remotes/origin/main".to_owned(),
        RefMove {
            before: Some(oid(1)),
            after: Some(oid(2)),
        },
    )]));
    let line = explain(&entry, &failure, &[], &names, None, &crate::keys::Letters);
    assert_eq!(
        line,
        format!("{today} \u{b7} origin/main moved before it stopped")
    );

    entry.fetched_refs = Some(BTreeMap::new());
    let line = explain(&entry, &failure, &[], &names, None, &crate::keys::Letters);
    assert_eq!(line, today, "nothing moved: today's words");

    let unread = RunError::Unparsable("for-each-ref line \"x\"".to_owned());
    let line = explain(
        &entry,
        &failure,
        &[],
        &names,
        Some(&unread),
        &crate::keys::Letters,
    );
    assert_eq!(
        line,
        format!("{today} \u{b7} which refs it moved could not be read: {unread}")
    );
}

/// m7-power-tools/15 (:774): a rejected push names the whole way back — pull,
/// then merge or rebase — not a bare `p pull first`.
#[test]
fn rejected_push_names_pull_then_m_or_r() {
    let (entry, names) = journaled(&push_op("main", "origin"));
    let line = explain(
        &entry,
        &RemoteFailure::NonFastForward,
        &[],
        &names,
        None,
        &crate::keys::Letters,
    );
    assert_eq!(
        line,
        "push rejected: origin/main has commits you don't · p pull, then m or r · P force with lease"
    );
}

/// 20b R3: git's "does not appear to be a git repository" names a location,
/// not a remote: the journal's short reason says what git's two causes are.
#[test]
fn short_reason_for_no_such_remote_names_the_two_causes() {
    assert_eq!(
        short(&RemoteFailure::NoSuchRemote {
            name: "/nonexistent/x".to_owned()
        }),
        "no repository at '/nonexistent/x', or no access to it"
    );
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
    let line = explain(
        &entry,
        &RemoteFailure::NonFastForward,
        &[],
        &names,
        None,
        &crate::keys::Letters,
    );
    assert!(
        line.starts_with("push rejected: team/origin/a→b has commits you don't"),
        "{line}"
    );

    let (entry, names) = journaled(&pull_op("x→y", "team/origin"));
    let line = explain(
        &entry,
        &RemoteFailure::NotFastForward,
        &[],
        &names,
        None,
        &crate::keys::Letters,
    );
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
        None,
        &crate::keys::Letters,
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
        index: crate::operation::IndexFile::Real,
    };
    assert!(remote_invocation(&invocation(true), Some(&hooks)));
    assert!(!remote_invocation(&invocation(false), Some(&hooks)));
    assert!(!remote_invocation(&invocation(true), None));
    assert!(!remote_invocation(&invocation(false), None));
}

/// Refuses every ask, as `Esc` on the prompt does.
struct Refuses;
impl AskPass for Refuses {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        None
    }
}

/// Answers every ask.
struct Answers;
impl AskPass for Answers {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        Some(Secret::new(b"pw".to_vec()))
    }
}

fn exited() -> RunError {
    RunError::Exited {
        status: 128,
        stderr_first_line: "fatal: could not read Password".to_owned(),
        command: "git push".to_owned(),
    }
}

/// The wrapper records a refusal, and only a refusal.
#[test]
fn refusals_remember_an_unanswered_ask() {
    let refusing = Refusals::new(Arc::new(Refuses));
    assert!(!refusing.refused(), "nothing asked yet");
    assert!(refusing.ask("Password for 'http://h':").is_none());
    assert!(refusing.refused());

    let answered = Refusals::new(Arc::new(Answers));
    assert!(answered.ask("Password for 'http://h':").is_some());
    assert!(!answered.refused(), "an answered ask is no refusal");
}

/// m7-power-tools/27b: git exiting on a refused ask, with the stop asked, is
/// the user's cancel; either half alone leaves git's exit as it is, and a
/// stop the runner already saw stays a stop.
#[test]
fn an_exit_after_a_refused_ask_with_the_stop_asked_is_the_cancel() {
    assert!(matches!(
        cancelled_at_the_prompt(exited(), true, || true),
        RunError::Stopped
    ));
    assert!(matches!(
        cancelled_at_the_prompt(exited(), true, || false),
        RunError::Exited { .. }
    ));
    assert!(matches!(
        cancelled_at_the_prompt(exited(), false, || panic!("not polled without a refusal")),
        RunError::Exited { .. }
    ));
    assert!(matches!(
        cancelled_at_the_prompt(RunError::Stopped, false, || true),
        RunError::Stopped
    ));
}

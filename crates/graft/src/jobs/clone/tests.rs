//! The clone job against real git and local remotes only: a `file://`
//! bare, smart HTTP on loopback with Basic auth (`HttpRemote`), and a remote
//! that never answers (`HungRemote`). Never the network.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use graft_core::job::JobId;
use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};
use graft_git::run::askpass::Secret;

use super::{CloneAnswer, CloneRequest};
use crate::jobs::JobRunner;
use crate::repos::RepoStore;
use crate::update::{AppEvent, Product, Progress};

/// Long enough for a loopback clone on a loaded laptop; the tests wait on
/// events, so a passing run takes what git takes.
const WITHIN: Duration = Duration::from_secs(60);

/// A one-worker pool with no repository: the store is never opened (the
/// clone job reads none), as the empty state's own pool.
fn pool() -> (
    JobRunner<std::sync::mpsc::Sender<AppEvent>>,
    Receiver<AppEvent>,
) {
    let (sender, events) = channel();
    let store = Arc::new(RepoStore::new(std::env::temp_dir(), None));
    let jobs = JobRunner::start(sender, 1, store).expect("a test machine starts a thread");
    (jobs, events)
}

/// The next of `id`'s events, or a panic naming what was waited for.
fn next_for(events: &Receiver<AppEvent>, id: JobId, deadline: Instant) -> AppEvent {
    next_watched(events, id, deadline, |_| {})
}

/// As [`next_for`], showing `watch` every event received on the way,
/// another job's included: what a check over the whole stream needs.
fn next_watched(
    events: &Receiver<AppEvent>,
    id: JobId,
    deadline: Instant,
    mut watch: impl FnMut(&AppEvent),
) -> AppEvent {
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        let event = events
            .recv_timeout(left)
            .unwrap_or_else(|why| panic!("no event for {id:?} within {WITHIN:?}: {why}"));
        watch(&event);
        let ours = match &event {
            AppEvent::Produced(of, _) | AppEvent::Progress(of, _) | AppEvent::Started(of, _) => {
                *of == id
            }
            AppEvent::Core(_) | AppEvent::Rested => true,
        };
        if ours {
            return event;
        }
    }
}

/// `id`'s answer, skipping its progress lines.
fn answer_of(events: &Receiver<AppEvent>, id: JobId) -> CloneAnswer {
    let deadline = Instant::now() + WITHIN;
    loop {
        match next_for(events, id, deadline) {
            AppEvent::Produced(_, Product::Cloned(answer)) => return answer,
            AppEvent::Progress(_, Progress::RemoteProgress { .. }) => {}
            other => panic!("not a clone's event: {other:?}"),
        }
    }
}

fn local() -> LocalRemote {
    LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote")
}

fn head(remote: &LocalRemote, dir: &Path) -> String {
    remote
        .git(dir, &["rev-parse", "HEAD"])
        .unwrap_or_else(|why| panic!("rev-parse in {}: {why}", dir.display()))
}

#[test]
fn clones_a_file_bare() {
    let remote = local();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let (jobs, events) = pool();

    let id = jobs.clone_repo(CloneRequest {
        url: remote.url(),
        into: into.clone(),
    });

    assert_eq!(
        answer_of(&events, id),
        CloneAnswer::Done {
            path: into.clone(),
            notice: None
        }
    );
    assert_eq!(head(&remote, &into), head(&remote, remote.bare()));
}

#[test]
fn cancel_stops_it_and_leaves_nothing() {
    let hung = HungRemote::listen().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let (jobs, events) = pool();

    let id = jobs.clone_repo(CloneRequest {
        url: hung.url(),
        into: into.clone(),
    });
    // git makes the folder before it connects: once it has connected, the
    // stop lands on a clone that has something to clean up.
    assert!(
        hung.wait_for_connection(WITHIN),
        "git never reached the hung remote"
    );
    jobs.cancel(id);

    assert_eq!(
        answer_of(&events, id),
        CloneAnswer::Ended {
            words: "clone stopped \u{b7} nothing kept".to_owned()
        }
    );
    assert!(!into.exists(), "the stopped clone left its folder");
}

const USER: &str = "tester";
const PASSWORD: &str = "correct-horse-battery-staple";

/// Graft's own binary, which git runs as the askpass helper. A unit test
/// has no `CARGO_BIN_EXE_trib` (Cargo sets it for integration tests only),
/// so it is found where Cargo puts it: beside this test binary's `deps/`.
/// Cargo builds it whenever the package's integration tests are built, as
/// every workspace-wide gate does.
fn graft_binary() -> PathBuf {
    let test_binary = std::env::current_exe().unwrap();
    let profile = test_binary
        .parent()
        .and_then(Path::parent)
        .expect("a test binary lives in <target>/<profile>/deps");
    let graft = profile.join(crate::cli::COMMAND);
    assert!(
        graft.is_file(),
        "{} is not built: run the workspace's tests (cargo nextest run), which build it",
        graft.display()
    );
    graft
}

/// Over HTTP Basic auth: git asks through the pool, the question arrives as
/// `Progress::CredentialNeeded`, `answer_credential` answers it, and the
/// clone lands.
///
/// Runs in a child of this test binary with the system and global git
/// config switched off: a clone has no repository whose
/// `credential.helper` could be blanked first, and the user's keychain
/// helper must neither answer for the test nor keep its password (as
/// graft-ops' `asks_and_clones_over_http`).
#[test]
fn asks_through_the_pool_and_clones() {
    let empty_config = tempfile::NamedTempFile::new().unwrap();
    let child = "jobs::clone::tests::child_asks_through_the_pool_and_clones";
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            child,
            "--include-ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_config.path())
        .env("GRAFT_CLONE_JOB_CHILD", "1")
        .output()
        .expect("the child test runs");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the child clone over HTTP failed:\n{said}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A filter that matched nothing would also exit 0.
    assert!(
        said.contains(&format!("test {child} ... ok")),
        "the child test did not run:\n{said}"
    );
}

/// The child half of the test above; does nothing unless it started it.
#[test]
#[ignore = "run only as the child of asks_through_the_pool_and_clones"]
fn child_asks_through_the_pool_and_clones() {
    if std::env::var_os("GRAFT_CLONE_JOB_CHILD").is_none() {
        return;
    }
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let (sender, events) = channel();
    let store = Arc::new(RepoStore::new(std::env::temp_dir(), None));
    let jobs = JobRunner::start(sender, 1, store)
        .unwrap()
        .with_askpass_helper(graft_binary());

    let id = jobs.clone_repo(CloneRequest {
        url: http.url(),
        into: into.clone(),
    });
    // The password must reach git through the askpass answer only: no event
    // the loop receives (the ask itself included) may carry it, since the
    // loop logs events and draws from them.
    let mut seen = 0;
    let mut no_password = |event: &AppEvent| {
        seen += 1;
        let shown = format!("{event:?}");
        assert!(
            !shown.contains(PASSWORD),
            "event {seen} carries the password: {shown}"
        );
    };
    let deadline = Instant::now() + WITHIN;
    let mut asked = 0;
    let answer = loop {
        match next_watched(&events, id, deadline, &mut no_password) {
            AppEvent::Progress(_, Progress::CredentialNeeded { .. }) => {
                asked += 1;
                assert!(
                    jobs.answer_credential(id, Some(Secret::new(PASSWORD.as_bytes().to_vec()))),
                    "the ask was not waiting for its answer"
                );
            }
            AppEvent::Progress(_, Progress::RemoteProgress { .. }) => {}
            AppEvent::Produced(_, Product::Cloned(answer)) => break answer,
            other => panic!("not a clone's event: {other:?}"),
        }
    };

    assert!(asked >= 1, "git never asked through the pool");
    assert_eq!(
        answer,
        CloneAnswer::Done {
            path: into.clone(),
            notice: None
        }
    );
    assert_eq!(head(&remote, &into), head(&remote, remote.bare()));
}

/// m8/20b B6 (R1): the clone's askpass notice is `graft_ops`'s words, so a
/// refusal by another user's program never reads as a cleanup failure;
/// a cleanup failure alone keeps the old words.
#[test]
fn a_refused_askpass_notice_uses_the_ops_words() {
    let refused = std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        graft_git::run::askpass::Refused::new("uid 502 turned away", 1),
    );
    let notice = super::askpass_notice(&refused);
    assert_eq!(
        notice,
        format!(
            "cloned \u{b7} {}",
            graft_ops::apply::askpass_words(&refused)
        )
    );
    assert!(
        notice.contains("another user's program asked for your login"),
        "{notice}"
    );
    assert!(!notice.contains("could not be cleaned up"), "{notice}");

    let cleanup = std::io::Error::other("busy");
    assert_eq!(
        super::askpass_notice(&cleanup),
        "cloned \u{b7} the credential prompt's socket could not be cleaned up: busy"
    );
}

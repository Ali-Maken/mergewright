//! `mergewright_ops::clone` against real git and local remotes only: a `file://`
//! bare, smart HTTP on loopback (`HttpRemote`), a closed loopback port and a
//! remote that never answers (`HungRemote`). Never the network. Every claim
//! is read back from disk or from git, not from the return value alone.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use mergewright_fixtures::Shape;
use mergewright_fixtures::remote::{HttpRemote, HungRemote, LocalRemote};
use mergewright_git::run::Stop;
use mergewright_git::run::askpass::{AskPass, Secret};
use mergewright_ops::apply::RemoteHooks;
use mergewright_ops::clone::{CloneError, CloneParams, clone};

use crate::support::Never;

fn local() -> LocalRemote {
    LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote")
}

fn params(url: &str, into: &Path) -> CloneParams {
    CloneParams {
        url: url.to_owned(),
        into: into.to_path_buf(),
    }
}

/// `git rev-parse HEAD` in `dir`, through the fixture's hermetic git.
fn head(remote: &LocalRemote, dir: &Path) -> String {
    remote
        .git(dir, &["rev-parse", "HEAD"])
        .unwrap_or_else(|why| panic!("rev-parse in {}: {why}", dir.display()))
}

/// Counts every poll; never stops.
#[derive(Default)]
struct Counting(AtomicUsize);

impl Stop for Counting {
    fn should_stop(&self) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst);
        false
    }
}

/// The names in `dir`, sorted.
fn listing(dir: &Path) -> Vec<PathBuf> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    names.sort();
    names
}

#[test]
fn clones_a_file_bare() {
    let remote = local();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");

    let cloned = clone(&params(&remote.url(), &into), None, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(cloned.path, into);
    assert!(cloned.askpass.is_none());
    assert!(into.join(".git").is_dir(), "into is not a repository");
    assert_eq!(head(&remote, &into), head(&remote, remote.bare()));
}

/// A clone into a folder that exists and is empty is a clone, not a refusal.
#[test]
fn clones_into_an_existing_empty_folder() {
    let remote = local();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    std::fs::create_dir(&into).unwrap();

    clone(&params(&remote.url(), &into), None, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain()));

    assert_eq!(head(&remote, &into), head(&remote, remote.bare()));
}

/// Answers every prompt with one password, counting the asks.
struct Answers {
    password: &'static str,
    asked: AtomicUsize,
}

impl AskPass for Answers {
    fn ask(&self, _prompt: &str) -> Option<Secret> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Some(Secret::new(self.password.as_bytes().to_vec()))
    }
}

const USER: &str = "tester";
const PASSWORD: &str = "correct-horse-battery-staple";

/// Over HTTP Basic auth, git asks through the hooks' askpass and the clone
/// lands.
///
/// Runs in a child of this test binary with the system and global git
/// config switched off (`GIT_CONFIG_NOSYSTEM`, `GIT_CONFIG_GLOBAL`): a clone
/// has no repository whose `credential.helper` could be blanked first, and
/// the user's keychain helper must neither answer for the test nor store its
/// password once the clone succeeds.
#[test]
fn asks_and_clones_over_http() {
    let empty_config = tempfile::NamedTempFile::new().unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "clone::child_asks_and_clones_over_http",
            "--include-ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", empty_config.path())
        .env("MWR_CLONE_CHILD", "1")
        .output()
        .expect("the child test runs");
    let said = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "the child clone over HTTP failed:\n{said}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // A filter that matched nothing would also exit 0: the child must have
    // run, and passed.
    assert!(
        said.contains("test clone::child_asks_and_clones_over_http ... ok"),
        "the child test did not run:\n{said}"
    );
}

/// The child half of the test above; does nothing unless it was started by it.
#[test]
#[ignore = "run only as the child of asks_and_clones_over_http"]
fn child_asks_and_clones_over_http() {
    if std::env::var_os("MWR_CLONE_CHILD").is_none() {
        return;
    }
    let remote = local();
    let http = HttpRemote::serve(remote.bare(), USER, PASSWORD).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let answers = Arc::new(Answers {
        password: PASSWORD,
        asked: AtomicUsize::new(0),
    });
    let hooks = RemoteHooks {
        progress: Box::new(|_| {}),
        askpass: answers.clone(),
        helper: PathBuf::from(env!("CARGO_BIN_EXE_askpass_test_helper")),
    };

    let cloned = clone(&params(&http.url(), &into), Some(&hooks), &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain()));

    assert!(
        answers.asked.load(Ordering::SeqCst) >= 1,
        "git asked through the helper"
    );
    assert!(cloned.askpass.is_none(), "{:?}", cloned.askpass);
    assert_eq!(head(&remote, &into), head(&remote, remote.bare()));
}

/// A missing bare and a closed loopback port both fail as a remote failure,
/// and git's own cleanup takes the folder it made: nothing left behind.
#[test]
fn a_failed_clone_leaves_no_folder() {
    let scratch = tempfile::tempdir().unwrap();
    let missing = format!("file://{}", scratch.path().join("nope.git").display());
    let closed = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        format!("http://127.0.0.1:{port}/repo.git")
    };
    for url in [missing, closed] {
        let into = scratch.path().join("x");

        let answer = clone(&params(&url, &into), None, &Never);

        if url.starts_with("file://")
            && let Err(error) = &answer
        {
            // 20b R3: nothing there, in git's two causes, not "no remote named".
            let words = error.explain();
            assert!(
                words.starts_with("clone failed: no repository at '")
                    && words.contains("nope.git")
                    && words.ends_with("', or no access to it"),
                "{words}"
            );
        }
        let Err(CloneError::Remote { left, failure, .. }) = answer else {
            panic!("{url}: not a remote failure: {answer:?}");
        };
        assert_eq!(left, None, "{url}: {failure:?}");
        assert!(!into.exists(), "{url}: the folder was left behind");
    }
}

/// A failure into a folder that existed empty keeps the folder, empty, and
/// says nothing was left (git made nothing that stayed).
#[test]
fn a_failed_clone_into_an_empty_folder_leaves_it_empty() {
    let scratch = tempfile::tempdir().unwrap();
    let missing = format!("file://{}", scratch.path().join("nope.git").display());
    let into = scratch.path().join("x");
    std::fs::create_dir(&into).unwrap();

    let answer = clone(&params(&missing, &into), None, &Never);

    let Err(CloneError::Remote { left, .. }) = answer else {
        panic!("not a remote failure: {answer:?}");
    };
    assert_eq!(left, None);
    assert!(into.is_dir(), "the user's folder went");
    assert!(listing(&into).is_empty(), "{:?}", listing(&into));
}

/// Stops as soon as `into` exists — git has made the folder and is waiting
/// on the remote — or at the deadline, which only a broken clone reaches
/// (the remote never answers, so without a stop git waits forever).
struct StopOnceMade {
    into: PathBuf,
    deadline: Instant,
    made: AtomicBool,
}

impl Stop for StopOnceMade {
    fn should_stop(&self) -> bool {
        if self.into.exists() {
            self.made.store(true, Ordering::SeqCst);
            return true;
        }
        Instant::now() >= self.deadline
    }
}

/// How long the stopped clone may take to make its folder before the test
/// gives up on it: generous, since the run polls every 50 ms and git
/// creates the folder before it connects.
const MADE_WITHIN: Duration = Duration::from_secs(30);

#[test]
fn a_stopped_clone_leaves_no_folder() {
    let hung = HungRemote::listen().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    let stop = StopOnceMade {
        into: into.clone(),
        deadline: Instant::now() + MADE_WITHIN,
        made: AtomicBool::new(false),
    };

    let answer = clone(&params(&hung.url(), &into), None, &stop);

    assert!(
        stop.made.load(Ordering::SeqCst),
        "git never made {} within {MADE_WITHIN:?}",
        into.display()
    );
    let Err(CloneError::Stopped { left }) = answer else {
        panic!("not a stop: {answer:?}");
    };
    assert_eq!(left, None);
    assert!(!into.exists(), "the stopped clone left its folder");
}

#[test]
fn a_non_empty_target_is_refused_before_git_runs() {
    let remote = local();
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    std::fs::create_dir(&into).unwrap();
    std::fs::write(into.join("notes.txt"), "mine").unwrap();
    let before = listing(&into);
    let stop = Counting::default();

    let answer = clone(&params(&remote.url(), &into), None, &stop);

    assert!(
        matches!(&answer, Err(CloneError::TargetNotEmpty(path)) if *path == into),
        "{answer:?}"
    );
    assert_eq!(stop.0.load(Ordering::SeqCst), 0, "the stop was polled");
    assert_eq!(listing(&into), before);
    assert_eq!(std::fs::read(into.join("notes.txt")).unwrap(), b"mine");
}

#[test]
fn a_url_like_an_option_is_refused() {
    let scratch = tempfile::tempdir().unwrap();
    let into = scratch.path().join("x");
    for url in ["-uhttps://x", "", "https://h/"] {
        let stop = Counting::default();

        let answer = clone(&params(url, &into), None, &stop);

        assert!(
            matches!(answer, Err(CloneError::BadUrl)),
            "{url:?}: {answer:?}"
        );
        assert_eq!(
            stop.0.load(Ordering::SeqCst),
            0,
            "{url:?}: the stop was polled"
        );
        assert!(!into.exists(), "{url:?}");
    }
}

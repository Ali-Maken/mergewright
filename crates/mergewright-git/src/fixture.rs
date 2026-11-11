//! Tiny repositories built by real `git`, for tests in this crate.
//!
//! Real `git` rather than handwritten `.git` directories, because the awkward
//! cases this crate has to survive — a linked worktree, a submodule, a
//! detached `HEAD` — are exactly the ones whose on-disk shape is easy to write
//! down wrongly. Every repository here is a handful of empty commits, which
//! keeps the fast tier fast (PROMPT.md §7.1); the `fast-import` machinery for
//! large fixtures arrives with the graph-layout milestone.
//!
//! Every `git` this module runs is hermetic: no global or system config, no
//! credential prompt, no network, a fixed identity and clock, and a deadline
//! so a wedged child fails the test instead of hanging the suite.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long any one `git` in a fixture may take before the test fails.
///
/// Generous for the commands used here (all of them touch a handful of files)
/// and still far under the fast tier's budget.
const DEADLINE: Duration = Duration::from_secs(30);

/// A temporary directory with, usually, a repository in it.
pub struct Fixture {
    /// Deletes the directory when the fixture is dropped.
    dir: tempfile::TempDir,
    /// The directory's canonical path — macOS hands out `/var/folders/…`,
    /// which is a symlink to `/private/var/folders/…`, and `gix` reports the
    /// resolved spelling.
    root: PathBuf,
}

impl Fixture {
    /// A temporary directory with nothing in it.
    pub fn empty_dir() -> Self {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let root = dir
            .path()
            .canonicalize()
            .expect("the temporary directory exists");
        Fixture { dir, root }
    }

    /// A repository with no commits: `git init`, and nothing else.
    pub fn empty_repo() -> Self {
        let fixture = Self::empty_dir();
        fixture.git(&["init", "-b", "main"]);
        fixture
    }

    /// A repository with two commits on `main`.
    pub fn with_two_commits() -> Self {
        let fixture = Self::empty_repo();
        fixture.commit("first");
        fixture.commit("second");
        fixture
    }

    /// A bare repository: a git directory with no working tree.
    pub fn bare_repo() -> Self {
        let fixture = Self::empty_dir();
        fixture.git(&["init", "--bare", "-b", "main"]);
        fixture
    }

    /// The root of the temporary directory, canonicalised.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `root/relative`, created if it does not exist yet.
    pub fn subdirectory(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(&path).expect("a subdirectory of a temporary directory");
        path
    }

    /// Runs `git` in the fixture's root and returns its trimmed stdout.
    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.root, args)
    }

    /// Runs `git` in `cwd` and returns its trimmed stdout.
    pub fn git_in(&self, cwd: &Path, args: &[&str]) -> String {
        // Held only so that a fixture cannot outlive its directory; the
        // canonical `root` is what every command uses.
        debug_assert!(self.dir.path().exists(), "the fixture directory is alive");
        run_git(cwd, args)
    }

    /// Records an empty commit on the current branch.
    pub fn commit(&self, message: &str) {
        self.git(&["commit", "--allow-empty", "-m", message]);
    }

    /// The full object id `revision` resolves to, as hex.
    pub fn rev_parse(&self, revision: &str) -> String {
        self.git(&["rev-parse", revision])
    }
}

/// Runs one hermetic `git`, and panics with everything it said if it fails.
fn run_git(cwd: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // No prompt, no global or system config, no ambient repository: the
        // fixture must behave the same on every machine (PROMPT.md §7.1).
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        // A fixed identity and clock, so two runs produce the same ids.
        .env("GIT_AUTHOR_NAME", "Maker Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", "2020-01-01T00:00:00+00:00")
        .env("GIT_COMMITTER_NAME", "Maker Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_DATE", "2020-01-01T00:00:00+00:00");

    let mut child = command
        .spawn()
        .unwrap_or_else(|error| panic!("git {args:?} could not be started: {error}"));

    let started = Instant::now();
    loop {
        match child.try_wait().expect("the child's state can be read") {
            Some(_) => break,
            None if started.elapsed() >= DEADLINE => {
                child.kill().expect("a running child can be killed");
                child.wait().expect("a killed child can be reaped");
                panic!("git {args:?} did not finish within {DEADLINE:?}");
            }
            // Both pipes are drained below; every command here writes far less
            // than a pipe buffer, so neither can fill while we wait.
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }

    let output = child
        .wait_with_output()
        .expect("the child's output can be read");
    assert!(
        output.status.success(),
        "git {args:?} in {} failed: {}\n{}",
        cwd.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );

    String::from_utf8(output.stdout)
        .expect("git's output here is ASCII")
        .trim()
        .to_owned()
}

//! "Lost work": the journal's list fixture (m7-power-tools/22, "The
//! fixture"), for journal-screen and the acceptance walk.
//!
//! Built fresh per test, not cached in the corpus: three of its steps are
//! Graft's own operations, and this crate depends on no Graft crate. The
//! caller supplies them as a [`Graft`]; this runs the shell's side with real
//! `git`, every step at a fixed second of 2024-03-12, 10:12–10:44 UTC, so the
//! object ids and the reflog's times are the same every run.
//!
//! | Time  | Who   | Step |
//! |-------|-------|------|
//! | 10:12 | shell | `main`: "Start the api" (A) |
//! | 10:13 | shell | "Add request logging" (B) |
//! | 10:14 | shell | "Add metrics endpoint" (C); `origin` cloned bare from here, `origin/main` = C |
//! | 10:15 | shell | "Add rate limiter" (D) |
//! | 10:16 | Graft | discard of an edit to `src/a.rs` ([`DISCARDED_EDIT`]) |
//! | 10:16 | Graft | he edits `src/a.rs` again ([`LOST_EDIT`]); undo forced over it (10:16:40): **his edit is lost** to the undo's backup |
//! | 10:18 | shell | `git switch -c spike HEAD~1`: off C, so E and D are two lost runs, not one |
//! | 10:21 | shell | "Try redis cache" (E) on `spike` |
//! | 10:30 | —     | a teammate pushes two commits to `origin` |
//! | 10:37 | shell | `git switch main`, `git branch -D spike`: **E is lost, spike is gone** |
//! | 10:41 | shell | `git reset --keep HEAD~1`: **D is lost** |
//! | 10:42 | shell | `git stash push -- src/metrics.rs`: the stash holds S (parent C), which is not lost |
//! | 10:44 | Graft | fetch `origin`: `origin/main` moves 2 commits |
//!
//! Order follows the planner's ruling (m7-power-tools/22c): the discard and
//! its undo sit below the shell's spike steps, so the reset is the newest
//! step with a `✗` under it and E's `✗` is above the edit's. The shell
//! steps after the undo keep the undone file on disk: E commits only its own
//! file, the reset is `--keep` (`src/a.rs` is the same in C and D), and the
//! stash takes `src/metrics.rs` alone.
//!
//! The undo reverses a discard, not a switch (the M6/77 tests' vehicle): a
//! Graft step that moved `HEAD` would write reflog records on the wall
//! clock, which the list pairs with its entry by time, and the fixed times
//! could not hold. A discard and its undo write no reflog record at all.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::git::{self, GitError};
use crate::scratch::Scratch;

/// 2024-03-12 10:12:00 UTC, the first step, in Unix seconds.
pub const START: i64 = 1_710_238_320;
/// 10:44:00, the last step (Graft's fetch).
pub const END: i64 = at(10, 44, 0);

/// D's subject: lost by the shell's `reset --hard`.
pub const LOST_ON_MAIN: &str = "Add rate limiter";
/// E's subject: lost with `spike`.
pub const LOST_ON_SPIKE: &str = "Try redis cache";
/// The file whose edit the undo replaced.
pub const EDITED: &str = "src/a.rs";
/// What Graft's discard dropped, and the undo put back: on disk at the end.
pub const DISCARDED_EDIT: &[u8] = b"pub fn a() -> u32 {\n    1\n}\n";
/// His edit the forced undo replaced: kept only in the undo's backup.
pub const LOST_EDIT: &[u8] = b"pub fn a() -> u32 {\n    2\n}\n";

/// 2024-03-12 at `hour:minute:second` UTC, in Unix seconds.
const fn at(hour: i64, minute: i64, second: i64) -> i64 {
    START - (10 * 3600 + 12 * 60) + hour * 3600 + minute * 60 + second
}

/// Graft's own operations, at the fixture's Graft points. Each gets the
/// working repository and the journal time to stamp, in Unix milliseconds;
/// an `Err` is the operation's own explanation.
pub trait Graft {
    /// Fetch `origin`.
    ///
    /// # Errors
    /// Why the fetch failed.
    fn fetch(&mut self, work: &Path, at_ms: i64) -> Result<(), String>;
    /// Discard the working-tree edit to `path`.
    ///
    /// # Errors
    /// Why the discard failed.
    fn discard(&mut self, work: &Path, path: &str, at_ms: i64) -> Result<(), String>;
    /// Undo the newest operation, forced past drift.
    ///
    /// # Errors
    /// Why the undo failed.
    fn undo_forced(&mut self, work: &Path, at_ms: i64) -> Result<(), String>;
}

/// The commits the fixture made, as 40-hex ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commits {
    /// "Start the api".
    pub a: String,
    /// "Add request logging".
    pub b: String,
    /// "Add metrics endpoint": `main` at the end.
    pub c: String,
    /// "Add rate limiter": lost.
    pub d: String,
    /// "Try redis cache": lost, `spike` gone.
    pub e: String,
    /// The stash commit: held by `refs/stash`.
    pub s: String,
    /// `origin/main` after Graft's fetch, two past C.
    pub fetched: String,
}

/// Why the fixture could not be built.
#[derive(Debug, thiserror::Error)]
pub enum LostWorkError {
    /// A file or directory could not be made.
    #[error("{context}: {source}")]
    Io {
        /// What was being done.
        context: String,
        /// What the operating system said.
        source: io::Error,
    },
    /// A shell step failed.
    #[error(transparent)]
    Git(#[from] GitError),
    /// One of the caller's Graft operations failed.
    #[error("Graft's {step} failed: {why}")]
    Graft {
        /// Which Graft point.
        step: &'static str,
        /// The operation's explanation.
        why: String,
    },
}

/// The built fixture, deleted on drop.
#[derive(Debug)]
pub struct LostWork {
    /// Owns every directory below; deleting it on drop is its whole job.
    _scratch: Scratch,
    work: PathBuf,
    origin: PathBuf,
    commits: Commits,
}

impl LostWork {
    /// Builds the fixture, calling `graft` at its three Graft points.
    ///
    /// # Errors
    /// [`LostWorkError`] if a directory cannot be made, a `git` step fails,
    /// or a Graft operation does.
    pub fn build(graft: &mut dyn Graft) -> Result<LostWork, LostWorkError> {
        let scratch = Scratch::empty("lost-work").map_err(|source| LostWorkError::Io {
            context: "creating the fixture's scratch directory".to_owned(),
            source,
        })?;
        let root = scratch.path().to_path_buf();
        let work = root.join("work");
        let origin = root.join("origin.git");
        let shell = Shell { work: &work };

        let commits = shell_steps(&shell, graft, &root, &origin)?;
        graft
            .fetch(shell.work, ms(END))
            .map_err(|why| LostWorkError::Graft { step: "fetch", why })?;

        Ok(LostWork {
            _scratch: scratch,
            work,
            origin,
            commits,
        })
    }

    /// The working repository: `main` checked out at C.
    pub fn work(&self) -> &Path {
        &self.work
    }

    /// The bare `origin` the fetch reads.
    pub fn origin(&self) -> &Path {
        &self.origin
    }

    /// The commits made, by letter.
    pub fn commits(&self) -> &Commits {
        &self.commits
    }
}

/// 10:12–10:42: everything the shell does, up to the stash, with Graft's
/// discard and undo at 10:16.
fn shell_steps(
    shell: &Shell<'_>,
    graft: &mut dyn Graft,
    root: &Path,
    origin: &Path,
) -> Result<Commits, LostWorkError> {
    git::run_at(root, &["init", "-q", "-b", "main", "work"], &date(START))?;
    shell.harden()?;

    let start = shell.commit(
        at(10, 12, 0),
        "src/a.rs",
        b"pub fn a() {}\n",
        "Start the api",
    )?;
    let logging = shell.commit(
        at(10, 13, 0),
        "src/log.rs",
        b"// log\n",
        "Add request logging",
    )?;
    let metrics = shell.commit(
        at(10, 14, 0),
        "src/metrics.rs",
        b"// metrics\n",
        "Add metrics endpoint",
    )?;
    shell.remote_at_c(root, origin)?;
    let limiter = shell.commit(at(10, 15, 0), "src/limit.rs", b"// limit\n", LOST_ON_MAIN)?;
    graft_edit_steps(shell, graft)?;

    shell.git(at(10, 18, 0), &["switch", "-q", "-c", "spike", "HEAD~1"])?;
    let cache = shell.commit(at(10, 21, 0), "src/cache.rs", b"// redis\n", LOST_ON_SPIKE)?;
    let fetched = teammate_pushes(root, origin)?;
    shell.git(at(10, 37, 0), &["switch", "-q", "main"])?;
    shell.git(at(10, 37, 30), &["branch", "-q", "-D", "spike"])?;

    shell.git(at(10, 41, 0), &["reset", "-q", "--keep", "HEAD~1"])?;

    shell.write("src/metrics.rs", b"// metrics, tuned\n")?;
    shell.git(
        at(10, 42, 0),
        &[
            "stash",
            "push",
            "-q",
            "-m",
            "tune metrics",
            "--",
            "src/metrics.rs",
        ],
    )?;
    let stash = shell.git(at(10, 42, 0), &["rev-parse", "refs/stash"])?;

    Ok(Commits {
        a: start,
        b: logging,
        c: metrics,
        d: limiter,
        e: cache,
        s: stash,
        fetched,
    })
}

/// 10:16: Graft's discard, his edit, Graft's forced undo over it.
fn graft_edit_steps(shell: &Shell<'_>, graft: &mut dyn Graft) -> Result<(), LostWorkError> {
    let failed = |step| move |why| LostWorkError::Graft { step, why };
    shell.write(EDITED, DISCARDED_EDIT)?;
    graft
        .discard(shell.work, EDITED, ms(at(10, 16, 0)))
        .map_err(failed("discard"))?;
    shell.write(EDITED, LOST_EDIT)?;
    graft
        .undo_forced(shell.work, ms(at(10, 16, 40)))
        .map_err(failed("undo"))
}

/// The shell's side, in the working repository.
struct Shell<'a> {
    work: &'a Path,
}

impl Shell<'_> {
    /// `git` at `at`: its commits and reflog records carry that second.
    fn git(&self, at: i64, args: &[&str]) -> Result<String, LostWorkError> {
        let out = git::run_at(self.work, args, &date(at))?;
        Ok(String::from_utf8_lossy(&out).trim_end().to_owned())
    }

    /// The local config Graft's own `git` runs read: the machine's signing,
    /// hooks and line-ending settings switched off where local wins, and an
    /// identity (the fixture's `git::run_at` has its own).
    fn harden(&self) -> Result<(), LostWorkError> {
        for (key, value) in [
            ("user.name", git::IDENTITY_NAME),
            ("user.email", git::IDENTITY_EMAIL),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("core.hooksPath", "/dev/null"),
        ] {
            self.git(START, &["config", key, value])?;
        }
        Ok(())
    }

    fn write(&self, relative: &str, bytes: &[u8]) -> Result<(), LostWorkError> {
        let path = self.work.join(relative);
        if let Some(directory) = path.parent() {
            fs::create_dir_all(directory).map_err(|source| LostWorkError::Io {
                context: format!("creating {}", directory.display()),
                source,
            })?;
        }
        fs::write(&path, bytes).map_err(|source| LostWorkError::Io {
            context: format!("writing {}", path.display()),
            source,
        })
    }

    /// Writes `file`, commits everything as `subject` at `at`, returns the id.
    fn commit(
        &self,
        at: i64,
        file: &str,
        bytes: &[u8],
        subject: &str,
    ) -> Result<String, LostWorkError> {
        self.write(file, bytes)?;
        // Only its own file: an uncommitted edit elsewhere stays one.
        self.git(at, &["add", "--", file])?;
        self.git(at, &["commit", "-q", "-m", subject])?;
        self.git(at, &["rev-parse", "HEAD"])
    }

    /// `origin`, a bare clone of `main` at C, fetched once from the shell
    /// (a remote-tracking log, which the list does not read) and set as
    /// `main`'s upstream. Made before D, so no remote ref holds D.
    fn remote_at_c(&self, root: &Path, origin: &Path) -> Result<(), LostWorkError> {
        let when = date(at(10, 14, 30));
        git::run_at(
            root,
            &["clone", "-q", "--bare", "work", "origin.git"],
            &when,
        )?;
        let url = origin.display().to_string();
        self.git(at(10, 14, 30), &["remote", "add", "origin", &url])?;
        self.git(at(10, 14, 30), &["fetch", "-q", "origin"])?;
        self.git(
            at(10, 14, 30),
            &["branch", "-q", "--set-upstream-to=origin/main", "main"],
        )?;
        Ok(())
    }
}

/// A teammate clones `origin` and pushes two commits on `main`. Returns the
/// new `origin/main`. The clone is removed after: it plays no further part.
fn teammate_pushes(root: &Path, origin: &Path) -> Result<String, LostWorkError> {
    let url = origin.display().to_string();
    git::run_at(
        root,
        &["clone", "-q", &url, "teammate"],
        &date(at(10, 30, 0)),
    )?;
    let teammate = root.join("teammate");
    let mate = Shell { work: &teammate };
    mate.commit(at(10, 30, 0), "docs/api.md", b"# api\n", "Document the api")?;
    let tip = mate.commit(
        at(10, 31, 0),
        "docs/ops.md",
        b"# ops\n",
        "Add an ops runbook",
    )?;
    mate.git(at(10, 32, 0), &["push", "-q", "origin", "main"])?;
    fs::remove_dir_all(&teammate).map_err(|source| LostWorkError::Io {
        context: format!("removing {}", teammate.display()),
        source,
    })?;
    Ok(tip)
}

/// A date `git` reads exactly: `@<seconds> +0000`.
fn date(at: i64) -> String {
    format!("@{at} +0000")
}

const fn ms(at: i64) -> i64 {
    at * 1000
}

#[cfg(test)]
mod tests;

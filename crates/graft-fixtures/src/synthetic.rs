//! The heavy tier's synthetic repository: a history of any size, built by
//! `git fast-import`.
//!
//! PROMPT.md §7.1 puts one fixture in the heavy tier and nowhere else — "the
//! 100k-commit synthetic repo and the performance budgets". This module is
//! the repository; the budgets are a later ticket's assertions.
//!
//! The whole module is behind the `heavy-tests` feature, so a fast-tier
//! `cargo test` cannot call it, cannot compile it, and therefore cannot
//! spend a minute of somebody's laptop generating a hundred thousand
//! commits by accident.
//!
//! # What the history looks like
//!
//! Not a straight line. A straight line of 100k commits would measure list
//! scrolling and nothing else, and the thing under test is a *graph*. So:
//!
//! ```text
//! main   ──●──●── … ──●───────────────●──●── …      (runs of 10)
//!                      \             /
//! topic                 ●──●──●─────╯                (3 commits, merged)
//! ```
//!
//! repeated until the requested number of commits is reached, with:
//!
//! - every 32nd topic branch left **unmerged**, so there are open lanes with
//!   nowhere to land — the case a layout engine gets wrong;
//! - a lightweight tag every 250 commits and an annotated tag every 1000, so
//!   ref lookup and badge drawing have something to do at scale;
//! - merged topic branches mostly left ref-less, the way a team that deletes
//!   merged branches leaves them, with every 64th kept.
//!
//! Everything about it is a function of the requested size, so two builds of
//! the same [`Scale`] are byte-identical — same object ids, same refs.
//!
//! # Cost
//!
//! Generation is a one-off per [`Scale`], cached under `target/fixtures/`
//! exactly like the named corpus. Measured numbers, and how to delete the
//! cache, are in `TESTING.md`.

use std::fs::File;
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cache::{self, CacheError};
use crate::digest;
use crate::git::{self, GitError};
use crate::permissions;

/// Commits on the trunk between one topic branch and the next.
const TRUNK_RUN: u32 = 10;
/// Commits on a topic branch before it lands.
const TOPIC_RUN: u32 = 3;
/// One in this many topic branches is left unmerged, as an open lane.
const OPEN_EVERY: u32 = 32;
/// One in this many *merged* topic branches keeps its ref.
const KEEP_REF_EVERY: u32 = 64;
/// A lightweight tag this often, counted in commits.
const LIGHT_TAG_EVERY: u32 = 250;
/// An annotated tag this often, counted in commits.
const ANNOTATED_TAG_EVERY: u32 = 1000;
/// Commits written into the stream file between drains, so the generator's
/// own memory never grows with the size of the fixture.
const DRAIN_EVERY: u32 = 4096;

/// The branch every throwaway topic is built on, reused each time and
/// deleted after the import. Reusing one name keeps the repository's ref
/// count realistic without asking `fast-import` to delete refs.
const SCRATCH_BRANCH: &str = "refs/heads/topic-scratch";

/// The default branch.
const MAIN: &[u8] = b"refs/heads/main";

mod history;
mod reflog;
mod scale;

pub use reflog::{LOST, RECORDS, Reflogged, with_reflog};
pub use scale::{CommitGraph, CommitSize, Scale};

use history::History;

/// What one call to [`ensure`] found or did.
#[derive(Clone, Debug)]
pub struct Report {
    /// The repository's working-tree root.
    pub root: PathBuf,
    /// What was asked for.
    pub scale: Scale,
    /// `true` if this call built it, `false` if it was already there.
    pub built: bool,
    /// How long the call took, building or not.
    pub elapsed: Duration,
    /// The repository's size on disk.
    pub bytes: u64,
}

/// Why the synthetic repository could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum SyntheticError {
    /// A `git` command failed.
    #[error("building the {scale} fixture: {source}")]
    Git {
        /// The scale being built.
        scale: String,
        /// What `git` did.
        source: GitError,
    },
    /// It could not be cached, published or measured.
    #[error(transparent)]
    Cache(#[from] CacheError),
    /// A file could not be written or a directory created.
    #[error("building the {scale} fixture: cannot write {}: {source}", path.display())]
    Io {
        /// The scale being built.
        scale: String,
        /// The path that could not be written.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// The built repository does not hold the number of commits that was
    /// asked for. A generator bug, reported rather than tolerated: a
    /// performance budget measured against the wrong size means nothing.
    #[error("the {scale} fixture was asked for {want} commits and holds {got}")]
    WrongSize {
        /// The scale being built.
        scale: String,
        /// How many commits were asked for.
        want: u32,
        /// How many the repository holds.
        got: u32,
    },
    /// `git rev-list --count` said something that is not a count.
    #[error("counting the commits in the {scale} fixture: `{output}` is not a number")]
    Uncountable {
        /// The scale being built.
        scale: String,
        /// What `git` printed.
        output: String,
    },
}

/// Builds the repository if it is not there, and reports on it.
pub fn ensure(scale: Scale) -> Result<Report, SyntheticError> {
    let base = cache::base_dir();
    let started = Instant::now();
    let key = digest::synthetic_key(&scale.name()).map_err(git_error(scale))?;
    let root = base.join(format!("{}-{key}", scale.name()));

    if cache::is_built(&root) {
        return finish(root, scale, false, started);
    }

    std::fs::create_dir_all(&base).map_err(io_error(scale, &base))?;
    let staging = cache::staging_in(&base, "synthetic-")?;
    build(scale, staging.path())?;
    // Sealed before it is published, so no one can ever see a writable
    // fixture at its final name.
    permissions::seal(staging.path()).map_err(io_error(scale, staging.path()))?;
    cache::claim(&staging.keep(), &root)?;
    finish(root, scale, true, started)
}

/// The repository's directory, ready to open read-only.
pub fn borrow(scale: Scale) -> Result<PathBuf, SyntheticError> {
    Ok(ensure(scale)?.root)
}

fn finish(
    root: PathBuf,
    scale: Scale,
    built: bool,
    started: Instant,
) -> Result<Report, SyntheticError> {
    let bytes = cache::tree_size(&root)?;
    Ok(Report {
        root,
        scale,
        built,
        elapsed: started.elapsed(),
        bytes,
    })
}

/// Builds the repository into `dir`, which already exists and is empty.
fn build(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    git::init_repo(dir).map_err(git_error(scale))?;

    let stream_path = dir.join("history.fi");
    write_stream(scale, &stream_path).map_err(io_error(scale, &stream_path))?;
    // `--done` makes a truncated stream an error instead of a fixture that
    // looks fine until a measurement disagrees with it.
    git::run_with_stdin_file(dir, &["fast-import", "--quiet", "--done"], &stream_path)
        .map_err(git_error(scale))?;
    // The stream lived inside the staging directory so that a crash cannot
    // leave a hundred megabytes behind anywhere else; it must not survive
    // into the published fixture.
    std::fs::remove_file(&stream_path).map_err(io_error(scale, &stream_path))?;

    drop_scratch_branch(scale, dir)?;
    add_many_refs(scale, dir)?;
    // The final tree is three small files however long the history is, so a
    // checkout costs nothing and the fixture behaves like a normal
    // repository rather than one with every file deleted.
    git::run(dir, &["reset", "-q", "--hard", "refs/heads/main"]).map_err(git_error(scale))?;
    if scale.commit_graph == CommitGraph::Present {
        git::run(dir, &["commit-graph", "write", "--reachable"]).map_err(git_error(scale))?;
    }
    check_size(scale, dir)?;

    let marker = dir.join(cache::MARKER);
    let note = format!(
        "{}\n{}\n",
        scale.name(),
        git::version().map_err(git_error(scale))?
    );
    std::fs::write(&marker, note).map_err(io_error(scale, &marker))
}

/// Every hundredth extra branch is this many commits ahead of its upstream.
const DRIFT_EVERY: u32 = 100;

/// Adds [`Scale::with_refs`]'s branches, upstreams and `[branch]` sections,
/// then packs every ref. Nothing to do when none were asked for.
fn add_many_refs(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    let branches = scale.many_refs / 2;
    if branches == 0 {
        return Ok(());
    }
    // One commit more than there are branches, so a drifted upstream can sit
    // on the commit after its branch's.
    let max_count = format!("--max-count={}", branches + 1);
    let listed =
        git::run(dir, &["rev-list", &max_count, "refs/heads/main"]).map_err(git_error(scale))?;
    let listed = String::from_utf8_lossy(&listed);
    let ids: Vec<&str> = listed.lines().collect();
    if ids.len() < 2 {
        // A history this short cannot give a drifted upstream a commit of
        // its own; reported, not papered over with a fixture of fewer refs.
        return Err(SyntheticError::WrongSize {
            scale: scale.name(),
            want: 2,
            got: u32::from(!ids.is_empty()),
        });
    }

    let mut transaction = String::new();
    let mut config = String::from(
        "[remote \"origin\"]\n\turl = https://example.invalid/many.git\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n",
    );
    for n in 0..branches {
        let at = n as usize % (ids.len() - 1);
        let upstream_at = if n % DRIFT_EVERY == 0 { at + 1 } else { at };
        let name = format!("many/{n:06}");
        transaction.push_str(&format!("create refs/heads/{name} {}\n", ids[at]));
        transaction.push_str(&format!(
            "create refs/remotes/origin/{name} {}\n",
            ids[upstream_at]
        ));
        config.push_str(&format!(
            "[branch \"{name}\"]\n\tremote = origin\n\tmerge = refs/heads/{name}\n"
        ));
    }

    let script = dir.join("many-refs.txt");
    std::fs::write(&script, transaction).map_err(io_error(scale, &script))?;
    git::run_with_stdin_file(dir, &["update-ref", "--stdin"], &script).map_err(git_error(scale))?;
    std::fs::remove_file(&script).map_err(io_error(scale, &script))?;
    git::run(dir, &["pack-refs", "--all", "--prune"]).map_err(git_error(scale))?;

    let config_path = dir.join(".git").join("config");
    let mut existing =
        std::fs::read_to_string(&config_path).map_err(io_error(scale, &config_path))?;
    existing.push_str(&config);
    std::fs::write(&config_path, existing).map_err(io_error(scale, &config_path))
}

/// Removes the branch the throwaway topics were built on, if any topic was
/// ever thrown away. At a scale too small for a single full cycle there is
/// none, which is why its absence is a legitimate answer and not an error.
fn drop_scratch_branch(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    let found = git::run(
        dir,
        &["for-each-ref", "--format=%(refname)", SCRATCH_BRANCH],
    )
    .map_err(git_error(scale))?;
    if found.is_empty() {
        return Ok(());
    }
    git::run(dir, &["update-ref", "-d", SCRATCH_BRANCH]).map_err(git_error(scale))?;
    Ok(())
}

/// Asserts the built repository holds exactly the number of commits asked
/// for. This is the generator's own end-to-end check: everything downstream
/// measures against this number.
fn check_size(scale: Scale, dir: &Path) -> Result<(), SyntheticError> {
    let counted = git::run(dir, &["rev-list", "--count", "--all"]).map_err(git_error(scale))?;
    let printed = String::from_utf8_lossy(&counted).trim().to_owned();
    let got: u32 = printed.parse().map_err(|_| SyntheticError::Uncountable {
        scale: scale.name(),
        output: printed,
    })?;
    if got == scale.commits {
        Ok(())
    } else {
        Err(SyntheticError::WrongSize {
            scale: scale.name(),
            want: scale.commits,
            got,
        })
    }
}

/// Writes the whole `fast-import` stream to `path`, a chunk at a time.
fn write_stream(scale: Scale, path: &Path) -> Result<(), io::Error> {
    let mut out = BufWriter::new(File::create(path)?);
    let mut history = History::new(scale.commits, scale.commit_size);
    history.write(&mut out)?;
    out.into_inner()
        .map_err(io::IntoInnerError::into_error)?
        .sync_all()
}

fn git_error(scale: Scale) -> impl FnOnce(GitError) -> SyntheticError {
    move |source| SyntheticError::Git {
        scale: scale.name(),
        source,
    }
}

fn io_error(scale: Scale, path: &Path) -> impl FnOnce(io::Error) -> SyntheticError {
    let path = path.to_path_buf();
    move |source| SyntheticError::Io {
        scale: scale.name(),
        path,
        source,
    }
}

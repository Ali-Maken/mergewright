//! Building the corpus once, and finding it afterwards.
//!
//! The cache key is a digest of the generator, and it is part of the path
//! (`target/fixtures/<key>/`). So "rebuilt only when the generating script
//! changes" needs no staleness check: a changed generator names a directory
//! that does not exist, and an old corpus is still valid for the generator
//! that made it.
//!
//! A build happens in a staging directory and is moved into place with a
//! single `rename`, which is atomic. Two test binaries running at once
//! therefore cannot see a half-built corpus, and the loser of the race throws
//! its own copy away rather than overwriting the winner's.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::digest;
use crate::filesystem::{self, Support};
use crate::git::{self, GitError};
use crate::scratch::Scratch;
use crate::shape::Shape;
use crate::shapes::{self, BuildError};

/// Written last, inside the staging directory. Its presence in a corpus
/// directory means every fixture in it finished.
const MARKER: &str = "corpus.ok";

/// Written inside a fixture's directory instead of the repository, when the
/// machine cannot hold that fixture. Its contents are the stated reason.
const UNAVAILABLE: &str = "UNAVAILABLE";

/// Set this to build the corpus somewhere other than the workspace's
/// `target/fixtures`.
const OVERRIDE_VAR: &str = "GRAFT_FIXTURES_DIR";

/// What one call to [`ensure`] found or did.
#[derive(Clone, Debug)]
pub struct Report {
    /// The directory holding one subdirectory per shape.
    pub root: PathBuf,
    /// `true` if this call built the corpus, `false` if it was already there.
    pub built: bool,
    /// How long the call took, building or not.
    pub elapsed: Duration,
    /// The corpus's total size on disk.
    pub bytes: u64,
    /// The shapes this machine cannot hold, and why.
    pub unavailable: Vec<Unavailable>,
}

/// A shape that was skipped, with the reason the skip must state.
#[derive(Clone, Debug)]
pub struct Unavailable {
    /// The shape that could not be built.
    pub shape: Shape,
    /// Why — naming the filesystem, for the non-UTF-8 shape.
    pub reason: String,
}

/// Why the corpus could not be produced.
#[derive(Debug, thiserror::Error)]
pub enum CorpusError {
    /// One fixture's `git` work failed.
    #[error(transparent)]
    Build(#[from] BuildError),
    /// `git` itself could not be run.
    #[error(transparent)]
    Git(#[from] GitError),
    /// A directory could not be created, read, moved or removed.
    #[error("{context}: {source}")]
    Io {
        /// What was being attempted.
        context: String,
        /// What the operating system said.
        source: io::Error,
    },
    /// The corpus is fine, but this machine cannot hold this shape.
    #[error("the {shape} fixture is not available here: {reason}")]
    Unavailable {
        /// The shape that was asked for.
        shape: Shape,
        /// Why it is missing.
        reason: String,
    },
    /// An earlier call in this process already failed; the message is that
    /// failure, kept so every later caller sees the real cause.
    #[error("the fixture corpus could not be built: {0}")]
    Unusable(String),
}

fn io_error(context: impl Into<String>) -> impl FnOnce(io::Error) -> CorpusError {
    let context = context.into();
    |source| CorpusError::Io { context, source }
}

/// Builds the corpus if it is not there, and reports on it.
///
/// Called any number of times from any number of threads: the first call does
/// the work and the rest wait for it.
pub fn ensure() -> Result<Report, CorpusError> {
    static ONCE: OnceLock<Result<Report, String>> = OnceLock::new();
    match ONCE.get_or_init(|| build_once().map_err(|why| why.to_string())) {
        Ok(report) => Ok(report.clone()),
        Err(why) => Err(CorpusError::Unusable(why.clone())),
    }
}

/// The directory holding one fixture, ready to open read-only.
pub fn try_borrow(shape: Shape) -> Result<PathBuf, CorpusError> {
    let report = ensure()?;
    if let Some(skipped) = report.unavailable.iter().find(|s| s.shape == shape) {
        return Err(CorpusError::Unavailable {
            shape,
            reason: skipped.reason.clone(),
        });
    }
    Ok(report.root.join(shape.name()))
}

/// A writable copy of one fixture, deleted when the returned value is
/// dropped.
pub fn try_copy_into_temp(shape: Shape) -> Result<Scratch, CorpusError> {
    let source = try_borrow(shape)?;
    Scratch::from(&source, shape.name()).map_err(io_error(format!(
        "copying the {shape} fixture to a scratch directory"
    )))
}

/// Builds the corpus under `base` instead of the shared cache, and reports
/// on it.
///
/// The determinism test needs two corpora built independently so it can
/// compare their object ids; [`ensure`]'s once-only cache is exactly what
/// stops that. Everything else about the build is the same, digest-named
/// subdirectory included.
pub fn build_into(base: &Path) -> Result<Report, CorpusError> {
    build_in(base)
}

fn build_once() -> Result<Report, CorpusError> {
    build_in(&base_dir())
}

fn build_in(base: &Path) -> Result<Report, CorpusError> {
    let started = Instant::now();
    let root = base.join(digest::corpus_key()?);

    if root.join(MARKER).is_file() {
        return finish(root, false, started);
    }

    fs::create_dir_all(base).map_err(io_error(format!("creating {}", base.display())))?;
    let staging = build_staging(base)?;
    claim(&staging, &root)?;
    finish(root, true, started)
}

/// Builds every fixture into a fresh directory under `base` and returns it.
fn build_staging(base: &Path) -> Result<PathBuf, CorpusError> {
    let staging = tempfile::Builder::new()
        .prefix("staging-")
        .tempdir_in(base)
        .map_err(io_error(format!(
            "creating a staging directory in {}",
            base.display()
        )))?;

    // Streams are large-ish and belong to nobody afterwards, so they live
    // outside the directory that becomes the corpus.
    let streams = tempfile::Builder::new()
        .prefix("streams-")
        .tempdir_in(base)
        .map_err(io_error(format!(
            "creating a stream directory in {}",
            base.display()
        )))?;

    // Probed inside the staging directory, so the answer is about the
    // filesystem the corpus will actually live on.
    let non_utf8 = filesystem::non_utf8_names(staging.path()).map_err(io_error(
        "probing the filesystem for non-UTF-8 name support",
    ))?;

    for shape in Shape::ALL {
        let dir = staging.path().join(shape.name());
        match (*shape, &non_utf8) {
            (Shape::NonUtf8Names, Support::Unavailable(reason)) => skip(&dir, reason)?,
            _ => shapes::build(*shape, &dir, streams.path())?,
        }
    }

    let marker = staging.path().join(MARKER);
    fs::write(&marker, format!("{}\n", git::version()?))
        .map_err(io_error(format!("writing {}", marker.display())))?;
    Ok(staging.keep())
}

/// Records that a shape could not be built here, in a form a later run can
/// read back without rebuilding.
fn skip(dir: &Path, reason: &str) -> Result<(), CorpusError> {
    fs::create_dir_all(dir).map_err(io_error(format!("creating {}", dir.display())))?;
    let note = dir.join(UNAVAILABLE);
    fs::write(&note, format!("{reason}\n")).map_err(io_error(format!("writing {}", note.display())))
}

/// Moves a finished staging directory to its final name, or — if another
/// process got there first — throws it away and uses theirs.
fn claim(staging: &Path, root: &Path) -> Result<(), CorpusError> {
    match fs::rename(staging, root) {
        Ok(()) => Ok(()),
        Err(why) if root.join(MARKER).is_file() => {
            // A concurrent build won the race. Its corpus is identical to
            // ours by construction — same key, same generator — so the only
            // thing to do is stop littering.
            fs::remove_dir_all(staging).map_err(io_error(format!(
                "removing {} after losing the build race ({why})",
                staging.display()
            )))
        }
        Err(source) => Err(CorpusError::Io {
            context: format!("moving {} to {}", staging.display(), root.display()),
            source,
        }),
    }
}

fn finish(root: PathBuf, built: bool, started: Instant) -> Result<Report, CorpusError> {
    let unavailable = read_skips(&root)?;
    let bytes = tree_size(&root).map_err(io_error(format!("measuring {}", root.display())))?;
    Ok(Report {
        root,
        built,
        elapsed: started.elapsed(),
        bytes,
        unavailable,
    })
}

fn read_skips(root: &Path) -> Result<Vec<Unavailable>, CorpusError> {
    let mut found = BTreeMap::new();
    for shape in Shape::ALL {
        let note = root.join(shape.name()).join(UNAVAILABLE);
        match fs::read_to_string(&note) {
            Ok(reason) => {
                found.insert(*shape, reason.trim().to_owned());
            }
            Err(why) if why.kind() == io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(CorpusError::Io {
                    context: format!("reading {}", note.display()),
                    source,
                });
            }
        }
    }
    Ok(found
        .into_iter()
        .map(|(shape, reason)| Unavailable { shape, reason })
        .collect())
}

fn tree_size(dir: &Path) -> Result<u64, io::Error> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            total += tree_size(&entry.path())?;
        } else if kind.is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

/// Where the corpus is built: the workspace's `target/fixtures`, unless
/// `CARGO_TARGET_DIR` moved `target` or [`OVERRIDE_VAR`] moved the corpus.
fn base_dir() -> PathBuf {
    if let Some(explicit) = std::env::var_os(OVERRIDE_VAR) {
        return PathBuf::from(explicit);
    }
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => workspace_root().join("target"),
    };
    target.join("fixtures")
}

/// `crates/graft-fixtures` is two levels below the workspace root, and the
/// manifest directory is fixed at compile time, so this is the same answer
/// whether a binary or a test binary is asking.
fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .unwrap_or(manifest)
        .to_path_buf()
}

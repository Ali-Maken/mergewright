//! Building the corpus once, and finding it afterwards.
//!
//! The corpus lives at `target/fixtures/<generator digest>/`. Caching,
//! atomic publication and the build race are [`crate::cache`]'s job — shared
//! with the heavy tier's synthetic repository, so both get the same
//! guarantees. What is here is what is specific to the named corpus: one
//! directory per [`Shape`], and an honest record of the shapes this machine
//! cannot hold.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::cache::{self, CacheError};
use crate::digest;
use crate::filesystem::{self, Support};
use crate::git::{self, GitError};
use crate::permissions;
use crate::scratch::Scratch;
use crate::shape::Shape;
use crate::shapes::{self, BuildError};

/// Written inside a fixture's directory instead of the repository, when the
/// machine cannot hold that fixture. Its contents are the stated reason.
const UNAVAILABLE: &str = "UNAVAILABLE";

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
    /// The corpus could not be cached, published or measured.
    #[error(transparent)]
    Cache(#[from] CacheError),
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
    match ONCE.get_or_init(|| build_into(&cache::base_dir()).map_err(|why| why.to_string())) {
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
    let started = Instant::now();
    let root = base.join(digest::corpus_key()?);

    if cache::is_built(&root) {
        return finish(root, false, started);
    }

    fs::create_dir_all(base).map_err(io_error(format!("creating {}", base.display())))?;
    let staging = build_staging(base)?;
    // Sealed before it is published, so no one can ever see a writable
    // corpus at its final name.
    permissions::seal(&staging).map_err(io_error(format!("sealing {}", staging.display())))?;
    cache::claim(&staging, &root)?;
    finish(root, true, started)
}

/// Builds every fixture into a fresh directory under `base` and returns it.
fn build_staging(base: &Path) -> Result<PathBuf, CorpusError> {
    let staging = cache::staging_in(base, "staging-")?;

    // Streams are large-ish and belong to nobody afterwards, so they live
    // outside the directory that becomes the corpus.
    let streams = cache::staging_in(base, "streams-")?;

    // Probed inside the staging directory, so the answer is about the
    // filesystem the corpus will actually live on.
    let non_utf8 = filesystem::non_utf8_names(staging.path()).map_err(io_error(
        "probing the filesystem for non-UTF-8 name support",
    ))?;

    for shape in Shape::every() {
        let dir = staging.path().join(shape.name());
        if let (Shape::NonUtf8RefName, Support::Unavailable(reason)) = (shape, &non_utf8) {
            skip(&dir, reason)?;
        } else {
            shapes::build(shape, &dir, streams.path())?;
        }
    }

    let marker = staging.path().join(cache::MARKER);
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

fn finish(root: PathBuf, built: bool, started: Instant) -> Result<Report, CorpusError> {
    let unavailable = read_skips(&root)?;
    let bytes = cache::tree_size(&root)?;
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
    for shape in Shape::every() {
        let note = root.join(shape.name()).join(UNAVAILABLE);
        match fs::read_to_string(&note) {
            Ok(reason) => {
                found.insert(shape, reason.trim().to_owned());
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

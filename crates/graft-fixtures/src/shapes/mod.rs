//! How each named shape is built.
//!
//! One function per [`Shape`], grouped by what the shape is for: plain
//! histories in [`history`], the merge zoo in [`merges`], and everything
//! about refs and `HEAD` in [`refs`]. Each one writes a `fast-import` stream
//! and, if the shape needs something `fast-import` cannot express — a
//! detached `HEAD`, a stash, an upstream — follows it with a few plain `git`
//! commands. The split is deliberate: topology comes from the stream, because
//! that is what makes object ids reproducible; everything else is state
//! *around* the objects, where `git` itself is the only honest way to produce
//! what git would produce.

mod history;
mod merges;
mod refs;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::git::{self, GitError};
use crate::shape::Shape;
use crate::stream::{FileContent, Stream};

/// The default branch of every fixture.
pub(super) const MAIN: &[u8] = b"refs/heads/main";

/// The remote every fixture with remote-tracking refs pretends to have.
/// Unreachable by design: `.invalid` can never resolve (RFC 2606), so a bug
/// that makes Graft touch the network fails loudly instead of quietly
/// working.
pub(super) const REMOTE_URL: &str = "https://example.invalid/graft.git";

/// Why one fixture could not be built.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// A `git` command failed.
    #[error("building the {shape} fixture: {source}")]
    Git {
        /// The shape being built.
        shape: Shape,
        /// What `git` did.
        source: GitError,
    },
    /// A file could not be written or a directory created.
    #[error("building the {shape} fixture: cannot write {}: {source}", path.display())]
    Io {
        /// The shape being built.
        shape: Shape,
        /// The path that could not be written.
        path: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
}

/// One fixture's repository, mid-build.
pub struct Site<'a> {
    shape: Shape,
    /// The repository's working-tree root.
    dir: &'a Path,
    /// Where `fast-import` streams are parked. Outside `dir`, so no stream
    /// ends up inside the fixture it built.
    streams: &'a Path,
}

impl<'a> Site<'a> {
    /// Creates the directory and runs `git init` in it.
    pub fn init(shape: Shape, dir: &'a Path, streams: &'a Path) -> Result<Self, BuildError> {
        let site = Site {
            shape,
            dir,
            streams,
        };
        site.mkdir(dir)?;
        // `--object-format=sha1` is stated rather than inherited: the corpus
        // records object ids, and a machine configured for sha256 by default
        // would silently produce a different corpus.
        site.git(&["init", "-q", "-b", "main", "--object-format=sha1", "."])?;
        Ok(site)
    }

    /// Runs one `git` in the fixture and returns its stdout.
    pub fn git(&self, args: &[&str]) -> Result<Vec<u8>, BuildError> {
        git::run(self.dir, args).map_err(|source| BuildError::Git {
            shape: self.shape,
            source,
        })
    }

    /// Feeds a finished stream to `git fast-import`.
    pub fn import(&self, stream: Stream) -> Result<(), BuildError> {
        let path = self.streams.join(format!("{}.fi", self.shape.name()));
        self.write_file(&path, &stream.finish())?;
        // `--done` makes a truncated stream an error instead of a partial
        // corpus that looks fine until a test disagrees with it.
        git::run_with_stdin_file(self.dir, &["fast-import", "--quiet", "--done"], &path).map_err(
            |source| BuildError::Git {
                shape: self.shape,
                source,
            },
        )?;
        Ok(())
    }

    /// Puts the working tree and index on the default branch.
    pub fn checkout_main(&self) -> Result<(), BuildError> {
        self.git(&["reset", "-q", "--hard", "refs/heads/main"])?;
        Ok(())
    }

    /// Writes a file in the working tree.
    pub fn write(&self, relative: &str, content: &str) -> Result<(), BuildError> {
        let path = self.dir.join(relative);
        self.write_file(&path, content.as_bytes())
    }

    fn mkdir(&self, path: &Path) -> Result<(), BuildError> {
        fs::create_dir_all(path).map_err(|source| BuildError::Io {
            shape: self.shape,
            path: path.to_path_buf(),
            source,
        })
    }

    fn write_file(&self, path: &Path, content: &[u8]) -> Result<(), BuildError> {
        fs::write(path, content).map_err(|source| BuildError::Io {
            shape: self.shape,
            path: path.to_path_buf(),
            source,
        })
    }
}

/// One file in a commit.
///
/// Spelled out as a call rather than a bare tuple so that each byte-string
/// literal coerces to a slice on its own; written as a tuple, a list of files
/// with different name lengths would try to unify as fixed-size arrays.
pub(super) fn file<'a>(path: &'a [u8], content: &'a [u8]) -> FileContent<'a> {
    (path, content)
}

/// Builds one fixture into `dir`, which must not exist yet.
pub fn build(shape: Shape, dir: &Path, streams: &Path) -> Result<(), BuildError> {
    let site = Site::init(shape, dir, streams)?;
    match shape {
        Shape::Trivial => history::trivial(&site),
        Shape::Linear => history::linear(&site),
        Shape::FeatureBranches => history::feature_branches(&site),
        Shape::OctopusMerge => merges::octopus_merge(&site),
        Shape::CrissCrossMerge => merges::criss_cross_merge(&site),
        Shape::FoxtrotMerge => merges::foxtrot_merge(&site),
        Shape::BackwardMerge => merges::backward_merge(&site),
        Shape::EvilMerge => merges::evil_merge(&site),
        Shape::AccidentalPullMerge => merges::accidental_pull_merge(&site),
        Shape::OrphanBranch => history::orphan_branch(&site),
        Shape::CherryPickDuplicates => history::cherry_pick_duplicates(&site),
        Shape::Tags => refs::tags(&site),
        Shape::UnicodeNames => refs::unicode_names(&site),
        Shape::RefStates => refs::ref_states(&site),
        Shape::DetachedHead => refs::detached_head(&site),
        Shape::UnbornBranch => refs::unborn_branch(&site),
        Shape::Stash => refs::stash(&site),
        Shape::NonUtf8Path => refs::non_utf8_path(&site),
        Shape::NonUtf8RefName => refs::non_utf8_ref_name(&site),
    }
}

//! Why `HEAD` could not be read or resolved.

use std::path::PathBuf;

/// Why `HEAD` could not be read.
#[derive(Debug, thiserror::Error)]
pub enum HeadError {
    /// `HEAD` itself could not be read.
    #[error("cannot read HEAD in {}: {source}", git_dir.display())]
    Read {
        /// The repository whose `HEAD` was being read.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `HeadError` small.
        source: Box<gix::reference::find::existing::Error>,
    },
    /// `HEAD` was read but could not be followed to an object.
    #[error("cannot resolve HEAD in {}: {source}", git_dir.display())]
    Peel {
        /// The repository whose `HEAD` was being resolved.
        git_dir: PathBuf,
        /// What `gix` said. Boxed to keep `HeadError` small.
        source: Box<gix::head::peel::Error>,
    },
    /// `HEAD` was read, but names something git's own rules reject: a ref name
    /// with a control character in it, or an object id of the wrong length.
    #[error("HEAD in {} is malformed: {source}", git_dir.display())]
    Malformed {
        /// The repository whose `HEAD` was being read.
        git_dir: PathBuf,
        /// Which invariant the value broke.
        source: mergewright_core::error::Error,
    },
}

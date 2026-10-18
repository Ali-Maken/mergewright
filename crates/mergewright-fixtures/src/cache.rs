//! Where a built fixture is cached, and how it is published atomically.
//!
//! Two things are built and cached here: the named corpus ([`crate::corpus`])
//! and, in the heavy tier, the synthetic repository ([`crate::synthetic`]).
//! They are different generators with different contents, and they share this
//! module so that both get the same three guarantees rather than two
//! implementations of them:
//!
//! - **Rebuilt only when the generator changes**, because the generator's
//!   digest is part of the path. A changed generator names a directory that
//!   does not exist yet, so there is no staleness check to get wrong and an
//!   old build is still valid for the generator that made it.
//! - **Never half-visible.** A build happens in a staging directory and is
//!   moved into place with a single `rename`, which is atomic. Two test
//!   binaries running at once cannot see a partial build.
//! - **Never two winners.** The loser of a race throws its own copy away
//!   rather than overwriting the winner's.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// Written last, inside the staging directory. Its presence in a cached
/// directory means everything in it finished.
pub(crate) const MARKER: &str = "corpus.ok";

/// Set this to build fixtures somewhere other than the workspace's
/// `target/fixtures`.
const OVERRIDE_VAR: &str = "MWR_FIXTURES_DIR";

/// Why a cached build could not be published or measured.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    /// A staging directory could not be created.
    #[error("creating a staging directory in {}: {source}", base.display())]
    Staging {
        /// The directory it was to be created in.
        base: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// A finished build could not be moved to its final name.
    #[error("moving {} to {}: {source}", staging.display(), root.display())]
    Publish {
        /// The finished staging directory.
        staging: PathBuf,
        /// The name it was to take.
        root: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
    /// Another process published first, and this one could not clean up after
    /// itself. Its own copy is still on disk.
    #[error(
        "another build published {} first, and removing this one's {} failed: \
         {source} (the rename had failed with: {rename})",
        root.display(),
        staging.display()
    )]
    LostRace {
        /// The staging directory that is still on disk.
        staging: PathBuf,
        /// The name the winner took.
        root: PathBuf,
        /// Why the rename failed, which is what revealed the race.
        rename: io::Error,
        /// Why the cleanup failed.
        source: io::Error,
    },
    /// A built tree could not be measured.
    #[error("measuring {}: {source}", root.display())]
    Measure {
        /// The tree being measured.
        root: PathBuf,
        /// What the operating system said.
        source: io::Error,
    },
}

/// Where fixtures are built: the workspace's `target/fixtures`, unless
/// `CARGO_TARGET_DIR` moved `target` or [`OVERRIDE_VAR`] moved the fixtures.
pub(crate) fn base_dir() -> PathBuf {
    if let Some(explicit) = std::env::var_os(OVERRIDE_VAR) {
        return PathBuf::from(explicit);
    }
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => workspace_root().join("target"),
    };
    target.join("fixtures")
}

/// `crates/mergewright-fixtures` is two levels below the workspace root, and the
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

/// A fresh directory under `base` to build into.
pub(crate) fn staging_in(base: &Path, prefix: &str) -> Result<TempDir, CacheError> {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir_in(base)
        .map_err(|source| CacheError::Staging {
            base: base.to_path_buf(),
            source,
        })
}

/// `true` if a finished build is already cached at `root`.
pub(crate) fn is_built(root: &Path) -> bool {
    root.join(MARKER).is_file()
}

/// Moves a finished staging directory to its final name, or — if another
/// process got there first — throws it away and uses theirs.
pub(crate) fn claim(staging: &Path, root: &Path) -> Result<(), CacheError> {
    match fs::rename(staging, root) {
        Ok(()) => Ok(()),
        Err(rename) if is_built(root) => {
            // A concurrent build won the race. Its contents are identical to
            // ours by construction — same key, same generator — so the only
            // thing to do is stop littering.
            fs::remove_dir_all(staging).map_err(|source| CacheError::LostRace {
                staging: staging.to_path_buf(),
                root: root.to_path_buf(),
                rename,
                source,
            })
        }
        Err(source) => Err(CacheError::Publish {
            staging: staging.to_path_buf(),
            root: root.to_path_buf(),
            source,
        }),
    }
}

/// Every byte of every file under `root`.
pub(crate) fn tree_size(root: &Path) -> Result<u64, CacheError> {
    walk(root).map_err(|source| CacheError::Measure {
        root: root.to_path_buf(),
        source,
    })
}

fn walk(dir: &Path) -> Result<u64, io::Error> {
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            total += walk(&entry.path())?;
        } else if kind.is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

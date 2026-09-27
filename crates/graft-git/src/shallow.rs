//! Where a shallow clone's history was cut (m7-power-tools/32).
//!
//! `git clone --depth N` writes `<common_dir>/shallow`: one hex id per line,
//! each a commit whose parents were not fetched. Git treats each listed
//! commit as parentless (`git rev-list --parents`, `git log --graph`), and
//! so does every Graft reader: the history walks, `read_commit`, blame's
//! boundary mark and trace's boundary rule all read this one list.

use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;

use graft_core::oid::Oid;

use crate::repo::RepoHandle;

/// The commits `<common_dir>/shallow` lists: where a shallow clone's history
/// was cut. Git treats each as parentless.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShallowList {
    ids: BTreeSet<Oid>,
}

impl ShallowList {
    /// No file: an empty list. A file that can't be read, or a line that
    /// isn't an id: an error — never "not shallow".
    ///
    /// # Errors
    /// [`ShallowError`] naming the file and why it could not be read.
    pub fn read(repo: &RepoHandle) -> Result<ShallowList, ShallowError> {
        let path = repo.common_dir().join("shallow");
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(ShallowList::default());
            }
            Err(source) => return Err(ShallowError { path, source }),
        };
        let ids = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|hex| {
                Oid::from_hex(hex).map_err(|why| ShallowError {
                    path: path.clone(),
                    source: io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{hex:?} is not a commit id: {why}"),
                    ),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(ShallowList { ids })
    }

    /// Whether the repository is complete: nothing was cut.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Whether `id` is a boundary: git treats it as having no parents.
    #[must_use]
    pub fn contains(&self, id: &Oid) -> bool {
        self.ids.contains(id)
    }

    /// The listed ids, for a reader that keeps a set of its own (trace).
    pub(crate) fn into_ids(self) -> BTreeSet<Oid> {
        self.ids
    }
}

/// `<common_dir>/shallow` exists but could not be read, or holds a line that
/// is not a commit id.
#[derive(Debug, thiserror::Error)]
#[error("couldn't read {}: {source}", path.display())]
pub struct ShallowError {
    /// The shallow file.
    pub path: PathBuf,
    /// Why it could not be read.
    #[source]
    pub source: io::Error,
}

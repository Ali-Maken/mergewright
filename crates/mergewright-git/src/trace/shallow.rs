//! A shallow clone's boundary (m7-power-tools/18c). git diffs a boundary
//! commit against nothing, so every line in it reads as added: a trace
//! must not name it as where the lines began. The list itself is read by
//! [`crate::shallow::ShallowList`], the one reader every walk shares (32).

use std::collections::BTreeSet;

use mergewright_core::oid::Oid;

use super::TraceError;
use crate::repo::RepoHandle;
use crate::shallow::ShallowList;

/// The boundary commits git lists in `<common_dir>/shallow`. No file: not
/// shallow, an empty set. Any other failure to read it — or a line that is
/// not an id — is an error, never "not shallow".
pub(crate) fn shallow_boundaries(repo: &RepoHandle) -> Result<BTreeSet<Oid>, TraceError> {
    ShallowList::read(repo)
        .map(ShallowList::into_ids)
        .map_err(|error| TraceError::Shallow {
            path: error.path,
            source: error.source,
        })
}

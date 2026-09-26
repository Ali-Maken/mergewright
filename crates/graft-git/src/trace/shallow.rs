//! A shallow clone's boundary (m7-power-tools/18c). git diffs a boundary
//! commit against nothing, so every line in it reads as added: a trace
//! must not name it as where the lines began.

use std::collections::BTreeSet;
use std::io;

use graft_core::oid::Oid;

use super::TraceError;
use crate::repo::RepoHandle;

/// The boundary commits git lists in `<common_dir>/shallow`, one hex id per
/// line. No file: not shallow, an empty set. Any other failure to read it —
/// or a line that is not an id — is an error, never "not shallow".
pub(crate) fn shallow_boundaries(repo: &RepoHandle) -> Result<BTreeSet<Oid>, TraceError> {
    let path = repo.common_dir().join("shallow");
    let failed = |source| TraceError::Shallow {
        path: path.clone(),
        source,
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(error) => return Err(failed(error)),
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|hex| {
            Oid::from_hex(hex).map_err(|why| {
                failed(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{hex:?} is not a commit id: {why}"),
                ))
            })
        })
        .collect()
}

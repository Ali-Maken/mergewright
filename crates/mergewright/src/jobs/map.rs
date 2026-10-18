//! The Map view's job: one `build_map` run off the loop's thread.
//!
//! Cheap enough to redo from scratch every time the view opens (`map.rs`'s
//! own doc comment) — this file is the thinnest possible wrapper: read the
//! store, call `build_map`, report progress and the one `Arc<MapData>` (or
//! why not) back. No git bytes and no assembly happen in `update`.

use std::sync::Arc;

use crate::map::{self, MapData, MapRequest};
use crate::repos::RepoStore;

use super::Cancel;

/// Build the Map view's data for `req`, reporting `(done, total)` once per
/// finished row through `progress`.
///
/// A `stop` that fires is not folded into `Outcome::Cancelled`: the
/// acceptance criteria for this job is that a stopped run's product is an
/// `Err` line like any other failure, not a silent cancellation — `update`
/// treats a stale id the same way either way, and the difference only
/// matters to whoever is testing the job directly.
pub(super) fn build_map(
    repos: &RepoStore,
    req: &MapRequest,
    progress: &mut dyn FnMut(u32, u32),
    stop: &Cancel,
) -> Result<Arc<MapData>, String> {
    map::build_map(repos, req, progress, stop)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

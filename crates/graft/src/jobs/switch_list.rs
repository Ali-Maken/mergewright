//! The branch picker's read (m7-power-tools/10): `graft_ops::switch_list`,
//! off the loop's thread like every other read.

use std::sync::Arc;

use graft_ops::switch_list::{SwitchList, switch_list};

use super::Cancel;
use crate::repos::RepoStore;

/// Every branch the picker lists, in drawn order, or why they could not be
/// read.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, or
/// a ref, `HEAD` or the remotes could not be read.
#[expect(
    dead_code,
    reason = "dispatched from `jobs::runner`, which 30j holds; the arm is ticket 10's seam (its NOTES)"
)]
pub(super) fn read_switch_list(
    repos: &RepoStore,
    cancel: &Cancel,
) -> Result<Arc<SwitchList>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    switch_list(&handle, cancel)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

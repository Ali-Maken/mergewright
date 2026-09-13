//! The lanes a layout starts with, from the repository's refs.

use std::collections::HashMap;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::branches::Branches;
use graft_git::repo::{RefEntry, RefTarget};
use graft_graph::{BranchIdentity, BranchSeed, Seeds};

const HEADS: &[u8] = b"refs/heads/";
const REMOTES: &[u8] = b"refs/remotes/";

/// The default branch on lane 0, the integration branches in the lanes
/// reserved beside it, and every local *and* remote-tracking branch tip known
/// for colour.
///
/// The same choice the corpus snapshots make (`graft-graph/tests/it/corpus.rs`).
/// Remote tips are included on purpose: when a remote is ahead of its local
/// branch, the lane opens at the remote's tip, and without it that lane would
/// carry a short SHA instead of the remote's name. A branch whose ref is
/// symbolic is left out — the ref it aliases carries the tip.
#[must_use]
pub fn seeds_for(refs: &[RefEntry], branches: &Branches) -> Seeds {
    let tips: HashMap<&[u8], Oid> = refs
        .iter()
        .filter_map(|entry| match entry.target {
            RefTarget::Object(id) => Some((entry.name.as_bytes(), id)),
            RefTarget::Symbolic(_) => None,
        })
        .collect();
    let seed = |name: &RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            tip,
            identity: BranchIdentity::of_name(name.shorthand()),
        })
    };
    Seeds {
        default_branch: branches.default_branch().and_then(seed),
        integration_branches: branches.integration().iter().filter_map(seed).collect(),
        known_branches: refs
            .iter()
            .filter(|entry| {
                let name = entry.name.as_bytes();
                name.starts_with(HEADS) || name.starts_with(REMOTES)
            })
            .filter_map(|entry| seed(&entry.name))
            .collect(),
    }
}

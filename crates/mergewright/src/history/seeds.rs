//! The lanes a layout starts with, from the repository's refs.

use std::collections::HashMap;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::branches::Branches;
use mergewright_git::repo::{RefEntry, RefTarget};
use mergewright_graph::{BranchIdentity, BranchSeed, Seeds};

use super::state::WalkFacts;
use crate::refs::RefBadges;

const HEADS: &[u8] = b"refs/heads/";
const REMOTES: &[u8] = b"refs/remotes/";

/// The default branch on lane 0, the integration branches in the lanes
/// reserved beside it, and every local *and* remote-tracking branch tip known
/// for colour.
///
/// The same choice the corpus snapshots make (`mergewright-graph/tests/it/corpus.rs`).
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

/// The seeds a Ghost Preview lays its bounded slice out with (m5/08).
///
/// The walk builds its own from the raw refs on its thread; the app never
/// sees those. It does see [`WalkFacts`], which already carries ADR-0001's
/// answer (which branch owns lane 0, which sit beside it), and
/// [`RefBadges`], which carries every branch tip — so the same seeds can be
/// rebuilt here without a second ref read.
///
/// They have to be the same seeds: the preview replaces the graph's rows
/// while it is open, and the whole point of drawing ghosts *inside* the
/// real picture is that the eye compares them against real rows. A preview
/// laid out with [`Seeds::default`] would put the default branch in a
/// different lane and every branch in a different colour, and the before
/// and after would read as two graphs rather than one.
#[must_use]
pub fn preview_seeds(walk: Option<&WalkFacts>, refs: &RefBadges) -> Seeds {
    let tips: HashMap<Vec<u8>, Oid> = refs
        .branches()
        .map(|(tip, short, _, _)| ([HEADS, short].concat(), tip))
        .chain(
            refs.remotes()
                .map(|(tip, short)| ([REMOTES, short].concat(), tip)),
        )
        .collect();
    let seed = |name: &RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            tip,
            identity: BranchIdentity::of_name(name.shorthand()),
        })
    };
    Seeds {
        default_branch: walk
            .and_then(|facts| facts.default_branch.as_ref())
            .and_then(seed),
        integration_branches: walk
            .map(|facts| facts.integration.as_ref())
            .unwrap_or_default()
            .iter()
            .filter_map(seed)
            .collect(),
        known_branches: tips
            .iter()
            .map(|(name, &tip)| BranchSeed {
                tip,
                identity: BranchIdentity::of_name(shorthand_of(name)),
            })
            .collect(),
    }
}

/// `refs/heads/main` → `main`, the same rule [`RefName::shorthand`] uses,
/// for a name already assembled as bytes.
fn shorthand_of(full: &[u8]) -> &[u8] {
    for prefix in [HEADS, REMOTES] {
        if let Some(rest) = full.strip_prefix(prefix) {
            return rest;
        }
    }
    full
}

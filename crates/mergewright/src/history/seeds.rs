//! The lanes a layout starts with, from the repository's refs.

use std::collections::{HashMap, HashSet};

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
    seeds_with_upstreams(refs, branches, &HashMap::new())
}

/// [`seeds_for`] that also knows each local branch's configured upstream
/// (`branch.<n>.merge`, read through the open repository, no process
/// spawned): a remote-tracking ref follows the local branch that names it
/// as upstream, even when the two are named differently; otherwise the
/// same-short-name rule applies.
#[must_use]
pub fn seeds_with_upstreams<S: std::hash::BuildHasher>(
    refs: &[RefEntry],
    branches: &Branches,
    upstreams: &HashMap<RefName, RefName, S>,
) -> Seeds {
    // remote-tracking full name -> the local branch's short name
    let followed: HashMap<&[u8], &[u8]> = upstreams
        .iter()
        .filter_map(|(local, remote)| {
            let short = local.as_bytes().strip_prefix(HEADS)?;
            Some((remote.as_bytes(), short))
        })
        .collect();
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
            follows: follows_of(name.as_bytes(), &followed, |branch| {
                tips.contains_key([HEADS, branch].concat().as_slice())
            }),
        })
    };
    Seeds {
        default_branch: branches.default_branch().and_then(seed),
        integration_branches: branches.integration().iter().filter_map(seed).collect(),
        known_branches: {
            // One identity per tip: where a local branch sits, a remote at
            // the same commit adds no seed. Otherwise the remote (listed
            // after `heads`) would win the tip and the lane would be
            // `origin/x` while the pane, `C` and the column look for `x`.
            let local: HashSet<Oid> = refs
                .iter()
                .filter(|entry| entry.name.as_bytes().starts_with(HEADS))
                .filter_map(|entry| tips.get(entry.name.as_bytes()).copied())
                .collect();
            refs.iter()
                .filter(|entry| {
                    let name = entry.name.as_bytes();
                    name.starts_with(HEADS)
                        || (name.starts_with(REMOTES)
                            && tips.get(name).is_none_or(|tip| !local.contains(tip)))
                })
                .filter_map(|entry| seed(&entry.name))
                .collect()
        },
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
    let local: HashSet<Oid> = tips
        .iter()
        .filter(|(name, _)| name.starts_with(HEADS))
        .map(|(_, &tip)| tip)
        .collect();
    // The same remote → local map the graph's seeds follow, from the badges'
    // read of the configured upstreams.
    let followed: HashMap<&[u8], &[u8]> = refs
        .followed()
        .iter()
        .map(|(remote, local)| (remote.as_slice(), local.as_slice()))
        .collect();
    let seed = |name: &RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            tip,
            identity: BranchIdentity::of_name(name.shorthand()),
            follows: follows_of(name.as_bytes(), &followed, |branch| {
                tips.contains_key([HEADS, branch].concat().as_slice())
            }),
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
            .filter(|(name, tip)| name.starts_with(HEADS) || !local.contains(*tip))
            .map(|(name, &tip)| BranchSeed {
                tip,
                identity: BranchIdentity::of_name(shorthand_of(name)),
                follows: follows_of(name, &followed, |branch| {
                    tips.contains_key([HEADS, branch].concat().as_slice())
                }),
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

/// The local branch a remote-tracking ref is the copy of: `refs/remotes/o/x`
/// follows the local branch whose configured upstream it is (`followed`), else
/// `refs/heads/x` when that exists (the `↑n to push` pairing), else nothing.
/// `None` for a local ref.
fn follows_of(
    full: &[u8],
    followed: &HashMap<&[u8], &[u8]>,
    local_exists: impl Fn(&[u8]) -> bool,
) -> Option<BranchIdentity> {
    let rest = full.strip_prefix(REMOTES)?;
    if let Some(local) = followed.get(full) {
        return Some(BranchIdentity::of_name(local));
    }
    let branch = rest.splitn(2, |&byte| byte == b'/').nth(1)?;
    local_exists(branch).then(|| BranchIdentity::of_name(branch))
}

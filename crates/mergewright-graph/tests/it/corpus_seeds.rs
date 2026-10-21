//! The seeds the app would build from a corpus repo's refs (split from `corpus.rs`).

use std::collections::HashMap;

use mergewright_core::oid::Oid;
use mergewright_git::branches::Branches;
use mergewright_git::repo::{RefEntry, RefTarget};
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::seeds::{BranchSeed, Seeds};

const HEADS: &[u8] = b"refs/heads/";
const REMOTES: &[u8] = b"refs/remotes/";

/// The seeds the app would build from these refs: the default branch on
/// lane 0, the integration branches next to it, and every local and
/// remote-tracking branch known — a remote tip ahead of its local branch is
/// a tip in its own right, and its lane should carry the remote's name, not
/// a short SHA.
pub fn seeds_for(refs: &[RefEntry], branches: &Branches) -> Seeds {
    let tips: HashMap<&[u8], Oid> = refs
        .iter()
        .filter_map(|entry| match entry.target {
            RefTarget::Object(id) => Some((entry.name.as_bytes(), id)),
            // A symbolic branch aliases another ref, whose own entry carries
            // the tip; nothing is lost by leaving the alias out.
            RefTarget::Symbolic(_) => None,
        })
        .collect();
    // As the app's seeds: a remote-tracking ref follows the local branch of
    // the same short name (ADR-0064), so a lane is never named `origin/x`.
    let seed = |name: &mergewright_core::ref_name::RefName| {
        tips.get(name.as_bytes()).map(|&tip| BranchSeed {
            follows: name
                .as_bytes()
                .strip_prefix(b"refs/remotes/")
                .and_then(|rest| rest.splitn(2, |&byte| byte == b'/').nth(1))
                .filter(|branch| {
                    tips.contains_key([b"refs/heads/".as_slice(), branch].concat().as_slice())
                })
                .map(BranchIdentity::of_name),
            ..BranchSeed::new(tip, BranchIdentity::of_name(name.shorthand()))
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

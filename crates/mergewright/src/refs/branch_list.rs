//! Every branch the branch pane may name (ease-pass 09 ruling R), sorted by
//! full name: the index `badges` builds once per refs read (ease-pass/18).
//!
//! The pane asks which branch is in focus on every update at rest, on every
//! loop turn mid-burst (`update::rest_owed`) and in every frame it draws.
//! Built per ask, the list cost 5–10 ms a call with 50 000 refs — every
//! name formatted, the list sorted, every name parsed for its lane
//! identity — and queued keys behind it (`docs/perf/18-lag.txt`). Here it
//! is built once, with the lane identity and the tips hash the asks need.
//!
//! Kinds (trunk, integration) are not held: they come from the walk's
//! names, which land apart from the refs. `update::branch_pane` gives each
//! entry its kind from [`BranchEntry::local`].

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_graph::identity::BranchIdentity;

use super::RefBadges;

/// Where a local branch's name lives.
const HEADS: &str = "refs/heads/";
/// Where a remote-tracking branch's name lives.
const REMOTES: &str = "refs/remotes/";

/// One branch: its full name, its tip, and the name its kind is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchEntry {
    /// Full ref name: `refs/heads/main`, `refs/remotes/origin/fix`.
    pub name: String,
    /// The commit it points at.
    pub tip: Oid,
    /// The local branch name it stands for: its shorthand for a local
    /// branch, the shorthand with the remote stripped for a remote one.
    local: Box<[u8]>,
}

impl BranchEntry {
    /// The local branch name its kind is read from (`main` of both
    /// `refs/heads/main` and `refs/remotes/origin/main`).
    #[must_use]
    pub fn local(&self) -> &[u8] {
        &self.local
    }
}

/// Every branch, sorted by full name, with the lookups the pane asks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BranchList {
    entries: Vec<BranchEntry>,
    /// Lane identity → the first entry, in name order, drawn under it.
    by_identity: HashMap<BranchIdentity, usize>,
    /// Local name → the first entry, in name order, standing for it.
    by_local: HashMap<Box<[u8]>, usize>,
    /// Every entry's name and tip, hashed in order.
    tips: u64,
}

impl BranchList {
    /// Each local branch, and each remote-tracking branch no local branch
    /// has as its upstream (ruling R), from `refs`.
    pub(super) fn of(refs: &RefBadges) -> Self {
        let locals = refs.branches().map(|(tip, short, _, _)| BranchEntry {
            name: format!("{HEADS}{}", String::from_utf8_lossy(short)),
            tip,
            local: short.into(),
        });
        let remotes = refs.remote_only().map(|(tip, short)| {
            let local = short
                .iter()
                .position(|&byte| byte == b'/')
                .map_or(short, |slash| &short[slash + 1..]);
            BranchEntry {
                name: format!("{REMOTES}{}", String::from_utf8_lossy(short)),
                tip,
                local: local.into(),
            }
        });
        let mut entries: Vec<BranchEntry> = locals.chain(remotes).collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let mut by_identity = HashMap::with_capacity(entries.len());
        let mut by_local = HashMap::with_capacity(entries.len());
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for (at, entry) in entries.iter().enumerate() {
            by_identity
                .entry(identity_of_name(&entry.name))
                .or_insert(at);
            by_local.entry(entry.local.clone()).or_insert(at);
            entry.name.hash(&mut hasher);
            entry.tip.hash(&mut hasher);
        }
        Self {
            entries,
            by_identity,
            by_local,
            tips: hasher.finish(),
        }
    }

    /// Every branch, sorted by full name.
    #[must_use]
    pub fn entries(&self) -> &[BranchEntry] {
        &self.entries
    }

    /// The first branch, in name order, whose lane identity is `identity`.
    #[must_use]
    pub fn drawn_under(&self, identity: BranchIdentity) -> Option<&BranchEntry> {
        self.entries.get(*self.by_identity.get(&identity)?)
    }

    /// The first branch, in name order, standing for local name `local`.
    #[must_use]
    pub fn standing_for(&self, local: &[u8]) -> Option<&BranchEntry> {
        self.entries.get(*self.by_local.get(local)?)
    }

    /// Every branch's name and tip, hashed in name order: what a branch
    /// read's key holds, so a moved tip asks again.
    #[must_use]
    pub fn tips(&self) -> u64 {
        self.tips
    }
}

/// The lane identity a branch of full name `name` is drawn under: the
/// layout's own short form (`RefName::shorthand`), not ops' display
/// `short_name`.
#[must_use]
pub fn identity_of_name(name: &str) -> BranchIdentity {
    match RefName::new(name.as_bytes()) {
        Ok(ref_name) => BranchIdentity::of_name(ref_name.shorthand()),
        // Safe to fall back: names here are built from names git listed,
        // which always parse; and `shorthand` of a name with no known
        // prefix is the whole name, which is what this draws.
        Err(_) => BranchIdentity::of_name(name.as_bytes()),
    }
}

impl RefBadges {
    /// Every branch the branch pane may name, built once per read.
    #[must_use]
    pub fn branch_list(&self) -> &BranchList {
        &self.branch_list
    }
}

//! Which refs sit on which commit, and how far each branch is from its
//! upstream: everything a ref badge needs (PROMPT.md §1.3).
//!
//! Read by a job, never by `update`: counting ahead/behind walks commits.
//! [`read`] gathers the refs, `HEAD` and each branch's upstream, and asks
//! [`AheadBehindCache`] for the counts — which only walks again when a
//! branch's tip or its upstream's tip has moved since it last counted.
//!
//! Names stay bytes here; `view` sanitises them on the way into `ui`.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasher;
use std::sync::{Mutex, PoisonError};

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::history::{AheadBehind, ahead_behind};
use mergewright_git::repo::{HeadState, RepoHandle};

use crate::repos::RepoStore;

const HEADS: &[u8] = b"refs/heads/";
const REMOTES: &[u8] = b"refs/remotes/";
const TAGS: &[u8] = b"refs/tags/";

/// The refs on every commit that has one, and where `HEAD` is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RefBadges {
    by_commit: HashMap<Oid, Vec<RefLabel>>,
    head: Option<Oid>,
    stashes: Vec<(Oid, usize)>,
}

impl RefBadges {
    /// The refs on `id`, in drawing order: `HEAD`, branches, remotes, tags,
    /// stash. Empty for a commit no ref names.
    #[must_use]
    pub fn on(&self, id: Oid) -> &[RefLabel] {
        self.by_commit.get(&id).map_or(&[], Vec::as_slice)
    }

    /// The commit `HEAD` resolves to; `None` while the branch is unborn.
    #[must_use]
    pub fn head(&self) -> Option<Oid> {
        self.head
    }

    /// The newest stash entry's commit, if there is a stash.
    #[must_use]
    pub fn stash(&self) -> Option<Oid> {
        self.stashes.first().map(|&(commit, _)| commit)
    }

    /// Every stash entry's commit with its index, `stash@{0}` first. The
    /// graph badges each of them, and `H` hides them all together — older
    /// entries live only in `refs/stash`'s reflog, which is why they had no
    /// badge until now (`KNOWN_GAPS.md`,
    /// m3-working-tree-undo-and-stash/06).
    #[must_use]
    pub fn stashes(&self) -> &[(Oid, usize)] {
        &self.stashes
    }

    /// How many labels the graph can badge, over every commit: branches,
    /// remotes, tags and whatever else [`Self::on`] lists. The frame trace's
    /// `refs=` (m8-polish-and-ship G-trace), a size, not a list of names.
    #[must_use]
    pub fn count(&self) -> usize {
        self.by_commit.values().map(Vec::len).sum()
    }

    /// Whether `id` is any stash entry's own commit (`stash@{n}`, within
    /// the list's cap of `mergewright_git::stash::MAX_ENTRIES`). The evil-merge
    /// probe skips these: a stash commit is a merge whose tree differs
    /// from its parents by construction.
    #[must_use]
    pub fn is_stash(&self, id: Oid) -> bool {
        self.stashes.iter().any(|&(commit, _)| commit == id)
    }

    /// Every local branch: its tip, shorthand name, whether `HEAD` is on it,
    /// and its upstream (`view::radar::branch_facts`). Order is not
    /// meaningful.
    pub fn branches(&self) -> impl Iterator<Item = (Oid, &[u8], bool, &Upstream)> {
        self.by_commit.iter().flat_map(|(&id, labels)| {
            labels.iter().filter_map(move |label| match label {
                RefLabel::Branch {
                    name,
                    current,
                    upstream,
                } => Some((id, name.as_slice(), *current, upstream)),
                _ => None,
            })
        })
    }

    /// Every remote-tracking branch that has a badge of its own, as its tip
    /// and shorthand (`origin/main`). Order is not meaningful.
    ///
    /// A remote level with a local branch tracking it carries no badge —
    /// that branch's own chip says so — and so is not listed. For the Ghost
    /// Preview's seeds ([`crate::history::preview_seeds`]) that is right:
    /// the lane is already seeded by the local branch at the same commit.
    pub fn remotes(&self) -> impl Iterator<Item = (Oid, &[u8])> {
        self.by_commit.iter().flat_map(|(&id, labels)| {
            labels.iter().filter_map(move |label| match label {
                RefLabel::Remote(name) => Some((id, name.as_slice())),
                _ => None,
            })
        })
    }

    /// Every tag that has a badge, as its commit and shorthand (`v2.3.0`).
    /// Order is not meaningful.
    ///
    /// What the name prompt's `exists` preview asks for a typed tag name —
    /// the branch half already had [`RefBadges::branches`], and asking the
    /// same source of truth for both keeps the two previews one rule
    /// (m5-history-and-ghost-preview/10).
    pub fn tags(&self) -> impl Iterator<Item = (Oid, &[u8])> {
        self.by_commit.iter().flat_map(|(&id, labels)| {
            labels.iter().filter_map(move |label| match label {
                RefLabel::Tag(name) => Some((id, name.as_slice())),
                _ => None,
            })
        })
    }
}

/// One ref on a commit, before sanitising.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RefLabel {
    /// `HEAD`, detached on this commit.
    DetachedHead,
    /// A local branch.
    Branch {
        /// Short name, `main`.
        name: Vec<u8>,
        /// `HEAD` is on it.
        current: bool,
        /// Where it stands against its upstream.
        upstream: Upstream,
    },
    /// A remote-tracking branch, short name `origin/main`. Left out when it is
    /// level with a local branch that tracks it: that branch's badge says so.
    Remote(Vec<u8>),
    /// A tag, short name `v2.3.0`.
    Tag(Vec<u8>),
    /// A stash entry, by its position in the list: `0` is `stash@{0}`.
    Stash(usize),
}

/// Where a local branch stands against its upstream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Upstream {
    /// No upstream configured, or the ref it names does not exist.
    None,
    /// Level with the upstream on remote `remote`.
    InSync {
        /// The remote's name, `origin`.
        remote: Vec<u8>,
    },
    /// Apart from the upstream by these counts.
    Apart(AheadBehind),
}

/// Ahead/behind counts already worked out, keyed by (branch tip, upstream
/// tip): a pair that has not moved is never walked twice.
///
/// Held by the job runner for the life of the run and shared by every refs
/// job, so a refresh after a fetch that moved one remote walks only that
/// branch.
#[derive(Debug, Default)]
pub struct AheadBehindCache {
    counts: Mutex<HashMap<(Oid, Oid), AheadBehind>>,
    #[cfg(test)]
    walked_on: Mutex<Vec<std::thread::ThreadId>>,
}

impl AheadBehindCache {
    /// The counts for `(local, upstream)`, walking only when this pair has
    /// not been counted before.
    ///
    /// # Errors
    /// The walk's failure, when it had to walk and could not.
    pub fn get_or_count(
        &self,
        local: Oid,
        upstream: Oid,
        walk: impl FnOnce() -> Result<AheadBehind, String>,
    ) -> Result<AheadBehind, String> {
        if let Some(&known) = self.lock().get(&(local, upstream)) {
            return Ok(known);
        }
        #[cfg(test)]
        self.walked_on
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(std::thread::current().id());
        // Not held across the walk: two jobs counting the same pair at once
        // both walk and agree, which beats one blocking the other.
        let counted = walk()?;
        self.lock().insert((local, upstream), counted);
        Ok(counted)
    }

    /// Every thread a walk ran on, in order (tests: proving it was never the
    /// loop's).
    #[cfg(test)]
    pub(crate) fn walked_on(&self) -> Vec<std::thread::ThreadId> {
        self.walked_on
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(Oid, Oid), AheadBehind>> {
        // A poisoned lock means a walk panicked mid-insert; the map itself is
        // still a valid set of finished counts, so it is safe to keep using.
        self.counts.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Reads the repository's refs into [`RefBadges`], counting ahead/behind
/// through `cache`.
///
/// # Errors
/// The failure text, ready to show: the repository, its refs, its `HEAD`,
/// an upstream setting or a walk could not be read.
pub fn read(store: &RepoStore, cache: &AheadBehindCache) -> Result<RefBadges, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    // The store's one reading, shared with first paint's history job: a
    // second pass over thousands of loose refs, on another worker at the
    // same time, cost first paint ~90 ms at 1M commits (m1-the-graph/55).
    let snapshot = store.refs().map_err(|error| error.to_string())?;
    let refs = &snapshot.peeled;
    let head = handle.head().map_err(|error| error.to_string())?;
    let mut upstreams = HashMap::new();
    for (name, _) in refs
        .iter()
        .filter(|(name, _)| name.as_bytes().starts_with(HEADS))
    {
        if let Some(upstream) = handle.upstream(name).map_err(|error| error.to_string())? {
            upstreams.insert(name.clone(), upstream);
        }
    }
    // The reflog only: `stash::list` would spawn a `diff-tree` per entry,
    // and a badge never says how many files a stash holds
    // (m3-working-tree-undo-and-stash/06).
    let stashes = mergewright_git::stash::commits(&handle, &()).map_err(|error| error.to_string())?;
    badges(refs, &head, &upstreams, &stashes, |local, upstream| {
        cache.get_or_count(local, upstream, || count(&handle, local, upstream))
    })
}

fn count(handle: &RepoHandle, local: Oid, upstream: Oid) -> Result<AheadBehind, String> {
    ahead_behind(handle, local, upstream).map_err(|error| error.to_string())
}

/// The badges, from what [`read`] gathered. Pure but for `count`, so every
/// ref state can be tested without a repository.
///
/// # Errors
/// Whatever `count` fails with.
pub fn badges<S: BuildHasher>(
    refs: &[(RefName, Oid)],
    head: &HeadState,
    upstreams: &HashMap<RefName, RefName, S>,
    stashes: &[(Oid, usize)],
    mut count: impl FnMut(Oid, Oid) -> Result<AheadBehind, String>,
) -> Result<RefBadges, String> {
    let tips: HashMap<&RefName, Oid> = refs.iter().map(|(name, id)| (name, *id)).collect();
    let mut result = RefBadges::default();
    // Remote-tracking refs whose local branch's badge already says "in sync".
    // A set, not a list: every remote-tracking ref is checked against it, and
    // with 25k in-sync branches a list made that check quadratic (1.3 s of a
    // 50k-ref read in debug, m1-the-graph/30d).
    let mut merged: HashSet<&RefName> = HashSet::new();

    let head_id = match head {
        HeadState::Detached(id) => {
            result.push(*id, RefLabel::DetachedHead);
            Some(*id)
        }
        HeadState::Branch(name) => tips.get(name).copied(),
        HeadState::Unborn(_) => None,
    };
    result.head = head_id;

    for (name, id) in refs {
        let full = name.as_bytes();
        if full.starts_with(HEADS) {
            let current = matches!(head, HeadState::Branch(on) if on == name);
            let upstream = match upstreams
                .get(name)
                .and_then(|up| Some((up, *tips.get(up)?)))
            {
                None => Upstream::None,
                Some((up, up_id)) => {
                    let counts = count(*id, up_id)?;
                    if counts.in_sync() {
                        merged.insert(up);
                        Upstream::InSync {
                            remote: remote_of(up).to_vec(),
                        }
                    } else {
                        Upstream::Apart(counts)
                    }
                }
            };
            result.push(
                *id,
                RefLabel::Branch {
                    name: full[HEADS.len()..].to_vec(),
                    current,
                    upstream,
                },
            );
        }
    }
    result.push_remotes_tags_and_stash(refs, &merged, stashes);
    Ok(result)
}

/// The remote a remote-tracking ref belongs to: `origin` for
/// `refs/remotes/origin/main`.
fn remote_of(name: &RefName) -> &[u8] {
    let short = &name.as_bytes()[REMOTES.len().min(name.as_bytes().len())..];
    short.split(|&byte| byte == b'/').next().unwrap_or(short)
}

impl RefBadges {
    /// Everything after the local branches: remote-tracking refs not merged
    /// into an in-sync branch's badge, then tags, then the stash.
    fn push_remotes_tags_and_stash(
        &mut self,
        refs: &[(RefName, Oid)],
        merged: &HashSet<&RefName>,
        stashes: &[(Oid, usize)],
    ) {
        let result = self;
        for (name, id) in refs {
            let full = name.as_bytes();
            if full.starts_with(REMOTES) {
                // `origin/HEAD` is a pointer to the remote's default branch, which
                // already has its own badge.
                if full.ends_with(b"/HEAD") || merged.contains(name) {
                    continue;
                }
                result.push(*id, RefLabel::Remote(full[REMOTES.len()..].to_vec()));
            }
        }
        for (name, id) in refs {
            let full = name.as_bytes();
            if full.starts_with(TAGS) {
                result.push(*id, RefLabel::Tag(full[TAGS.len()..].to_vec()));
            }
        }
        // Every entry, not only `refs/stash` itself: that ref names the
        // newest, and the rest live behind it in its reflog, which the
        // refs list does not carry at all.
        result.stashes = stashes.to_vec();
        for &(commit, index) in stashes {
            result.push(commit, RefLabel::Stash(index));
        }
    }

    fn push(&mut self, id: Oid, label: RefLabel) {
        self.by_commit.entry(id).or_default().push(label);
    }
}

//! The palette's finds (m8-polish-and-ship/12, ADR-0061): the branch and
//! tracked-path reads once per `⌃P` open (12b), and the per-keystroke name
//! match over branches, tags and paths. All run off the loop: matching 50k
//! ref names or 100k paths is not the loop's to do inside a 16 ms frame
//! (PROMPT §6).

use std::sync::Arc;

use graft_core::oid::Oid;
use graft_git::status::{StatusError, TrackedPaths, tracked_paths};
use graft_git::switch_refs::{SwitchRefs, switch_refs};
use graft_ops::matcher::fuzzy_score;

use super::Cancel;
use crate::refs::RefBadges;
use crate::repos::RepoStore;

/// The most rows of one group a names answer carries: a widened group's
/// cap (`update::palette::WIDE_ROWS`, kept equal by a test).
pub(crate) const TOP: usize = 50;

/// How many names are matched between two looks at the stop flag.
const STOP_EVERY: usize = 4096;

/// Every local and remote-tracking branch with its tip's date, read once
/// when `⌃P` opens (the `C` picker's read, `graft_git::switch_refs`).
///
/// # Errors
/// The failure text, ready to show in the BRANCHES group: the repository
/// could not be opened, or the refs could not be read.
pub(crate) fn read_branches(repos: &RepoStore, cancel: &Cancel) -> Result<Arc<SwitchRefs>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    switch_refs(&handle, cancel)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// Every tracked path, read once when `⌃P` opens (12b).
///
/// # Errors
/// The failure text, ready to show in the FILES group after `files: `:
/// `too many to list` past `graft_git::status::TRACKED_CAP`, else git's.
pub(crate) fn read_tracked(
    repos: &RepoStore,
    cancel: &Cancel,
) -> Result<Arc<TrackedPaths>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    tracked_paths(&handle, cancel)
        .map(Arc::new)
        .map_err(|error| tracked_failure_words(&error))
}

/// FILES' words for a failed read: the cap in his terms, else git's.
pub(crate) fn tracked_failure_words(error: &StatusError) -> String {
    match error {
        StatusError::TooLarge { .. } => "too many to list".to_owned(),
        other => other.to_string(),
    }
}

/// What one keystroke asks the names job: the query and the sources read
/// so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamesRequest {
    /// What he has typed, trimmed.
    pub query: Arc<str>,
    /// Every branch, once the open's read has landed.
    pub branches: Option<Arc<SwitchRefs>>,
    /// The refs as the graph has them, once the refs job has answered.
    pub tags: Option<Arc<RefBadges>>,
    /// Every tracked path, once the open's read has landed (12b).
    pub files: Option<Arc<TrackedPaths>>,
}

/// The names job's answer: per source, `None` when the request did not
/// carry it (not read yet), so the group still says `reading…`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct NameHits {
    /// BRANCHES.
    pub branches: Option<Ranked<RefHit>>,
    /// TAGS.
    pub tags: Option<Ranked<RefHit>>,
    /// FILES: indexes into the request's [`TrackedPaths`] (12b).
    pub files: Option<Ranked<u32>>,
}

/// The best rows of one group and how many matched in all.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Ranked<T> {
    /// At most [`TOP`], best first.
    pub top: Vec<T>,
    /// Every match, `top` included.
    pub total: usize,
}

/// One branch or tag that matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefHit {
    /// The short name: `main`, `origin/feat`, `v2.1.0`.
    pub name: Box<[u8]>,
    /// The commit it points at.
    pub tip: Oid,
    /// A remote-tracking branch.
    pub remote: bool,
}

/// A branch's rank: `fuzzy_score`'s tier, remote after local, newer first.
type BranchKey = (u32, bool, std::cmp::Reverse<i64>);

/// Matches `req.query` against every branch and tag name. Branches rank by
/// the `C` picker's rule — segment start first (`fuzzy_score`'s tier), then
/// local before remote, newer tip, name; tags by tier, then name. `None`
/// when `stop` said so (checked every [`STOP_EVERY`] names).
pub(crate) fn match_names(req: &NamesRequest, stop: &dyn Fn() -> bool) -> Option<NameHits> {
    let query = req.query.trim();
    let mut seen = 0usize;
    let mut stopped = || {
        seen += 1;
        seen.is_multiple_of(STOP_EVERY) && stop()
    };
    let branches = match &req.branches {
        Some(refs) => {
            let mut hits: Vec<(BranchKey, RefHit)> = Vec::new();
            for listing in &refs.refs {
                if stopped() {
                    return None;
                }
                let short = listing.name.shorthand();
                let Some(tier) = tier(query, short) else {
                    continue;
                };
                let remote = listing.name.as_bytes().starts_with(b"refs/remotes/");
                hits.push((
                    (tier, remote, std::cmp::Reverse(listing.when)),
                    RefHit {
                        name: short.into(),
                        tip: listing.tip,
                        remote,
                    },
                ));
            }
            Some(ranked(hits))
        }
        None => None,
    };
    let tags = match &req.tags {
        Some(badges) => Some(match_tags(query, badges, &mut stopped)?),
        None => None,
    };
    let files = match &req.files {
        Some(paths) => Some(match_files(query, paths, &mut stopped)?),
        None => None,
    };
    if stop() {
        return None;
    }
    Some(NameHits {
        branches,
        tags,
        files,
    })
}

/// TAGS: every tag `fuzzy_score` matches, by tier, then name. `None` when
/// `stopped` said so.
fn match_tags(
    query: &str,
    badges: &RefBadges,
    stopped: &mut dyn FnMut() -> bool,
) -> Option<Ranked<RefHit>> {
    let mut hits: Vec<(u32, RefHit)> = Vec::new();
    for (tip, name) in badges.tags() {
        if stopped() {
            return None;
        }
        let Some(tier) = tier(query, name) else {
            continue;
        };
        hits.push((
            tier,
            RefHit {
                name: name.into(),
                tip,
                remote: false,
            },
        ));
    }
    Some(ranked(hits))
}

/// FILES: every path `fuzzy_score` matches, best score, then shorter path,
/// then index order (a total key). `None` when `stopped` said so.
fn match_files(
    query: &str,
    paths: &TrackedPaths,
    stopped: &mut dyn FnMut() -> bool,
) -> Option<Ranked<u32>> {
    let mut hits: Vec<(u32, usize, u32)> = Vec::new();
    for (ix, path) in paths.iter().enumerate() {
        if stopped() {
            return None;
        }
        let Some(score) = fuzzy_score(query, &String::from_utf8_lossy(path)) else {
            continue;
        };
        // `TrackedPaths` holds under 4 GiB of path bytes, so far fewer
        // than `u32::MAX` paths: the conversion cannot fail.
        let Ok(ix) = u32::try_from(ix) else { break };
        hits.push((score, path.len(), ix));
    }
    let total = hits.len();
    if hits.len() > TOP {
        hits.select_nth_unstable(TOP - 1);
        hits.truncate(TOP);
    }
    hits.sort_unstable();
    Some(Ranked {
        top: hits.into_iter().map(|(_, _, ix)| ix).collect(),
        total,
    })
}

/// `fuzzy_score`'s tier for a name: 0 when every word starts a segment.
/// The offset is not a rank here, as in the `C` picker
/// (`graft_ops::switch_list::filter_switch_rows`): ties go to the newest.
fn tier(query: &str, name: &[u8]) -> Option<u32> {
    fuzzy_score(query, &String::from_utf8_lossy(name)).map(|score| score / 1024)
}

/// The best [`TOP`] of `hits` by key then name, and how many there were.
fn ranked<K: Ord>(mut hits: Vec<(K, RefHit)>) -> Ranked<RefHit> {
    let total = hits.len();
    let order =
        |a: &(K, RefHit), b: &(K, RefHit)| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name));
    if hits.len() > TOP {
        hits.select_nth_unstable_by(TOP - 1, order);
        hits.truncate(TOP);
    }
    hits.sort_by(order);
    Ranked {
        top: hits.into_iter().map(|(_, hit)| hit).collect(),
        total,
    }
}

#[cfg(test)]
mod tests;

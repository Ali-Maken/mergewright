//! One local branch's bounded walk into buckets: everything [`super::build_map`]
//! needs before it can build a row (ref lookups, the span, the default
//! branch's first-parent chain) and the walk itself
//! ([`walk_branch`]/[`RowWalk`]). Split out of `map.rs` (m2 review M8) —
//! [`super::rows`] is the layer above this one, one call to [`walk_branch`]
//! per row.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::{CommitTable, Order, Walk, first_parent_chain, read_commit};
use graft_git::run::Stop;
use graft_ops::mistake_radar::rules::classify_commit;
use graft_ops::mistake_radar::{CommitFacts, Policy, RepoFacts};

use super::{
    CHUNK_LIMIT, MAX_COMMITS_PER_ROW, MAX_ROWS, MAX_SPAN_DAYS, MIN_SPAN_DAYS, MapError, MapMark,
    SECONDS_PER_DAY, SKEW_TOLERANCE_SECONDS, bucket_index,
};

fn under(name: &RefName, prefix: &[u8]) -> bool {
    name.as_bytes().starts_with(prefix)
}

/// Local branches (`refs/heads/*`) and the default branch's tip among them,
/// if it has one.
pub(crate) fn local_branches(
    refs: &graft_git::repo::RefSnapshot,
    default_ref: Option<&RefName>,
) -> (Vec<(RefName, Oid)>, Option<Oid>) {
    let locals: Vec<(RefName, Oid)> = refs
        .peeled
        .iter()
        .filter(|(name, _)| under(name, b"refs/heads/"))
        .map(|(name, oid)| (name.clone(), *oid))
        .collect();
    let default_tip = default_ref
        .and_then(|name| locals.iter().find(|(n, _)| n == name))
        .map(|(_, oid)| *oid);
    (locals, default_tip)
}

/// Tags (`refs/tags/*`), by their short name and peeled commit.
pub(crate) fn tag_targets_of(refs: &graft_git::repo::RefSnapshot) -> Vec<(Vec<u8>, Oid)> {
    refs.peeled
        .iter()
        .filter(|(name, _)| under(name, b"refs/tags/"))
        .map(|(name, oid)| (name.shorthand().to_vec(), *oid))
        .collect()
}

/// Every local branch's tip commit date, read one `read_commit` at a time
/// (DESIGN.md §5.1's row order needs every tip's date before any row can be
/// sorted).
pub(crate) fn local_tip_dates(
    handle: &graft_git::repo::RepoHandle,
    locals: &[(RefName, Oid)],
    stop: &dyn Stop,
) -> Result<HashMap<RefName, i64>, MapError> {
    let mut tip_dates = HashMap::with_capacity(locals.len());
    for (name, oid) in locals {
        if stop.should_stop() {
            return Err(MapError::Stopped);
        }
        let commit =
            read_commit(handle, *oid).map_err(|error| MapError::Read(error.to_string()))?;
        tip_dates.insert(name.clone(), commit.committer.when.seconds);
    }
    Ok(tip_dates)
}

/// Row order: current branch first, then the default, then the rest by tip
/// date newest-first, capped at [`MAX_ROWS`]; the rest are only counted.
pub(crate) fn sort_and_cap_locals(
    mut locals: Vec<(RefName, Oid)>,
    current_ref: Option<&RefName>,
    default_ref: Option<&RefName>,
    tip_dates: &HashMap<RefName, i64>,
) -> (Vec<(RefName, Oid)>, usize) {
    let more = locals.len().saturating_sub(MAX_ROWS);
    locals.sort_by(|(a_name, a_oid), (b_name, b_oid)| {
        let a_current = current_ref == Some(a_name);
        let b_current = current_ref == Some(b_name);
        let a_default = default_ref == Some(a_name);
        let b_default = default_ref == Some(b_name);
        b_current
            .cmp(&a_current)
            .then_with(|| b_default.cmp(&a_default))
            .then_with(|| tip_dates[b_name].cmp(&tip_dates[a_name]))
            .then_with(|| a_oid.cmp(b_oid))
    });
    locals.truncate(MAX_ROWS);
    (locals, more)
}

/// The span every row's buckets share: `to` is `req`'s clock, `from` is the
/// oldest tip date, clamped to `[MIN_SPAN_DAYS, MAX_SPAN_DAYS]` before `to`.
pub(crate) fn time_span(tip_dates: &HashMap<RefName, i64>, to: i64) -> (i64, i64) {
    let earliest_tip = tip_dates.values().copied().min().unwrap_or(to);
    let from = earliest_tip.clamp(
        to - MAX_SPAN_DAYS * SECONDS_PER_DAY,
        to - MIN_SPAN_DAYS * SECONDS_PER_DAY,
    );
    (from, to)
}

/// The default branch's first-parent chain, as the set of [`Oid`]s on it —
/// what every other row's walk skips, and what marks a commit
/// `on_default_chain` for the Mistake Radar.
pub(crate) fn build_default_chain(
    handle: &graft_git::repo::RepoHandle,
    table: &Arc<CommitTable>,
    default_tip: Option<Oid>,
    stop: &dyn Stop,
) -> Result<HashSet<Oid>, MapError> {
    match default_tip {
        Some(tip) => {
            let handles = first_parent_chain(handle, table, tip, stop).map_err(MapError::Walk)?;
            Ok(handles
                .into_iter()
                .map(|handle| table.id_of(handle))
                .collect())
        }
        None => Ok(HashSet::new()),
    }
}

/// One row's finished walk: its bucket counts, marks, completeness, and the
/// set of commits it visited (default row only — `merged_into` reads it).
pub(crate) struct RowWalk {
    pub(crate) counts: Vec<u32>,
    pub(crate) marks: Vec<(usize, MapMark)>,
    pub(crate) complete: bool,
    pub(crate) visited: HashSet<Oid>,
}

/// A row's walk in progress: the accumulators [`walk_branch`] fills one
/// chunk at a time, split out so the per-commit body reads as one thing
/// instead of adding another 30 lines to the walk's own loop.
struct RowAccum<'a> {
    buckets: usize,
    from: i64,
    to: i64,
    is_default: bool,
    default_chain: &'a HashSet<Oid>,
    tag_targets: &'a [(Vec<u8>, Oid)],
    repo_facts: &'a RepoFacts<'a>,
    policy: &'a Policy,
    counts: Vec<u32>,
    radar_best: BTreeMap<usize, graft_core::mistake_radar::Severity>,
    marks: Vec<(usize, MapMark)>,
    visited: HashSet<Oid>,
    commits_seen: usize,
}

impl RowAccum<'_> {
    /// Fold one commit in. Returns `false` once [`MAX_COMMITS_PER_ROW`] is
    /// exceeded, telling the caller to stop the walk early.
    fn record(&mut self, commit: &graft_core::commit::Commit) -> bool {
        self.visited.insert(commit.id);
        let on_chain = self.default_chain.contains(&commit.id);
        if !self.is_default && on_chain {
            // A merged branch keeps the bars it had before its merge:
            // commits also reachable from the default branch's first-parent
            // chain are the default row's to count.
            return true;
        }
        self.commits_seen += 1;
        if self.commits_seen > MAX_COMMITS_PER_ROW {
            return false;
        }
        let bucket = bucket_index(
            commit.committer.when.seconds,
            self.from,
            self.to,
            self.buckets,
        );
        self.counts[bucket] += 1;
        for (name, oid) in self.tag_targets {
            if *oid == commit.id {
                self.marks.push((bucket, MapMark::Tag(name.clone())));
            }
        }
        let parents_on_default_chain: Vec<bool> = commit
            .parents
            .iter()
            .map(|parent| self.default_chain.contains(parent))
            .collect();
        let subject = String::from_utf8_lossy(commit.subject());
        let facts = CommitFacts {
            id: commit.id,
            parents: &commit.parents,
            subject: subject.as_ref(),
            on_default_chain: on_chain,
            parents_on_default_chain: &parents_on_default_chain,
        };
        if let Some(finding) = classify_commit(&facts, self.repo_facts, self.policy) {
            self.radar_best
                .entry(bucket)
                .and_modify(|severity| *severity = (*severity).max(finding.severity))
                .or_insert(finding.severity);
        }
        true
    }
}

/// Walk one branch's tip into buckets. `is_default` counts every commit in
/// the span; every other row skips commits on `default_chain` (a merged
/// branch keeps the bars it had before its merge).
#[allow(
    clippy::too_many_arguments,
    reason = "each argument is the caller's to say; a struct for one caller would only rename them"
)]
pub(crate) fn walk_branch(
    store_repo: &graft_git::repo::RepoHandle,
    table: &Arc<CommitTable>,
    tip: Oid,
    buckets: usize,
    from: i64,
    to: i64,
    is_default: bool,
    default_chain: &HashSet<Oid>,
    tag_targets: &[(Vec<u8>, Oid)],
    repo_facts: &RepoFacts<'_>,
    policy: &Policy,
    stop: &dyn Stop,
) -> Result<RowWalk, MapError> {
    let mut walk = Walk::from_tips(store_repo, Order::CommitDate, &[tip], Arc::clone(table))
        .map_err(MapError::Walk)?;
    let mut accum = RowAccum {
        buckets,
        from,
        to,
        is_default,
        default_chain,
        tag_targets,
        repo_facts,
        policy,
        counts: vec![0u32; buckets.max(1)],
        radar_best: BTreeMap::new(),
        marks: Vec::new(),
        visited: HashSet::new(),
        commits_seen: 0,
    };
    let mut complete = true;

    'walk: loop {
        let chunk = walk
            .next_chunk(CHUNK_LIMIT, &|| stop.should_stop())
            .map_err(MapError::Walk)?;
        if chunk.stopped {
            return Err(MapError::Stopped);
        }
        for commit in &chunk.commits {
            if !accum.record(commit) {
                complete = false;
                break 'walk;
            }
        }
        if chunk.done {
            break;
        }
        // Dated well before the span even under `CommitDate` order's own
        // skew (`Shape::SkewedDates`): the rest of this branch's history is
        // off the front of the window, so counting it further would only
        // ever land in bucket 0.
        if chunk
            .commits
            .last()
            .is_some_and(|oldest| oldest.committer.when.seconds < from - SKEW_TOLERANCE_SECONDS)
        {
            break;
        }
    }

    let mut marks = accum.marks;
    for (bucket, severity) in accum.radar_best {
        marks.push((bucket, MapMark::Radar(severity)));
    }

    Ok(RowWalk {
        counts: accum.counts,
        marks,
        complete,
        visited: accum.visited,
    })
}

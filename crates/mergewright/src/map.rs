//! The Map view's per-branch bucket counts (DESIGN.md §5.1): one bounded walk
//! per local branch into fixed time buckets, cheap enough to build fresh
//! every time the view opens and throw away when it closes. No drawing here —
//! that is `view::map` (`map-view-screen`, a later ticket); this module only
//! answers "how many commits, per branch, per bucket" and the marks that ride
//! along with them.
//!
//! M2 shows local branches only: a remote never gets its own row, and the
//! merge elbow (`MapBranch::merged_into`) only ever points at the default
//! branch's row — the design note's `map-view-data` deferrals (remote rows,
//! merges into a non-default branch) are M8 polish gaps, not bugs here.
//!
//! Split into files (m2 review M8, public items unchanged): [`walk`] is the
//! per-branch bounded walk (refs, span, default chain, `walk_branch`);
//! [`rows`] is the layer above it, one row per local branch; this file keeps
//! the public data shapes and [`build_map`], the entry point that wires the
//! two together.

mod rows;
mod walk;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::HeadState;
use mergewright_git::run::Stop;
use mergewright_ops::mistake_radar::{Policy, RepoFacts};

use crate::repos::{RepoStore, StoreError};
use crate::session::Clock;
use rows::RowContext;

/// Rows drawn before the rest collapse into the footer's `+N more`
/// (DESIGN.md §5.1: cap 200).
pub const MAX_ROWS: usize = 200;

/// A row's walk gives up counting past this many commits in the span and
/// marks itself, and the whole [`MapData`], incomplete — a branch with an
/// unbounded amount of history in it must not turn "open the Map view" into
/// an unbounded walk (DESIGN.md §5.1 "Cost").
pub const MAX_COMMITS_PER_ROW: usize = 50_000;

/// The widest a time span is allowed to stretch: a branch whose tip is older
/// than this is clamped, so one ancient branch does not flatten every other
/// row's bars into a single bucket.
const MAX_SPAN_DAYS: i64 = 366;
/// The narrowest a span is allowed to be: a repository committed to today
/// still gets a span wide enough to draw.
const MIN_SPAN_DAYS: i64 = 1;
const SECONDS_PER_DAY: i64 = 24 * 3_600;

/// How far past `from` a chunk's oldest commit may be dated before a row's
/// walk treats itself as having run off the front of the span and stops.
/// Wider than zero because `Shape::SkewedDates` has a commit dated a day
/// ahead of its children — an ordinary consequence of `CommitDate` order,
/// not a reason to cut a row's history short.
const SKEW_TOLERANCE_SECONDS: i64 = 36 * 3_600;

/// Commits asked for per [`mergewright_git::history::Walk::next_chunk`] call.
const CHUNK_LIMIT: usize = 500;

/// What the Map view job is asked to build.
///
/// `PartialEq`/`Eq` added beyond the ticket's own signature (a derive, not a
/// signature change, ticket 18's own allowance for `Policy: Clone`): every
/// field already has both, and [`crate::update::Command`] derives them over
/// its whole enum, so `Command::BuildMap` needs them here too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRequest {
    /// Columns in every row's sparkline: the width the view has to draw in.
    pub buckets: usize,
    /// The instant `to` is measured against (`update::Harness::clock`), read
    /// once by the job — never by `update` itself.
    pub clock: Clock,
    /// Mistake Radar policy: which kinds mark a bucket, and how loud.
    pub policy: Policy,
    /// The default branch's previous reflog tips, for [`RepoFacts`]. Empty
    /// until `state.radar.context` has landed — no backward-merge marks
    /// before then (`update::map::request` leaves it empty rather than
    /// waiting).
    pub previous_default_tips: Vec<Oid>,
}

/// A mark on one bucket of one row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapMark {
    /// A tag whose peeled commit landed in this bucket, by its short name
    /// (`v1.0`, not `refs/tags/v1.0`).
    Tag(Vec<u8>),
    /// The loudest Mistake Radar finding among this bucket's commits.
    Radar(mergewright_core::mistake_radar::Severity),
}

/// One row: a local branch's density sparkline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapBranch {
    /// The branch's full ref name (`refs/heads/main`).
    pub name: RefName,
    /// Lane colour, the same identity the main graph's lanes use.
    pub identity: mergewright_graph::identity::BranchIdentity,
    /// The branch's tip commit.
    pub tip: Oid,
    /// The tip commit's committer date.
    pub tip_date: i64,
    /// Commits per bucket; always [`MapData::buckets`] long.
    pub counts: Vec<u32>,
    /// `HEAD` resolves to this branch.
    pub is_current: bool,
    /// This is the repository's default branch (ADR-0001).
    pub is_default: bool,
    /// `Some((row, bucket))` when this branch's tip is reachable from the
    /// default branch's tip: the row index of the default branch, and the
    /// bucket of this row's own tip date (not the merge commit's — the elbow
    /// sits where the branch's line ends, per DESIGN.md §5.1). `None` for
    /// the default row itself, and for any branch not merged into it — M2
    /// tracks merges into the default branch only.
    pub merged_into: Option<(usize, usize)>,
    /// Tag and Mistake Radar marks, by bucket. At most one [`MapMark::Radar`]
    /// per bucket (the loudest finding wins); a bucket can also carry any
    /// number of [`MapMark::Tag`]s.
    pub marks: Vec<(usize, MapMark)>,
    /// `false` once this row's walk hit [`MAX_COMMITS_PER_ROW`] and gave up
    /// before the walk itself ended.
    pub complete: bool,
}

/// Every row the Map view draws, and the time span they share.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapData {
    /// Rows, current branch first, then the default, then the rest by tip
    /// date, newest first; at most [`MAX_ROWS`].
    pub branches: Vec<MapBranch>,
    /// Local branches beyond [`MAX_ROWS`], not drawn as rows at all.
    pub more: usize,
    /// The span's start, Unix seconds.
    pub from: i64,
    /// The span's end, Unix seconds (`MapRequest::clock` at build time).
    pub to: i64,
    /// Columns every row's `counts` has.
    pub buckets: usize,
    /// `false` if any row's walk hit [`MAX_COMMITS_PER_ROW`].
    pub complete: bool,
}

impl MapData {
    /// The bucket `date` falls into, clamped to `0..buckets`.
    #[must_use]
    pub fn bucket_of(&self, date: i64) -> usize {
        bucket_index(date, self.from, self.to, self.buckets)
    }

    /// The largest count in any bucket of any row: what a sparkline's glyphs
    /// scale against.
    #[must_use]
    pub fn max_count(&self) -> u32 {
        self.branches
            .iter()
            .flat_map(|branch| branch.counts.iter().copied())
            .max()
            .unwrap_or(0)
    }
}

/// Why [`build_map`] could not finish.
#[derive(Debug)]
pub enum MapError {
    /// The repository's refs, branches or commit-graph table could not be
    /// read.
    Store(StoreError),
    /// A branch's walk failed.
    Walk(mergewright_git::history::WalkError),
    /// A tip commit could not be read for its date.
    Read(String),
    /// `stop` fired before the walk finished.
    Stopped,
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapError::Store(error) => write!(f, "{error}"),
            MapError::Walk(error) => write!(f, "{error}"),
            MapError::Read(line) => write!(f, "{line}"),
            MapError::Stopped => write!(f, "cancelled"),
        }
    }
}

impl std::error::Error for MapError {}

/// The bucket `date` falls into over `[from, to]`, split into `buckets`
/// equal-width columns and clamped to `0..buckets` either side of the span
/// (a commit dated before `from`, or after `to`, still lands in a real
/// bucket rather than being dropped).
pub(crate) fn bucket_index(date: i64, from: i64, to: i64, buckets: usize) -> usize {
    if buckets == 0 {
        return 0;
    }
    let span = (to - from).max(1);
    let offset = (date - from).clamp(0, span);
    let bucket = (offset * i64::try_from(buckets).unwrap_or(i64::MAX)) / span;
    usize::try_from(bucket).unwrap_or(0).min(buckets - 1)
}

/// Build the Map view's data: one bounded walk per local branch
/// (DESIGN.md §5.1 "Cost" — no lanes, no crossing resolution).
///
/// # Errors
/// The repository's refs, branches or commit-graph table could not be read,
/// a branch's walk failed, or `stop` fired before every row finished.
pub fn build_map(
    store: &RepoStore,
    req: &MapRequest,
    progress: &mut dyn FnMut(u32, u32),
    stop: &dyn Stop,
) -> Result<MapData, MapError> {
    let refs = store.refs().map_err(MapError::Store)?;
    let branches = store.branches().map_err(MapError::Store)?;
    let repo = store
        .shared()
        .map_err(StoreError::from)
        .map_err(MapError::Store)?;
    let handle = repo.handle();
    let table = store.table().map_err(MapError::Store)?;

    let default_ref = branches.default_branch().cloned();
    let current_ref = match handle.head() {
        Ok(HeadState::Branch(name) | HeadState::Unborn(name)) => Some(name),
        Ok(HeadState::Detached(_)) | Err(_) => None,
    };

    let (locals, default_tip) = walk::local_branches(&refs, default_ref.as_ref());

    let tip_dates = walk::local_tip_dates(&handle, &locals, stop)?;
    let (locals, more) = walk::sort_and_cap_locals(
        locals,
        current_ref.as_ref(),
        default_ref.as_ref(),
        &tip_dates,
    );

    let (from, to) = walk::time_span(&tip_dates, req.clock.now_seconds());
    let default_chain = walk::build_default_chain(&handle, &table, default_tip, stop)?;

    let tag_targets = walk::tag_targets_of(&refs);

    let default_branch_str = default_ref
        .as_ref()
        .and_then(|name| std::str::from_utf8(name.shorthand()).ok());
    let repo_facts = RepoFacts {
        default_branch: default_branch_str,
        remotes: &[],
        previous_default_tips: &req.previous_default_tips,
    };

    let rows = RowContext {
        handle: &handle,
        table: &table,
        current_ref: current_ref.as_ref(),
        default_chain: &default_chain,
        tag_targets: &tag_targets,
        repo_facts: &repo_facts,
        policy: &req.policy,
        from,
        to,
        buckets: req.buckets,
    };

    let (result_branches, overall_complete) = rows::build_rows(
        &rows,
        &locals,
        default_ref.as_ref(),
        &tip_dates,
        progress,
        stop,
    )?;

    Ok(MapData {
        branches: result_branches,
        more,
        from,
        to,
        buckets: req.buckets,
        complete: overall_complete,
    })
}

#[cfg(test)]
mod tests {
    use mergewright_core::mistake_radar::Severity;
    use mergewright_fixtures::{CORPUS_NOW, Shape};
    use mergewright_ops::mistake_radar::Policy;

    use super::*;

    struct NeverStop;
    impl Stop for NeverStop {
        fn should_stop(&self) -> bool {
            false
        }
    }

    struct StopAfter(std::sync::atomic::AtomicU32);
    impl Stop for StopAfter {
        fn should_stop(&self) -> bool {
            let n = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            n >= 1
        }
    }

    fn store(shape: Shape) -> RepoStore {
        RepoStore::new(mergewright_fixtures::borrow(shape), None)
    }

    fn request(buckets: usize) -> MapRequest {
        MapRequest {
            buckets,
            clock: Clock::Fixed(CORPUS_NOW),
            policy: Policy::default(),
            previous_default_tips: Vec::new(),
        }
    }

    #[test]
    fn feature_branches_rows_order_and_merged_into() {
        let store = store(Shape::FeatureBranches);
        let mut done = 0;
        let mut report = |d, _t| done += d;
        let data = build_map(&store, &request(40), &mut report, &NeverStop).expect("builds");

        assert!(done > 0);
        let names: Vec<String> = data
            .branches
            .iter()
            .map(|b| String::from_utf8_lossy(b.name.shorthand()).into_owned())
            .collect();
        assert_eq!(names[0], "main");
        assert!(data.branches[0].is_current);
        assert!(data.branches[0].is_default);
        for branch in &data.branches {
            assert_eq!(branch.counts.len(), 40);
        }

        let one = data
            .branches
            .iter()
            .find(|b| String::from_utf8_lossy(b.name.shorthand()) == "feature/one")
            .expect("feature/one is a row");
        let (row, bucket) = one.merged_into.expect("feature/one merged into main");
        assert_eq!(row, 0);
        assert_eq!(bucket, data.bucket_of(one.tip_date));

        let two = data
            .branches
            .iter()
            .find(|b| String::from_utf8_lossy(b.name.shorthand()) == "feature/two")
            .expect("feature/two is a row");
        assert!(two.merged_into.is_some());

        let three = data
            .branches
            .iter()
            .find(|b| String::from_utf8_lossy(b.name.shorthand()) == "feature/three")
            .expect("feature/three is a row");
        assert!(three.merged_into.is_none());

        let main = &data.branches[0];
        let main_sum: u32 = main.counts.iter().sum();
        assert!(main_sum > 0);
        let one_sum: u32 = one.counts.iter().sum();
        assert_eq!(one_sum, 2);
    }

    #[test]
    fn tags_mark_the_bucket_of_their_peeled_commit() {
        let store = store(Shape::Tags);
        let mut report = |_, _| {};
        let data = build_map(&store, &request(40), &mut report, &NeverStop).expect("builds");
        let main = data
            .branches
            .iter()
            .find(|b| b.is_default)
            .expect("a default row");
        let has_v1 = main
            .marks
            .iter()
            .any(|(_, mark)| matches!(mark, MapMark::Tag(name) if name == b"v1.0"));
        assert!(has_v1, "expected a v1.0 tag mark, got {:?}", main.marks);
    }

    #[test]
    fn foxtrot_merge_marks_a_warning_at_the_merge() {
        let store = store(Shape::FoxtrotMerge);
        let mut report = |_, _| {};
        let data = build_map(&store, &request(40), &mut report, &NeverStop).expect("builds");
        let warnings: usize = data
            .branches
            .iter()
            .flat_map(|b| b.marks.iter())
            .filter(|(_, mark)| matches!(mark, MapMark::Radar(Severity::Warning)))
            .count();
        assert_eq!(warnings, 1);
    }

    #[test]
    fn unborn_branch_has_no_current_row_and_no_panic() {
        let store = store(Shape::UnbornBranch);
        let mut report = |_, _| {};
        let data = build_map(&store, &request(40), &mut report, &NeverStop).expect("builds");
        assert!(data.branches.iter().all(|b| !b.is_current));
    }

    #[test]
    fn stop_after_the_first_chunk_is_an_error() {
        let store = store(Shape::FeatureBranches);
        let mut report = |_, _| {};
        let stop = StopAfter(std::sync::atomic::AtomicU32::new(0));
        let result = build_map(&store, &request(40), &mut report, &stop);
        assert!(matches!(result, Err(MapError::Stopped)));
    }
}

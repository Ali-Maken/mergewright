//! Property tests for the PROMPT.md §1.2 invariants, each asserted as the
//! concrete property DESIGN.md §3.7 maps it to.
//!
//! The checker is an outside observer. It replays the rows the layout
//! produced — knowing only what a renderer would know: the seeds and the
//! rows — and tracks which columns carry a live edge from one row to the
//! next. Nothing here reads the cursor's private state; the one test that
//! looks at `active_lanes()` does so through the public method.
//!
//! Histories are generated in walk order: a commit's parents always lie
//! later in the sequence, which is the precondition `extend` documents.

use std::collections::{BTreeSet, HashMap};

use proptest::prelude::*;

use super::*;

/// The most commits a generated history holds. The invariants are about
/// shape, not scale: 32 commits with up to three parents each produce every
/// lane event — open, continue, converge, close, reuse — many times over.
const MAX_COMMITS: usize = 32;

/// The tier's case count. `None` means `PROPTEST_CASES` is set in the
/// environment, and proptest's own default reads it.
fn tier() -> ProptestConfig {
    match graft_fixtures::proptest_cases() {
        Some(cases) => ProptestConfig::with_cases(cases),
        None => ProptestConfig::default(),
    }
}

/// Which kind of subject a generated commit carries. The two merge shapes
/// exercise branch-name recovery; `Plain` exercises the short-SHA fallback.
#[derive(Clone, Copy, Debug)]
enum Subject {
    Plain,
    MergeBranch,
    PullRequest,
}

/// One generated commit before it becomes a [`Commit`]: the positions of
/// its parents (all later than its own) and its subject kind.
#[derive(Clone, Debug)]
struct Draft {
    parents: Vec<usize>,
    subject: Subject,
}

/// Which positions the seeds point at.
#[derive(Clone, Debug)]
struct SeedPlan {
    default: Option<usize>,
    integration: Vec<usize>,
    known: Vec<usize>,
}

/// A generated history: commits in walk order, and where the seeds point.
#[derive(Clone, Debug)]
struct History {
    commits: Vec<Commit>,
    plan: SeedPlan,
}

impl History {
    fn seeds(&self) -> Seeds {
        let at = |position: usize| BranchSeed {
            tip: self.commits[position].id,
            identity: BranchIdentity::of_name(format!("branch-{position}").as_bytes()),
        };
        Seeds {
            default_branch: self.plan.default.map(at),
            integration_branches: self.plan.integration.iter().copied().map(at).collect(),
            known_branches: self.plan.known.iter().copied().map(at).collect(),
        }
    }

    /// The columns the seeds reserve: 0 for the default branch, then one
    /// per integration branch.
    fn reserved_columns(&self) -> BTreeSet<usize> {
        let mut columns = BTreeSet::new();
        if self.plan.default.is_some() {
            columns.insert(0);
        }
        columns.extend(1..=self.plan.integration.len());
        columns
    }
}

fn position_oid(position: usize) -> Oid {
    oid(u8::try_from(position + 1).expect("MAX_COMMITS fits in a byte"))
}

/// Up to three distinct parents, all later in the walk than `position`, in
/// a random order so the first parent is not always the nearest one.
fn parents_after(position: usize, len: usize) -> BoxedStrategy<Vec<usize>> {
    let later: Vec<usize> = (position + 1..len).collect();
    if later.is_empty() {
        return Just(Vec::new()).boxed();
    }
    let most = later.len().min(3);
    proptest::sample::subsequence(later, 0..=most)
        .prop_shuffle()
        .boxed()
}

fn subject() -> impl Strategy<Value = Subject> {
    prop_oneof![
        4 => Just(Subject::Plain),
        1 => Just(Subject::MergeBranch),
        1 => Just(Subject::PullRequest),
    ]
}

fn drafts() -> impl Strategy<Value = Vec<Draft>> {
    (1..=MAX_COMMITS).prop_flat_map(|len| {
        (0..len)
            .map(|position| {
                (parents_after(position, len), subject())
                    .prop_map(|(parents, subject)| Draft { parents, subject })
            })
            .collect::<Vec<_>>()
    })
}

fn build(drafts: &[Draft]) -> Vec<Commit> {
    drafts
        .iter()
        .enumerate()
        .map(|(position, draft)| {
            let subject = match draft.subject {
                Subject::Plain => format!("commit {position}"),
                Subject::MergeBranch => format!("Merge branch 'topic-{position}'"),
                Subject::PullRequest => {
                    format!("Merge pull request #{position} from fork/topic-{position}")
                }
            };
            // Two commits per second: equal dates are routine in scripted
            // history, and the layout must not care.
            let seconds = 1_000 - i64::try_from(position / 2).expect("small");
            Commit {
                id: position_oid(position),
                parents: draft.parents.iter().map(|&p| position_oid(p)).collect(),
                author: identity_at(seconds),
                committer: identity_at(seconds),
                message: subject.into_bytes(),
            }
        })
        .collect()
}

fn history() -> impl Strategy<Value = History> {
    drafts().prop_flat_map(|drafts| {
        let len = drafts.len();
        let all: Vec<usize> = (0..len).collect();
        (
            Just(drafts),
            proptest::option::of(0..len),
            proptest::sample::subsequence(all.clone(), 0..=len.min(2)),
            proptest::sample::subsequence(all, 0..=len.min(3)),
        )
            .prop_map(|(drafts, default, integration, known)| History {
                commits: build(&drafts),
                plan: SeedPlan {
                    default,
                    integration,
                    known,
                },
            })
    })
}

/// Replays rows the way a renderer consumes them, one at a time, tracking
/// which column carries an edge to which commit.
struct Replay {
    /// Column → the commit the edge travelling down that column is going to.
    live: HashMap<usize, Oid>,
    /// Reserved columns that have closed. A reserved column is never handed
    /// back, so nothing may ever use one of these again.
    retired: BTreeSet<usize>,
    reserved: BTreeSet<usize>,
}

impl Replay {
    fn new(history: &History) -> Self {
        let seeds = history.seeds();
        let mut live = HashMap::new();
        if let Some(seed) = seeds.default_branch {
            live.insert(0, seed.tip);
        }
        for (offset, seed) in seeds.integration_branches.iter().enumerate() {
            live.insert(1 + offset, seed.tip);
        }
        Replay {
            live,
            retired: BTreeSet::new(),
            reserved: history.reserved_columns(),
        }
    }

    /// Checks one row against the edges in flight, then advances them.
    fn step(&mut self, row: &Row) -> Result<(), TestCaseError> {
        self.arrive(row)?;
        self.depart(row)?;
        self.retire();
        Ok(())
    }

    /// The node itself and the edges that end here.
    fn arrive(&mut self, row: &Row) -> Result<(), TestCaseError> {
        let connected: BTreeSet<usize> = self
            .live
            .iter()
            .filter(|&(_, &target)| target == row.commit.id)
            .map(|(&column, _)| column)
            .collect();

        // §1.2: no edge passes through a node it isn't connected to. The
        // node sits either on a lane that was waiting for it or on a column
        // nothing is travelling down.
        prop_assert!(
            !self.live.contains_key(&row.lane) || connected.contains(&row.lane),
            "row {} sits on column {}, which carries an edge to {}",
            row.commit.id,
            row.lane,
            self.live[&row.lane]
        );
        prop_assert!(
            !self.retired.contains(&row.lane),
            "row {} reuses retired reserved column {}",
            row.commit.id,
            row.lane
        );
        // DESIGN.md §3.2 step 3: among the lanes waiting for it, the node
        // takes the lowest-indexed one — which is lane 0 whenever lane 0 is
        // among them, so a single rule covers both halves of the sentence.
        if let Some(&lowest) = connected.first() {
            prop_assert_eq!(
                row.lane,
                lowest,
                "row {} should sit on the lowest lane waiting for it",
                row.commit.id
            );
        }

        // Everything waiting for this commit bends in here — and nothing
        // else does. This is the same invariant seen from the closing side:
        // no live edge is drawn past its own target, and no lane that is
        // not connected to this row bends toward it.
        let expected_incoming: Vec<usize> = connected
            .iter()
            .copied()
            .filter(|&column| column != row.lane)
            .collect();
        prop_assert_eq!(
            &row.incoming,
            &expected_incoming,
            "incoming lanes of row {}",
            row.commit.id
        );
        for column in &connected {
            self.live.remove(column);
        }
        Ok(())
    }

    /// The edges that start here.
    fn depart(&mut self, row: &Row) -> Result<(), TestCaseError> {
        // §1.2: every parent edge is drawn — one outgoing entry per parent,
        // in parent order, the first one continuing the row's own lane.
        let targets: Vec<Oid> = row.outgoing.iter().map(|&(_, target)| target).collect();
        prop_assert_eq!(
            &targets,
            &row.commit.parents,
            "outgoing edges of row {}",
            row.commit.id
        );
        if let Some(&(column, _)) = row.outgoing.first() {
            prop_assert_eq!(
                column,
                row.lane,
                "the first parent continues the row's own lane"
            );
        }

        for &(column, target) in &row.outgoing {
            // A column freed on this row is not reusable until the next one.
            prop_assert!(
                !row.incoming.contains(&column),
                "row {} reuses column {} on the row that freed it",
                row.commit.id,
                column
            );
            prop_assert!(
                !self.retired.contains(&column),
                "row {} opens an edge on retired reserved column {}",
                row.commit.id,
                column
            );
            // Two edges never share a column: a new edge only ever opens on
            // a column nothing is travelling down.
            prop_assert!(
                self.live.insert(column, target).is_none(),
                "row {} puts two edges in column {}",
                row.commit.id,
                column
            );
        }
        Ok(())
    }

    /// A reserved column with nothing travelling down it has closed for good.
    fn retire(&mut self) {
        for &column in &self.reserved {
            if !self.live.contains_key(&column) {
                self.retired.insert(column);
            }
        }
    }
}

/// What one row may do to one column of open lanes: nothing, unless the lane
/// there was waiting for this very commit — in which case it closes, or (on
/// the row's own lane only) continues in place with its identity intact.
fn check_column(
    column: usize,
    before: Option<&Lane>,
    after: Option<&Lane>,
    row: &Row,
) -> Result<(), TestCaseError> {
    let Some(previous) = before else {
        return Ok(());
    };
    if previous.waiting_for != row.commit.id {
        prop_assert_eq!(
            after,
            Some(previous),
            "row {} touched column {}, which was waiting for {}",
            row.commit.id,
            column,
            previous.waiting_for
        );
        return Ok(());
    }
    let Some(now) = after else {
        return Ok(());
    };
    prop_assert_eq!(column, row.lane, "only the row's own lane continues");
    prop_assert_eq!(
        now.identity,
        previous.identity,
        "a continuing lane keeps its identity"
    );
    prop_assert_eq!(now.reserved, previous.reserved);
    prop_assert_eq!(Some(now.waiting_for), row.commit.parents.first().copied());
    Ok(())
}

/// The first-parent chain from `tip`, by position.
fn first_parent_chain(commits: &[Commit], tip: usize) -> Vec<usize> {
    let position: HashMap<Oid, usize> = commits
        .iter()
        .enumerate()
        .map(|(position, commit)| (commit.id, position))
        .collect();
    let mut chain = vec![tip];
    let mut at = tip;
    while let Some(parent) = commits[at].parents.first() {
        at = position[parent];
        chain.push(at);
    }
    chain
}

proptest! {
    #![proptest_config(tier())]

    /// Every parent edge is drawn, no edge passes through an unrelated node,
    /// a freed column waits a row before reuse, and a reserved column is
    /// never handed back — all checked from the rows alone.
    #[test]
    fn rows_replay_cleanly_from_the_outside(history in history()) {
        let rows = layout(&history.commits, history.seeds());
        prop_assert_eq!(rows.len(), history.commits.len());

        let mut replay = Replay::new(&history);
        for row in &rows {
            replay.step(row)?;
        }
        // Every seed and every parent is in the sequence, so every edge
        // must have reached its target and closed.
        prop_assert!(
            replay.live.is_empty(),
            "edges still in flight after the last row: {:?}",
            replay.live
        );
    }

    /// The default branch's first-parent chain is lane 0 for its whole life,
    /// whatever else converges on it along the way.
    #[test]
    fn the_default_branch_is_lane_zero_and_straight(history in history()) {
        let rows = layout(&history.commits, history.seeds());
        if let Some(tip) = history.plan.default {
            for position in first_parent_chain(&history.commits, tip) {
                prop_assert_eq!(
                    rows[position].lane,
                    0,
                    "commit {} is on the default branch's first-parent chain",
                    rows[position].commit.id
                );
            }
        }
    }

    /// A lane never changes column mid-life: a lane not connected to the row
    /// being laid out is untouched by it, and a lane that is connected either
    /// closes or continues in place with its identity intact.
    #[test]
    fn open_lanes_are_only_ever_touched_by_the_row_they_are_waiting_for(history in history()) {
        let mut cursor = LayoutCursor::new(history.seeds());
        for commit in &history.commits {
            let before = cursor.active_lanes().to_vec();
            let row = cursor.extend(std::slice::from_ref(commit)).remove(0);
            let after = cursor.active_lanes();

            for (column, lane) in after.iter().enumerate() {
                if let Some(lane) = lane {
                    prop_assert_eq!(lane.index, column, "a lane's index is its column");
                }
            }
            for (column, previous) in before.iter().enumerate() {
                check_column(column, previous.as_ref(), after[column].as_ref(), &row)?;
            }
        }
    }

    /// The same commits and seeds produce byte-identical rows, however the
    /// commits are chunked across `extend` calls.
    #[test]
    fn output_is_deterministic_and_independent_of_chunking(
        history in history(),
        chunk_sizes in proptest::collection::vec(1usize..=6, 0..=8),
    ) {
        let whole = layout(&history.commits, history.seeds());
        prop_assert_eq!(&whole, &layout(&history.commits, history.seeds()));

        let mut cursor = LayoutCursor::new(history.seeds());
        let mut chunked = Vec::new();
        let mut rest = history.commits.as_slice();
        for size in chunk_sizes {
            let take = size.min(rest.len());
            let (chunk, tail) = rest.split_at(take);
            chunked.extend(cursor.extend(chunk));
            rest = tail;
        }
        chunked.extend(cursor.extend(rest));
        prop_assert_eq!(&whole, &chunked);
    }
}

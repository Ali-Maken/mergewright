//! The layout engine's and the pixel tier's hot paths, under `criterion`
//! (PROMPT.md §6.3, ticket m1-the-graph/30): one `StreamingLayout::advance`
//! chunk, one strip drawn as its layers (backdrop + emphasis + composite,
//! ADR-0053), and a strip-cache hit and miss for the composited strip.
//!
//! Review-only (PROMPT.md §7.1), with short runs:
//! `cargo bench -p mergewright-graph -- --warm-up-time 1 --measurement-time 3`.
//! The input is generated in memory, so no fixture is built.

#![allow(
    clippy::expect_used,
    reason = "a benchmark whose input cannot be built has nothing to measure"
)]

use std::collections::{BTreeSet, HashMap};
use std::convert::Infallible;
use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use mergewright_core::capabilities::Rgb;
use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::commit_handle::{CommitHandle, Link};
use mergewright_core::oid::Oid;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::layout::layout;
use mergewright_graph::layout::source::{CommitSource, Pull};
use mergewright_graph::layout::stream::{StreamConfig, StreamingLayout};
use mergewright_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache};
use mergewright_graph::raster::{
    Backdrop, Emphasis, Lit, PlainRow, PlainStrip, Selection, composite, rasterise_backdrop,
    rasterise_emphasis,
};
use mergewright_graph::row::Row;
use mergewright_graph::seeds::{BranchSeed, Seeds};

/// Commits in the generated history: several chunks past the first.
const COMMITS: usize = 20_000;
/// The user's measured cell (ADR-0012's Ghostty bench).
const CELL: (u32, u32) = (17, 37);
const PALETTE: [Rgb; 3] = [
    Rgb {
        red: 0x00,
        green: 0x77,
        blue: 0xBB,
    },
    Rgb {
        red: 0xEE,
        green: 0x88,
        blue: 0x66,
    },
    Rgb {
        red: 0xAA,
        green: 0x33,
        blue: 0x77,
    },
];
const MUTED: Rgb = Rgb {
    red: 0x88,
    green: 0x88,
    blue: 0x88,
};

fn oid(index: usize) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    bytes[..8].copy_from_slice(&(index as u64 + 1).to_be_bytes());
    Oid::from_bytes(&bytes).expect("20 bytes is a valid oid length")
}

fn at(seconds: i64) -> Identity {
    Identity {
        name: b"Gen".to_vec(),
        email: b"gen@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    }
}

/// `count` commits in walk order: a first-parent chain with a side commit
/// every fifth row merged back three rows up, so rows carry several lanes.
/// The shape `strip_cache.rs`'s tests use.
fn commits(count: usize) -> Vec<Commit> {
    (0..count)
        .map(|index| {
            let mut parents = Vec::new();
            if index + 1 < count {
                parents.push(oid(index + 1));
            }
            if index % 5 == 0 && index + 3 < count {
                parents.push(oid(index + 3));
            }
            let seconds = 2_000_000_000 - i64::try_from(index).expect("small index") * 60;
            Commit {
                id: oid(index),
                parents,
                author: at(seconds),
                committer: at(seconds),
                message: format!("commit {index}").into_bytes(),
            }
        })
        .collect()
}

fn seeds() -> Seeds {
    Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(0),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    }
}

/// A walk already in memory: what a history job's walk hands the layout,
/// without git.
struct InMemory {
    walk: Vec<Commit>,
    by_id: HashMap<Oid, usize>,
    next: usize,
}

impl InMemory {
    fn new(walk: Vec<Commit>) -> Self {
        let by_id = walk
            .iter()
            .enumerate()
            .map(|(index, commit)| (commit.id, index))
            .collect();
        Self {
            walk,
            by_id,
            next: 0,
        }
    }
}

impl CommitSource for InMemory {
    type Error = Infallible;

    fn pull(&mut self, limit: usize, _stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        let end = (self.next + limit).min(self.walk.len());
        let handles: Vec<CommitHandle> = (self.next..end)
            .map(|index| CommitHandle(u32::try_from(index).expect("a bench history fits in u32")))
            .collect();
        self.next = end;
        Ok(Pull {
            links: self.links(&handles)?,
            done: end == self.walk.len(),
            stopped: false,
        })
    }

    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Self::Error> {
        Ok(commits
            .iter()
            .map(|handle| self.walk[handle.index()].clone())
            .collect())
    }

    // Handles are walk positions: the benches never ask for a parent's.
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Self::Error> {
        Ok(commits
            .iter()
            .map(|&handle| Link {
                commit: handle,
                parents: self.walk[handle.index()]
                    .parents
                    .iter()
                    .filter_map(|parent| self.handle_of(parent))
                    .collect(),
            })
            .collect())
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.by_id
            .get(id)
            .map(|&index| CommitHandle(u32::try_from(index).expect("a bench history fits in u32")))
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.walk[handle.index()].id
    }
}

fn width_of(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0)
}

fn strip<'a>(rows: &'a [PlainRow<'a>], above: Option<&'a Row>, lanes: usize) -> PlainStrip<'a> {
    PlainStrip {
        rows,
        above,
        lane_columns: lanes,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
        roles: None,
    }
}

fn advance_per_chunk(c: &mut Criterion) {
    let walk = commits(COMMITS);
    let config = StreamConfig::default();
    let chunk = config.chunk_rows;
    c.bench_function("StreamingLayout::advance/one chunk", |b| {
        b.iter_batched(
            || StreamingLayout::new(InMemory::new(walk.clone()), seeds(), config),
            |mut layout| {
                let advanced = layout.advance(chunk, &|| false).expect("infallible source");
                black_box(advanced)
            },
            BatchSize::LargeInput,
        );
    });
}

fn strips(c: &mut Criterion) {
    let rows = layout(&commits(4_000), seeds());
    let lanes = width_of(&rows);
    let first = 32 * STRIP_ROWS;
    let raster_rows: Vec<PlainRow<'_>> = rows[first..first + STRIP_ROWS]
        .iter()
        .map(PlainRow::of)
        .collect();
    let above = Some(&rows[first - 1]);
    // The cursor's thread on lane 0, as the app paints it: every row lit
    // there, so the strip has an emphasis layer to draw and composite.
    let lit: BTreeSet<usize> = [0].into();
    let lits = vec![
        Lit {
            emphasis: Emphasis::Normal,
            thread: Some(&lit),
        };
        STRIP_ROWS
    ];
    let sel = Selection {
        rows: &lits,
        backdrop: Backdrop::lowest(lits.iter().copied()),
    };

    c.bench_function("layers/backdrop+emphasis+composite one strip", |b| {
        b.iter(|| {
            let plain = strip(&raster_rows, above, lanes);
            let base = rasterise_backdrop(&plain, sel.backdrop).expect("backdrop rasterises");
            let top = rasterise_emphasis(&plain, &sel).expect("emphasis rasterises");
            black_box(top.map(|top| composite(&base, &top)))
        });
    });

    let mut warm = StripCache::with_budget(ByteBudget(usize::MAX), 16);
    warm.composited(first, &strip(&raster_rows, above, lanes), &sel)
        .expect("strip rasterises");
    c.bench_function("StripCache::composited/hit", |b| {
        b.iter(|| black_box(warm.composited(first, &strip(&raster_rows, above, lanes), &sel)));
    });

    c.bench_function("StripCache::composited/miss", |b| {
        b.iter_batched(
            || StripCache::with_budget(ByteBudget(usize::MAX), 16),
            |mut cold| {
                black_box(
                    cold.composited(first, &strip(&raster_rows, above, lanes), &sel)
                        .map(|_| ()),
                )
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, advance_per_chunk, strips);
criterion_main!(benches);

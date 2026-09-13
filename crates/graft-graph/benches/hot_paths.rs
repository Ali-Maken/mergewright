//! The layout engine's and the pixel tier's hot paths, under `criterion`
//! (PROMPT.md §6.3, ticket m1-the-graph/30): one `StreamingLayout::advance`
//! chunk, one strip rasterised, and a strip-cache hit and miss.
//!
//! Review-only (PROMPT.md §7.1), with short runs:
//! `cargo bench -p graft-graph -- --warm-up-time 1 --measurement-time 3`.
//! The input is generated in memory, so no fixture is built.

#![allow(
    clippy::expect_used,
    reason = "a benchmark whose input cannot be built has nothing to measure"
)]

use std::collections::HashMap;
use std::convert::Infallible;
use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use graft_core::capabilities::Rgb;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::layout::source::{CommitSource, Pull};
use graft_graph::layout::stream::{StreamConfig, StreamingLayout};
use graft_graph::raster::cache::{STRIP_ROWS, StripCache};
use graft_graph::raster::{RasterRow, Strip};
use graft_graph::row::Row;
use graft_graph::seeds::{BranchSeed, Seeds};

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

    fn next(&mut self, limit: usize, _stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        let end = (self.next + limit).min(self.walk.len());
        let commits = self.walk[self.next..end].to_vec();
        self.next = end;
        Ok(Pull {
            commits,
            done: end == self.walk.len(),
            stopped: false,
        })
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Self::Error> {
        Ok(ids
            .iter()
            .filter_map(|id| self.by_id.get(id).map(|&index| self.walk[index].clone()))
            .collect())
    }
}

fn width_of(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0)
}

fn strip<'a>(rows: &'a [RasterRow<'a>], above: Option<&'a Row>, lanes: usize) -> Strip<'a> {
    Strip {
        rows,
        above,
        lane_columns: lanes,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
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
    let raster_rows: Vec<RasterRow<'_>> = rows[first..first + STRIP_ROWS]
        .iter()
        .map(RasterRow::plain)
        .collect();
    let above = Some(&rows[first - 1]);

    c.bench_function("rasterise/one strip", |b| {
        b.iter(|| {
            black_box(graft_graph::raster::rasterise(&strip(
                &raster_rows,
                above,
                lanes,
            )))
        });
    });

    let mut warm = StripCache::new(STRIP_ROWS, 16);
    warm.get(first, &strip(&raster_rows, above, lanes))
        .expect("strip rasterises");
    c.bench_function("StripCache::get/hit", |b| {
        b.iter(|| black_box(warm.get(first, &strip(&raster_rows, above, lanes))));
    });

    c.bench_function("StripCache::get/miss", |b| {
        b.iter_batched(
            || StripCache::new(STRIP_ROWS, 16),
            |mut cold| {
                black_box(
                    cold.get(first, &strip(&raster_rows, above, lanes))
                        .map(|_| ()),
                )
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, advance_per_chunk, strips);
criterion_main!(benches);

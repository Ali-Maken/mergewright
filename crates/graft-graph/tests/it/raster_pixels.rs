//! Byte-for-byte guards on the rasteriser's output (m1-the-graph/30g).
//!
//! `raster.rs` snapshots strips as text, which survives a sub-unit rounding
//! change on purpose. These guards do not: each strip's premultiplied RGBA
//! buffer is hashed, so a change to *how* strokes are laid down (one stroke
//! per lane run instead of per row, reused path builders) must leave every
//! byte where it was. A legitimate visual change fails here first and is
//! shown to the user as before/after captures; the hashes are never
//! re-blessed silently.
//!
//! The strips cover what the drawing order could disturb: straight lanes
//! over many rows, merges and forks crossing other lanes, an octopus, a
//! wide (~28-lane) strip mid-history (with the row above it), a thread's
//! per-column emphasis, whole-row dimming and highlighting, and the `+N`
//! marker punching its label through the lines.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use graft_core::capabilities::Rgb;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::raster::{Emphasis, Marker, RasterRow, Strip, rasterise};
use graft_graph::row::Row;
use graft_graph::seeds::{BranchSeed, Seeds};

use super::corpus::{open_with_seeds, walk_everything};

/// The user's measured cell (ADR-0012's Ghostty bench) and the snapshot cell.
const CELLS: [(u32, u32); 2] = [(17, 37), (8, 16)];
const PALETTE: [Rgb; 4] = [
    rgb(0x00, 0x77, 0xBB),
    rgb(0xEE, 0x88, 0x66),
    rgb(0xAA, 0x33, 0x77),
    rgb(0x00, 0x99, 0x88),
];
const MUTED: Rgb = rgb(0x88, 0x88, 0x88);

const fn rgb(red: u8, green: u8, blue: u8) -> Rgb {
    Rgb { red, green, blue }
}

fn oid(index: usize) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    bytes[..8].copy_from_slice(&(index as u64 + 1).to_be_bytes());
    Oid::from_bytes(&bytes).unwrap_or_else(|why| panic!("20 bytes is a valid oid length: {why}"))
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

/// `count` commits in `lanes` interleaved chains (commit `i`'s first parent
/// is `i + lanes`), with every seventh commit also merging the commit three
/// rows down, so lanes pass straight through for many rows while arcs cross
/// them.
pub(crate) fn synthetic(count: usize, lanes: usize) -> Vec<Row> {
    let commits: Vec<Commit> = (0..count)
        .map(|index| {
            let mut parents = Vec::new();
            if index + lanes < count {
                parents.push(oid(index + lanes));
            }
            if index % 7 == 0 && index + 3 < count {
                parents.push(oid(index + 3));
            }
            let seconds = 2_000_000_000
                - i64::try_from(index).unwrap_or_else(|why| panic!("row {index}: {why}")) * 60;
            Commit {
                id: oid(index),
                parents,
                author: at(seconds),
                committer: at(seconds),
                message: format!("commit {index}").into_bytes(),
            }
        })
        .collect();
    let seeds = Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(0),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    };
    layout(&commits, seeds)
}

fn corpus(name: &str) -> Vec<Row> {
    let shape = *graft_fixtures::Shape::ALL
        .iter()
        .find(|shape| shape.name() == name)
        .unwrap_or_else(|| panic!("no corpus shape named {name}"));
    let dir = graft_fixtures::try_borrow(shape)
        .unwrap_or_else(|why| panic!("borrowing the {shape} fixture: {why}"));
    let (handle, seeds) = open_with_seeds(shape, &dir);
    let (commits, _) = walk_everything(&handle, shape);
    layout(&commits, seeds)
}

fn width_of(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0)
}

/// FNV-1a over the buffer: a change to any byte changes the digest.
fn digest(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

struct Case<'a> {
    name: &'static str,
    rows: Vec<RasterRow<'a>>,
    above: Option<&'a Row>,
    lane_columns: usize,
    marker: Option<Marker>,
}

fn line(out: &mut String, case: &Case<'_>, cell: (u32, u32)) {
    let frame = rasterise(&Strip {
        rows: &case.rows,
        above: case.above,
        lane_columns: case.lane_columns,
        marker: case.marker,
        cell_width: cell.0,
        cell_height: cell.1,
        palette: &PALETTE,
        muted: MUTED,
    })
    .unwrap_or_else(|why| panic!("{}: {why}", case.name));
    writeln!(
        out,
        "{} @{}x{}: {}x{} {:016x}",
        case.name,
        cell.0,
        cell.1,
        frame.width,
        frame.height,
        digest(&frame.pixels)
    )
    .unwrap_or_else(|why| panic!("writing to a String: {why}"));
}

/// A strip of `rows[range]` drawn plainly, with the row above it if any.
fn plain<'a>(
    name: &'static str,
    rows: &'a [Row],
    range: std::ops::Range<usize>,
    marker: Option<Marker>,
) -> Case<'a> {
    Case {
        name,
        above: range.start.checked_sub(1).map(|above| &rows[above]),
        lane_columns: width_of(rows),
        rows: rows[range].iter().map(RasterRow::plain).collect(),
        marker,
    }
}

/// Rows 40..72 of `rows` with a thread on two rows in three (`even`, then
/// `few`) and some rows dimmed whole.
fn threaded<'a>(rows: &'a [Row], even: &'a BTreeSet<usize>, few: &'a BTreeSet<usize>) -> Case<'a> {
    Case {
        name: "wide-thread",
        rows: rows[40..72]
            .iter()
            .enumerate()
            .map(|(index, row)| RasterRow {
                thread: match index % 3 {
                    0 => Some(even),
                    1 => Some(few),
                    _ => None,
                },
                emphasis: if index % 5 == 0 {
                    Emphasis::Dimmed
                } else {
                    Emphasis::Normal
                },
                ..RasterRow::plain(row)
            })
            .collect(),
        above: Some(&rows[39]),
        lane_columns: width_of(rows),
        marker: None,
    }
}

#[test]
fn strips_keep_every_byte() {
    let shapes = ["foxtrot-merge", "criss-cross-merge", "octopus-merge"];
    let corpora: Vec<(&str, Vec<Row>)> = shapes.iter().map(|&name| (name, corpus(name))).collect();
    let chain = synthetic(64, 1);
    let parallel = synthetic(64, 6);
    let wide = synthetic(96, 28);
    let wide_columns = width_of(&wide);
    assert!(wide_columns >= 28, "the wide strip is {wide_columns} lanes");
    let even: BTreeSet<usize> = (0..wide_columns).step_by(2).collect();
    let few: BTreeSet<usize> = [0, 3, 7].into();

    let mut cases: Vec<Case<'_>> = corpora
        .iter()
        .map(|(name, rows)| Case {
            name,
            rows: rows.iter().map(RasterRow::plain).collect(),
            above: None,
            lane_columns: width_of(rows),
            marker: None,
        })
        .collect();
    cases.push(plain("chain", &chain, 0..32, None));
    cases.push(plain("parallel-mid", &parallel, 32..64, None));
    cases.push(plain("wide-mid", &wide, 32..64, None));
    cases.push(threaded(&wide, &even, &few));
    cases.push(Case {
        lane_columns: 12,
        ..plain(
            "wide-marker",
            &wide,
            32..64,
            Some(Marker {
                column: 11,
                folded: 17,
            }),
        )
    });

    let mut out = String::new();
    for cell in CELLS {
        for case in &cases {
            line(&mut out, case, cell);
        }
    }
    insta::assert_snapshot!("raster-pixel-digests", out);
}

/// Per-strip rasterise cost at the user's cell, reported and not asserted
/// (the budget is the frames test's, ADR-0021). Run in release to compare a
/// rasteriser change: `cargo test --release -p graft-graph --test it
/// reports_per_strip_cost -- --nocapture`.
#[test]
fn reports_per_strip_cost() {
    use std::time::Instant;
    for (name, count, lanes) in [("chain", 64, 1), ("parallel", 64, 6), ("wide", 96, 28)] {
        let rows = synthetic(count, lanes);
        let raster_rows: Vec<RasterRow<'_>> = rows[32..64].iter().map(RasterRow::plain).collect();
        let strip = Strip {
            rows: &raster_rows,
            above: Some(&rows[31]),
            lane_columns: width_of(&rows),
            marker: None,
            cell_width: CELLS[0].0,
            cell_height: CELLS[0].1,
            palette: &PALETTE,
            muted: MUTED,
        };
        let mut times: Vec<u128> = (0..31)
            .map(|_| {
                let start = Instant::now();
                let frame = rasterise(&strip).unwrap_or_else(|why| panic!("{name}: {why}"));
                assert_eq!(frame.rows, 32);
                start.elapsed().as_micros()
            })
            .collect();
        times.sort_unstable();
        eprintln!(
            "per-strip {name} ({} lanes, 32 rows, 17x37): p50 {} µs, min {} µs, max {} µs",
            width_of(&rows),
            times[15],
            times[0],
            times[30],
        );
    }
}

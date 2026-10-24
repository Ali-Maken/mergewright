//! The cursor row's second line in pixels (ease-pass 13, G9): one cell row
//! with every lane leaving the row drawn straight through it, measured in
//! the frame buffer. Whether it *looks* right is the user's to judge.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a panic is the intended failure (PROMPT.md 6.3)"
)]

use std::collections::BTreeSet;

use mergewright_core::capabilities::Rgb;
use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_core::raster::RasterFrame;
use mergewright_graph::layout::layout;
use mergewright_graph::raster::{
    Backdrop, Lit, Marker, PlainRow, PlainStrip, Selection, continuation_key,
    rasterise_continuation,
};
use mergewright_graph::row::Row;
use mergewright_graph::seeds::Seeds;

const CELL_W: u32 = 16;
const CELL_H: u32 = 32;
const PALETTE: [Rgb; 2] = [
    Rgb {
        red: 200,
        green: 0,
        blue: 0,
    },
    Rgb {
        red: 0,
        green: 0,
        blue: 200,
    },
];
const MUTED: Rgb = Rgb {
    red: 90,
    green: 90,
    blue: 90,
};

/// A merge over two lines of four commits: under the merge, both lanes run
/// down side by side.
fn rows() -> Vec<Row> {
    let id = |line: u8, index: u8| Oid::from_bytes(&[line, index].repeat(10)).unwrap();
    let who = Identity {
        name: b"T".to_vec(),
        email: b"t@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    let commit = |id, parents| Commit {
        id,
        parents,
        author: who.clone(),
        committer: who.clone(),
        message: b"row".to_vec(),
    };
    let mut commits = vec![commit(id(0, 0), vec![id(1, 1), id(2, 1)])];
    for line in 1..=2 {
        for index in 1..=4 {
            let parents = if index < 4 {
                vec![id(line, index + 1)]
            } else {
                Vec::new()
            };
            commits.push(commit(id(line, index), parents));
        }
    }
    layout(&commits, Seeds::default())
}

/// Lane columns leaving `row` through the bottom of its band, read off the
/// row's own fields (not the rasteriser's helper).
fn leaving(row: &Row, marker: Option<usize>) -> BTreeSet<usize> {
    let mut columns: BTreeSet<usize> = row.outgoing.iter().map(|&(column, _)| column).collect();
    for (column, identity) in row.lanes.iter().enumerate() {
        if identity.is_some()
            && column != row.lane
            && !row.incoming.contains(&column)
            && Some(column) != marker
        {
            columns.insert(column);
        }
    }
    columns
}

fn lanes(rows: &[Row]) -> usize {
    rows.iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap()
}

fn alpha(frame: &RasterFrame, x: u32, y: u32) -> u8 {
    frame.pixels[((y * frame.width + x) * 4 + 3) as usize]
}

/// Whether lane column `column`'s centre line is drawn at the band's top,
/// middle and bottom scanlines.
fn straight_in(frame: &RasterFrame, column: usize) -> bool {
    let centre = (u32::try_from(column).unwrap() * 2 + 1) * CELL_W;
    [0, CELL_H / 2, CELL_H - 1]
        .into_iter()
        .all(|y| (centre - 1..=centre + 1).any(|x| alpha(frame, x, y) > 64))
}

/// Whether anything at all is drawn in lane column `column`'s cells.
fn anything_in(frame: &RasterFrame, column: usize) -> bool {
    let left = u32::try_from(column).unwrap() * 2 * CELL_W;
    (0..CELL_H).any(|y| (left..left + 2 * CELL_W).any(|x| alpha(frame, x, y) > 0))
}

fn strip<'a>(plain: &'a [PlainRow<'a>], lane_columns: usize) -> PlainStrip<'a> {
    PlainStrip {
        rows: plain,
        above: None,
        lane_columns,
        marker: None,
        cell_width: CELL_W,
        cell_height: CELL_H,
        palette: &PALETTE,
        muted: MUTED,
        roles: None,
    }
}

#[test]
fn every_lane_leaving_the_row_runs_straight_through_its_second_line() {
    let rows = rows();
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let strip = strip(&plain, lanes(&rows));
    let lit = vec![Lit::default(); rows.len()];
    let sel = Selection {
        rows: &lit,
        backdrop: Backdrop::Full,
    };
    let mut checked = 0;
    for (index, row) in rows.iter().enumerate() {
        let through = leaving(row, None);
        let frame = rasterise_continuation(&strip, &sel, index).unwrap();
        let Some(frame) = frame else {
            assert!(
                through.is_empty(),
                "row {index}: {through:?} leave, no band"
            );
            continue;
        };
        assert_eq!((frame.rows, frame.height), (1, CELL_H), "one cell row");
        assert_eq!(
            frame.width,
            u32::try_from(strip.lane_columns).unwrap() * 2 * CELL_W
        );
        for column in 0..strip.lane_columns {
            if through.contains(&column) {
                assert!(straight_in(&frame, column), "row {index} column {column}");
                checked += 1;
            } else {
                assert!(!anything_in(&frame, column), "row {index} column {column}");
            }
        }
    }
    assert!(checked >= rows.len(), "two lanes side by side were checked");
}

#[test]
fn the_last_row_of_a_line_draws_no_band_and_a_row_past_the_strip_is_none() {
    let rows = rows();
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let strip = strip(&plain, lanes(&rows));
    let lit = vec![Lit::default(); rows.len()];
    let sel = Selection {
        rows: &lit,
        backdrop: Backdrop::Full,
    };
    // The bottom row is a root on the last lane alive: nothing leaves it.
    let last = rows.len() - 1;
    assert!(leaving(&rows[last], None).is_empty());
    assert_eq!(rasterise_continuation(&strip, &sel, last).unwrap(), None);
    assert_eq!(continuation_key(&strip, &sel, last).unwrap(), None);
    assert_eq!(
        rasterise_continuation(&strip, &sel, rows.len()).unwrap(),
        None
    );
}

#[test]
fn the_thread_lane_stays_lit_through_the_band_and_the_key_follows_the_paint() {
    let rows = rows();
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let strip = strip(&plain, lanes(&rows));
    // Row 1 is the first line's first commit: both lanes run on under it.
    let row = 1;
    assert!(leaving(&rows[row], None).len() >= 2);
    let own = rows[row].lane;
    let thread: BTreeSet<usize> = [own].into();
    let lit: Vec<Lit<'_>> = rows
        .iter()
        .map(|_| Lit {
            thread: Some(&thread),
            ..Lit::default()
        })
        .collect();
    let on_thread = Selection {
        rows: &lit,
        backdrop: Backdrop::OffThread,
    };
    let frame = rasterise_continuation(&strip, &on_thread, row)
        .unwrap()
        .unwrap();
    let peak = |column: usize| {
        let centre = (u32::try_from(column).unwrap() * 2 + 1) * CELL_W;
        (0..frame.width.min(centre + 3))
            .skip(centre as usize - 2)
            .map(|x| alpha(&frame, x, CELL_H / 2))
            .max()
            .unwrap()
    };
    let other = *leaving(&rows[row], None)
        .iter()
        .find(|&&column| column != own)
        .unwrap();
    assert!(peak(own) > 200, "the thread at full alpha: {}", peak(own));
    assert!(
        (100..160).contains(&peak(other)),
        "off the thread at half: {}",
        peak(other)
    );

    let plain_lit = vec![Lit::default(); rows.len()];
    let unlit = Selection {
        rows: &plain_lit,
        backdrop: Backdrop::Full,
    };
    let key = |sel: &Selection<'_>, row| continuation_key(&strip, sel, row).unwrap();
    assert_eq!(key(&on_thread, row), key(&on_thread, row), "stable");
    assert_ne!(key(&on_thread, row), key(&unlit, row), "the paint is in it");
}

#[test]
fn the_rail_runs_through_the_band_when_lanes_fold() {
    let rows = rows();
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let columns = lanes(&rows) + 1;
    let marker = columns - 1;
    let strip = PlainStrip {
        marker: Some(Marker { column: marker }),
        ..strip(&plain, columns)
    };
    let lit = vec![Lit::default(); rows.len()];
    let sel = Selection {
        rows: &lit,
        backdrop: Backdrop::Full,
    };
    let frame = rasterise_continuation(&strip, &sel, 1).unwrap().unwrap();
    assert!(anything_in(&frame, marker), "the rail's dashes");
}

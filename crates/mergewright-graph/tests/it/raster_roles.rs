//! A lane's stroke by role (ease-pass 13, G1/G2), measured in the frame
//! buffer: Other dotted, Trunk two parallel strokes, This wider than
//! Under. Whether it *looks* right is the user's to judge in Ghostty.

use mergewright_core::capabilities::Rgb;
use mergewright_core::commit::{Commit, Identity, Timestamp};
use mergewright_core::oid::Oid;
use mergewright_core::raster::RasterFrame;
use mergewright_graph::LaneRole;
use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::layout::layout;
use mergewright_graph::raster::{Backdrop, LaneStroke, PlainRow, PlainStrip, rasterise_backdrop};
use mergewright_graph::row::Row;
use mergewright_graph::seeds::Seeds;

const CELL: (u32, u32) = (16, 32);
const CELL_W: usize = 16;
const CELL_H: usize = 32;
const PALETTE: [Rgb; 4] = [
    Rgb {
        red: 200,
        green: 0,
        blue: 0,
    },
    Rgb {
        red: 0,
        green: 200,
        blue: 0,
    },
    Rgb {
        red: 0,
        green: 0,
        blue: 200,
    },
    Rgb {
        red: 128,
        green: 128,
        blue: 128,
    },
];
const MUTED: Rgb = Rgb {
    red: 90,
    green: 90,
    blue: 90,
};

/// A merge over two lines of four commits: the second line's lane passes
/// straight through the first line's rows.
fn rows() -> Vec<Row> {
    let id = |line: u8, index: u8| {
        Oid::from_bytes(&[line, index].repeat(10)).unwrap_or_else(|why| panic!("{why}"))
    };
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

/// A row where lane column `column` passes straight through, and that
/// lane's branch.
fn straight_through(rows: &[Row], column: usize) -> (usize, BranchIdentity) {
    rows.iter()
        .enumerate()
        .find_map(|(index, row)| {
            let identity = row.lanes.get(column).copied().flatten()?;
            let edge = row.lane == column
                || row.incoming.contains(&column)
                || row.outgoing.iter().any(|&(out, _)| out == column);
            (!edge).then_some((index, identity))
        })
        .unwrap_or_else(|| panic!("no row passes straight through column {column}"))
}

fn draw(rows: &[Row], roles: Option<&[(BranchIdentity, LaneRole)]>) -> RasterFrame {
    let plain: Vec<PlainRow<'_>> = rows.iter().map(PlainRow::of).collect();
    let lanes = rows
        .iter()
        .map(|row| row.lanes.len().max(row.lane + 1))
        .max()
        .unwrap_or(0);
    let strip = PlainStrip {
        rows: &plain,
        above: None,
        lane_columns: lanes,
        marker: None,
        cell_width: CELL.0,
        cell_height: CELL.1,
        palette: &PALETTE,
        muted: MUTED,
        roles,
    };
    rasterise_backdrop(&strip, Backdrop::Full).unwrap_or_else(|why| panic!("{why}"))
}

fn alpha(frame: &RasterFrame, x: usize, y: usize) -> u8 {
    let width = frame.pixels.len() / 4 / (frame.rows * CELL_H);
    frame.pixels[(y * width + x) * 4 + 3]
}

/// The pixels `x0..x1` one cell wide around lane column `column`'s centre
/// line, which is the boundary between the lane's two cells (ease-pass/29).
fn lane_cell(column: usize) -> std::ops::Range<usize> {
    let width = CELL_W;
    let centre = (column * 2 + 1) * width;
    centre - width / 2..centre + width / 2
}

/// Runs of drawn pixels (alpha > 64) across lane column `column`'s cell at
/// the middle of band `row`: their widths, left to right.
fn runs_across(frame: &RasterFrame, column: usize, row: usize) -> Vec<usize> {
    let y = row * CELL_H + 16;
    let mut runs = Vec::new();
    let mut run = 0;
    for x in lane_cell(column) {
        if alpha(frame, x, y) > 64 {
            run += 1;
        } else if run > 0 {
            runs.push(run);
            run = 0;
        }
    }
    if run > 0 {
        runs.push(run);
    }
    runs
}

/// Whether the lane's centre pixel column down band `row` has gaps.
fn gaps_down(frame: &RasterFrame, column: usize, row: usize) -> usize {
    let cells = lane_cell(column);
    let x = usize::midpoint(cells.start, cells.end);
    let height = CELL_H;
    let top = row * height;
    // Brightest pixel column near the centre, so a half-lit edge is not read.
    let x = (x - 2..x + 2)
        .max_by_key(|&x| {
            (top..top + height)
                .map(|y| u32::from(alpha(frame, x, y)))
                .sum::<u32>()
        })
        .unwrap_or(x);
    let mut gaps = 0;
    let mut lit = false;
    for y in top..top + height {
        let on = alpha(frame, x, y) > 64;
        if lit && !on {
            gaps += 1;
        }
        lit = on;
    }
    gaps
}

fn with_role(rows: &[Row], role: LaneRole) -> (RasterFrame, usize) {
    let (row, identity) = straight_through(rows, 1);
    (draw(rows, Some(&[(identity, role)])), row)
}

#[test]
fn other_is_dotted_and_trunk_is_two_strokes_and_this_is_wider_than_under() {
    let rows = rows();
    let (other, row) = with_role(&rows, LaneRole::Other);
    assert!(
        gaps_down(&other, 1, row) >= 2,
        "Other is dotted: gaps every few px"
    );
    assert!(runs_across(&other, 1, row).len() <= 1);

    let (trunk, _) = with_role(&rows, LaneRole::Trunk);
    assert_eq!(
        runs_across(&trunk, 1, row).len(),
        2,
        "Trunk: two parallel strokes"
    );
    assert_eq!(gaps_down(&trunk, 1, row), 0);

    let (this, _) = with_role(&rows, LaneRole::This);
    let (under, _) = with_role(&rows, LaneRole::Under);
    let this_runs = runs_across(&this, 1, row);
    let under_runs = runs_across(&under, 1, row);
    assert_eq!((this_runs.len(), under_runs.len()), (1, 1));
    assert!(
        this_runs[0] > under_runs[0],
        "This {this_runs:?} wider than Under {under_runs:?}"
    );
    assert_eq!(gaps_down(&this, 1, row) + gaps_down(&under, 1, row), 0);
}

#[test]
fn no_roles_draws_as_before_and_an_absent_branch_is_other() {
    let rows = rows();
    let (row, _) = straight_through(&rows, 1);
    let plain = draw(&rows, None);
    assert_eq!(
        gaps_down(&plain, 1, row),
        0,
        "without roles, one plain stroke"
    );
    let absent = draw(&rows, Some(&[]));
    assert!(
        gaps_down(&absent, 1, row) >= 2,
        "absent from the roles: Other, dotted"
    );
}

#[test]
fn plain_strip_is_still_copy() {
    fn copy<T: Copy>() {}
    copy::<PlainStrip<'static>>();
    assert_eq!(LaneStroke::of(LaneRole::Trunk), LaneStroke::Double);
}

use graft_core::capabilities::ColorDepth;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::layout::layout;
use graft_graph::seeds::{BranchSeed, Seeds};
use ratatui::style::Color;

use super::*;
use crate::test_support::terminal;
use crate::theme::ThemeKind;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

fn commit(id: u8, parents: &[u8], subject: &[u8]) -> Commit {
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: i64::from(id),
            offset_seconds: 0,
        },
    };
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: who.clone(),
        committer: who,
        message: subject.to_vec(),
    }
}

fn main_seed(tip: u8) -> Seeds {
    Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(tip),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    }
}

/// `b` merged into main, then `second` reusing the column `b` freed.
fn reuse_rows_with(second: &str) -> Vec<Row> {
    let merge_second = format!("Merge branch '{second}'");
    layout(
        &[
            commit(6, &[5, 4], b"Merge branch 'b'"),
            commit(4, &[5], b"b: work"),
            commit(5, &[3], b"main: work"),
            commit(3, &[1, 2], merge_second.as_bytes()),
            commit(2, &[1], b"second: work"),
            commit(1, &[], b"root"),
        ],
        main_seed(6),
    )
}

fn reuse_rows() -> Vec<Row> {
    reuse_rows_with("c")
}

fn render(lines: &[GraphLine<'_>], theme: &Theme, width: u16) -> Buffer {
    let area = Rect::new(0, 0, width, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(area);
    GraphColumn::new(lines, theme).render(area, &mut buf);
    buf
}

fn text(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect()
}

fn dark(depth: ColorDepth) -> Theme {
    Theme::new(ThemeKind::Dark, &terminal(depth, false))
}

#[test]
fn a_merge_forks_and_the_branch_closes_with_rounded_elbows() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let buf = render(&lines, &dark(ColorDepth::TrueColor), 4);

    let drawn: Vec<String> = (0..6).map(|y| text(&buf, y)).collect();
    assert_eq!(
        drawn,
        ["◉─╮ ", "│ ● ", "●─╯ ", "◉─╮ ", "│ ● ", "◇─╯ "],
        "top to bottom"
    );
}

#[test]
fn colour_follows_the_branch_not_the_column() {
    let theme = dark(ColorDepth::TrueColor);
    let b = BranchIdentity::of_name(b"b");
    // A second branch whose hue differs from b's, so equal colours could
    // only mean the column decided them.
    let second = ["c", "d", "e", "f", "g", "h"]
        .into_iter()
        .find(|name| BranchIdentity::of_name(name.as_bytes()).slot(12) != b.slot(12))
        .unwrap_or_else(|| panic!("six names cannot all share b's hue"));
    let c = BranchIdentity::of_name(second.as_bytes());

    let rows = reuse_rows_with(second);
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let buf = render(&lines, &theme, 4);

    assert_eq!(buf[(2, 1)].fg, color(theme.branch(b)), "column 1 as b");
    assert_eq!(buf[(2, 4)].fg, color(theme.branch(c)), "column 1 as c");
}

#[test]
fn special_commits_differ_in_shape() {
    let kinds = [
        NodeKind::Commit,
        NodeKind::Merge,
        NodeKind::Root,
        NodeKind::Head,
        NodeKind::Stash,
        NodeKind::Uncommitted,
    ];
    let mut glyphs: Vec<char> = kinds.iter().map(|kind| kind.glyph()).collect();
    glyphs.sort_unstable();
    glyphs.dedup();
    assert_eq!(glyphs.len(), kinds.len());
}

#[test]
fn the_caller_decides_head_and_the_row_decides_the_rest() {
    let rows = reuse_rows();
    assert_eq!(NodeKind::of(&rows[0]), NodeKind::Merge);
    assert_eq!(NodeKind::of(&rows[1]), NodeKind::Commit);
    assert_eq!(NodeKind::of(&rows[5]), NodeKind::Root);

    let head = GraphLine {
        node: NodeKind::Head,
        ..GraphLine::plain(&rows[1])
    };
    let buf = render(&[head], &dark(ColorDepth::TrueColor), 4);
    assert_eq!(text(&buf, 0), "│ ◆ ");
}

#[test]
fn paint_dims_or_emphasises_every_cell_of_the_row() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    for (paint, modifier) in [
        (RowPaint::Dimmed, Modifier::DIM),
        (RowPaint::Highlighted, Modifier::BOLD),
    ] {
        let line = GraphLine {
            paint,
            ..GraphLine::plain(&rows[0])
        };
        let buf = render(&[line], &theme, 4);
        for x in 0..4 {
            assert!(buf[(x, 0)].modifier.contains(modifier), "{paint:?} at {x}");
        }
    }
}

#[test]
fn the_column_is_exactly_its_area_wide_even_when_lanes_run_past_it() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let mut buf = Buffer::empty(Rect::new(0, 0, 8, 6));
    buf[(3, 0)].set_char('x');
    GraphColumn::new(&lines, &dark(ColorDepth::TrueColor)).render(Rect::new(0, 0, 3, 6), &mut buf);

    assert_eq!(text(&buf, 0), "◉─╮x    ", "nothing written past the area");
    assert_eq!(GraphColumn::width_for(2), 4);
}

#[test]
fn a_256_colour_terminal_gets_palette_indices() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let buf = render(&lines, &dark(ColorDepth::Palette256), 4);

    assert!(matches!(buf[(0, 0)].fg, Color::Indexed(_)));
}

/// Which cells of a rendered row are dashed, as a string of `╎`/`╌` positions.
fn dashes(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| match buf[(x, y)].symbol() {
            "╎" | "╌" => 'd',
            _ => '.',
        })
        .collect()
}

#[test]
fn where_hue_cannot_carry_lanes_odd_columns_are_dashed() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let truecolor = render(&lines, &dark(ColorDepth::TrueColor), 4);
    for depth in [ColorDepth::Palette16, ColorDepth::None] {
        let buf = render(&lines, &dark(depth), 4);
        for y in 0..buf.area.height {
            let solid: String = text(&truecolor, y).replace('│', "|").replace('─', "-");
            let drawn = text(&buf, y);
            // Same picture, only the lane in column 1 (cell 2) and the
            // horizontal edge it owns (cell 1) swap to their dashed forms.
            let expected: String = solid
                .chars()
                .enumerate()
                .map(|(x, glyph)| match (x, glyph) {
                    (2, '|') => '╎',
                    (1, '-') => '╌',
                    (_, '|') => '│',
                    (_, '-') => '─',
                    (_, other) => other,
                })
                .collect();
            assert_eq!(drawn, expected, "{depth:?} row {y}");
        }
        assert!(
            (0..buf.area.height).any(|y| dashes(&buf, y).contains('d')),
            "{depth:?}: the fixture has an odd-column lane to dash"
        );
    }
}

#[test]
fn with_enough_colour_every_lane_stays_solid() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    for depth in [ColorDepth::TrueColor, ColorDepth::Palette256] {
        let buf = render(&lines, &dark(depth), 4);
        for y in 0..buf.area.height {
            assert_eq!(dashes(&buf, y), "....", "{depth:?} row {y}");
        }
    }
}

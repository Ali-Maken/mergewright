use graft_core::capabilities::ColorDepth;
use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_graph::identity::BranchIdentity;
use graft_graph::layout::layout;
use graft_graph::seeds::{BranchSeed, Seeds};
use ratatui::style::Color;

use super::*;
use crate::test_support::{terminal, wide_history};
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

fn render_ascii(lines: &[GraphLine<'_>], theme: &Theme, width: u16) -> Buffer {
    let area = Rect::new(0, 0, width, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(area);
    GraphColumn::new(lines, theme)
        .ascii(true)
        .render(area, &mut buf);
    buf
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
fn a_thread_lights_its_lanes_bold_and_dims_the_rest_of_the_row() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let merge = &rows[0];
    let lit = merge.lane;
    let line = GraphLine {
        thread: Some([lit].into()),
        ..GraphLine::plain(merge)
    };
    let width = GraphColumn::width_for(merge.lanes.len().max(merge.lane + 1));
    let buf = render(&[line], &theme, width);
    let node = u16::try_from(lit).expect("small") * CELLS_PER_LANE;
    assert!(buf[(node, 0)].modifier.contains(Modifier::BOLD));
    assert!(!buf[(node, 0)].modifier.contains(Modifier::DIM));
    let other = merge
        .outgoing
        .iter()
        .map(|(column, _)| *column)
        .find(|c| *c != lit);
    let other = other.expect("a merge has a second lane");
    let cell = u16::try_from(other).expect("small") * CELLS_PER_LANE;
    assert!(buf[(cell, 0)].modifier.contains(Modifier::DIM), "{other}");
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

/// The twelve merge rows of a twelve-topic history: thirteen columns open.
fn wide_merges() -> Vec<Row> {
    let mut rows = wide_history(12);
    rows.truncate(12);
    rows
}

fn no_pins() -> graft_graph::fold::Keep {
    graft_graph::fold::Keep::default()
}

fn render_through(lines: &[GraphLine<'_>], fold: &Fold, theme: &Theme) -> Buffer {
    let width = GraphColumn::width_through(fold);
    let area = Rect::new(0, 0, width, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(area);
    GraphColumn::new(lines, theme)
        .through(fold)
        .render(area, &mut buf);
    buf
}

#[test]
fn folded_lanes_draw_a_rail_and_their_edges_as_stubs() {
    let rows = wide_merges();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let fold = Fold::plan(8, &no_pins(), &rows);
    let theme = dark(ColorDepth::TrueColor);
    let buf = render_through(&lines, &fold, &theme);

    assert_eq!(
        GraphColumn::width_through(&fold),
        16,
        "seven lanes, then the rail and a gap"
    );
    // The top merge's second parent lives in a folded lane: a stub right of
    // the node in the lane's colour, and nothing runs to the rail.
    assert_eq!(text(&buf, 0), "◉▾            ⋮ ");
    let topic_12 = rows[0].lanes[1].expect("row 0 opened column 1 for topic-12");
    assert_eq!(buf[(1, 0)].fg, color(theme.branch(topic_12)));
    assert_eq!(buf[(14, 0)].fg, color(theme.token(Token::TextMuted)));
    // Row 6 opened column 7, the first kept lane after lane 0: no stub.
    assert_eq!(text(&buf, 6), "◉─╮           ⋮ ");
    assert_eq!(text(&buf, 11), "◉─│─│─│─│─│─╮ ⋮ ");
}

#[test]
fn every_row_ends_in_the_rail_with_no_count_at_every_colour_depth() {
    let rows = wide_merges();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let fold = Fold::plan(8, &no_pins(), &rows);
    let truecolor: Vec<String> = {
        let buf = render_through(&lines, &fold, &dark(ColorDepth::TrueColor));
        (0..buf.area.height).map(|y| text(&buf, y)).collect()
    };
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Palette16,
        ColorDepth::None,
    ] {
        let buf = render_through(&lines, &fold, &dark(depth));
        for y in 0..buf.area.height {
            let row = text(&buf, y);
            assert_eq!(buf[(14, y)].symbol(), "⋮", "{depth:?} row {y}: {row:?}");
            assert!(
                !row.chars().any(|c| c.is_ascii_digit() || c == '+'),
                "{depth:?} row {y}: {row:?} carries a count"
            );
        }
        // Rows 0–5 fold their merge's second parent: a stub on every one.
        for y in 0..6 {
            assert_eq!(buf[(1, y)].symbol(), "▾", "{depth:?} row {y}");
        }
    }
    // Colour depth never moves a stub or the rail (dash parity aside).
    assert_eq!(truecolor[0], "◉▾            ⋮ ");
}

#[test]
fn a_folded_incoming_edge_stubs_upward() {
    let rows = wide_merges();
    let fold = Fold::plan(8, &no_pins(), &rows);
    let folded = fold.folded()[0];
    // A commit on lane 0 that a folded lane closes into from above.
    let mut row = rows[11].clone();
    row.outgoing.truncate(1);
    row.incoming = vec![folded];
    let lines = [GraphLine::plain(&row)];
    let buf = render_through(&lines, &fold, &dark(ColorDepth::TrueColor));

    assert_eq!(buf[(1, 0)].symbol(), STUB_ABOVE.to_string());
    assert_eq!(buf[(14, 0)].symbol(), RAIL.to_string());
}

#[test]
fn the_ascii_tier_draws_only_the_six_glyph_table() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let buf = render_ascii(&lines, &dark(ColorDepth::TrueColor), 4);

    let drawn: Vec<String> = (0..6).map(|y| text(&buf, y)).collect();
    assert_eq!(
        drawn,
        ["*-\\ ", "| * ", "*-/ ", "*-\\ ", "| * ", "*-/ "],
        "every node collapses to *, elbows to / or \\"
    );
    for row in &drawn {
        for ch in row.chars() {
            assert!(
                "*|/\\-+ ".contains(ch),
                "{ch:?} is not in the ASCII glyph table"
            );
        }
    }
}

#[test]
fn ascii_never_draws_the_dash_lane_convention() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    for depth in [ColorDepth::Palette16, ColorDepth::None] {
        let buf = render_ascii(&lines, &dark(depth), 4);
        for y in 0..buf.area.height {
            let row = text(&buf, y);
            assert!(
                !row.contains('╎') && !row.contains('╌'),
                "{depth:?} row {y}: {row:?} has a Unicode dash the ASCII table has no room for"
            );
        }
    }
}

#[test]
fn ascii_folded_lanes_draw_the_rail_and_stubs_from_the_table() {
    let rows = wide_merges();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let fold = Fold::plan(8, &no_pins(), &rows);
    let theme = dark(ColorDepth::TrueColor);
    let width = GraphColumn::width_through(&fold);
    let area = Rect::new(0, 0, width, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(area);
    GraphColumn::new(&lines, &theme)
        .through(&fold)
        .ascii(true)
        .render(area, &mut buf);

    assert_eq!(text(&buf, 0), "*v            : ");
    assert_eq!(text(&buf, 11), "*-|-|-|-|-|-\\ : ");
}

#[test]
fn a_fold_that_folds_nothing_draws_as_laid_out() {
    let rows = reuse_rows();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let fold = Fold::plan(8, &no_pins(), &rows);
    let theme = dark(ColorDepth::TrueColor);
    let plain = render(&lines, &theme, 4);
    let through = render_through(&lines, &fold, &theme);
    assert_eq!(through, plain);
}

#[test]
fn dashes_follow_the_real_column_through_a_fold() {
    let rows = wide_merges();
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    // Seven folded: real column 8 draws at position 1, real 9 at position 2.
    let fold = Fold::plan(7, &no_pins(), &rows);
    assert_eq!(fold.kept(), [0, 8, 9, 10, 11, 12]);
    let buf = render_through(&lines, &fold, &dark(ColorDepth::Palette16));

    // Both lanes pass through the bottom merge row.
    assert_eq!(text(&buf, 11), "◉─│─╎─│─╎─╮ ⋮ ");
    assert_eq!(buf[(2, 11)].symbol(), "│", "real column 8 is even: solid");
    assert_eq!(buf[(4, 11)].symbol(), "╎", "real column 9 is odd: dashed");
}

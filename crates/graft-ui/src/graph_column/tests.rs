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

mod fold;

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
        NodeKind::StashOnly,
    ];
    let mut glyphs: Vec<char> = kinds.iter().map(|kind| kind.glyph()).collect();
    glyphs.sort_unstable();
    glyphs.dedup();
    assert_eq!(glyphs.len(), kinds.len());
}

/// m7-power-tools/16: a commit only a stash reaches draws a hollow square —
/// the stash's `■`, emptied: kin to the stash, not a commit any branch has.
/// Not the ticket's `◇`, which is the root's (`NodeKind::Root`; see
/// `special_commits_differ_in_shape`). At the ASCII tier it is `*`, as every
/// node is: the row's `only in stash@{n}` badge carries it there.
#[test]
fn stash_only_node_draws_a_hollow_square() {
    let rows = reuse_rows();
    let line = GraphLine {
        node: NodeKind::StashOnly,
        ..GraphLine::plain(&rows[1])
    };
    let theme = dark(ColorDepth::TrueColor);
    let buf = render(std::slice::from_ref(&line), &theme, 4);
    assert_eq!(text(&buf, 0), "│ □ ");
    let buf = render_ascii(&[line], &theme, 4);
    assert_eq!(text(&buf, 0), "| * ");
}

/// ease-pass 06f: a folded (or first-parent) merge still draws `◉`, as the
/// main-screen mock's folded rows do; the kind is git's commit (06 D1).
#[test]
fn a_merge_whose_side_is_not_drawn_is_still_a_merge() {
    let rows = reuse_rows();
    let mut folded = rows[0].clone();
    assert!(folded.commit.parents.len() > 1, "row 0 is a merge");
    folded.drawn_parents.truncate(1);
    folded.outgoing.truncate(1);
    assert_eq!(NodeKind::of(&folded), NodeKind::Merge);
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
    // At truecolor the fade is the hue itself (m8-polish-and-ship/17 item 3).
    assert!(!buf[(cell, 0)].modifier.contains(Modifier::DIM), "{other}");
    assert!(!buf[(cell, 0)].modifier.contains(Modifier::BOLD), "{other}");
}

/// m8-polish-and-ship/17 item 3: an off-thread lane cell draws its hue
/// mixed toward `bg` (`Theme::lane_muted`) at truecolor and 256 colours,
/// with no DIM (terminals ignoring SGR 2 would show no fade); at 16 colours
/// and `NO_COLOR` it keeps its colour and DIM.
#[test]
fn off_thread_lanes_fade_by_hue_at_truecolor_and_by_dim_at_16() {
    let rows = reuse_rows();
    let merge = &rows[0];
    let lit = merge.lane;
    let other = merge
        .outgoing
        .iter()
        .map(|(column, _)| *column)
        .find(|c| *c != lit)
        .expect("a merge has a second lane");
    let cell = u16::try_from(other).expect("small") * CELLS_PER_LANE;
    let width = GraphColumn::width_for(merge.lanes.len().max(merge.lane + 1));
    for depth in [
        ColorDepth::TrueColor,
        ColorDepth::Palette256,
        ColorDepth::Palette16,
        ColorDepth::None,
    ] {
        let theme = dark(depth);
        let plain = render(&[GraphLine::plain(merge)], &theme, width);
        let hue = plain[(cell, 0)].fg;
        let slot = (0..12)
            .find(|slot| crate::paint::color(theme.lane(*slot)) == hue)
            .expect("the cell draws a lane hue");
        let threaded = GraphLine {
            thread: Some([lit].into()),
            ..GraphLine::plain(merge)
        };
        let buf = render(&[threaded], &theme, width);
        let faded = &buf[(cell, 0)];
        if theme.fades_by_hue() {
            assert_eq!(
                faded.fg,
                crate::paint::color(theme.lane_muted(slot)),
                "{depth:?}"
            );
            assert_ne!(faded.fg, hue, "{depth:?}: the muted hue differs");
            assert!(!faded.modifier.contains(Modifier::DIM), "{depth:?}");
        } else {
            assert_eq!(faded.fg, hue, "{depth:?}");
            assert!(faded.modifier.contains(Modifier::DIM), "{depth:?}");
        }
    }
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

/// Every real column on `row`: the preview owns the whole row, which is
/// what a slice with no real branch running through it looks like.
fn all_lanes(row: &Row) -> BTreeSet<usize> {
    (0..row.lanes.len().max(row.lane + 1)).collect()
}

/// A ghost row's node draws the anchor glyph (`○`/`◎`), never the real
/// [`NodeKind`] glyph, and every straight edge on the row dashes — at any
/// colour depth, unconditionally (unlike the 16-colour/`NO_COLOR` odd-column
/// convention, which only ever applies to real rows).
#[test]
fn ghost_paint_dashes_every_straight_edge_and_forces_the_anchor_glyph() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let plain_lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    let plain = render(&plain_lines, &theme, 8);

    let ghost_lines: Vec<GraphLine<'_>> = rows
        .iter()
        .map(|row| GraphLine::plain(row).ghost(all_lanes(row)))
        .collect();
    let ghost = render(&ghost_lines, &theme, 8);

    for (y, row) in rows.iter().enumerate() {
        let y = u16::try_from(y).expect("few rows");
        let node_x = lane_x(row.lane);
        for x in 0..plain.area.width {
            let before = plain[(x, y)].symbol();
            let after = ghost[(x, y)].symbol();
            if x == node_x {
                assert!(
                    after == "○" || after == "◎",
                    "node cell at ({x},{y}) drew {after:?}, not an anchor glyph"
                );
                assert_ne!(before, after, "a ghost node must never draw the real glyph");
            } else {
                match before {
                    "│" => assert_eq!(after, "┊", "vertical edge at ({x},{y})"),
                    "─" => assert_eq!(after, "┈", "horizontal edge at ({x},{y})"),
                    other => assert_eq!(after, other, "cell at ({x},{y}) changed unexpectedly"),
                }
            }
        }
    }
}

/// The ghost rows draw nothing outside the `Rect` they were given — the
/// ticket's piece-9 bounds claim for this widget, at an offset so a
/// column-relative `x` that leaked would land on a sentinel cell.
#[test]
fn a_ghost_row_draws_nothing_outside_its_rect() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let lines: Vec<GraphLine<'_>> = rows
        .iter()
        .map(|row| GraphLine::plain(row).ghost(all_lanes(row)))
        .collect();

    let outer = Rect::new(0, 0, 24, 12);
    let inner = Rect::new(3, 2, 8, u16::try_from(lines.len()).expect("few rows"));
    let mut buf = Buffer::empty(outer);
    for y in 0..outer.height {
        for x in 0..outer.width {
            buf[(x, y)].set_char('#');
        }
    }
    GraphColumn::new(&lines, &theme).render(inner, &mut buf);

    for y in 0..outer.height {
        for x in 0..outer.width {
            let inside = x >= inner.x && x < inner.right() && y >= inner.y && y < inner.bottom();
            if !inside {
                assert_eq!(
                    buf[(x, y)].symbol(),
                    "#",
                    "drew outside its rect at ({x}, {y})"
                );
            }
        }
    }
}

/// DESIGN.md §1.7 draws `┊│┊●b┈`: the real lane running through a ghost row
/// stays solid and keeps its own branch colour. Only the lanes the preview
/// owns dash and mute — a real lane that changed appearance because a ghost
/// was drawn beside it leaves the user unable to tell what is real
/// (`docs/design/05-visualization.md`, "real nodes stay solid").
#[test]
fn a_real_lane_through_a_ghost_row_stays_solid_and_keeps_its_colour() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let (row, through) = rows
        .iter()
        .find_map(|row| {
            let through = (0..row.lanes.len()).find(|&column| {
                column != row.lane && row.lanes.get(column).copied().flatten().is_some()
            })?;
            Some((row, through))
        })
        .expect("the fixture has a row with a real lane passing through it");

    let plain = render(std::slice::from_ref(&GraphLine::plain(row)), &theme, 8);
    // Only the anchor's own lane belongs to the preview.
    let ghosted = GraphLine::plain(row).ghost(BTreeSet::from([row.lane]));
    let ghost = render(std::slice::from_ref(&ghosted), &theme, 8);

    let node_x = lane_x(row.lane);
    let through_x = lane_x(through);
    assert!(
        matches!(ghost[(node_x, 0)].symbol(), "○" | "◎"),
        "the anchor itself must still be a ghost: {:?}",
        ghost[(node_x, 0)].symbol()
    );
    assert_eq!(
        ghost[(through_x, 0)].symbol(),
        plain[(through_x, 0)].symbol(),
        "the real lane's glyph changed on a ghost row"
    );
    assert_eq!(
        ghost[(through_x, 0)].fg,
        plain[(through_x, 0)].fg,
        "the real lane lost its branch colour on a ghost row"
    );
    assert_ne!(
        ghost[(through_x, 0)].fg,
        ghost[(node_x, 0)].fg,
        "a real lane and the anchor beside it must not paint the same"
    );
}

/// A ghost merge (two or more parents) draws the merge anchor `◎`, distinct
/// from the ordinary ghost anchor `○` — a replayed merge still reads as a
/// merge without ever reading as [`NodeKind::Merge`]'s real `◉`.
#[test]
fn ghost_merge_draws_the_merge_anchor_not_the_ordinary_one() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let merge_row = &rows[0]; // id 6: "Merge branch 'b'"
    let line = GraphLine::plain(merge_row).ghost(all_lanes(merge_row));
    assert_eq!(line.node, NodeKind::Merge);
    let buf = render(std::slice::from_ref(&line), &theme, 4);
    assert!(text(&buf, 0).contains('◎'));
    assert!(!text(&buf, 0).contains('◉'));
}

/// Ghost's ASCII glyph table (`: . o O`), distinct from the ASCII table
/// real rows use, so an anchor is unmistakable at that tier too
/// (ADR-0038 §5).
#[test]
fn ghost_row_ascii_glyphs_are_colon_dot_o_capital_o() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let lines: Vec<GraphLine<'_>> = rows
        .iter()
        .map(|row| GraphLine::plain(row).ghost(all_lanes(row)))
        .collect();
    let buf = render_ascii(&lines, &theme, 8);
    let drawn: String = (0..buf.area.height).map(|y| text(&buf, y)).collect();
    for c in drawn.chars() {
        assert!(
            matches!(c, ':' | '.' | 'o' | 'O' | ' ' | '/' | '\\' | '+'),
            "unexpected ASCII glyph on a ghost row: {c:?} in {drawn:?}"
        );
    }
    assert!(drawn.contains('o') || drawn.contains('O'), "{drawn:?}");
}

/// A ghost row's cells all paint `text-muted`, never a branch's own colour —
/// the whole point of an anchor is that it never competes visually with a
/// real commit's lane colour (ADR-0038 §5).
#[test]
fn ghost_row_paints_every_owned_cell_text_muted() {
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    let muted = crate::paint::foreground(&theme, crate::theme::Token::TextMuted)
        .fg
        .expect("text-muted always sets a foreground colour");
    let line = GraphLine::plain(&rows[0]).ghost(all_lanes(&rows[0]));
    let buf = render(std::slice::from_ref(&line), &theme, 4);
    let mut saw_a_coloured_cell = false;
    for x in 0..buf.area.width {
        let cell = &buf[(x, 0)];
        if cell.symbol() != " " {
            saw_a_coloured_cell = true;
            assert_eq!(cell.fg, muted, "cell at x={x} did not paint text-muted");
        }
    }
    assert!(saw_a_coloured_cell, "the row drew nothing at all");
}

/// Not an assertion — dumps a ghost slice, Unicode and ASCII, next to the
/// same rows drawn `Normal`, so a ghost anchor can be told apart from a real
/// parent by eye (ADR-0038 §5, ticket acceptance: "a ghost slice whose
/// parents are clipped"). The first two rows are real; the last two are
/// ghost rows standing in for parents that fell outside the preview slice —
/// clipped, not loaded, drawn as anchors rather than a truncated shape.
/// Run by hand to regenerate: `MWR_UPDATE_SHOTS=1 cargo test -p graft-ui graph_column::tests::capture_for_docs_shots`.
/// What the user is judging in `docs/shots/m5-ui-ghost-rows.txt`.
const GHOST_CAPTION: &str = "\
# docs/shots/m5-ui-ghost-rows.txt — ghost anchor vs real parent
#
# What right looks like: the top rows are real commits — solid
# lines, the node's real shape (●/◉/◇). The bottom two rows are the
# ghost slice: parents clipped at the preview's edge, drawn with a
# hollow anchor (○, or ◎ for a ghost merge) and dashed edges (┊/┈).
# No ghost row may look like a real parent at either tier.
#
# Two cases, because which lanes belong to the preview is the
# caller's to say:
#  - \"the preview owns the row\": every edge on the ghost rows is
#    the preview's, so every edge dashes.
#  - \"a real lane runs through\": only the anchor's own lane is the
#    preview's. The real branch crossing those rows keeps its solid
#    glyph and its colour — DESIGN.md §1.7's `┊│┊●b┈`, so the
#    before and the after read as one picture, not two graphs.
#
# Colour is unverified by this capture (CLAUDE.md rule 5) — the
# shape difference is what a colourless terminal, and NO_COLOR,
# rely on.

";

#[test]
fn capture_for_docs_shots() {
    let mut shots = graft_fixtures::Captures::new();
    let rows = reuse_rows();
    let theme = dark(ColorDepth::TrueColor);
    // The last two rows stand in for real parents that fall outside the
    // preview slice — a ghost anchor, not a partial real glyph.
    let clip_from = rows.len() - 2;
    let clipped = |whole_row: bool| -> Vec<GraphLine<'_>> {
        rows.iter()
            .enumerate()
            .map(|(index, row)| {
                if index < clip_from {
                    GraphLine::plain(row)
                } else if whole_row {
                    GraphLine::plain(row).ghost(all_lanes(row))
                } else {
                    GraphLine::plain(row).ghost(BTreeSet::from([row.lane]))
                }
            })
            .collect()
    };

    let mut out = String::from(GHOST_CAPTION);
    for (case, whole_row) in [
        ("the preview owns the row", true),
        ("a real lane runs through", false),
    ] {
        let lines = clipped(whole_row);
        for (label, ascii) in [("Unicode", false), ("ASCII", true)] {
            out.push_str(&format!("=== {case} — {label} tier, 8 cols ===\n"));
            let buf = if ascii {
                render_ascii(&lines, &theme, 8)
            } else {
                render(&lines, &theme, 8)
            };
            for y in 0..buf.area.height {
                out.push_str(&text(&buf, y));
                out.push('\n');
            }
            out.push('\n');
        }
    }
    shots.check(
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/shots/m5-ui-ghost-rows.txt"
        ),
        out,
    );
    shots.finish();
}

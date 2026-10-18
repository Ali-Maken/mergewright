//! Drawn through a fold (DESIGN.md §3.5, ADR-0022): the rail, stubs in
//! the branch's colour, a folded node on the rail (ease-pass 06b), and
//! dash parity that follows the real column.

use super::*;

/// The twelve merge rows of a twelve-topic history, and the fold at `cap`
/// planned over them and their topic commits: each topic lane holds a node
/// in that window, so it folds rather than drops (ease-pass 06b).
fn wide_merges(cap: usize) -> (Vec<Row>, Fold) {
    let rows = wide_history(12);
    let fold = Fold::plan(cap, &no_pins(), &rows[..24]);
    (rows[..12].to_vec(), fold)
}

fn no_pins() -> mergewright_graph::fold::Keep {
    mergewright_graph::fold::Keep::default()
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
    let (rows, fold) = wide_merges(8);
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
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
    let (rows, fold) = wide_merges(8);
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
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

/// Ease-pass 06b: a node's column can fold over the cap; the node then
/// sits on the rail and shows there, so no row draws without its commit.
#[test]
fn a_node_whose_column_folded_draws_on_the_rail_in_its_place() {
    let rows = wide_history(12);
    let (_, fold) = wide_merges(8);
    let topics = &rows[12..24];
    let lines: Vec<_> = topics.iter().map(GraphLine::plain).collect();
    let buf = render_through(&lines, &fold, &dark(ColorDepth::TrueColor));

    // topic-12's work sits in column 1, which folds: its node is on the
    // rail, its lane down to the root a stub beside it.
    assert_eq!(fold.drawn(topics[0].lane), None, "{:?}", fold.folded());
    assert_eq!(buf[(14, 0)].symbol(), "●", "{:?}", text(&buf, 0));
    assert_eq!(buf[(15, 0)].symbol(), STUB_BELOW.to_string());
    // topic-6's column 7 is kept: its node draws there, the rail stays.
    assert_eq!(fold.drawn(topics[6].lane), Some(1));
    assert_eq!(buf[(2, 6)].symbol(), "●", "{:?}", text(&buf, 6));
    assert_eq!(buf[(14, 6)].symbol(), RAIL.to_string());
}

#[test]
fn a_folded_incoming_edge_stubs_upward() {
    let (rows, fold) = wide_merges(8);
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
fn ascii_folded_lanes_draw_the_rail_and_stubs_from_the_table() {
    let (rows, fold) = wide_merges(8);
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
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
    let (rows, fold) = wide_merges(7);
    let lines: Vec<_> = rows.iter().map(GraphLine::plain).collect();
    // Seven folded: real column 8 draws at position 1, real 9 at position 2.
    assert_eq!(fold.kept(), [0, 8, 9, 10, 11, 12]);
    let buf = render_through(&lines, &fold, &dark(ColorDepth::Palette16));

    // Both lanes pass through the bottom merge row.
    assert_eq!(text(&buf, 11), "◉─│─╎─│─╎─╮ ⋮ ");
    assert_eq!(buf[(2, 11)].symbol(), "│", "real column 8 is even: solid");
    assert_eq!(buf[(4, 11)].symbol(), "╎", "real column 9 is odd: dashed");
}

// --- Ghost Preview (m5-history-and-ghost-preview/04, ADR-0038 §5) ---

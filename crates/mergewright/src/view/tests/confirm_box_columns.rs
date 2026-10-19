//! The confirm box covers whole columns and keeps the graph beside it
//! (ADR-0040 clause 1). Split out of `view::tests` for size (ease-pass 09).

use super::*;

/// ADR-0040 clause 1: the confirm box does **not** clear — the rows beside
/// it are the graph it is about to act on, and seeing them is the M5
/// promise — and its left edge sits on the subject column's own boundary.
///
/// Before this, at 80 columns, the box started at column 8 and left
/// `●   Comm` beside it: the subject column, split. It passed on the right
/// only because "Push main → origin/main" happens to be the length it is.
#[test]
fn the_confirm_box_keeps_the_graph_beside_it_and_starts_on_the_subject_column() {
    for columns in [80u16, 120, 160] {
        let rows = 24;
        let with = draw_cells(&confirm_over_graph(columns, rows), columns, rows);
        let without = draw_cells(&graph_with_no_overlay(columns, rows), columns, rows);
        let rect = drawn_box_rect(&with);

        // Where the subject column starts, read off untouched rows rather
        // than assumed: the leftmost cell any row's content reaches past
        // the graph glyph. A row's own text may start further right — a ref
        // badge pushes the subject along — so it is the minimum across the
        // rows that says where the column itself begins.
        //
        // The **marker slot** sits between the two and is not the subject
        // column: it carries the Mistake Radar's `⚠`/`ⓘ`, the compare base's
        // `▏` and a drop target's `→` (m8-polish-and-ship/07). Counting those
        // made this read 2 instead of 4 and fail — the box had not moved.
        // So they are skipped by name.
        // The graph glyph sits after the two-cell gutter (ADR-0058 §4).
        let past_glyph = usize::from(mergewright_ui::graph_screen::GUTTER) + 1;
        let subject_x = without
            .iter()
            .filter(|row| row.concat().contains("Commit"))
            .filter_map(|row| {
                row.iter().skip(past_glyph).position(|cell| {
                    cell.trim() != "" && !matches!(cell.as_str(), "▏" | "→" | "⚠" | "ⓘ")
                })
            })
            .map(|index| {
                u16::try_from(index + past_glyph).expect("a screen is narrower than u16::MAX")
            })
            .min()
            .expect("the graph draws subjects");

        // Or on the branch column's, one gap and the column's width left of
        // it (ease-pass G5): a box too wide for the message column covers
        // the branch column whole.
        let full = columns;
        let branch_x = subject_x - 1 - mergewright_ui::graph_screen::branch_width_for(full);
        assert!(
            rect.x == subject_x || rect.x == branch_x,
            "{columns}x{rows}: the box starts at column {} and the subject column starts at \
             {subject_x}, the branch column at {branch_x} — the box splits one",
            rect.x
        );
        // It covers rather than clears: on the box's own rows the age column
        // is still drawn whole beside it — or the box runs to the frame's
        // edge and covers that column whole (ADR-0040: whole columns or
        // clear). Either way the lanes left of the box survive, which a
        // cleared band would not.
        let row = &with[usize::from(rect.y) + 1];
        // The rows end at the branch pane's slot (ease-pass 09) where it
        // has room; the box covers rows, never the pane.
        let rows_end =
            columns - mergewright_ui::branch_pane::pane_width(columns).map_or(0, |width| width + 2);
        let from = rect.right().min(rows_end);
        let beside: String = row[usize::from(from)..usize::from(rows_end)].concat();
        let age_survives = beside
            .split_whitespace()
            .last()
            .is_some_and(|age| age.ends_with(['m', 'h', 'd', 'w', 'y']));
        let covers_to_the_edge = rect.right() >= rows_end && beside.trim().is_empty();
        assert!(
            age_survives || covers_to_the_edge,
            "{columns}x{rows}: right of the box is neither the age whole nor the frame's \
             edge: {beside:?}"
        );
        let left_of_box: String = row[..usize::from(rect.x)].concat();
        assert!(
            !left_of_box.trim().is_empty(),
            "{columns}x{rows}: nothing survives left of the box — it cleared instead of \
             covering: {left_of_box:?}"
        );
    }
}

//! The cursor row's fill, cell by cell: where it starts and stops, and the
//! worst contrast on it. Split out of `look_chrome` for size (ease-pass 09).

/// The cursor row `row` of `buffer`: asserts the fill starts right of the
/// gutter and three lanes and stops at the margin or the branch
/// pane, and returns the worst contrast of any character on it against `selection`.
pub(super) fn worst_on_the_fill(
    buffer: &ratatui::buffer::Buffer,
    row: u16,
    selection: mergewright_core::capabilities::Rgb,
    name: &str,
) -> f32 {
    use mergewright_core::capabilities::Rgb;
    use ratatui::style::Color;

    let columns = buffer.area.width;
    let fill = Color::Rgb(selection.red, selection.green, selection.blue);
    let slot = (0..columns)
        .find(|x| buffer[(*x, row)].bg == fill)
        .expect("the cursor row is filled");
    // The gutter and the three lanes sit left of the fill.
    assert!(slot >= 8, "{name}: the fill starts at {slot}");
    // The fill stops at the body's right edge: the margin, or since
    // ease-pass 09 the branch pane's `│` where the margin was.
    let end =
        columns - mergewright_ui::branch_pane::pane_width(columns).map_or(1, |width| width + 2);
    assert_ne!(
        buffer[(end, row)].bg,
        fill,
        "{name}: the margin or the pane's rule"
    );
    let mut worst = f32::INFINITY;
    for x in slot..end {
        let cell = &buffer[(x, row)];
        assert_eq!(cell.bg, fill, "{name}: x {x} is on the fill");
        if cell.symbol().trim().is_empty() {
            continue;
        }
        let Color::Rgb(red, green, blue) = cell.fg else {
            panic!("{name}: x {x} fg {:?}", cell.fg);
        };
        worst = worst.min(mergewright_ui::theme::contrast(
            Rgb { red, green, blue },
            selection,
        ));
    }
    worst
}

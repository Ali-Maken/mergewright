//! `update` tests: pointer. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

#[test]
fn a_click_selects_the_row_under_the_pointer() {
    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 2, // under the cursor row and its second line (ease-pass/07b)
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);

    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(1));
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

#[test]
fn a_click_under_a_harness_banner_lands_on_the_row_the_banner_pushed_down() {
    let id = JobId::new(20);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 7,
    };
    state.harness.banner = Some(Banner {
        title: String::from("gallery 3/23"),
        lines: vec![String::from("shows: a merge")],
    });
    // Three lines; the cursor's second line takes one (ease-pass/07b).
    assert_eq!(state.graph_rows(), 2, "header, two banner rows, footer");

    // The screen row a click on the header would have hit without the
    // banner is now the banner's title: nothing selected.
    let on_banner = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, on_banner);
    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(0));

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 2 + 2, // under the cursor row and its second line (ease-pass/07b)
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id)
    );
}

#[test]
fn the_wheel_scrolls_the_viewport_and_leaves_the_selection_alone() {
    let id = JobId::new(21);
    let (mut state, _) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let scroll_down = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 0,
        row: 0,
        modifiers: Modifiers::NONE,
    }));
    let (down, _) = update(&state, scroll_down);
    assert_eq!(down.history.as_ref().map(|history| history.top), Some(1));
    assert_eq!(down.history.as_ref().map(|history| history.cursor), Some(0));

    let scroll_up = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollUp,
        column: 0,
        row: 0,
        modifiers: Modifiers::NONE,
    }));
    let (up, _) = update(&down, scroll_up);
    assert_eq!(up.history.as_ref().map(|history| history.top), Some(0));
}

/// A left click on the overview strip's column (`overview-strip`,
/// m2-details-and-diffs/20) selects the clicked cell's bucket's first row,
/// same as `History::select_at` — not the ordinary graph click, which the
/// strip's column now shadows.
#[test]
fn a_click_on_the_strip_selects_the_buckets_first_row() {
    let id = JobId::new(23);
    let (mut state, merge_id) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    state.strip_shown = true;
    state.history.as_mut().expect("a walk").rows = 2;

    // Bucket math (`overview_strip::bucket_range`), height 5 over 2 rows:
    // cell 2's range is `1..2` — the merge row — landing on screen row 2
    // (`HEADER_ROWS` + 1).
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 79,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, click);

    assert_eq!(next.history.as_ref().map(|history| history.cursor), Some(1));
    assert_eq!(
        next.history
            .as_ref()
            .and_then(History::selected)
            .map(|row| row.commit.id),
        Some(merge_id),
        "the strip's click landed on the merge row, not the ordinary graph click"
    );
}

/// The strip hides no click from the wheel: `wheel` only special-cases the
/// detail pane's own regions (`detail_hit`), so a notch over the strip's
/// column falls through to the same viewport scroll as anywhere else on the
/// graph (`overview-strip`, m2-details-and-diffs/20).
#[test]
fn the_wheel_over_the_strip_still_scrolls_the_viewport() {
    let id = JobId::new(24);
    let (mut state, _) = merge_on_screen(id, 0);
    state.size = Size {
        columns: 80,
        rows: 5,
    };
    state.strip_shown = true;
    state.history.as_mut().expect("a walk").rows = 2;

    let scroll_down = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 79,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (down, _) = update(&state, scroll_down);
    assert_eq!(down.history.as_ref().map(|history| history.top), Some(1));
}

#[test]
fn hover_names_the_row_under_the_pointer() {
    let id = JobId::new(22);
    let (mut state, _) = merge_on_screen(id, 1);
    state.size = Size {
        columns: 80,
        rows: 5,
    };

    let hover = core(Event::Mouse(MouseInput {
        kind: MouseKind::Moved,
        column: 5,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers::NONE,
    }));
    let (hovered, _) = update(&state, hover);
    assert_eq!(hovered.hover, Some(1), "hover names the row under it");
}

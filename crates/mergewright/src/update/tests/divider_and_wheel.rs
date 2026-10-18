//! `update` tests: divider and wheel. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

// --- Divider drag and wheel routing (m2-details-and-diffs/10b, ADR-0028) ---

#[test]
fn a_click_on_the_divider_starts_a_drag_and_up_ends_it() {
    let (mut state, _) = merge_on_screen(JobId::new(50), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    // The current selection is already "followed", and its evil-merge probe
    // already answered (m2-details-and-diffs/14): no `follow_selection`
    // fetch or probe fires behind the click and confuses the command
    // assertions below.
    state.detail.spec = state.selected_spec();
    if let Some(crate::update::detail::DetailSpec::Commit(oid)) = state.detail.spec {
        state.detail.evil_at = Some(oid);
        state.detail.evil = Loading::Ready(
            mergewright_ops::mistake_radar::probes::EvilVerdict::NotCheckable("not a merge".into()),
        );
    }
    // Right layout, 42% of 200 = 84 columns; the divider sits at column
    // 200 - 84 = 116, the pane's first column.
    let down = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 116,
        row: 10,
        modifiers: Modifiers::NONE,
    }));
    let (dragging, commands) = update(&state, down);
    assert!(dragging.detail.dragging, "the divider column starts a drag");
    assert!(
        commands.is_empty(),
        "no resize yet, just the grab: {commands:?}"
    );
    assert_eq!(
        dragging.history.as_ref().map(|history| history.cursor),
        state.history.as_ref().map(|history| history.cursor),
        "the divider swallows the click; nothing selected under it"
    );

    let drag = core(Event::Mouse(MouseInput {
        kind: MouseKind::Drag(MouseButton::Left),
        column: 100,
        row: 20,
        modifiers: Modifiers::NONE,
    }));
    let (dragged, commands) = update(&dragging, drag.clone());
    assert_eq!(dragged.detail.pane_size.right_columns, Some(100));
    assert_eq!(
        commands,
        vec![Command::SaveLayout(dragged.detail.pane_size)]
    );

    let up = core(Event::Mouse(MouseInput {
        kind: MouseKind::Up(MouseButton::Left),
        column: 100,
        row: 20,
        modifiers: Modifiers::NONE,
    }));
    let (released, _) = update(&dragged, up);
    assert!(!released.detail.dragging);

    // With the drag over, further motion at the same column no longer
    // resizes: the guard is `next.detail.dragging`, not the column.
    let (stayed, commands) = update(&released, drag);
    assert_eq!(stayed.detail.pane_size, released.detail.pane_size);
    assert!(commands.is_empty());
}

#[test]
fn wheel_over_the_files_region_moves_its_cursor_not_the_graph() {
    let (mut state, _) = merge_on_screen(JobId::new(51), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    let (files, list) = two_files();
    state.detail.files = Loading::Ready(files);
    state.detail.list = Some(list);
    state.detail.spec = state.selected_spec();
    let before_top = state.history.as_ref().map(|history| history.top);

    // Right layout: pane starts at column 116, header is 5 rows, so row 6
    // is the files region's first row.
    let wheel = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 150,
        row: 6,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, wheel);
    assert_eq!(next.detail.files, {
        let Loading::Ready(files) = &state.detail.files else {
            unreachable!()
        };
        let mut files = (**files).clone();
        files.move_cursor(WHEEL_LINES);
        Loading::Ready(std::sync::Arc::new(files))
    });
    assert_eq!(
        next.history.as_ref().map(|history| history.top),
        before_top,
        "the graph viewport does not move"
    );
}

#[test]
fn wheel_over_the_pane_never_scrolls_the_graph_underneath_it() {
    let (mut state, _) = merge_on_screen(JobId::new(52), 0);
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;
    state.size = Size {
        columns: 200,
        rows: 50,
    };
    let before_top = state.history.as_ref().map(|history| history.top);

    // Body region: past the (empty) files list, still inside the pane.
    let wheel = core(Event::Mouse(MouseInput {
        kind: MouseKind::ScrollDown,
        column: 150,
        row: 10,
        modifiers: Modifiers::NONE,
    }));
    let (next, _) = update(&state, wheel);
    assert_eq!(
        next.history.as_ref().map(|history| history.top),
        before_top,
        "wheel over the diff body must not fall through to the graph scroll"
    );
}

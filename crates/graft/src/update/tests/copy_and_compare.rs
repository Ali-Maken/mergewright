//! `update` tests: copy and compare. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// `y` on the selected row copies its full SHA and says what was sent
/// (ADR-0023 decision 4).
#[test]
fn y_on_a_selected_row_copies_the_full_sha_and_says_so() {
    let (state, _) = merge_on_screen(JobId::new(31), 0);
    let (copied, commands) = update(&state, key(KeyCode::Char('y')));
    let sha = "01".repeat(20);
    assert_eq!(commands, vec![Command::CopyToClipboard(sha)]);
    assert_eq!(
        copied.status,
        Status::Copied {
            short: "0101010".to_owned()
        }
    );
}

#[test]
fn y_with_no_rows_sends_nothing_and_leaves_the_status_alone() {
    let state = AppState::default();
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.status, Status::Ready);
}

#[test]
fn the_next_key_clears_the_copy_notice() {
    let (state, _) = merge_on_screen(JobId::new(32), 0);
    let (copied, _) = update(&state, key(KeyCode::Char('y')));
    let (moved, _) = update(&copied, key(KeyCode::Char('j')));
    assert_eq!(moved.status, Status::Ready);
}

/// A failure is never dropped for a notice: the SHA is still sent, the
/// failure stays on the status line.
#[test]
fn y_sends_the_sha_but_keeps_a_failure_on_screen() {
    let (mut state, _) = merge_on_screen(JobId::new(33), 0);
    state.status = Status::Failed("the refs could not be read".to_owned());
    let (next, commands) = update(&state, key(KeyCode::Char('y')));
    assert_eq!(commands.len(), 1, "{commands:?}");
    assert_eq!(next.status, state.status);
}

/// `x` on the selected row pins it as a compare base and opens the pane on
/// it (m7-14; was pin-only), focus left on the graph so `j` moves the cursor.
#[test]
fn x_on_a_row_with_no_base_pins_it_and_opens_the_pane() {
    let (state, _) = merge_on_screen(JobId::new(40), 0);
    let plain_id = Oid::from_bytes(&[1; Oid::SHA1_BYTES]).expect("20 bytes is an oid");
    let (next, commands) = update(&state, key(KeyCode::Char('x')));
    assert!(!commands.is_empty(), "the compare on the base loads");
    assert!(next.detail.shown);
    assert_eq!(next.detail.focus, state.detail.focus);
    assert_eq!(
        next.compare_base.map(|base| base.oid),
        Some(plain_id),
        "row 0 is the plain commit"
    );
}

/// m7-14 journey `pr-contents`: pane closed, `x`, `j` — the compare of the
/// base against the new row is on show, no second `x`.
#[test]
fn the_diff_follows_the_cursor_after_x() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(47), 0);
    state.focus = true;
    assert!(!state.detail.shown, "the pane starts closed");
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let base = pinned.compare_base.expect("x pins").oid;
    let (moved, commands) = update(&pinned, key(KeyCode::Char('j')));
    assert!(!commands.is_empty(), "the new compare loads");
    assert!(moved.detail.shown);
    assert_eq!(
        moved.detail.spec,
        Some(DetailSpec::Compare {
            base,
            head: merge_id
        })
    );
    let (back, _) = update(&moved, key(KeyCode::Char('k')));
    assert_eq!(
        back.detail.spec,
        Some(DetailSpec::Compare { base, head: base }),
        "back on the base row, the pane says it is the base"
    );
}

/// `x` again on the same row clears the pinned base.
#[test]
fn x_on_the_same_row_again_clears_the_base() {
    let (state, _) = merge_on_screen(JobId::new(41), 0);
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let (cleared, commands) = update(&pinned, key(KeyCode::Char('x')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(cleared.compare_base, None);
}

/// `x` on a second row opens the compare and keeps the base pinned, so the
/// pane can keep following the cursor.
#[test]
fn x_on_a_different_row_opens_the_compare_and_keeps_the_base() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(42), 0);
    state.focus = true;
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let base_id = pinned.compare_base.map(|base| base.oid);

    let (moved, _) = update(&pinned, key(KeyCode::Char('j')));
    // m7-14: the pane is already open (first `x`), so `j` has loaded this
    // compare; the second `x` names what is shown and asks for nothing new.
    let (opened, _) = update(&moved, key(KeyCode::Char('x')));

    assert_eq!(opened.compare_base.map(|base| base.oid), base_id);
    assert!(opened.detail.shown);
    assert_eq!(
        opened.detail.spec,
        Some(DetailSpec::Compare {
            base: base_id.expect("pinned above"),
            head: merge_id,
        })
    );
}

/// `x` on the working-tree row is a no-op: a base can only be a commit.
#[test]
fn x_on_the_working_tree_row_is_a_no_op() {
    let (mut state, _) = merge_on_screen(JobId::new(43), 0);
    state.on_working_tree = true;
    let (next, commands) = update(&state, key(KeyCode::Char('x')));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.compare_base, None);
}

/// `Esc` clears a pinned base and, when the pane is open, closes it in the
/// same press (ADR-0023's existing `Escape` order).
#[test]
fn esc_clears_a_pinned_base_and_closes_the_open_pane() {
    let (state, _) = merge_on_screen(JobId::new(44), 0);
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    let (moved, _) = update(&pinned, key(KeyCode::Char('j')));
    let (opened, _) = update(&moved, key(KeyCode::Char('x')));
    assert!(opened.detail.shown);

    let (closed, _) = update(&opened, key(KeyCode::Escape));
    assert_eq!(closed.compare_base, None);
    assert!(!closed.detail.shown);
}

/// `Shift`+click on a second row does the same as `x`.
#[test]
fn shift_click_on_a_second_row_opens_the_compare() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(45), 0);
    state.size = Size {
        columns: 160,
        rows: 20,
    };
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));
    // m7-14: the first `x` opened the pane; that the click still lands on
    // graph row 1 is what this checks.
    assert!(pinned.detail.shown);
    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    }));
    // m7-14: the first `x` opened the pane; the click lands the compare.
    let (next, _) = update(&pinned, click);
    assert!(next.detail.shown);
    assert_eq!(
        next.detail.spec,
        Some(DetailSpec::Compare {
            base: pinned.compare_base.expect("pinned above").oid,
            head: merge_id,
        })
    );
}

/// `Ctrl`+click does the same as `Shift`+click.
#[test]
fn ctrl_click_on_a_second_row_opens_the_compare() {
    let (mut state, merge_id) = merge_on_screen(JobId::new(46), 0);
    state.size = Size {
        columns: 160,
        rows: 20,
    };
    let (pinned, _) = update(&state, key(KeyCode::Char('x')));

    let click = core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 10,
        row: HEADER_ROWS + 1,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }));
    // m7-14: the first `x` opened the pane; the click lands the compare.
    let (next, _) = update(&pinned, click);
    assert_eq!(
        next.detail.spec,
        Some(DetailSpec::Compare {
            base: pinned.compare_base.expect("pinned above").oid,
            head: merge_id,
        })
    );
}

/// Moving the selection inside the rows on hand asks the history job for
/// nothing: the thread and its fade are worked out from those rows, so a
/// selection change makes no layout or walk call.
#[test]
fn a_selection_change_inside_the_window_asks_for_no_rows() {
    let id = JobId::new(3);
    let (mut state, _) = update(&awaiting(id), rows_for(id, 1, 0..30, 500));
    state.focus = true;
    for code in [KeyCode::Char('j'), KeyCode::Char('j'), KeyCode::Char('k')] {
        let (moved, commands) = update(&state, key(code));
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
        state = moved;
    }
    assert_eq!(state.history.as_ref().map(|h| h.cursor), Some(1));
}

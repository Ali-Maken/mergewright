//! The resolver is modal, and inert below its floor (m6-conflicts/04).

use super::*;

// --------------------------------------------------------- modal, and small

#[test]
fn resolver_is_modal() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");
    let before = state.clone();

    for code in [
        KeyCode::Char('u'),
        KeyCode::Tab,
        KeyCode::Char('/'),
        KeyCode::Char('.'),
        KeyCode::Char('c'),
        KeyCode::Char('a'),
        KeyCode::Char('s'),
        KeyCode::Char('S'),
    ] {
        let (after, commands) = press_for_commands(&before, KeyPress::plain(code));
        if code == KeyCode::Char('c') {
            // `c` is the resolver's own in a merge: with a file it has not
            // read, its door asks the review's read and draws nothing until
            // the answer (ease-pass/05s). Nothing leaves the resolver.
            assert!(
                matches!(commands.as_slice(), [Command::ReadFinishSummary(_)]),
                "{code:?} queued {commands:?}"
            );
            assert!(after.finish.is_none(), "{code:?} drew a review early");
        } else {
            assert!(commands.is_empty(), "{code:?} queued {commands:?}");
        }
        assert!(after.resolver.is_some(), "{code:?} closed the resolver");
        assert!(after.confirm.is_none(), "{code:?} opened a box");
        assert!(after.search.is_none(), "{code:?} opened the search field");
        assert!(after.context_menu.is_none(), "{code:?} opened the menu");
        assert!(!after.stash.shown, "{code:?} opened the stash list");
        assert_eq!(
            after.resolver.as_ref().map(|open| open.path.clone()),
            before.resolver.as_ref().map(|open| open.path.clone()),
            "{code:?} moved the resolver somewhere else"
        );
    }

    // `Ctrl+C` is the one exception: it is not the resolver's to swallow.
    let (after, _) = press_for_commands(
        &before,
        KeyPress {
            code: KeyCode::Char('c'),
            modifiers: graft_core::event::Modifiers::CONTROL,
        },
    );
    assert!(
        after.quit.is_some(),
        "Ctrl+C no longer quits: {:?}",
        after.quit
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn tiny_frame_is_inert() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");
    let state = app.resize(&state, 50, 8);

    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('o')));
    assert!(
        commands.is_empty(),
        "`o` acted on a document nobody can see: {commands:?}"
    );
    assert!(after.resolver.is_some(), "and it is still open");

    let (after, _) = press_for_commands(&state, KeyPress::plain(KeyCode::Escape));
    assert!(after.resolver.is_none(), "Esc is the way out at every size");

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn mouse_uses_the_key_functions() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    // Small enough that the document runs past the bottom of the window:
    // with nothing below the fold there is nothing for the wheel to do.
    let state = app.resize(&state, 80, 14);
    let top = open(&state).top;

    // The wheel moves the window the same three lines the rest of the app
    // scrolls by (`update::mouse::WHEEL_LINES`).
    let state = app.mouse(&state, MouseKind::ScrollDown, 20, 20);
    assert_eq!(open(&state).top, top + 3);
    let state = app.mouse(&state, MouseKind::ScrollUp, 20, 20);
    assert_eq!(open(&state).top, top);

    // A click picks the block under the row it landed on, and **never**
    // reaches the graph behind the resolver. The cursor is put on the last
    // block first and the window wound back to the top, so a click that
    // lands on the first block's rows is a move this test can see. No key
    // walks down past an open block since ease-pass 05 (`n` is the first
    // open clash), so it is put there.
    let mut state = state;
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let state = app.key(&state, KeyCode::Char('g'));
    assert_eq!(open(&state).top, 0, "`g` winds the window back");

    let selected = state.history.as_ref().map(|walk| walk.cursor);
    let mut blocks = std::collections::BTreeSet::new();
    let mut state = state;
    // From row 5: past the banner, a strip of at most three rows and its
    // rule. A click on the list opens another file, which is
    // `update::resolver::tests::a_click_on_a_file_opens_it`'s to check.
    for row in 5..20 {
        state = app.mouse_down(&state, graft_core::event::MouseButton::Left, 20, row);
        blocks.insert(open(&state).cursor_block);
        assert_eq!(
            state.history.as_ref().map(|walk| walk.cursor),
            selected,
            "the click on row {row} reached the graph behind the resolver"
        );
    }
    assert!(
        blocks.contains(&Some(0)),
        "no row of the window put the cursor on the block it draws: {blocks:?}"
    );

    drop(state);
    app.close();
    drop(scratch);
}

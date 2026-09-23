//! The per-block takes, the walk, and `⏎` (m6-conflicts/04).

use super::*;

// -------------------------------------------------------------- the takes

#[test]
fn o_takes_ours_on_disk() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);

    assert_eq!(
        on_disk(&repo, "both.txt"),
        "same on both sides\nthe line, as we changed it\nand a tail\n",
        "`o` wrote our side, markers and all removed"
    );
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "and the document on screen has no open block left"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "took yours · main in both.txt · 0 of 1 open"),
        "{:?}",
        state.status
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn t_and_b_take() {
    for (key, want) in [
        (
            't',
            "same on both sides\nthe line, as they changed it\nand a tail\n",
        ),
        (
            'b',
            "same on both sides\nthe line, as we changed it\nthe line, as they changed it\nand a tail\n",
        ),
    ] {
        let (scratch, app, state) = stopped(Shape::StoppedMerge);
        let repo = scratch.path().to_owned();
        let state = resolver_on(&app, state, "both.txt");
        let state = app.key(&state, KeyCode::Char(key));
        let state = settled(&app, state);

        assert_eq!(
            on_disk(&repo, "both.txt"),
            want,
            "`{key}` wrote the wrong thing"
        );
        drop(state);
        app.close();
        drop(scratch);
    }
}

#[test]
#[allow(
    non_snake_case,
    reason = "`n` and `N` are two different keys, and the test is named for them"
)]
fn n_and_N_walk_open_blocks_only() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    assert_eq!(open(&state).cursor_block, Some(0));
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        3,
        "three.txt has three blocks"
    );

    // Take block 0, then walk: `n` never stops on a block already taken.
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).cursor_block, Some(1));
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).cursor_block, Some(2));

    // Backwards from block 2: block 0 is taken, so `N` twice stops at 1 and
    // then refuses rather than landing on it.
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(open(&state).cursor_block, Some(1));
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(open(&state).cursor_block, Some(1), "0 is taken: no move");
    let Status::Sticky { reason, keys } = &state.status else {
        panic!("`N` past the first open block said {:?}", state.status)
    };
    assert_eq!(reason, "no more conflicts");
    assert_eq!(keys.as_deref(), Some("⏎ marks three.txt resolved"));

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn n_past_the_last_block_opens_the_next_conflicted_file() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    // Straight past all three, from the first block: two `n` reach block 2,
    // the third has nothing left in this file.
    let state = app.key(&state, KeyCode::Char('n'));
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).cursor_block, Some(2));
    let state = app.key(&state, KeyCode::Char('n'));
    let state = settled(&app, state);

    assert_eq!(
        open(&state).path,
        Path::new("zdiff3.txt"),
        "the next conflicted path in status order"
    );
    assert_eq!(
        open(&state).opened_from,
        OpenedFrom::Pane,
        "and it keeps where it was opened from"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_refuses_while_open() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");

    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(_, _))),
        "`⏎` staged a file with conflicts still in it: {commands:?}"
    );
    let Status::Sticky { reason, keys } = &after.status else {
        panic!("`⏎` said {:?}", after.status)
    };
    assert_eq!(reason, "3 conflicts still open in three.txt");
    assert_eq!(keys.as_deref(), Some("n next"));

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_stages_and_advances() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    // Take all three, then mark it resolved.
    let mut state = state;
    for block in 0..3 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(&app, state);
        // Not after the last one: `n` with nothing open left in this file
        // opens the next conflicted file, which is what this `⏎` is for.
        if block < 2 {
            state = app.key(&state, KeyCode::Char('n'));
        }
    }
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "every block taken"
    );

    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_some(), "`⏎` queued the stage");
    let state = app.pump(state, "the stage and the next file", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_none_or(|open| open.path != Path::new("three.txt"))
    });

    assert_eq!(
        open(&state).path,
        Path::new("zdiff3.txt"),
        "on to the next conflicted file"
    );

    drop(state);
    app.close();
    drop(scratch);
}

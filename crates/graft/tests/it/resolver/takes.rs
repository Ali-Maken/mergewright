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
        matches!(&state.status, Status::Done { what: line, .. } if line == "took yours · main in both.txt · 0 of 1 open"),
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
fn n_and_N_walk_every_block() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    assert_eq!(open(&state).cursor_block, Some(0));
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        3,
        "three.txt has three blocks"
    );

    // Take block 0: the cursor moves on to block 1 by itself
    // (m6-conflicts/15), and `N` goes back to the block just taken, so a
    // take can be fixed by keyboard.
    let state = app.key(&state, KeyCode::Char('o'));
    assert_eq!(open(&state).cursor_block, Some(1), "on at the keypress");
    let state = settled(&app, state);
    assert_eq!(open(&state).cursor_block, Some(1), "and still there after");
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(
        open(&state).cursor_block,
        Some(0),
        "N reaches a taken block"
    );
    let state = app.key(&state, KeyCode::Char('n'));
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).cursor_block, Some(2));

    drop(state);
    app.close();
    drop(scratch);
}

/// `t t t` on `three.txt` before any answer lands: one take out, two
/// queued behind it, all three written in order — the bytes on disk are
/// git's own `checkout --theirs` (m6-conflicts/15).
#[test]
fn a_queued_take_writes_what_git_would() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "three.txt");
    // `app.key` sends each key's commands to the pool but lands no answer:
    // answers are taken only by `pump`. So all three are pressed before
    // the first take has answered.
    let state = app.key(&state, KeyCode::Char('t'));
    let state = app.key(&state, KeyCode::Char('t'));
    let state = app.key(&state, KeyCode::Char('t'));
    assert!(open(&state).in_flight.is_some(), "one take out");
    assert_eq!(open(&state).queued.len(), 2, "the rest queued");
    let state = app.pump(state, "every queued take", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.job.is_none() && open.queued.is_empty())
    });
    assert_eq!(open(&state).file.as_ref().expect("a document").open(), 0);
    assert_eq!(
        on_disk(&repo, "three.txt"),
        git_theirs(&repo, "three.txt"),
        "three queued takes wrote what git's --theirs does"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// What `git checkout --theirs` writes for `path`, from a copy of the
/// repository, so the one under test is left as it is.
fn git_theirs(repo: &Path, path: &str) -> String {
    let copy = tempfile::tempdir().expect("a scratch directory");
    let status = std::process::Command::new("cp")
        .args(["-R", "."])
        .arg(copy.path())
        .current_dir(repo)
        .status()
        .expect("cp runs");
    assert!(status.success(), "copying the repository");
    crate::app_harness::git(copy.path(), &["checkout", "--theirs", "--", path]);
    on_disk(copy.path(), path)
}

/// `three.txt` after one `t`: block 1 took theirs, the cursor already on
/// block 2 — the captures the user judges by eye (m6-conflicts/15).
#[test]
fn advance_capture() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    let state = app.key(&state, KeyCode::Char('t'));
    let state = settled(&app, state);
    assert_eq!(open(&state).cursor_block, Some(1), "{:?}", state.status);
    for (columns, rows) in [(120u16, 40u16), (187, 50)] {
        let frame = crate::captures::frame(&state, columns, rows);
        let body = frame.join("\n");
        assert!(
            body.contains("▸ three.txt · 3 conflicts · 2 open"),
            "{body}"
        );
        // The card's header, or the result pane's rule in the three panes.
        let header = if columns < 180 {
            "conflict 1 of 3 · took theirs · feature"
        } else {
            "── 1 of 3 · took theirs · feature"
        };
        assert!(body.contains(header), "{body}");
        crate::captures::check_shot(
            &format!("m6/fix-advance-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
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
            .any(|command| matches!(command, Command::Operate(..))),
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
    // Each take moves the cursor on by itself (m6-conflicts/15).
    for _ in 0..3 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(&app, state);
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

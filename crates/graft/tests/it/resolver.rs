//! The conflict resolver, against a real stopped repository
//! (m6-conflicts/04).
//!
//! The fixture is `Shape::StoppedMerge`/`StoppedRebase`: **ours** is `main`
//! and **theirs** is `feature` in both, so a line named for a side means the
//! same thing whichever stop the test picked.

#![allow(
    clippy::expect_used,
    reason = "a test that cannot reach its own fixture has already failed"
)]

use std::path::Path;

use graft::update::resolver::OpenedFrom;
use graft::update::{AppState, Command, GraphOverlay, Status};
use graft_core::event::{KeyCode, KeyPress, MouseKind};
use graft_fixtures::{Scratch, Shape};

use crate::app_harness::{App, changes_pane, cursor_to, press_for_commands};

/// A repository stopped mid-operation, its first status read landed.
fn stopped(shape: Shape) -> (Scratch, App, AppState) {
    let scratch = graft_fixtures::copy_into_temp(shape);
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped operation", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    (scratch, app, state)
}

/// The banner has the keyboard, as it does the moment a stop is discovered.
fn on_the_banner(app: &App, state: AppState) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while !state.banner_focus {
        assert!(guard < 6, "Tab never came round to the banner");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    state
}

/// The changes pane, with the cursor on `path`'s row.
fn on_the_row(app: &App, state: AppState, path: &str) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while state.banner_focus {
        assert!(guard < 6, "Tab never let the banner go");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    let state = changes_pane(app, state);
    cursor_to(app, state, path)
}

/// The resolver open on `path`, its first read landed.
fn resolver_on(app: &App, state: AppState, path: &str) -> AppState {
    let state = on_the_row(app, state, path);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the conflict file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some())
    })
}

/// The open resolver. Panics if it is closed, which is never what a test
/// that calls this meant.
fn open(state: &AppState) -> &graft::update::resolver::ResolverState {
    state.resolver.as_ref().expect("the resolver is open")
}

/// Waits for the take or the read in flight to land.
fn settled(app: &App, state: AppState) -> AppState {
    app.pump(state, "the job the resolver started", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none())
    })
}

/// The file's bytes as they stand on disk.
fn on_disk(repo: &Path, name: &str) -> String {
    std::fs::read_to_string(repo.join(name)).expect("the conflicted file")
}

// ------------------------------------------------------------ opening it

#[test]
fn enter_on_conflicts_row_opens_the_resolver() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_row(&app, state, "both.txt");

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    let [Command::OpenConflict(_, path)] = commands.as_slice() else {
        panic!("`⏎` on a CONFLICTS row queued {commands:?}")
    };
    assert_eq!(path, Path::new("both.txt"));

    let state = app.key(&state, KeyCode::Enter);
    let state = settled(&app, state);
    assert_eq!(state.graph_overlay(), Some(GraphOverlay::Resolver));
    assert_eq!(
        open(&state).cursor_block,
        Some(0),
        "on the first open block"
    );
    assert_eq!(open(&state).opened_from, OpenedFrom::Pane);

    // The bar under the document is the resolver's own, not the pane's.
    let drawn = crate::captures::frame(&state, 120, 40);
    let footer = drawn.last().expect("a footer row").to_owned();
    for key in ["o ours", "t theirs", "b both", "n next", "esc close"] {
        assert!(footer.contains(key), "{key} missing from {footer:?}");
    }

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_on_banner_opens_and_esc_returns() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_banner(&app, state);

    let state = app.key(&state, KeyCode::Enter);
    let opened = open(&state);
    assert_eq!(opened.opened_from, OpenedFrom::Banner);
    assert_eq!(
        opened.path,
        Path::new("added-by-them.txt"),
        "the banner opens the first conflicted path"
    );

    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closes it");
    assert!(state.banner_focus, "and gives the banner its keyboard back");
    assert!(
        state.in_progress.is_some(),
        "Esc never aborts what stopped (§2.6)"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_refusals_in_words() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");

    // Already open: `⏎` inside the resolver means "mark resolved", so the
    // second open can only come from the code, which is what this asserts.
    let mut commands = Vec::new();
    let mut next = state.clone();
    let mut raw = 9_001;
    let opened = graft::update::resolver::open_resolver(
        &mut next,
        &mut commands,
        "three.txt".into(),
        OpenedFrom::Pane,
        &mut || {
            raw += 1;
            graft_core::job::JobId::new(raw)
        },
    );
    assert!(opened, "the refusal is still an answer");
    assert!(commands.is_empty(), "and it queued nothing");
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "the resolver is already open"),
        "{:?}",
        next.status
    );
    assert_eq!(open(&next).path, Path::new("both.txt"), "still the first");

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn whole_file_refused_in_words_for_now() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_row(&app, state, "theirs-deleted.txt");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the whole-file refusal", |state| {
        state.resolver.is_none() && !matches!(state.status, Status::Ready)
    });

    let Status::Sticky { reason, keys } = &state.status else {
        panic!("a whole-file conflict said {:?}", state.status)
    };
    assert!(
        reason.contains("binary and whole-file conflicts"),
        "{reason:?}"
    );
    assert_eq!(keys.as_deref(), Some("M mergetool  e edit"));

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn enter_elsewhere_unchanged() {
    // `both.txt` is staged before the app ever opens, so git stops calling
    // it unmerged: the same row, in the same pane, with none of the
    // resolver's claim on it.
    let scratch = graft_fixtures::copy_into_temp(Shape::StoppedMerge);
    crate::app_harness::git(scratch.path(), &["add", "both.txt"]);
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped operation", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = on_the_row(&app, state, "both.txt");
    assert_eq!(
        state.detail.focus,
        graft_ui::detail::DetailFocus::Files,
        "the cursor is in the file list"
    );

    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    assert!(after.resolver.is_none(), "no resolver on a resolved row");
    assert!(commands.is_empty(), "and nothing was read for one");
    assert_eq!(
        after.detail.focus,
        graft_ui::detail::DetailFocus::Diff,
        "`⏎` keeps the meaning it has everywhere else: focus the diff"
    );

    drop(state);
    app.close();
    drop(scratch);
}

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
        matches!(&state.status, Status::Done(line) if line == "took ours in both.txt · 0 of 1 open"),
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

// ------------------------------------------------- handing the file over

#[test]
fn e_hands_over_and_rereads() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    let state = app.key(&state, KeyCode::Char('n'));
    let block = open(&state).cursor_block.expect("a block under the cursor");

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let [Command::OpenExternalEditor { path, line }] = commands.as_slice() else {
        panic!("`e` queued {commands:?}")
    };
    assert_eq!(path, Path::new("three.txt"));
    let want = open(&state)
        .file
        .as_ref()
        .expect("a document")
        .blocks()
        .get(block)
        .expect("the cursor's block")
        .first_line;
    assert_eq!(
        *line,
        Some(u32::try_from(want).expect("a line number")),
        "`e` opens at the cursor block's own marker line"
    );

    // Back from the editor: the document on screen is not what is on disk
    // any more, whoever wrote it, so it is read again.
    let (_, commands) = graft::update::update(
        &state,
        graft::update::AppEvent::Produced(
            graft_core::job::JobId::new(9_100),
            graft::update::Product::HandedOver(Ok(())),
        ),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if path == Path::new("three.txt"))),
        "the hand-back queued no re-read: {commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree)),
        "and the CONFLICTS list behind it is as stale as the document was"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn resolved_outside_closes() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");

    // What `$EDITOR` or a mergetool would have done while the resolver sat
    // there: the markers gone and the path staged, so git no longer calls
    // it unmerged.
    std::fs::write(
        repo.join("both.txt"),
        "same on both sides\nsomething else entirely\nand a tail\n",
    )
    .expect("writing the file");
    crate::app_harness::git(&repo, &["add", "both.txt"]);

    // Any take now finds the file changed under it, throws the document
    // away and reads it again — and the read comes back "not conflicted".
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the re-read after the outside edit", |state| {
        state.resolver.is_none()
    });
    assert!(
        matches!(&state.status, Status::Done(line) if line == "both.txt resolved outside"),
        "{:?}",
        state.status
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
#[allow(
    non_snake_case,
    reason = "the key is `M`, and the test is named for it"
)]
fn M_hands_over() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both.txt");

    // No `merge.tool` in the fixture: the banner's own refusal, reused.
    let (after, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('M')));
    assert!(commands.is_empty(), "a refusal hands nothing over");
    let Status::Sticky { reason, .. } = &after.status else {
        panic!("`M` with no tool said {:?}", after.status)
    };
    assert!(reason.contains("no merge.tool configured"), "{reason:?}");

    let mut configured = state.clone();
    configured.mergetool = Some("vimdiff".to_owned());
    let (_, commands) = press_for_commands(&configured, KeyPress::plain(KeyCode::Char('M')));
    assert!(
        matches!(commands.as_slice(), [Command::OpenMergetool { path }] if path == Path::new("both.txt")),
        "{commands:?}"
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn changed_on_disk_rereads_once() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");

    // Something else writes the file between the read and the take.
    std::fs::write(
        repo.join("both.txt"),
        "same on both sides\n<<<<<<< HEAD\nrewritten\n=======\nby someone else\n>>>>>>> feature\nand a tail\n",
    )
    .expect("writing the file");

    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the refused take", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.notice.is_some())
    });
    let (words, problem) = open(&state).notice.clone().expect("a notice");
    assert!(words.contains("changed"), "{words:?}");
    assert!(problem, "a take that did not happen is a problem");
    assert!(
        open(&state).job.is_some(),
        "and the document is read again rather than written over"
    );

    let state = settled(&app, state);
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        1,
        "the re-read shows what is on disk now"
    );

    drop(state);
    app.close();
    drop(scratch);
}

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
        assert!(commands.is_empty(), "{code:?} queued {commands:?}");
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
    // lands on the first block's rows is a move this test can see.
    let state = app.key(&state, KeyCode::Char('n'));
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).cursor_block, Some(2));
    let state = app.key(&state, KeyCode::Char('g'));
    assert_eq!(open(&state).top, 0, "`g` winds the window back");

    let selected = state.history.as_ref().map(|walk| walk.cursor);
    let mut blocks = std::collections::BTreeSet::new();
    let mut state = state;
    for row in 0..14 {
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

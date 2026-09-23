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

use graft::jobs::Dispatch;
use graft::update::resolver::OpenedFrom;
use graft::update::{AppState, Command, GraphOverlay, Status};
use graft_core::event::{KeyCode, KeyPress, MouseKind};
use graft_fixtures::{Scratch, Shape};
use graft_git::status::UnmergedHow;
use graft_ops::operation::{Operation, ResolveWholeParams, Side};

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

/// The resolver open on `path`, its first read landed — a whole-file shape
/// (`file` stays `None` forever; [`resolver_on`] waits for it and hangs).
fn resolver_on_whole_file(app: &App, state: AppState, path: &str) -> AppState {
    let state = on_the_row(app, state, path);
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the whole-file card", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.whole_file.is_some())
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

/// The whole-file card's own key text, per shape and per binary
/// (m6-conflicts/05 scope 1).
#[test]
fn whole_file_table_through_the_keys() {
    for (how, binary, want) in [
        (
            UnmergedHow::DeletedByThem,
            false,
            "o keeps the file  t deletes it",
        ),
        (
            UnmergedHow::DeletedByUs,
            false,
            "o deletes it  t keeps theirs",
        ),
        (UnmergedHow::AddedByUs, false, "o keeps yours  t drops it"),
        (
            UnmergedHow::AddedByThem,
            false,
            "o drops it  t keeps theirs",
        ),
        (UnmergedHow::BothDeleted, false, "o or t removes it"),
        (
            UnmergedHow::BothModified,
            true,
            "o keeps ours  t keeps theirs",
        ),
        (UnmergedHow::BothAdded, true, "o keeps ours  t keeps theirs"),
    ] {
        assert_eq!(
            graft::update::resolver::whole_file_keys(how, binary),
            want,
            "{how:?} binary={binary}"
        );
    }
}

/// Every whole-file `(how, side)` row, driven through the real keys on the
/// real fixture: the exact `Operate` command `o`/`t` queue, and what the
/// file is once git has it (m6-conflicts/05 scope 1). `pic.bin` is the
/// binary `UU` row.
#[test]
#[allow(clippy::too_many_lines, reason = "a table of 12 (how, side) rows")]
fn whole_file_ops_per_row() {
    // (path, how, side, the file after: None = gone, Some(bytes) = kept)
    let rows: [(&str, UnmergedHow, Side, Option<&[u8]>); 12] = [
        (
            "theirs-deleted.txt",
            UnmergedHow::DeletedByThem,
            Side::Ours,
            Some(b"changed by us, deleted by them\n"),
        ),
        (
            "theirs-deleted.txt",
            UnmergedHow::DeletedByThem,
            Side::Theirs,
            None,
        ),
        (
            "ours-deleted.txt",
            UnmergedHow::DeletedByUs,
            Side::Ours,
            None,
        ),
        (
            "ours-deleted.txt",
            UnmergedHow::DeletedByUs,
            Side::Theirs,
            Some(b"deleted by us, changed by them\n"),
        ),
        (
            "added-by-us.txt",
            UnmergedHow::AddedByUs,
            Side::Ours,
            Some(b"renamed to a different name by each side\n"),
        ),
        (
            "added-by-us.txt",
            UnmergedHow::AddedByUs,
            Side::Theirs,
            None,
        ),
        (
            "added-by-them.txt",
            UnmergedHow::AddedByThem,
            Side::Ours,
            None,
        ),
        (
            "added-by-them.txt",
            UnmergedHow::AddedByThem,
            Side::Theirs,
            Some(b"renamed to a different name by each side\n"),
        ),
        (
            "both-deleted.txt",
            UnmergedHow::BothDeleted,
            Side::Ours,
            None,
        ),
        (
            "both-deleted.txt",
            UnmergedHow::BothDeleted,
            Side::Theirs,
            None,
        ),
        (
            "pic.bin",
            UnmergedHow::BothModified,
            Side::Ours,
            Some(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR ours\0"),
        ),
        (
            "pic.bin",
            UnmergedHow::BothModified,
            Side::Theirs,
            Some(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR theirs\0"),
        ),
    ];
    for (name, how, side, after) in rows {
        let (scratch, app, state) = stopped(Shape::StoppedMerge);
        let repo = scratch.path().to_owned();
        let state = resolver_on_whole_file(&app, state, name);
        let key = if side == Side::Ours { 'o' } else { 't' };

        let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char(key)));
        let [Command::Operate(_, operation)] = commands.as_slice() else {
            panic!("{name} `{key}` queued {commands:?}")
        };
        assert!(
            matches!(
                operation.as_ref(),
                Operation::ResolveWholeFile(ResolveWholeParams { path, side: s, how: h })
                    if path == Path::new(name) && *s == side && *h == how
            ),
            "{name} `{key}`: {operation:?}"
        );

        let state = app.key(&state, KeyCode::Char(key));
        let state = app.pump(state, "the whole-file resolve", |state| {
            state.operating.is_none()
        });
        assert!(
            !still_unmerged(&repo).lines().any(|line| line == name),
            "{name} `{key}`: still unmerged"
        );
        match after {
            None => assert!(
                !repo.join(name).exists(),
                "{name} `{key}`: file still there"
            ),
            Some(bytes) => {
                let got = std::fs::read(repo.join(name)).expect("the kept file");
                assert_eq!(got, bytes, "{name} `{key}`: wrong side kept");
            }
        }

        drop(state);
        app.close();
        drop(scratch);
    }
}

/// `theirs-deleted.txt` is `UD` (`DeletedByThem`): a whole-file card, not a
/// closed resolver — the shape scope 1 turns from a refusal into a card.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one acceptance box walked end to end, in its own order"
)]
fn deleted_by_them_o_keeps() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");
    {
        let whole = open(&state)
            .whole_file
            .as_ref()
            .expect("a whole-file card, not a closed resolver");
        assert_eq!(whole.how, UnmergedHow::DeletedByThem);
        assert!(!whole.binary);
        assert_eq!(
            whole.lines,
            vec![b"changed by us, deleted by them".to_vec()],
            "the card shows what is on disk"
        );
    }
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter()
            .any(|row| row.contains("deleted by them · o keeps the file  t deletes it")),
        "the card's line is not on the frame:\n{}",
        rows.join("\n")
    );

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('o')));
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("`o` on the card queued {commands:?}")
    };
    assert!(
        matches!(
            operation.as_ref(),
            Operation::ResolveWholeFile(ResolveWholeParams {
                path,
                side: Side::Ours,
                how: UnmergedHow::DeletedByThem,
            }) if path == Path::new("theirs-deleted.txt")
        ),
        "{operation:?}"
    );

    let state = app.key(&state, KeyCode::Char('o'));
    assert!(state.operating.is_some(), "{:?}", state.status);
    let state = app.pump(state, "the resolve and the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path != Path::new("theirs-deleted.txt")
                    && open.job.is_none()
                    && (open.file.is_some() || open.whole_file.is_some())
            })
    });

    assert_eq!(
        on_disk(&repo, "theirs-deleted.txt"),
        "changed by us, deleted by them\n",
        "`o` kept our side, staged"
    );
    assert!(
        !still_unmerged(&repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "git no longer calls it unmerged: {:?}",
        still_unmerged(&repo)
    );
    assert!(
        still_unmerged(&repo)
            .lines()
            .any(|line| Path::new(line) == open(&state).path),
        "the resolver moved on to another conflicted file: {:?}",
        open(&state).path
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn deleted_by_them_t_deletes() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");

    let state = app.key(&state, KeyCode::Char('t'));
    assert!(
        state.operating.is_some(),
        "`t` on a whole-file card queued no write: {:?}",
        state.status
    );
    let state = app.pump(state, "the whole-file resolve", |state| {
        state.operating.is_none()
    });

    assert!(
        !repo.join("theirs-deleted.txt").exists(),
        "`t` removed the file theirs never had"
    );
    assert_eq!(
        crate::app_harness::git(&repo, &["ls-files", "--", "theirs-deleted.txt"]).trim(),
        "",
        "and the index holds no entry for it at any stage"
    );
    assert!(
        !still_unmerged(&repo)
            .lines()
            .any(|line| line == "theirs-deleted.txt"),
        "git no longer calls it unmerged: {:?}",
        still_unmerged(&repo)
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// `b` has no meaning on a whole-file card: there are no blocks to keep both
/// sides of.
#[test]
fn b_refused_on_whole_file() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");

    let state = app.key(&state, KeyCode::Char('b'));
    assert!(state.operating.is_none(), "`b` wrote nothing");
    let Status::Sticky { reason, keys } = &state.status else {
        panic!("`b` on a whole-file card said {:?}", state.status)
    };
    assert_eq!(reason, "no both for a deleted by them file");
    assert_eq!(keys.as_deref(), Some("o ours  t theirs"));
    assert!(open(&state).whole_file.is_some(), "the card is still up");

    drop(state);
    app.close();
    drop(scratch);
}

/// The file's own `Block`s, in order — `ConflictFile::blocks()` answers
/// only position and state (`BlockAt`), not the `base`/`reproduced` fields a
/// base test needs, which live on the `Block` each region carries.
fn conflict_blocks(file: &graft_git::conflict::ConflictFile) -> Vec<&graft_git::conflict::Block> {
    file.regions
        .iter()
        .filter_map(|region| match region {
            graft_git::conflict::Region::Conflict(block)
            | graft_git::conflict::Region::Resolved { block, .. } => Some(block),
            graft_git::conflict::Region::Plain(_) => None,
        })
        .collect()
}

// --------------------------------------------------------------- B: base

/// `B` loads and shows the merged common ancestor inside every open block;
/// a second `B` hides it again, for free — no job (m6-conflicts/05 scope 2).
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `B`, and the test is named for it"
)]
fn B_shows_and_hides_base() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    assert!(!open(&state).base_shown, "not shown until asked");

    let state = app.key(&state, KeyCode::Char('B'));
    assert!(
        open(&state).base_job.is_some(),
        "`B` queued no load: {:?}",
        state.status
    );
    let state = app.pump(state, "the base load", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.base_job.is_none())
    });

    assert!(open(&state).base_shown, "`B` showed it");
    let file = open(&state).file.as_ref().expect("a document");
    for block in conflict_blocks(file) {
        assert!(
            block.reproduced.is_some(),
            "every open block of three.txt has a base attached: {block:?}"
        );
    }
    assert!(
        matches!(&state.status, Status::Done(line) if line == "base shown for 3 conflicts"),
        "{:?}",
        state.status
    );
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter().any(|row| row.contains("BASE")),
        "no BASE row drawn at 120 columns:\n{}",
        rows.join("\n")
    );

    let state = app.key(&state, KeyCode::Char('B'));
    assert!(!open(&state).base_shown, "the second `B` hides it");
    assert!(open(&state).base_job.is_none(), "hiding queues no job");
    let file = open(&state).file.as_ref().expect("a document");
    assert!(
        conflict_blocks(file)[0].reproduced.is_some(),
        "hiding keeps the base attached, only base_shown flips"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// `both-added.txt` is `AA`: two stages, no merge base for git's own `diff3`
/// to reproduce — `B` says so in words rather than drawing nothing.
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `B`, and the test is named for it"
)]
fn B_says_no_base() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "both-added.txt");

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('B')));
    assert!(
        commands.is_empty(),
        "an `AA` file has no stage 1: no base load to start: {commands:?}"
    );
    let state = app.key(&state, KeyCode::Char('B'));

    assert!(open(&state).base_job.is_none(), "no job");
    assert!(!open(&state).base_shown, "nothing was attached");
    let (words, problem) = open(&state).notice.clone().expect("a notice");
    assert_eq!(words, "no base (added on both sides)");
    assert!(
        !problem,
        "an `AA` file having no ancestor is a fact of the file, not a failure (planner ruling)"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// A re-read after the `e` hand-over carries no attached bases:
/// `open_conflict` never calls `attach_base`, only `B` does — and `B` then
/// loads them again (m6-conflicts/05 scope 2).
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one acceptance box walked end to end, in its own order"
)]
fn reread_drops_base() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = resolver_on(&app, state, "three.txt");
    let load_base = |app: &App, state: AppState| {
        let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('B')));
        assert!(
            matches!(commands.as_slice(), [Command::LoadBase(_, path, _)] if path == Path::new("three.txt")),
            "`B` queued {commands:?}"
        );
        let state = app.key(&state, KeyCode::Char('B'));
        app.pump(state, "the base load", |state| {
            state
                .resolver
                .as_ref()
                .is_some_and(|open| open.base_job.is_none())
        })
    };
    let state = load_base(&app, state);
    assert!(open(&state).base_shown, "shown before the re-read");

    // `e`: the editor itself is not run here (a test has no terminal to
    // hand over); its command is checked, then its answer is fed back the
    // way the real loop feeds it, and what that queues is run for real.
    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        matches!(commands.as_slice(), [Command::OpenExternalEditor { path, .. }] if path == Path::new("three.txt")),
        "`e` queued {commands:?}"
    );
    let (state, commands) = graft::update::update(
        &state,
        graft::update::AppEvent::Produced(
            graft_core::job::JobId::new(9_200),
            graft::update::Product::HandedOver(Ok(())),
        ),
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(_, path) if path == Path::new("three.txt"))),
        "the hand-back queued no re-read: {commands:?}"
    );
    for command in commands {
        app.jobs.dispatch(command);
    }
    let state = app.pump(state, "the re-read", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.job.is_none() && open.file.is_some())
    });

    assert!(!open(&state).base_shown, "a fresh read carries no base");
    let file = open(&state).file.as_ref().expect("a document");
    for block in conflict_blocks(file) {
        assert!(
            block.reproduced.is_none(),
            "and nothing was attached to reproduce a base from: {block:?}"
        );
    }

    // `B` again loads them again.
    let state = load_base(&app, state);
    assert!(open(&state).base_shown, "`B` reloads after the re-read");
    let file = open(&state).file.as_ref().expect("a document");
    for block in conflict_blocks(file) {
        assert!(block.reproduced.is_some(), "reloaded: {block:?}");
    }

    drop(state);
    app.close();
    drop(scratch);
}

// ---------------------------------------------------------------- d: restart

/// `d` raises a confirm box; answering it runs `checkout --merge` and the
/// file comes back with all its markers (m6-conflicts/05 scope 3).
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `d`, and the test is named for it"
)]
#[allow(
    clippy::too_many_lines,
    reason = "one acceptance box walked end to end, in its own order"
)]
fn d_starts_over_behind_a_box() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "both.txt's one block is taken"
    );

    let state = app.key(&state, KeyCode::Char('d'));
    let confirm = state.confirm.as_ref().expect("`d` raised a box");
    assert_eq!(confirm.view.title.as_str(), "Start both.txt over?");
    assert!(
        confirm
            .view
            .message
            .iter()
            .any(|line| line.as_str().contains("comes back with all its markers")),
        "{:?}",
        confirm.view.message
    );
    let message: Vec<&str> = confirm
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(
        message,
        vec![
            "the conflict comes back with all its markers",
            "takes and edits in this file are lost · not undoable while the merge is open",
        ]
    );
    let choices: Vec<&str> = confirm
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(choices, vec!["Cancel", "Start over"]);
    assert_eq!(
        confirm.view.command.as_str(),
        "git checkout --merge -- both.txt"
    );

    // `→` off `Cancel` onto `Start over`, then `Enter` runs it.
    let state = app.key(&state, KeyCode::Right);
    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("`Start over` queued {commands:?}")
    };
    assert!(
        matches!(
            operation.as_ref(),
            Operation::RestartConflict(graft_ops::operation::RestartParams { path, .. })
                if path == Path::new("both.txt")
        ),
        "{operation:?}"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none())
    });

    assert!(state.confirm.is_none(), "the box is gone");
    assert_eq!(open(&state).path, Path::new("both.txt"), "the same file");
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        1,
        "the take is gone: the block is open again"
    );
    assert!(
        on_disk(&repo, "both.txt").contains("<<<<<<<"),
        "the markers are back on disk"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// `Esc` (or `←` onto `Cancel`, then `Enter`) answers the box for free: no
/// write, the take stands.
#[test]
#[allow(
    non_snake_case,
    reason = "the key is `d`, and the test is named for it"
)]
fn d_cancel_is_free() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    let state = resolver_on(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);

    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some());
    let state = app.key(&state, KeyCode::Escape);

    assert!(state.confirm.is_none(), "`Esc` answers the box");
    assert!(state.operating.is_none(), "no write was launched");
    assert_eq!(
        open(&state).file.as_ref().expect("a document").open(),
        0,
        "the take still stands"
    );
    assert_eq!(
        on_disk(&repo, "both.txt"),
        "same on both sides\nthe line, as we changed it\nand a tail\n",
        "nothing on disk moved"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// The journal on disk, as the app's own reader sees it.
fn journal_entries(repo: &Path) -> Vec<graft_core::journal::JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the merge just wrote is readable")
        .entries()
        .to_vec()
}

/// A whole-file `o`/`t` and a `d` restart both run *inside* the merge's own
/// live operation (`apply::resume::inside_live_operation`) rather than as a
/// standalone write: neither adds a journal entry of its own — the merge's
/// one `Pending` entry is what `u` still undoes (m6-conflicts/05 scope 1, 3).
#[test]
fn whole_file_and_restart_fold_into_live_entry() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let repo = scratch.path().to_owned();
    // The fixture built the merge with raw git, never through the app, so
    // there is no journal entry for the merge itself here — only whether a
    // whole-file resolve or a restart adds one of its own is this test's,
    // and that is `before` either way.
    let before = journal_entries(&repo).len();

    // A whole-file `o`.
    let state = resolver_on_whole_file(&app, state, "theirs-deleted.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the whole-file resolve", |state| {
        state.operating.is_none()
    });
    assert_eq!(
        journal_entries(&repo).len(),
        before,
        "no new entry for the whole-file resolve"
    );

    // The resolve advanced to whatever conflicted file is next in the
    // list, keeping the resolver's modal keyboard — `Esc` gives the pane
    // back, already open on the files list (so `cursor_to` alone finds
    // `both.txt`; `on_the_row`/`changes_pane` assume a fresh graph-top
    // start, which this is not).
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "closed, back to the pane");
    let state = app.pump(state, "the files list after the resolve", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });

    // A `d` restart on a block take.
    let state = cursor_to(&app, state, "both.txt");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the conflict file", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some())
    });
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the restart", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none())
    });
    assert_eq!(
        journal_entries(&repo).len(),
        before,
        "no new entry for the restart either"
    );

    drop(state);
    app.close();
    drop(scratch);
}

// -------------------------------------------------- stash-pop conflict path

/// `KNOWN_GAPS.md` row 229: a conflicted `git stash pop` leaves an unmerged
/// index with no operation and so no banner — the resolver still opens on
/// it, names the sides in its own words, and the last file closes it with
/// no `c continue` offered (m6-conflicts/05 scope 4).
#[test]
fn stash_pop_conflict_resolves_without_a_banner() {
    let scratch = stash_pop_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");

    let state = resolver_on(&app, state, "sample.txt");
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter()
            .any(|row| row.contains("ours = your working tree")
                && row.contains("theirs = the stash")),
        "the stash-pop explainer is not on the frame:\n{}",
        rows.join("\n")
    );

    // The take journals nothing; `⏎` is an ordinary journaled `Stage` —
    // no live operation to fold into (planner ruling, 2026-09-23).
    let before = journal_entries(scratch.path()).len();
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(&app, state);
    }
    assert_eq!(
        journal_entries(scratch.path()).len(),
        before,
        "the take adds no journal entry"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage and the close", |state| {
        state.operating.is_none() && state.resolver.is_none()
    });
    let after = journal_entries(scratch.path());
    assert_eq!(after.len(), before + 1, "`⏎` adds exactly one entry");
    assert_eq!(
        after.last().map(|entry| entry.description.clone()),
        Some("stage sample.txt".to_owned()),
        "the one entry is the stage"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert!(!state.banner_focus, "there is no banner to hand back to");
    assert_eq!(
        crate::app_harness::git(scratch.path(), &["ls-files", "-u"]).trim(),
        "",
        "the index holds no unmerged entry"
    );

    drop(state);
    app.close();
    drop(scratch);
}

/// The whole-file half of the stash-pop box: the stash deleted a file the
/// branch then changed, so `sample.txt` is `deleted by them`. `o` is its own
/// journal entry, `resolved sample.txt (ours)`, and resolving the only file
/// closes the resolver with no `⏎` (m6-conflicts/05 scope 4).
#[test]
fn stash_pop_whole_file_journals_its_own_row() {
    let scratch = stash_pop_delete_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");
    let before = journal_entries(scratch.path()).len();

    let state = resolver_on_whole_file(&app, state, "sample.txt");
    let rows = crate::captures::frame(&state, 120, 40);
    assert!(
        rows.iter()
            .any(|row| row.contains("ours = your working tree")
                && row.contains("theirs = the stash")),
        "the stash-pop explainer is not on the frame:\n{}",
        rows.join("\n")
    );

    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the whole-file resolve and the close", |state| {
        state.operating.is_none() && state.resolver.is_none()
    });
    let after = journal_entries(scratch.path());
    assert_eq!(after.len(), before + 1, "`o` is exactly one entry");
    assert_eq!(
        after.last().map(|entry| entry.description.clone()),
        Some("resolved sample.txt (ours)".to_owned())
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert_eq!(
        crate::app_harness::git(scratch.path(), &["ls-files", "-u"]).trim(),
        "",
        "the index holds no unmerged entry"
    );
    assert!(scratch.path().join("sample.txt").exists(), "ours kept it");

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

// ------------------------------------- the last file, and the other refusals

/// `git` expected to exit non-zero: a merge or a `stash pop` that stops on
/// a conflict. Anything but a conflict stop is still a failed fixture.
fn git_stops(repo: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        !out.status.success(),
        "git {args:?} was meant to stop on a conflict and did not"
    );
    assert!(
        !still_unmerged(repo).is_empty(),
        "git {args:?} failed without a conflict: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// git's own answer: the paths still unmerged, one per line.
fn still_unmerged(repo: &Path) -> String {
    crate::app_harness::git(repo, &["diff", "--name-only", "--diff-filter=U"])
}

/// A merge stopped on exactly two text conflicts, `a.txt` and `b.txt` —
/// small enough that "the last conflicted file" is reached through the
/// real walk rather than a state built to skip it.
fn two_file_merge() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let commit_both = |side: &str| {
        for name in ["a.txt", "b.txt"] {
            std::fs::write(repo.join(name), format!("{side}\n")).expect("writable scratch");
        }
        git(repo, &["add", "a.txt", "b.txt"]);
        git_as_graft(repo, &["commit", "-m", side]);
    };
    commit_both("base");
    let home = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(repo, &["checkout", "-b", "side"]);
    commit_both("side");
    git(repo, &["checkout", home.trim()]);
    commit_both("main");
    git_stops(repo, &["merge", "side"]);
    scratch
}

/// A `git stash pop` stopped on a conflict in `sample.txt`: unmerged paths
/// and no operation, so no banner (`KNOWN_GAPS.md` row 229).
fn stash_pop_conflict() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "base\n").expect("writable scratch");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "base"]);
    std::fs::write(repo.join("sample.txt"), "stashed\n").expect("writable scratch");
    git_as_graft(repo, &["stash", "push"]);
    std::fs::write(repo.join("sample.txt"), "committed\n").expect("writable scratch");
    git_as_graft(repo, &["commit", "-am", "committed"]);
    git_stops(repo, &["stash", "pop"]);
    scratch
}

/// A `git stash pop` stopped on a modify/delete: the stash deleted
/// `sample.txt`, the branch then changed it — `deleted by them`, no banner.
fn stash_pop_delete_conflict() -> Scratch {
    use crate::app_harness::{git, git_as_graft};
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "base\n").expect("writable scratch");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "base"]);
    git(repo, &["rm", "-q", "sample.txt"]);
    git_as_graft(repo, &["stash", "push"]);
    std::fs::write(repo.join("sample.txt"), "committed\n").expect("writable scratch");
    git_as_graft(repo, &["commit", "-am", "committed"]);
    git_stops(repo, &["stash", "pop"]);
    scratch
}

/// The app open on `scratch`, its first status read landed.
fn opened_on(scratch: &Scratch) -> (App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the first status read", |state| {
        state.working_tree.is_some()
    });
    (app, state)
}

/// Take ours on every open block of the file on screen, then `⏎`, and wait
/// for the stage and whatever the resolver does after it.
fn take_all_and_mark(app: &App, state: AppState) -> AppState {
    let path = open(&state).path.clone();
    let mut state = state;
    while open(&state).file.as_ref().expect("a document").open() > 0 {
        state = app.key(&state, KeyCode::Char('o'));
        state = settled(app, state);
    }
    let state = app.key(&state, KeyCode::Enter);
    assert!(
        state.operating.is_some(),
        "`⏎` queued no stage for {}: {:?}",
        path.display(),
        state.status
    );
    app.pump(state, "the stage and what follows it", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_none_or(|open| open.path != path && open.file.is_some() && open.job.is_none())
    })
}

#[test]
fn last_file_closes_to_where_it_opened() {
    last_file_with_a_banner();
    last_file_without_one();
}

/// Opened from the banner, walked through both files, closed back onto it.
fn last_file_with_a_banner() {
    let scratch = two_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = app.pump(state, "the merge banner", |state| {
        state.in_progress.is_some()
    });
    let state = on_the_banner(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = settled(&app, state);
    assert_eq!(open(&state).path, Path::new("a.txt"));

    let state = take_all_and_mark(&app, state);
    assert_eq!(open(&state).path, Path::new("b.txt"), "on to the second");
    let state = take_all_and_mark(&app, state);

    assert!(
        state.resolver.is_none(),
        "the last file closes the resolver"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved · c continue"),
        "{:?}",
        state.status
    );
    assert!(state.banner_focus, "back on the banner it opened from");
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");
    drop(state);
    app.close();
    drop(scratch);
}

/// A conflicted stash pop, opened from the pane: no banner, so no `c`.
fn last_file_without_one() {
    let scratch = stash_pop_conflict();
    let (app, state) = opened_on(&scratch);
    assert!(state.in_progress.is_none(), "a stash pop is no operation");
    let state = resolver_on(&app, state, "sample.txt");
    let state = take_all_and_mark(&app, state);

    assert!(
        state.resolver.is_none(),
        "the only file closes the resolver"
    );
    assert!(
        matches!(&state.status, Status::Done(line) if line == "all conflicts resolved"),
        "no banner, so no `c continue`: {:?}",
        state.status
    );
    assert!(!state.banner_focus, "there is no banner to hand back to");
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn resolving_the_last_listed_file_first_goes_back_for_the_rest() {
    // `b.txt` is the last CONFLICTS row, but not the last conflict:
    // "all conflicts resolved" here would send the user to `c` with
    // `a.txt` still unmerged.
    let scratch = two_file_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "b.txt");
    let state = take_all_and_mark(&app, state);

    let Some(open) = state.resolver.as_ref() else {
        panic!(
            "the resolver closed with a.txt still unmerged: {:?}",
            state.status
        )
    };
    assert_eq!(open.path, Path::new("a.txt"), "back to the one left");
    assert_eq!(still_unmerged(scratch.path()).trim(), "a.txt");

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_refused_while_a_box_is_open() {
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    let state = on_the_banner(&app, state);
    // `a` asks "Abort the merge?" — a real box, raised by the real key.
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(
        state.confirm.is_some(),
        "`a` raised no box: {:?}",
        state.status
    );

    // `⏎` now answers the box, so no key reaches `open_resolver` while it
    // is up; the guard is for a caller that does — which is what this is.
    let mut next = state.clone();
    let mut commands = Vec::new();
    let mut raw = 9_001;
    let spent = graft::update::resolver::open_resolver(
        &mut next,
        &mut commands,
        "both.txt".into(),
        OpenedFrom::Banner,
        &mut || {
            raw += 1;
            graft_core::job::JobId::new(raw)
        },
    );
    assert!(spent, "the refusal is still an answer");
    assert!(commands.is_empty(), "and it queued nothing");
    assert!(next.resolver.is_none(), "no resolver over the box");
    assert!(next.confirm.is_some(), "the box keeps its question");
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "finish the box first"),
        "{:?}",
        next.status
    );

    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn open_refused_on_a_path_git_does_not_call_unmerged() {
    // The list on screen says `added-by-them.txt` is conflicted; git, by
    // the time the read runs, does not — it was staged behind the app's
    // back. The read's `Refusal::NotUnmerged` is what answers.
    let (scratch, app, state) = stopped(Shape::StoppedMerge);
    crate::app_harness::git(scratch.path(), &["add", "added-by-them.txt"]);
    let state = on_the_banner(&app, state);

    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Enter));
    assert!(
        matches!(commands.as_slice(), [Command::OpenConflict(_, path)] if path == Path::new("added-by-them.txt")),
        "the stale list still names it: {commands:?}"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the read's refusal", |state| {
        state.resolver.is_none()
    });

    let Status::Sticky { reason, keys } = &state.status else {
        panic!("a path git does not call unmerged said {:?}", state.status)
    };
    assert_eq!(reason, "added-by-them.txt is not conflicted");
    assert_eq!(keys.as_deref(), Some("e edit  M mergetool"));

    drop(state);
    app.close();
    drop(scratch);
}

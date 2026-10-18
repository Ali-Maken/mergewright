//! `B`: the merged common ancestor, shown and hidden (m6-conflicts/05 scope 2).

use super::*;

/// The file's own `Block`s, in order — `ConflictFile::blocks()` answers
/// only position and state (`BlockAt`), not the `base`/`reproduced` fields a
/// base test needs, which live on the `Block` each region carries.
fn conflict_blocks(
    file: &mergewright_git::conflict::ConflictFile,
) -> Vec<&mergewright_git::conflict::Block> {
    file.regions
        .iter()
        .filter_map(|region| match region {
            mergewright_git::conflict::Region::Conflict(block)
            | mergewright_git::conflict::Region::Resolved { block, .. } => Some(block),
            mergewright_git::conflict::Region::Plain(_) => None,
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
        matches!(&state.status, Status::Done { what: line, .. } if line == "base shown for 3 conflicts"),
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
    let (state, commands) = mergewright::update::update(
        &state,
        mergewright::update::AppEvent::Produced(
            mergewright_core::job::JobId::new(9_200),
            mergewright::update::Product::HandedOver(Ok(())),
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

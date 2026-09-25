//! The last file of a merge opens the finish summary on its own
//! (m6-conflicts/60), against real git — and the landings where it must not.

use super::*;
use graft::update::finish::{FinishFocus, ReturnTo};

/// The resolver landed on `path` with nothing in flight.
fn landed_on(app: &App, state: AppState, path: &str) -> AppState {
    app.pump(state, "the next file", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path)
                    && open.job.is_none()
                    && (open.file.is_some() || open.whole_file.is_some())
            })
    })
}

/// `gone_notes_and_readme_merge`, `gone.txt` kept (`o` on its card), the
/// resolver on `notes.txt` with its first block taken: one block left.
fn one_block_left(app: &App, state: AppState) -> AppState {
    let state = resolver_on_whole_file(app, state, "gone.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = landed_on(app, state, "notes.txt");
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(app, state);
    let left = open(&state).file.as_ref().expect("a document").open();
    assert_eq!(left, 1, "one block left");
    state
}

/// The stage, the done read, the status read and the summary's read, all
/// landed.
fn summary_landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the last stage and the summary", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// J13: `o ⏎` on the last block of the last file lands on the summary —
/// over the resolver, the file lists focused, the cursor on the first row
/// git combined on its own, the status saying every file is done.
#[test]
fn the_last_file_of_a_shell_merge_opens_the_summary() {
    let scratch = file_list::gone_notes_and_readme_merge();
    let (app, state) = opened_on(&scratch);
    let state = one_block_left(&app, state);
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    let before = crate::captures::frame(&state, 187, 50);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_some(), "⏎ staged: {:?}", state.status);
    let state = summary_landed(&app, state);

    let finish = state.finish.as_ref().expect("the summary");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert_eq!(finish.focus, FinishFocus::Files);
    assert!(state.resolver.is_some(), "the resolver stays under it");
    assert!(
        matches!(&state.status, Status::Done { what, keys: Some(keys) }
            if what == "notes.txt done" && keys == "every file done"),
        "{:?}",
        state.status
    );
    assert_eq!(still_unmerged(scratch.path()), "", "and git agrees");
    let after = crate::captures::frame(&state, 187, 50);
    let body = after.join("\n");
    assert!(
        after[1].contains("╭ Preview: finish merging feature into main"),
        "{body}"
    );
    let cursor = after
        .iter()
        .find(|row| row.contains('▸'))
        .unwrap_or_else(|| panic!("no cursor row:\n{body}"));
    assert!(cursor.contains("README.md"), "{cursor}");
    // The summary takes the whole frame, the bottom row included: the
    // status is set (above) but not drawn while it is open.
    let shot = format!(
        "{}\n--- after ⏎ ---\n{}",
        before.join("\n"),
        after.join("\n")
    );
    crate::captures::check_shot("m6/fix-last-file-opens-summary-187x50.txt", &shot);

    // `Esc` from it: the finish row, `c finish` leading the bar.
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.finish.is_none(), "Esc closed the summary");
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        frame
            .iter()
            .any(|row| row.contains("✓ every file done — c review and finish the merge")),
        "{body}"
    );
    assert!(
        frame.iter().any(|row| row.contains("files · all done ✓")),
        "{body}"
    );
    assert!(
        frame.last().is_some_and(|row| row.starts_with(" c finish")),
        "{body}"
    );
    crate::captures::check_shot("m6/fix-finish-row-120x40.txt", &body);
    drop(state);
    app.close();
    drop(scratch);
}

/// After `y` and `u`, the merge is back and resolved: the banner has the
/// keyboard (ticket 41), and nothing opens the summary for him.
#[test]
fn undo_after_finish_opens_no_summary() {
    let landing = shell_begun::u_landing();
    let state = &landing.state;
    assert!(state.finish.is_none(), "no summary: {:?}", state.status);
    assert!(state.banner_focus, "the banner has the keyboard");
}

/// Launched on a merge resolved in the shell: the banner, no summary.
#[test]
fn relaunch_on_a_resolved_merge_opens_no_summary() {
    let scratch = shell_begun::resolved_shell_merge();
    let (app, state) = shell_begun::launched_into(scratch.path());
    let state = app.pump(state, "the status read", |state| {
        state.status_read.is_none()
    });
    assert!(state.finish.is_none(), "no summary on launch");
    assert!(state.resolver.is_none(), "no resolver either");
    drop(state);
    app.close();
    drop(scratch);
}

/// `Space` on the last `CONFLICTS` row marks it resolved outside the
/// resolver: no summary opens on its own, and the keyboard stays in the
/// changes pane, whose bar offers `c finish` (the landing decided in
/// `docs/design-notes/m6-conflicts.md`, "The last file done outside the
/// resolver"; review m6-look-finish-wave2 M2).
#[test]
fn space_on_the_last_conflicts_row_opens_no_summary_and_keeps_the_pane() {
    use crate::conflict_interim::{about_to_conflict, mark_resolved, stopped_on_the_conflict};
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stopped_on_the_conflict(&app, state);
    std::fs::write(repo.join("shared.rs"), "fn main() { both() }\n").expect("writable");
    std::fs::write(repo.join("notes.md"), "the agreed plan\n").expect("writable");
    let state = app.key(&state, KeyCode::Tab);
    let state = crate::conflict_interim::open_changes_pane(&app, state);
    let state = mark_resolved(&app, state, "notes.md");
    let state = mark_resolved(&app, state, "shared.rs");
    let state = app.pump(state, "the status read", |state| {
        state.status_read.is_none() && state.operating.is_none()
    });
    assert!(state.finish.is_none(), "no summary: {:?}", state.status);
    assert!(state.resolver.is_none(), "no resolver");
    assert!(!state.banner_focus, "the pane keeps the keyboard");
    drop(state);
    app.close();
    drop(scratch);
}

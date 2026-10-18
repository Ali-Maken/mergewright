//! A merge's review opens at the resolver's `y` (ease-pass/05b; it no
//! longer opens on its own, P11 dropped), against real git — and the
//! landings where nothing opens.

use super::*;
use mergewright::update::finish::ReturnTo;

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

/// The review's read landed.
fn review_landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the review's read", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.finish.as_ref().is_some_and(|open| open.job.is_none())
    })
}

/// `o y` on the last block of the last file of a shell-begun merge: `o`
/// solves it and stages nothing, the notice offers `y`, and `y` lands on the
/// review — over the resolver, the cursor on the first clashed file
/// (ease-pass/12) — with the real index untouched (ease-pass/05b; was
/// J13's `o ⏎`, the summary opening on its own).
#[test]
fn y_on_a_solved_shell_merge_opens_the_review_over_the_resolver() {
    let scratch = file_list::gone_notes_and_readme_merge();
    let (app, state) = opened_on(&scratch);
    let state = one_block_left(&app, state);
    let state = app.key(&state, KeyCode::Char('o'));
    let state = settled(&app, state);
    assert!(state.finish.is_none(), "nothing opens on its own");
    assert_eq!(still_unmerged(scratch.path()).trim(), "notes.txt");
    let before = crate::captures::frame(&state, 187, 50);
    assert!(
        before
            .iter()
            .any(|row| row.contains("· y reviews the merge")),
        "{}",
        before.join("\n")
    );
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(state.operating.is_none(), "y wrote: {:?}", state.status);
    let state = review_landed(&app, state);

    let finish = state.finish.as_ref().expect("the review");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert!(state.resolver.is_some(), "the resolver stays under it");
    assert_eq!(
        still_unmerged(scratch.path()).trim(),
        "notes.txt",
        "the review staged nothing"
    );
    let after = crate::captures::frame(&state, 187, 50);
    let body = after.join("\n");
    assert!(
        body.contains("All 3 clashes solved. Nothing committed yet."),
        "{body}"
    );
    let cursor = after
        .iter()
        .find(|row| row.contains('▸'))
        .unwrap_or_else(|| panic!("no cursor row:\n{body}"));
    assert!(cursor.contains("✓ gone.txt"), "{cursor}");
    // The summary takes the whole frame, the bottom row included: the
    // status is set (above) but not drawn while it is open.
    let shot = format!(
        "{}\n--- after y ---\n{}",
        before.join("\n"),
        after.join("\n")
    );
    crate::captures::check_shot("m6/fix-y-opens-the-review-187x50.txt", &shot);
    let state = esc_back_to_the_resolver(&app, &state);
    drop(state);
    app.close();
    drop(scratch);
}

/// `Esc` from the review: the resolver unchanged, its notice still offering
/// `y` — no finish row, since git still calls `notes.txt` unmerged.
fn esc_back_to_the_resolver(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Escape);
    assert!(state.finish.is_none(), "Esc closed the review");
    assert!(state.resolver.is_some(), "back on the resolver");
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        frame
            .iter()
            .any(|row| row.contains("· y reviews the merge")),
        "{body}"
    );
    assert!(
        !frame.iter().any(|row| row.contains("every file done")),
        "{body}"
    );
    crate::captures::check_shot("m6/fix-solved-row-120x40.txt", &body);
    state
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

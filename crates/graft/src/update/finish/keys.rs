//! The review's keys (keymap §2.6b, ease-pass/12): matched by action id,
//! so a remapped key acts and the old one does not. The edge hands a
//! handler shipped chords only (`update/edge.rs`).

use graft_ui::keymap::{ActionId, KeyChord, Keymap};

use super::{
    AppState, Command, FinishState, JobId, KeyCode, KeyPress, Loading, PathBuf, Resume, ReturnTo,
    fits, keep_cursor_on_screen, with_finish,
};

/// The review's keys while it is open. `false` only for `Ctrl+C` (and
/// when it is closed): every other key is the review's (§2.1), and one
/// the review does not bind does nothing.
pub fn finish_key(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    key: KeyPress,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    let Some(finish) = state.finish.as_ref() else {
        return false;
    };
    if key.code == KeyCode::Char('c') && key.modifiers.control {
        return false;
    }
    // The review on screen is what is being committed: nothing changes it.
    // (`Esc` while Graft waits to quit on that commit never reaches here:
    // `keys::key_event` spends it on the wait first, m8/13b review M2.)
    if finish.committing.is_some() {
        return true;
    }
    // Below the widget's floor only one line is drawn: nothing on screen to
    // act on, so only `Esc` acts.
    if !fits(state) {
        if asks(key, "app.back") {
            escape(state);
        }
        return true;
    }
    if asks(key, "app.back") {
        escape(state);
    } else if asks(key, "confirm.yes") {
        commit(state, commands, next_id);
    } else if asks(key, "app.undo") {
        undo_pick(state, commands, next_id);
    } else if asks(key, "finish.toggle_diff") {
        toggle_diff(state, commands, next_id);
    } else if asks(key, "app.down") {
        move_cursor(state, 1);
    } else if asks(key, "app.up") {
        move_cursor(state, -1);
    } else if asks(key, "app.page_down") {
        move_cursor(state, page(state));
    } else if asks(key, "app.page_up") {
        move_cursor(state, -page(state));
    }
    true
}

/// `key` is one of `action`'s chords as shipped.
fn asks(key: KeyPress, action: &'static str) -> bool {
    KeyChord::from_press(&key)
        .is_some_and(|chord| Keymap::shipped().chords(&ActionId(action)).contains(&chord))
}

/// `u` in the review (ease-pass/05b): back to the resolver with the last
/// pick put back and the cursor on it — the resolver under the review, or
/// one opened on the pick when the review was asked for from the banner.
/// Nothing to undo: said, and the review stays.
fn undo_pick(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    if state.picks.is_empty() {
        with_finish(state, |finish| {
            finish.notice = Some((crate::update::resolver::NOTHING_TO_UNDO.to_owned(), false));
        });
        return;
    }
    let from_banner = state
        .finish
        .as_ref()
        .is_some_and(|finish| finish.return_to == ReturnTo::Banner);
    close(state);
    if from_banner {
        state.banner_focus = crate::update::banner::banner_view(state).is_some();
    }
    crate::update::resolver::undo_from_review(state, commands, from_banner, next_id);
}

/// One page: the list's visible height.
fn page(state: &AppState) -> isize {
    let Some(view) = crate::view::finish::finish_view(state) else {
        return 0;
    };
    let height =
        graft_ui::finish_summary::list_height(&view, crate::view::finish::summary_area(state));
    isize::try_from(height.max(1)).unwrap_or(isize::MAX)
}

/// Rows the cursor moves over: the clashed files, then one for what git
/// combined, when it combined anything.
fn row_count(finish: &FinishState) -> usize {
    match &finish.summary {
        Loading::Ready(summary) => summary.by_you.len() + usize::from(!summary.by_git.is_empty()),
        Loading::Idle | Loading::Pending(_) | Loading::Failed(_) => 0,
    }
}

/// `j`/`k`: the cursor over the rows, kept on screen.
pub(super) fn move_cursor(state: &mut AppState, by: isize) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let rows = row_count(finish);
    if rows == 0 {
        return;
    }
    finish.cursor = finish.cursor.saturating_add_signed(by).min(rows - 1);
    keep_cursor_on_screen(state);
}

/// The clashed file row `row` names; `None` on the combined row, whose
/// look is ticket 12b's.
pub(super) fn row_path(finish: &FinishState, row: usize) -> Option<PathBuf> {
    let Loading::Ready(summary) = &finish.summary else {
        return None;
    };
    summary.by_you.get(row).map(|file| file.path.clone())
}

/// `⏎`/`space`: show the cursor row's staged diff under it, or hide it.
pub(super) fn toggle_diff(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let row = finish.cursor;
    if finish.expanded.as_ref().is_some_and(|(at, _)| *at == row) {
        finish.expanded = None;
        finish.diff_job = None;
        return;
    }
    let Some(path) = row_path(finish, row) else {
        return;
    };
    let id = next_id();
    finish.expanded = Some((row, Loading::Pending(id)));
    finish.diff_job = Some(id);
    commands.push(Command::ReadFinishDiff(id, path));
    keep_cursor_on_screen(state);
}

/// `y`: commit the merge with git's own message (ticket decision 5).
pub(super) fn commit(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let Some(finish) = state.finish.as_mut() else {
        return;
    };
    let what = match &finish.summary {
        Loading::Idle | Loading::Pending(_) => {
            finish.notice = Some(("still reading · wait".to_owned(), false));
            return;
        }
        // Nothing read to stage inside the finish: git's own message, as
        // the unavailable line says.
        Loading::Failed(_) => Resume::Continue { solved: Vec::new() },
        Loading::Ready(_) => Resume::Continue {
            solved: finish.solved.clone(),
        },
    };
    if crate::update::operate::operation_in_flight(state) {
        with_finish(state, |finish| {
            finish.notice = Some(("a write is already running".to_owned(), true));
        });
        return;
    }
    if crate::update::banner::resume(state, commands, what, next_id) {
        let operating = state.operating;
        with_finish(state, |finish| {
            finish.committing = operating;
            finish.notice = None;
        });
    } else {
        with_finish(state, |finish| {
            finish.notice = Some(("no merge is in progress".to_owned(), true));
        });
    }
}

/// `Esc`: close; from the banner, the banner gets the keyboard back.
pub(super) fn escape(state: &mut AppState) {
    let back_to_banner = state
        .finish
        .as_ref()
        .is_some_and(|finish| finish.return_to == ReturnTo::Banner);
    close(state);
    if back_to_banner {
        state.banner_focus = true;
    }
}

/// Close the review. Nothing in it is the user's to keep: the message is
/// git's, read-only.
pub(super) fn close(state: &mut AppState) {
    state.finish = None;
}

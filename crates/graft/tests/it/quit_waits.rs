//! `q` then `[ Quit ]` while a merge runs: Graft waits for git, then quits
//! (m8-polish-and-ship/13b). Real repository, real job pool, real keys; the
//! merge and its journal entry are read back from disk.
//!
//! What it guards: before 13b, quitting mid-merge cancelled the job, and
//! the pool sent git a SIGTERM half-way through. Here no cancel is sent, the
//! merge lands whole, and its journal entry is committed.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::jobs::Dispatch;
use graft::update::{Command, Quit};
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::journal::{EntryStatus, OperationKind};

use crate::app_harness::{App, git, press_for_commands};
use crate::preview::{forked, on_main, opened};

/// `feature` checked out, the cursor on `main`, `m m`: the merge is queued
/// and running, its answer not yet pumped.
fn merging(app: &App, state: graft::update::AppState) -> graft::update::AppState {
    let mut state = opened(app, state);
    for _ in 0..20 {
        if on_main(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_main(&state), "never found main's row");
    let state = app.key(&state, KeyCode::Char('m'));
    let state = app.pump(state, "the preview's plan", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
    });
    let state = app.key(&state, KeyCode::Char('m'));
    assert!(state.operating.is_some(), "the merge is running");
    state
}

/// Press through `update`, dispatch what it asks, and hand the commands
/// back so the test can say none of them was a cancel.
fn press(
    app: &App,
    state: &graft::update::AppState,
    key: KeyPress,
) -> (graft::update::AppState, Vec<Command>) {
    let (next, commands) = press_for_commands(state, key);
    for command in &commands {
        app.jobs.dispatch(command.clone());
    }
    (next, commands)
}

fn assert_no_cancel(commands: &[Command], what: &str) {
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(..))),
        "{what} sent a cancel: {commands:?}"
    );
}

fn assert_merged_and_journaled(repo: &std::path::Path) {
    let parents = git(repo, &["rev-list", "--parents", "-1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "a whole merge: {parents}"
    );
    let entries = graft_ops::journal::Journal::open(&repo.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the merge wrote is readable")
        .entries()
        .to_vec();
    let merges: Vec<_> = entries
        .iter()
        .filter(|entry| entry.kind == OperationKind::Merge)
        .collect();
    assert_eq!(merges.len(), 1, "{entries:?}");
    assert_eq!(merges[0].status, EntryStatus::Committed);
}

#[test]
fn quit_mid_merge_waits_for_git_then_quits() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = merging(&app, state);

    let (state, commands) = press(&app, &state, KeyPress::plain(KeyCode::Char('q')));
    assert_no_cancel(&commands, "`q`");
    assert!(state.confirm.is_some(), "`q` asks first");
    let (state, commands) = press(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    assert_no_cancel(&commands, "`y`");
    assert_eq!(state.quit, None, "not yet: the merge still runs");
    assert!(state.quit_when_done.is_some(), "waiting to quit");

    let state = app.pump(state, "the merge's answer", |state| state.quit.is_some());
    assert_eq!(state.quit, Some(Quit::Requested));
    drop(state);
    app.close();
    assert_merged_and_journaled(repo);
}

/// `Ctrl+C` used to quit at once over anything; over a merge it is now the
/// same box as `q`, and nothing is cancelled.
#[test]
fn ctrl_c_mid_merge_boxes_and_never_cancels() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = merging(&app, state);

    let control_c = KeyPress {
        code: KeyCode::Char('c'),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    };
    let (state, commands) = press(&app, &state, control_c);
    assert_no_cancel(&commands, "`Ctrl+C`");
    assert_eq!(state.quit, None, "`Ctrl+C` does not quit over a merge");
    assert!(state.confirm.is_some(), "`Ctrl+C` asks, like `q`");
    let (state, commands) = press(&app, &state, KeyPress::plain(KeyCode::Char('y')));
    assert_no_cancel(&commands, "`y`");

    let state = app.pump(state, "the merge's answer", |state| state.quit.is_some());
    assert_eq!(state.quit, Some(Quit::Requested));
    drop(state);
    app.close();
    assert_merged_and_journaled(repo);
}

/// What right looks like (`m8-quit-waits-80x24.txt`): frame 1, `m m` then
/// `q` before the merge answers — a box `Quit?` reading `merge … is still
/// running · quit waits for it to finish` / `if it stops at a conflict or
/// fails, you stay to see it`, `[ Cancel ] [ Quit ]`, the bottom row led by
/// `y quit`. Frame 2, after `y`: no box, the status row `merge … ·
/// quitting when it finishes · esc stays`. The write is real and running:
/// its answer is simply not pumped between the keys. Colour unverified.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it quit_waits::capture_quit_waits`.
#[test]
fn capture_quit_waits() {
    use crate::preview::FORK_NOW;
    use graft::session::Clock;

    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = merging(&app, state);
    let mut boxed = app.key(&state, KeyCode::Char('q'));
    if let Some(facts) = boxed.repository.as_mut() {
        facts.path = std::path::PathBuf::from("/code/api");
    }
    let first = crate::captures::frame_at(&boxed, 80, 24, Some(Clock::Fixed(FORK_NOW)));
    let body = first.join("\n");
    // At 80 columns the box wraps its first line after `quit waits for it`.
    assert!(
        body.contains("is still running \u{b7} quit waits for it"),
        "{body}"
    );
    assert!(
        body.contains("if it stops at a conflict or fails"),
        "{body}"
    );
    assert!(
        first.last().is_some_and(|bar| bar.contains("y quit")),
        "{body}"
    );
    let waiting = app.key(&boxed, KeyCode::Char('y'));
    let second = crate::captures::frame_at(&waiting, 80, 24, Some(Clock::Fixed(FORK_NOW)));
    let body = second.join("\n");
    assert!(body.contains("quitting when it finishes"), "{body}");
    crate::captures::check_shot(
        "m8-quit-waits-80x24.txt",
        &format!(
            "=== `m m` then `q` while the merge runs ===\n{}\n\
             === then `y`: Graft waits for git ===\n{}\n",
            first.join("\n"),
            second.join("\n")
        ),
    );
    let state = app.pump(waiting, "the merge's answer", |state| state.quit.is_some());
    drop(state);
    app.close();
}

/// The window closing mid-merge: shutdown waits for the merge rather than
/// signalling it, so it lands whole with its entry committed.
#[test]
fn shutdown_mid_merge_lets_it_land() {
    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = merging(&app, state);
    drop(state);
    app.close();
    assert_merged_and_journaled(repo);
}

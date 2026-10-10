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
    let entries =
        graft_ops::journal::Journal::open(&repo.join(".git/tributary/undo-journal.jsonl"))
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

/// What right looks like (`m8-quit-stop-now-80x24.txt`): frame 1, after `q
/// y` and the offer — the status row `merge … · still running after 10 s ·
/// q stop it now · esc stays`, no box. Frame 2, after `q`: a box `Quit now?`
/// reading `merge … is still running · a git hook may be stuck` / `quit now
/// stops it half-way · the next launch shows where it stopped`, `[ Cancel ]
/// [ Quit ]` with the cursor on Cancel, the bottom row led by `y quit`. The
/// offer is **injected**: the harness calls `quit::waited_long` at once in
/// place of the loop's 10 s (`STOP_OFFER_AFTER`), so no time passes. Colour
/// unverified.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it quit_waits::capture_quit_stop_now`.
#[test]
fn capture_quit_stop_now() {
    use crate::preview::FORK_NOW;
    use graft::session::Clock;

    let scratch = forked();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let mut state = merging(&app, state);
    if let Some(facts) = state.repository.as_mut() {
        facts.path = std::path::PathBuf::from("/code/api");
    }
    let boxed = app.key(&state, KeyCode::Char('q'));
    let mut waiting = app.key(&boxed, KeyCode::Char('y'));
    graft::update::quit::waited_long(&mut waiting);
    let first = crate::captures::frame_at(&waiting, 80, 24, Some(Clock::Fixed(FORK_NOW)));
    let body = first.join("\n");
    assert!(body.contains("still running after 10 s"), "{body}");
    let stop_box = app.key(&waiting, KeyCode::Char('q'));
    let second = crate::captures::frame_at(&stop_box, 80, 24, Some(Clock::Fixed(FORK_NOW)));
    let body = second.join("\n");
    assert!(body.contains("Quit now?"), "{body}");
    assert!(body.contains("a git hook may be stuck"), "{body}");
    assert!(
        second.last().is_some_and(|bar| bar.contains("y quit")),
        "{body}"
    );
    crate::captures::check_shot(
        "m8-quit-stop-now-80x24.txt",
        &format!(
            "=== `q y` while the merge runs, then the offer (injected: no 10 s wait) ===\n{}\n\
             === then `q`: the stop box ===\n{}\n",
            first.join("\n"),
            second.join("\n")
        ),
    );
    // Cancel: the merge lands, nothing was stopped.
    let state = app.key(&stop_box, KeyCode::Escape);
    let state = app.pump(state, "the merge's answer", |state| state.quit.is_some());
    drop(state);
    app.close();
}

/// What right looks like (`m8-quit-stopped-merge-relaunch-80x24.txt`, 13c–
/// 13d): the caption is the one line the shell got when the stop box's `y`
/// stopped a merge stuck in its `pre-merge-commit` hook — `trib: stopped
/// merge main into feature after 10 s · the merge is paused · trib here
/// offers continue or abort` (the 10 s is the injected offer's). Below it,
/// the next launch at 80x24, as drawn today (graft-ui untouched): `MERGING`
/// at the top, the `Detail` tab of the narrow layout on the uncommitted
/// changes — `UNCOMMITTED · 2 staged`, the merge's result alone
/// (`A main-one.txt`, `A main-two.txt`; asserted to be the merge's own
/// paths) — and the bottom row `c finish  a abort  ⇥ graph  q quit`. His
/// edit to `base.txt` is not in the tree: the merge set it aside (19d,
/// ADR-0045 §4), it waits in the stash list, and `c` or `a` puts it back.
/// Nothing on this landing says so: at this width the banner's words
/// (`merge still in progress from a previous run · edits aside`, drawn at
/// 100x30) do not fit and only its keys are shown, and the stash row is on
/// the graph tab, not here (NOTES 19d-a3). Colour unverified; looks-right
/// is the user's.
///
/// Regenerate on purpose: `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it quit_waits::capture_relaunch_after_a_stopped_merge`.
#[test]
fn capture_relaunch_after_a_stopped_merge() {
    use crate::preview::FORK_NOW;
    use graft::session::Clock;

    let scratch = forked();
    let repo = scratch.path();
    let stopped = stuck::stop_a_stuck_merge(repo);
    let [forced] = stopped.shutdown.forced.as_slice() else {
        panic!("one forced write: {:?}", stopped.shutdown);
    };
    let line = stuck::exit_line_for(forced);
    assert!(line.contains("the merge is paused"), "{line}");
    // R6 (19d-a2): his edit waits aside while the merge is paused.
    let merge = stuck::entries(repo)
        .into_iter()
        .find(|entry| entry.kind == OperationKind::Merge)
        .expect("the merge's entry");
    stuck::assert_his_edit_waits_aside(repo, &merge);
    stuck::assert_staged_are_the_merges_own(repo);

    let (app, state) = App::open(repo);
    let state = app.resize(&state, 80, 24);
    // Every read the landing draws, the detail's diff included: a frame
    // taken before one of them lands would differ run to run.
    let mut state = app.pump(state, "the relaunch's reads", |state| {
        use graft::update::detail::Loading;
        state.in_progress.is_some()
            && state.refs.is_some()
            && !state.live_entries.is_empty()
            && state.working_tree.is_some()
            && state.operating.is_none()
            && !matches!(state.detail.files, Loading::Pending(_))
            && !matches!(state.detail.diff, Loading::Pending(_))
    });
    if let Some(facts) = state.repository.as_mut() {
        facts.path = std::path::PathBuf::from("/code/api");
    }
    let frame = crate::captures::frame_at(&state, 80, 24, Some(Clock::Fixed(FORK_NOW)));
    let body = frame.join("\n");
    assert!(body.contains("MERGING"), "{body}");
    assert!(body.contains("c finish  a abort"), "{body}");
    crate::captures::check_shot(
        "m8-quit-stopped-merge-relaunch-80x24.txt",
        &format!(
            "=== the shell, after `q y`, the offer, `q y` over a stuck pre-merge-commit hook ===\n\
             {line}\n\
             === the next launch ===\n{body}\n"
        ),
    );
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

mod hangup;
mod pty_session;
mod signal;
mod stuck;

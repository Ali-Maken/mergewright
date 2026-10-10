//! Usability review Q4 (`docs/reviews/m6-usability-final.md`), pinned for
//! m6-conflicts/69 and answered by m6-conflicts/77: after a checkout and an
//! edit, `u y` over the drift box takes the checkout back. The box names the
//! edit it replaces and the key that brings it back, and that key does.
//! Read back from git and the files, never from `AppState`.
//!
//! Regenerate the capture on purpose:
//! `GRAFT_UPDATE_SHOTS=1 cargo test -p graft --test it undo_over_drift`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at, radar_answered, settled_frame};
use crate::m6_stash_put_back::{main_and_fix, planned, rev};
use crate::preview::{FORK_NOW, dated_commit, opened};
use crate::remote_by_key::ready;
use crate::repo_truth::blob_of;

/// The edit made after the checkout: the drift the box names.
pub(crate) const EDIT: &str = "edited after the checkout\n";

/// The consequence line the box draws for [`EDIT`] (ticket 77, Decided 6).
const NAMES_THE_EDIT: &str =
    "Undo replaces your edit to fix.txt; a backup keeps it, and U brings it back.";

/// Moves the cursor onto `target`'s row, up first and then down. Not
/// counted: getting to the row is not the journey.
fn cursor_onto(app: &App, mut state: AppState, target: &str) -> AppState {
    let on = |state: &AppState| {
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected)
            .map(|row| row.commit.id.to_string())
    };
    for code in [KeyCode::Char('k'); 20]
        .into_iter()
        .chain([KeyCode::Char('j'); 40])
    {
        if on(&state).as_deref() == Some(target) {
            return state;
        }
        state = app.key(&state, code);
    }
    panic!("the cursor never reached {target}");
}

/// `. ⏎` on `commit`'s row: a checkout through the menu, settled. The tree
/// is clean, so it switches at once, no preview (m7-power-tools/10b).
fn checkout_through_menu(app: &App, state: AppState, commit: &str) -> AppState {
    let state = cursor_onto(app, state, commit);
    let state = app.key(&state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.preview.is_none(), "a clean checkout previews nothing");
    app.settle_write(state, "the checkout")
}

/// On [`main_and_fix`]: check out `fix` through the menu and edit
/// `fix.txt` — everything before the `u`.
fn checked_out_and_edited(app: &App, path: &Path, state: AppState) -> AppState {
    let state = checkout_through_menu(app, opened(app, state), &rev(path, "fix"));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    std::fs::write(path.join("fix.txt"), EDIT).expect("the scratch repo is writable");
    state
}

/// On [`main_and_fix`]: check out `fix` through the menu (`. ⏎ y`), edit
/// `fix.txt`, press `u` and wait for the drift box.
pub(crate) fn drift_box_after_a_checkout(app: &App, path: &Path, state: AppState) -> AppState {
    let state = checked_out_and_edited(app, path, state);
    let state = app.key(&state, KeyCode::Char('u'));
    drift_box(app, state)
}

fn drift_box(app: &App, state: AppState) -> AppState {
    app.pump(state, "the drift box", |state| {
        state.confirm.is_some() || (state.operating.is_none() && state.status != Status::Ready)
    })
}

/// The box's message as one line of words: graft wraps it, the words are
/// what the tests read.
fn box_words(state: &AppState) -> String {
    state
        .confirm
        .as_ref()
        .unwrap_or_else(|| panic!("no box is open; status {:?}", state.status))
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

/// What "as before" means for the way back: `HEAD`, the index entry by
/// entry, and git's own status of the working tree.
fn standing(path: &Path) -> [String; 3] {
    [
        git(path, &["symbolic-ref", "HEAD"]),
        git(path, &["ls-files", "-s"]),
        git(
            path,
            &[
                "--no-optional-locks",
                "status",
                "--porcelain=v2",
                "--untracked-files=all",
            ],
        ),
    ]
}

fn read(path: &Path, name: &str) -> Option<Vec<u8>> {
    match std::fs::read(path.join(name)) {
        Ok(bytes) => Some(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => panic!("{name} could not be read: {why}"),
    }
}

/// Counts the keys of a journey (UX 1.3) as it presses them.
struct Keys<'a> {
    app: &'a App,
    count: usize,
}

impl Keys<'_> {
    fn press(&mut self, state: &AppState, code: KeyCode) -> AppState {
        self.count += 1;
        self.app.key(state, code)
    }
}

/// `y` on an open drift box, settled, from the box's own words onward.
fn yes(app: &App, state: &AppState, what: &str) -> AppState {
    let state = app.key(state, KeyCode::Char('y'));
    app.settle_write(state, what)
}

/// Journey 2: from the landing after `u y`, one key — `U` — and the edit is
/// back, byte for byte, with the index and the status as they were.
#[test]
fn undo_over_drift_then_redo_brings_the_edit_back() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = checked_out_and_edited(&app, path, state);
    let before = standing(path);

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = drift_box(&app, keys.press(&state, KeyCode::Char('u')));
    assert_eq!(
        box_words(&state).matches(NAMES_THE_EDIT).count(),
        1,
        "{}",
        box_words(&state)
    );
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the undo over drift");
    assert_eq!(keys.count, 2, "u → y stays two keys");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    assert_eq!(
        read(path, "fix.txt"),
        None,
        "the checkout's file left with it"
    );
    assert_eq!(
        state.status,
        Status::done_with_keys("undo: checkout fix", "U brings back your edit to fix.txt")
    );

    // The landing: the way back is the key the line names.
    keys.count = 0;
    let state = keys.press(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    assert_eq!(keys.count, 1, "one key from the landing to the edit");
    assert_eq!(read(path, "fix.txt"), Some(EDIT.as_bytes().to_vec()));
    assert_eq!(
        standing(path),
        before,
        "HEAD, the index and the status as before `u`"
    );
    assert_eq!(state.status, Status::done("redo: checkout fix"));
    drop(state);
    app.close();
}

/// Journey 3 (69's pin, answered): a second `u` finds nothing to undo and
/// says which key goes the other way; that key brings the edit back.
#[test]
fn undo_over_drift_then_undo_again() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = drift_box_after_a_checkout(&app, path, state);
    let state = yes(&app, &state, "the undo over drift");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main",
        "`u y` took the checkout back"
    );
    assert_eq!(read(path, "fix.txt"), None);

    // Where the edit went: the undo entry's own backup holds its blob.
    let entries =
        graft_ops::journal::Journal::open(&path.join(".git/tributary/undo-journal.jsonl"))
            .expect("the journal the writes just made is readable")
            .entries()
            .to_vec();
    let backup = entries
        .last()
        .and_then(|entry| entry.backup_ref.clone())
        .expect("the undo over drift took a backup");
    let edited = blob_of(path, EDIT.as_bytes()).expect("git hashes a blob");
    let objects = git(path, &["rev-list", "--objects", &backup]);
    assert!(
        objects.contains(&format!("{edited} worktree/fix.txt")),
        "the undo's backup {backup} keeps the edit's blob {edited}:\n{objects}"
    );

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = keys.press(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the second u's answer", |state| {
        state.operating.is_none() && state.confirm.is_none()
    });
    assert_eq!(
        state.status,
        Status::done_with_keys("nothing to undo", "U redoes checkout fix"),
        "the second `u` names the way back, not a dead end"
    );
    assert_eq!(read(path, "fix.txt"), None, "nothing was written by it");
    let state = keys.press(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    assert_eq!(keys.count, 2, "u, then U");
    assert_eq!(read(path, "fix.txt"), Some(EDIT.as_bytes().to_vec()));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    drop(state);
    app.close();
}

/// The case the scout did not run: the edit is staged. It comes back
/// staged — the index and the working tree both as before `u`.
#[test]
fn a_staged_edit_comes_back_staged() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = checked_out_and_edited(&app, path, state);
    git(path, &["add", "fix.txt"]);
    let before = standing(path);
    let cached = git(path, &["diff", "--cached", "--raw"]);
    assert!(!cached.is_empty(), "the edit is staged");

    let state = drift_box(&app, app.key(&state, KeyCode::Char('u')));
    assert_eq!(
        box_words(&state).matches(NAMES_THE_EDIT).count(),
        1,
        "{}",
        box_words(&state)
    );
    let state = yes(&app, &state, "the undo over drift");
    assert_eq!(
        state.status,
        Status::done_with_keys("undo: checkout fix", "U brings back your edit to fix.txt")
    );
    let state = app.settle_write(app.key(&state, KeyCode::Char('U')), "the redo");

    assert_eq!(
        standing(path),
        before,
        "the index and the working tree as before `u`"
    );
    assert_eq!(
        git(path, &["diff", "--cached", "--raw"]),
        cached,
        "the same staged blob"
    );
    assert_eq!(git(path, &["diff", "--raw"]), "", "nothing unstaged");
    assert_eq!(read(path, "fix.txt"), Some(EDIT.as_bytes().to_vec()));
    drop(state);
    app.close();
}

/// Staged B1, then B2 on disk: both come back, each where it was.
#[test]
fn a_staged_and_unstaged_edit_both_come_back() {
    const ON_DISK: &str = "a second edit, not staged\n";
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = checked_out_and_edited(&app, path, state);
    git(path, &["add", "fix.txt"]);
    std::fs::write(path.join("fix.txt"), ON_DISK).expect("the scratch repo is writable");
    let before = standing(path);

    let state = drift_box(&app, app.key(&state, KeyCode::Char('u')));
    assert_eq!(
        box_words(&state).matches(NAMES_THE_EDIT).count(),
        1,
        "{}",
        box_words(&state)
    );
    let state = yes(&app, &state, "the undo over drift");
    let state = app.settle_write(app.key(&state, KeyCode::Char('U')), "the redo");

    assert_eq!(standing(path), before);
    assert_eq!(git(path, &["show", ":fix.txt"]), EDIT, "the index holds B1");
    assert_eq!(
        read(path, "fix.txt"),
        Some(ON_DISK.as_bytes().to_vec()),
        "the disk holds B2"
    );
    drop(state);
    app.close();
}

/// An edit to a path the checkout never changed is not the undo's to
/// replace: it is not named, and it is still on disk after `y`.
#[test]
fn an_edit_outside_the_undos_paths_stays_and_is_not_named() {
    const ELSEWHERE: &str = "base, edited too\n";
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = checked_out_and_edited(&app, path, state);
    std::fs::write(path.join("base.txt"), ELSEWHERE).expect("the scratch repo is writable");

    let state = drift_box(&app, app.key(&state, KeyCode::Char('u')));
    let words = box_words(&state);
    assert_eq!(words.matches(NAMES_THE_EDIT).count(), 1, "{words}");
    assert!(!words.contains("base.txt"), "{words}");
    let state = yes(&app, &state, "the undo over drift");
    assert_eq!(read(path, "base.txt"), Some(ELSEWHERE.as_bytes().to_vec()));
    assert_eq!(read(path, "fix.txt"), None);
    drop(state);
    app.close();
}

/// `U` over drift. The box says `Redo` and that a backup keeps the edit —
/// and names **no** key, because none brings it back: `u` after a redo takes
/// back the root operation (`graft_ops::journal::next_undo` steps over
/// `Undo`/`Redo` entries), writing the checkout's before-state, not the
/// edit. Pinned here so the words cannot start promising a key that does
/// not do it.
#[test]
fn redo_over_drift_names_the_backup_and_no_key() {
    const ON_MAIN: &str = "main-one, edited before the redo\n";
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = checkout_through_menu(&app, opened(&app, state), &rev(path, "fix"));
    let state = app.settle_write(app.key(&state, KeyCode::Char('u')), "the clean undo");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    std::fs::write(path.join("main-one.txt"), ON_MAIN).expect("the scratch repo is writable");

    let state = drift_box(&app, app.key(&state, KeyCode::Char('U')));
    let words = box_words(&state);
    assert!(
        words.contains("Redo replaces your edit to main-one.txt; a backup keeps it."),
        "{words}"
    );
    assert!(
        !words.contains("brings it back"),
        "no key is promised: {words}"
    );
    let state = yes(&app, &state, "the redo over drift");
    // m6-conflicts/80 (review m7): no key, so the backup's words are part
    // of what happened (`what`), not a key in the `keys` slot.
    assert_eq!(
        state.status,
        Status::done("redo: checkout fix; a backup keeps your edit to main-one.txt")
    );
    assert_eq!(read(path, "main-one.txt"), None, "fix has no main-one.txt");

    // Why no key is named: `u` now undoes the checkout itself.
    let state = app.settle_write(
        app.key(&state, KeyCode::Char('u')),
        "the undo after the redo",
    );
    assert_eq!(
        read(path, "main-one.txt"),
        Some(b"one\n".to_vec()),
        "u puts back main's own bytes, not the edit"
    );
    drop(state);
    app.close();
}

/// On [`main_and_fix`] plus `fix2` (one commit past `fix`, changing
/// `fix.txt`): the menu checks out `fix`, then `fix2`, and `fix.txt` is
/// edited. `J`, `j`, `⏎` opens the time machine's preview for "revert to
/// before `checkout fix`" — two steps — and `→ ⏎` runs it.
fn time_machine_over_an_edit(app: &App, path: &Path, state: AppState) -> AppState {
    let state = checkout_through_menu(app, opened(app, state), &rev(path, "fix"));
    // The first checkout's refresh must land first: a preview planned
    // against the old `HEAD` is refused (`BranchChanged`).
    let state = checkout_through_menu(app, ready(app, state), &rev(path, "fix2"));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix2"
    );
    std::fs::write(path.join("fix.txt"), EDIT).expect("the scratch repo is writable");
    let state = app.key(&state, KeyCode::Char('J'));
    let state = app.pump(state, "the journal browser", |state| {
        state.journal.shown
            && matches!(
                state.journal.entries,
                graft::update::detail::Loading::Ready(_)
            )
            && state.journal.reading.is_none()
    });
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Enter);
    let state = planned(app, state);
    let state = app.key(&state, KeyCode::Right);
    drift_box(app, app.key(&state, KeyCode::Enter))
}

fn main_fix_and_fix2() -> graft_fixtures::Scratch {
    let scratch = main_and_fix();
    let path = scratch.path();
    git(path, &["switch", "-c", "fix2", "fix"]);
    dated_commit(path, "fix.txt", "fixed twice\n", "fix it again", 4);
    git(path, &["switch", "main"]);
    scratch
}

/// The time machine's box and landing name the presses: the jump is two
/// `Undo` entries, and `U` walks them back one at a time.
#[test]
fn the_time_machine_names_the_presses() {
    let scratch = main_fix_and_fix2();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = time_machine_over_an_edit(&app, path, state);
    let words = box_words(&state);
    assert!(
        words.contains(
            "Undo replaces your edit to fix.txt; a backup keeps it, and U (2 times) brings it back."
        ),
        "{words}"
    );
    // `→ ⏎`, never `y`: the revert drift guard is one of ADR-0044 §3's
    // exceptions (`ConfirmThen::takes_y`).
    let state = app.key(&state, KeyCode::Right);
    let state = app.settle_write(app.key(&state, KeyCode::Enter), "the forced revert");
    // Fitted to the 80-column footer (m6-conflicts/80, review m5): with the
    // path, `… ✓ · U (2 times) brings back your edit to fix.txt` is 93
    // cells; the path goes first and the key stays.
    assert_eq!(
        state.status,
        Status::done_with_keys(
            "reverted to before \u{201c}checkout fix\u{201d} (2 steps)",
            "U (2 times) brings it back"
        )
    );
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );

    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = app.settle_write(keys.press(&state, KeyCode::Char('U')), "the first redo");
    let state = app.settle_write(keys.press(&state, KeyCode::Char('U')), "the second redo");
    assert_eq!(keys.count, 2, "the presses the line named");
    assert_eq!(read(path, "fix.txt"), Some(EDIT.as_bytes().to_vec()));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix2"
    );
    drop(state);
    app.close();
}

const SIZES: [(&str, u16, u16); 2] = [("120x40", 120, 40), ("80x24", 80, 24)];

fn header(size: &str) -> String {
    format!(
        "\
# TestBackend capture (not a pty), {size}, no colour recorded. Fixture:
# capture_fix_undo_drift_way_back in crates/graft/tests/it/undo_over_drift.rs
# (asserted). m6-conflicts/77: a menu checkout of `fix`, `fix.txt` edited,
# then `u`, `y`, `U`.
#
# What right looks like:
# 1. The drift box: under what moved, `Undo replaces your edit to fix.txt;
#    a backup keeps it, and U brings it back.` — whole, not cut. The graph
#    under it has settled: no `sorting…` (asserted; m6-conflicts/80).
# 2. After `y`: no changes row, and the bottom row reads `undo: checkout
#    fix ✓ · U brings back your edit to fix.txt`. `HEAD` is on `main` in
#    git and the graph marks `[◆ main]`, `fix` plain (both asserted;
#    m6-conflicts/79). The Radar has answered: its `ⓘ` is on `fix`.
# 3. After `U`: the changes row is back, `(1 unstaged)`, and `redo:
#    checkout fix ✓`. `HEAD` is on `fix` again and the edit's bytes are
#    back (both asserted from git and the file, not the frame).
# 4. A second repository: `fix`, then `fix2`, checked out, `fix.txt`
#    edited, and the journal's time machine run back to before `checkout
#    fix` past its drift box. The bottom row reads `reverted to before
#    “checkout fix” (2 steps) ✓ · U (2 times) brings it back`, whole at 80
#    columns: the path is dropped before the key (asserted; m6-conflicts/80).
# - Colour is unverified (CLAUDE.md rule 5).
#
"
    )
}

/// What each frame of `capture_fix_undo_drift_way_back` must hold, at one size.
fn assert_way_back_frames(size: &str, [asked, landed, back, jumped]: [&str; 4]) {
    assert!(!asked.contains("sorting"), "{size}: settled\n{asked}");
    assert!(
        asked.contains("Undo replaces your edit to fix.txt; a backup keeps it, and U"),
        "{size}\n{asked}"
    );
    assert!(
        landed.contains("U brings back your edit to fix.txt"),
        "{size}\n{landed}"
    );
    assert!(back.contains("redo: checkout fix"), "{size}\n{back}");
    // The time machine's landing, whole to its last word at 80 columns
    // (m6-conflicts/80, review m5).
    assert!(
        jumped.contains(
            "reverted to before \u{201c}checkout fix\u{201d} (2 steps) \u{2713} \u{b7} U (2 times) brings it back"
        ),
        "{size}\n{jumped}"
    );
}

#[test]
fn capture_fix_undo_drift_way_back() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open_with_clock(path, Clock::Fixed(FORK_NOW));
    // Frame 1 waits for the checkout's graph reload and the Radar before
    // `u` (m6-conflicts/80, review m6): the box used to open while the
    // reload still ran, and the frame caught `sorting…`.
    let edited = checked_out_and_edited(&app, path, state);
    let edited = radar_answered(&app, settled_frame(&app, edited));
    let asked = drift_box(&app, app.key(&edited, KeyCode::Char('u')));
    // Each landing is drawn once the write's re-read is on screen: the
    // answer ends the job first, and a frame taken between the two shows the
    // changes row as it was. An undo or redo that moves `HEAD` re-reads the
    // graph too (`graft_ops::restore::refresh_after`, m6-conflicts/79), so
    // each frame also waits for that reload and the Radar it rebuilds —
    // else it catches `sorting…` and no `ⓘ`.
    let drawn =
        |state: &AppState| frame_at(state, 120, 40, Some(Clock::Fixed(FORK_NOW))).join("\n");
    let landed = yes(&app, &asked, "the undo over drift");
    let landed = app.pump(landed, "the landing's re-read", |state| {
        !drawn(state).contains("Uncommitted changes")
    });
    let landed = radar_answered(&app, settled_frame(&app, landed));
    assert!(drawn(&landed).contains("[◆ main]"), "{}", drawn(&landed));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    let back = app.settle_write(app.key(&landed, KeyCode::Char('U')), "the redo");
    let back = app.pump(back, "the redo's re-read", |state| {
        drawn(state).contains("Uncommitted changes (1 unstaged)")
    });
    let back = radar_answered(&app, settled_frame(&app, back));
    assert!(drawn(&back).contains("[◆ fix]"), "{}", drawn(&back));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    assert_eq!(read(path, "fix.txt"), Some(EDIT.as_bytes().to_vec()));
    app.close();
    let jumped = time_machine_landing();

    for (size, columns, rows) in SIZES {
        let at = |state: &AppState| {
            frame_at(state, columns, rows, Some(Clock::Fixed(FORK_NOW))).join("\n")
        };
        let (asked, landed, back, jumped) = (at(&asked), at(&landed), at(&back), at(&jumped));
        assert_way_back_frames(size, [&asked, &landed, &back, &jumped]);
        check_shot(
            &format!("m6/fix-undo-drift-way-back-{size}.txt"),
            &format!(
                "{}=== frame 1: `u` over the edit, the drift box ===\n{asked}\n\n\
                 === frame 2: `y`, the landing ===\n{landed}\n\n\
                 === frame 3: `U`, the edit back ===\n{back}\n\n\
                 === frame 4: the time machine over an edit, `→ ⏎`, the landing ===\n{jumped}\n",
                header(size),
            ),
        );
    }
}

/// Frame 4's state: [`time_machine_over_an_edit`], `→ ⏎`, and the jump's
/// re-read and Radar settled.
fn time_machine_landing() -> AppState {
    let scratch = main_fix_and_fix2();
    let path = scratch.path();
    let (app, state) = App::open_with_clock(path, Clock::Fixed(FORK_NOW));
    let state = time_machine_over_an_edit(&app, path, state);
    let state = app.key(&state, KeyCode::Right);
    let state = app.settle_write(app.key(&state, KeyCode::Enter), "the forced revert");
    let state = radar_answered(&app, settled_frame(&app, state));
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    app.close();
    state
}

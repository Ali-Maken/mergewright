//! J's three journeys on the "lost work" fixture (m7-power-tools/24-b):
//! a commit lost to the shell's reset, a deleted branch, and his edit a
//! forced undo replaced — each `J`, a row, `⏎ y`, counted against the M7
//! note's `journey-counts` fence (`docs/design-notes/m7-power-tools.md`
//! :1268-1270). Run in sequence in one repository, as the note's F4 says:
//! main (13), then spike (14), then the edit (15).
//!
//! Also writes J's captures (`docs/shots/m7/journal-*.txt`,
//! `put-back-landed-187x50.txt`, and journey 15's
//! `put-back-edit-{preview,landed}-187x50.txt`).
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::KeyCode;
use graft_fixtures::lost_work::{self, Commits, Graft, LostWork};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_ops::apply::{Context, apply};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::operation::{FetchParams, Operation, Target};

use super::blame::Route;
use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at};

/// The instant every frame is drawn at, and every entry the app writes is
/// stamped with: a minute after the fixture's last step.
pub(crate) const NOW: i64 = lost_work::END + 60;

/// Graft's own operations at the fixture's Graft points, run through the
/// same `graft_ops` calls the app's jobs make (`graft-ops`'
/// `tests/it/lost_work.rs` does the same for its list tests).
struct AppOps;

/// A stop that never fires: the fixture's operations run to the end.
struct Never;

impl Stop for Never {
    fn should_stop(&self) -> bool {
        false
    }
}

/// Runs `run` over a context on `work`'s journal, stamped `at_ms`.
fn with_context<T>(
    work: &Path,
    at_ms: i64,
    run: impl FnOnce(&mut Context<'_>) -> Result<T, String>,
) -> Result<(), String> {
    let repo = RepoHandle::open(work).map_err(|why| why.to_string())?;
    let path = journal_path(&repo).map_err(|why| why.to_string())?;
    let mut journal = Journal::open(&path).map_err(|why| why.to_string())?;
    let version = graft_git::version::system_git()
        .map_err(|why| why.to_string())?
        .version();
    let never = Never;
    let mut ctx = Context::open(&repo, version, &mut journal, at_ms, [0; 10], &never)
        .map_err(|why| why.to_string())?;
    run(&mut ctx).map(|_| ())
}

impl Graft for AppOps {
    fn fetch(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        let op = Operation::Fetch(FetchParams {
            remote: Some("origin".to_owned()),
            prune: false,
        });
        with_context(work, at_ms, |ctx| {
            apply(ctx, &op).map_err(|why| why.explain())
        })
    }

    fn discard(&mut self, work: &Path, path: &str, at_ms: i64) -> Result<(), String> {
        let op = Operation::Discard(Target::Paths(vec![PathBuf::from(path)]));
        with_context(work, at_ms, |ctx| {
            apply(ctx, &op).map_err(|why| why.explain())
        })
    }

    fn undo_forced(&mut self, work: &Path, at_ms: i64) -> Result<(), String> {
        with_context(work, at_ms, |ctx| {
            graft_ops::undo::undo(ctx, true).map_err(|why| why.explain())
        })
    }
}

/// The fixture, built with the app's operations.
pub(crate) fn lost_work() -> LostWork {
    lost_work_with(&[])
}

/// The fixture with `config` set before its first commit
/// (`LostWork::build_with`): the Recover walk's `core.autocrlf=true`.
pub(crate) fn lost_work_with(config: &[(&str, &str)]) -> LostWork {
    LostWork::build_with(&mut AppOps, config)
        .unwrap_or_else(|why| panic!("the fixture builds: {why}"))
}

/// The frame at `columns`×`rows`, drawn at [`NOW`].
pub(crate) fn drawn(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, Some(Clock::Fixed(NOW)))
}

/// The frame as one string, for a failure message or a capture.
pub(crate) fn text(frame: &[String]) -> String {
    format!("{}\n", frame.join("\n"))
}

/// The app open on the fixture, its launch read answered.
pub(crate) fn opened(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open_with_clock(repo, Clock::Fixed(NOW));
    let state = app.pump(state, "the launch read", |state| {
        state.journal.reading.is_none()
    });
    (app, state)
}

/// The key `J`, counted, then every stage answered. Stage 3 is a wait, not
/// a key (the ticket).
pub(super) fn open_j(route: &mut Route, app: &App, state: &AppState) -> AppState {
    let state = route.press(app, state, "J", KeyCode::Char('J'));
    assert!(state.journal.shown, "J opened");
    assert!(state.journal.finding_lost(), "J asked for stage 2 at once");
    app.pump(state, "J's stages 2 and 3", |state| {
        !state.journal.finding_lost()
    })
}

/// `⏎`, counted, then its box's plan answered.
pub(super) fn enter_to_the_box(route: &mut Route, app: &App, state: &AppState) -> AppState {
    let state = route.press(app, state, "⏎", KeyCode::Enter);
    let state = app.pump(state, "what the row puts back", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(
                preview.plan,
                graft::update::detail::Loading::Idle | graft::update::detail::Loading::Pending(_)
            )
        }) || matches!(state.status, graft::update::Status::Sticky { .. })
    });
    assert!(
        matches!(
            state.preview.as_ref().map(|preview| &preview.plan),
            Some(graft::update::detail::Loading::Ready(_))
        ),
        "⏎ opened a put-back box with its plan: {:?}\n{}",
        state.status,
        text(&drawn(&state, 187, 50))
    );
    state
}

/// `y`, counted, then the write settled and the graph re-read.
pub(super) fn confirm(route: &mut Route, app: &App, state: &AppState, what: &str) -> AppState {
    let state = route.press(app, state, "y", KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    assert!(!state.journal.shown, "y closed J ({what})");
    crate::captures::settled_frame(app, state)
}

/// Walk 15's landing, read from the frame (review m7-30m-30o Minor 2): J
/// closed (no `JOURNAL ·` title row) and the changes pane's header drawn.
fn j_closed_and_pane_shown(landed: &[String], body: &str) {
    assert!(
        !landed.iter().any(|row| row.contains("JOURNAL ·")),
        "J is closed in the frame:\n{body}"
    );
    assert!(
        body.contains("UNCOMMITTED · 1 unstaged") && body.contains("UNSTAGED (1)"),
        "the changes pane's header is drawn:\n{body}"
    );
}

/// The selected row of J's frame: the one marked `▸`.
pub(crate) fn selected_row(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|row| row.starts_with('▸'))
        .map_or("", String::as_str)
}

pub(super) fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", "--verify", "-q", name])
        .trim()
        .to_owned()
}

/// Journeys 13, 14 and 15 in one repository, in the note's order.
///
/// Run in a child of this test binary with git's own clock pinned to
/// [`NOW`] (`GIT_COMMITTER_DATE`, inherited by every git the app spawns),
/// as `amend_into::capture_amend_into` does. The app stamps its entries
/// from the fixed clock; left on the wall clock, git would date the
/// put-back's reflog records years after its entry, and the list pairs an
/// entry with the *nearest* step moving the same ref the same way
/// (`graft_ops::journal_list::steps::claim`) — here the shell's own
/// `commit` of D at 10:15, which the put-back's C → D repeats. The entry
/// then takes the shell's commit and the put-back's own record draws as a
/// `$` row dated `future`: a clock the test invented, not a thing he can
/// meet (a real put-back and its records share one clock).
#[test]
fn the_three_lost_work_journeys() {
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "m7_walk::journal::the_three_lost_work_journeys_git_clock_pinned",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PINNED_CHILD, "1")
        .env("GIT_COMMITTER_DATE", format!("@{NOW} +0000"))
        .status()
        .expect("the child runs");
    assert!(status.success(), "the pinned-clock walk failed: {status:?}");
}

/// Set for [`the_three_lost_work_journeys_git_clock_pinned`] by its parent.
const PINNED_CHILD: &str = "GRAFT_M7_JOURNAL_PINNED_CHILD";

/// The child half of [`the_three_lost_work_journeys`]. Does nothing when
/// run on its own.
#[test]
#[ignore = "the child of the_three_lost_work_journeys; runs only under it, git's clock pinned"]
fn the_three_lost_work_journeys_git_clock_pinned() {
    if std::env::var_os(PINNED_CHILD).is_none() {
        return;
    }
    let fixture = lost_work();
    let repo = fixture.work();
    let commits = fixture.commits().clone();
    let (app, state) = opened(repo);

    let state = journey_13(&app, &state, repo, &commits);
    let state = journey_14(&app, &state, repo, &commits);
    let state = journey_15(&app, &state, repo);
    drop(state);
    app.close();
}

/// J's captures of the list as journey 13 finds it, at each width.
fn j_captures(app: &App, state: &AppState) {
    for (columns, rows) in [(187, 50), (100, 30), (187, 24)] {
        check_shot(
            &format!("m7/journal-{columns}x{rows}.txt"),
            &text(&drawn(state, columns, rows)),
        );
    }
    // At 140 there is no `git:` column: the selected `$` row's subject is
    // drawn under it. `k` onto the reset's row, on a copy — the journey
    // itself stays on the `✗` row.
    let on_the_reset = app.key(state, KeyCode::Char('k'));
    let frame = drawn(&on_the_reset, 140, 40);
    let body = text(&frame);
    let reset = frame
        .iter()
        .position(|row| row.starts_with('▸') && row.contains("reset main 1 commit back"))
        .unwrap_or_else(|| panic!("k selects the reset's row:\n{body}"));
    assert!(
        frame[reset + 1].contains("git: \"reset: moving to HEAD~1\""),
        "its subject under it:\n{body}"
    );
    check_shot("m7/journal-140x40.txt", &body);
}

/// Journey 13, from the graph to its landing.
fn journey_13(app: &App, state: &AppState, repo: &Path, commits: &Commits) -> AppState {
    // 13 · `lost-to-shell-reset`: J ⏎ y. The ✗ row under the reset is
    // preselected (F1), so no key moves to it.
    let mut route = Route::new("lost-to-shell-reset");
    let state = open_j(&mut route, app, state);
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains(lost_work::LOST_ON_MAIN)
            && selected_row(&frame).contains("✗ lost"),
        "D's ✗ row is preselected:\n{}",
        text(&frame)
    );
    j_captures(app, &state);
    let state = enter_to_the_box(&mut route, app, &state);
    let state = confirm(&mut route, app, &state, "13");
    route.within_count();
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main is back on D");
    let state = app.pump(state, "the graph re-read with D on it", |state| {
        drawn(state, 187, 50)
            .iter()
            .any(|row| row.contains(lost_work::LOST_ON_MAIN) && !row.contains("put back ✓"))
    });
    let state = crate::captures::settled_frame(app, state);
    let landed = drawn(&state, 187, 50);
    let body = text(&landed);
    assert!(body.contains("↺ put back"), "the put-back tag:\n{body}");
    assert!(
        body.contains("○ main was here"),
        "the was-here tag:\n{body}"
    );
    assert!(body.contains('✓'), "the ✓ line:\n{body}");
    check_shot("m7/put-back-landed-187x50.txt", &body);

    state
}

/// Journey 14, from the graph to its landing.
fn journey_14(app: &App, state: &AppState, repo: &Path, commits: &Commits) -> AppState {
    // 14 · `deleted-branch-back`: J n ⏎ y. D is on main again, so its ✗
    // row is gone and nothing is preselected; `n` finds E's.
    let mut route = Route::new("deleted-branch-back");
    let state = open_j(&mut route, app, state);
    let frame = drawn(&state, 120, 40);
    let body = text(&frame);
    assert!(
        frame
            .iter()
            .any(|row| row.contains("●  Graft") && row.contains("put main back")),
        "the put-back's own ● row:\n{body}"
    );
    assert!(
        !frame
            .iter()
            .any(|row| row.contains("✗ lost") && row.contains(&commits.d[..7])),
        "D's ✗ row is gone:\n{body}"
    );
    check_shot("m7/journal-after-put-back-120x40.txt", &body);
    let state = route.press(app, &state, "n", KeyCode::Char('n'));
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains(lost_work::LOST_ON_SPIKE),
        "n lands on E's ✗ row:\n{}",
        text(&frame)
    );
    let state = enter_to_the_box(&mut route, app, &state);
    let state = confirm(&mut route, app, &state, "14");
    route.within_count();
    // The landing, from the frame (30o R2). The graph's cursor rule is `▏`
    // (graft-ui `row_rule`); the ticket's `▸` is J's.
    let state = app.pump(state, "the graph re-read with E on it", |state| {
        drawn(state, 187, 50)
            .iter()
            .any(|row| row.contains(lost_work::LOST_ON_SPIKE) && row.contains("↺ put back"))
    });
    let state = crate::captures::settled_frame(app, state);
    let landed = drawn(&state, 187, 50);
    let body = text(&landed);
    let on_e = landed
        .iter()
        .find(|row| row.contains(lost_work::LOST_ON_SPIKE))
        .map_or("", String::as_str);
    assert!(
        on_e.contains('▏') && on_e.contains("↺ put back"),
        "the cursor and the put-back tag on E's row:\n{body}"
    );
    let bottom = landed.last().map_or("", String::as_str);
    assert!(
        bottom.contains("spike is back at \"Try redis cache\"")
            && bottom.contains('✓')
            && bottom.contains("u undoes"),
        "the bottom row:\n{body}"
    );
    assert_eq!(
        rev(repo, "refs/heads/spike"),
        commits.e,
        "spike is back on E"
    );
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main untouched");

    state
}

/// Journey 15 on the fixture as built, with no shell step: the file is as
/// the forced undo left it (`lost_work::DISCARDED_EDIT`, a real change of
/// his), and the put-back writes his lost edit over it. Its entry's
/// before-snapshot keeps what it replaced, so `u` brings that back
/// (m7/30o R1; graft-ops `u_after_a_file_put_back_brings_back_what_it_replaced`).
fn journey_15(app: &App, state: &AppState, repo: &Path) -> AppState {
    // 15 · `edit-in-undo-backup`: J n ⏎ y. E is on spike again; `n` finds
    // his edit the forced undo replaced.
    let mut route = Route::new("edit-in-undo-backup");
    let state = open_j(&mut route, app, state);
    let state = route.press(app, &state, "n", KeyCode::Char('n'));
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains("your edit to src/a.rs"),
        "n lands on the lost edit's row:\n{}",
        text(&frame)
    );
    let state = enter_to_the_box(&mut route, app, &state);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown, "the box is over J:\n{body}");
    assert!(
        body.contains("Preview: put back your edit to src/a.rs"),
        "the box's title:\n{body}"
    );
    assert!(
        body.contains("your files: src/a.rs comes back · what's in src/a.rs now is replaced"),
        "the box says what is replaced:\n{body}"
    );
    assert!(body.contains("u undoes it"), "the box's last line:\n{body}");
    check_shot("m7/put-back-edit-preview-187x50.txt", &body);
    let state = confirm(&mut route, app, &state, "15");
    route.within_count();

    let landed = drawn(&state, 187, 50);
    let body = text(&landed);
    // The changes pane, shown: its file list holds `src/a.rs` and the
    // diff beside it is that file's, which the pane draws for the file under
    // its cursor — the lost edit's `2` added.
    assert!(state.detail.shown, "the changes pane is shown:\n{body}");
    j_closed_and_pane_shown(&landed, &body);
    assert!(
        state.detail.selected_entry().is_some(),
        "its cursor is on a file:\n{body}"
    );
    assert!(
        landed.iter().any(|row| row.contains("▾ src/"))
            && landed.iter().any(|row| row.contains("M a.rs"))
            && landed.iter().any(|row| row.contains("+     2")),
        "the pane on src/a.rs, its diff the edit put back:\n{body}"
    );
    let bottom = landed.last().map_or("", String::as_str);
    assert!(
        bottom.contains("your edit to src/a.rs is back")
            && bottom.contains('✓')
            && bottom.contains("u undoes"),
        "the bottom row:\n{body}"
    );
    check_shot("m7/put-back-edit-landed-187x50.txt", &body);
    let on_disk = std::fs::read(repo.join(lost_work::EDITED)).expect("src/a.rs is on disk");
    assert_eq!(on_disk, lost_work::LOST_EDIT, "his edit is back on disk");

    // Uncounted, after the journey: `u` brings back what the put-back
    // replaced.
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo of the put-back");
    let on_disk = std::fs::read(repo.join(lost_work::EDITED)).expect("src/a.rs is on disk");
    assert_eq!(
        on_disk,
        lost_work::DISCARDED_EDIT,
        "u brings back what the put-back replaced"
    );
    state
}

/// A `●` row expanded with `space`: its commands under `commands Graft
/// ran`, each on a `┆ git …` line. Also
/// `docs/shots/m7/journal-expanded-120x40.txt`.
#[test]
fn journal_expanded_capture() {
    let fixture = lost_work();
    let repo = fixture.work();
    let (app, state) = opened(repo);
    let mut route = Route::new("lost-to-shell-reset");
    let mut state = open_j(&mut route, &app, &state);
    // Down to the discard's `●` row (not its undo's).
    let on_the_discard = |state: &AppState| {
        let frame = drawn(state, 120, 40);
        let row = selected_row(&frame);
        row.contains("discard src/a.rs") && !row.contains("undo:")
    };
    for _ in 0..20 {
        if on_the_discard(&state) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_the_discard(&state), "j reached the discard's row");
    let state = app.key(&state, KeyCode::Char(' '));
    let frame = drawn(&state, 120, 40);
    let body = text(&frame);
    let heading = frame
        .iter()
        .position(|row| row.contains("commands Graft ran"))
        .unwrap_or_else(|| panic!("the command log's heading:\n{body}"));
    assert!(
        frame
            .get(heading + 1)
            .is_some_and(|row| row.trim_start().starts_with("┆ git")),
        "a ┆ git line under it:\n{body}"
    );
    check_shot("m7/journal-expanded-120x40.txt", &body);
    drop(state);
    app.close();
}

/// J's `/` find (m7-power-tools/30o-c): `/spike ⏎` on the "lost work"
/// fixture. The title ends in blame's find tail, `▸` is on the first row
/// at or below the cursor that names spike, and the bar offers `n N match`.
/// Also `docs/shots/m7/journal-find-187x50.txt`.
#[test]
fn journal_find_capture() {
    let fixture = lost_work();
    let repo = fixture.work();
    let (app, state) = opened(repo);
    let mut route = Route::new("lost-to-shell-reset");
    let mut state = open_j(&mut route, &app, &state);
    for key in "/spike".chars() {
        state = app.key(&state, KeyCode::Char(key));
    }
    let state = app.key(&state, KeyCode::Enter);
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(
        frame
            .first()
            .is_some_and(|row| row.contains(" · find \"spike\" 1 of ")),
        "the title's find tail:\n{body}"
    );
    assert!(
        selected_row(&frame).contains("spike"),
        "▸ on a row naming spike:\n{body}"
    );
    assert!(
        frame
            .last()
            .is_some_and(|row| row.contains("/ find  n N match")),
        "the bar:\n{body}"
    );
    check_shot("m7/journal-find-187x50.txt", &body);
    drop(state);
    app.close();
}

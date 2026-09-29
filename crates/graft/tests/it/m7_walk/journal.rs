//! J's three journeys on the "lost work" fixture (m7-power-tools/24-b):
//! a commit lost to the shell's reset, a deleted branch, and his edit a
//! forced undo replaced — each `J`, a row, `⏎ y`, counted against the M7
//! note's `journey-counts` fence (`docs/design-notes/m7-power-tools.md`
//! :1268-1270). Run in sequence in one repository, as the note's F4 says:
//! main (13), then spike (14), then the edit (15).
//!
//! Also writes J's captures (`docs/shots/m7/journal-*.txt`,
//! `put-back-landed-187x50.txt`).

use std::path::{Path, PathBuf};

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::KeyCode;
use graft_fixtures::lost_work::{self, Graft, LostWork};
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
const NOW: i64 = lost_work::END + 60;

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
fn lost_work() -> LostWork {
    LostWork::build(&mut AppOps).unwrap_or_else(|why| panic!("the fixture builds: {why}"))
}

/// The frame at `columns`×`rows`, drawn at [`NOW`].
fn drawn(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, Some(Clock::Fixed(NOW)))
}

/// The frame as one string, for a failure message or a capture.
fn text(frame: &[String]) -> String {
    format!("{}\n", frame.join("\n"))
}

/// The app open on the fixture, its launch read answered.
fn opened(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open_with_clock(repo, Clock::Fixed(NOW));
    let state = app.pump(state, "the launch read", |state| {
        state.journal.reading.is_none()
    });
    (app, state)
}

/// The key `J`, counted, then every stage answered. Stage 3 is a wait, not
/// a key (the ticket).
fn open_j(route: &mut Route, app: &App, state: &AppState) -> AppState {
    let state = route.press(app, state, "J", KeyCode::Char('J'));
    assert!(state.journal.shown, "J opened");
    assert!(state.journal.finding_lost(), "J asked for stage 2 at once");
    app.pump(state, "J's stages 2 and 3", |state| {
        !state.journal.finding_lost()
    })
}

/// `⏎`, counted, then its box's plan answered.
fn enter_to_the_box(route: &mut Route, app: &App, state: &AppState) -> AppState {
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
fn confirm(route: &mut Route, app: &App, state: &AppState, what: &str) -> AppState {
    let state = route.press(app, state, "y", KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-back");
    assert!(!state.journal.shown, "y closed J ({what})");
    crate::captures::settled_frame(app, state)
}

/// The selected row of J's frame: the one marked `▸`.
fn selected_row(frame: &[String]) -> &str {
    frame
        .iter()
        .find(|row| row.starts_with('▸'))
        .map_or("", String::as_str)
}

fn rev(repo: &Path, name: &str) -> String {
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

    // 13 · `lost-to-shell-reset`: J ⏎ y. The ✗ row under the reset is
    // preselected (F1), so no key moves to it.
    let mut route = Route::new("lost-to-shell-reset");
    let state = open_j(&mut route, &app, &state);
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains(lost_work::LOST_ON_MAIN)
            && selected_row(&frame).contains("✗ lost"),
        "D's ✗ row is preselected:\n{}",
        text(&frame)
    );
    for (columns, rows) in [(187, 50), (100, 30), (187, 24)] {
        check_shot(
            &format!("m7/journal-{columns}x{rows}.txt"),
            &text(&drawn(&state, columns, rows)),
        );
    }
    // At 140 there is no `git:` column: the selected `$` row's subject is
    // drawn under it. `k` onto the reset's row, on a copy — the journey
    // itself stays on the `✗` row.
    let on_the_reset = app.key(&state, KeyCode::Char('k'));
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
    let state = enter_to_the_box(&mut route, &app, &state);
    let state = confirm(&mut route, &app, &state, "13");
    route.within_count();
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main is back on D");
    let state = app.pump(state, "the graph re-read with D on it", |state| {
        drawn(state, 187, 50)
            .iter()
            .any(|row| row.contains(lost_work::LOST_ON_MAIN) && !row.contains("put back ✓"))
    });
    let state = crate::captures::settled_frame(&app, state);
    let landed = drawn(&state, 187, 50);
    let body = text(&landed);
    assert!(body.contains("↺ put back"), "the put-back tag:\n{body}");
    assert!(body.contains("○ main was here"), "the was-here tag:\n{body}");
    assert!(body.contains('✓'), "the ✓ line:\n{body}");
    check_shot("m7/put-back-landed-187x50.txt", &body);

    // 14 · `deleted-branch-back`: J n ⏎ y. D is on main again, so its ✗
    // row is gone and nothing is preselected; `n` finds E's.
    let mut route = Route::new("deleted-branch-back");
    let state = open_j(&mut route, &app, &state);
    let frame = drawn(&state, 120, 40);
    let body = text(&frame);
    assert!(
        frame.iter().any(|row| row.contains("●  Graft") && row.contains("put main back")),
        "the put-back's own ● row:\n{body}"
    );
    assert!(
        !body.contains(&commits.d[..7]),
        "D's ✗ row is gone:\n{body}"
    );
    check_shot("m7/journal-after-put-back-120x40.txt", &body);
    let state = route.press(&app, &state, "n", KeyCode::Char('n'));
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains(lost_work::LOST_ON_SPIKE),
        "n lands on E's ✗ row:\n{}",
        text(&frame)
    );
    let state = enter_to_the_box(&mut route, &app, &state);
    let state = confirm(&mut route, &app, &state, "14");
    route.within_count();
    assert_eq!(rev(repo, "refs/heads/spike"), commits.e, "spike is back on E");
    assert_eq!(rev(repo, "refs/heads/main"), commits.d, "main untouched");

    // 15 · `edit-in-undo-backup`: J n ⏎ y. E is on spike again; `n` finds
    // his edit the forced undo replaced.
    //
    // On the fixture as built, `⏎` is refused: the undo put the discarded
    // edit back on disk (`lost_work::DISCARDED_EDIT`), and a file put-back
    // never writes over his own changes (`graft_ops::operation::recover`
    // :208, `Refusal::UnsavedEditsTo`, m7/23). The refusal lands on J's
    // status row, J still up. The note's count assumes a clean file — an
    // open question for the planner (ticket 24 NOTES 24-b).
    let mut refused = Route::new("edit-in-undo-backup");
    let state = open_j(&mut refused, &app, &state);
    let state = refused.press(&app, &state, "n", KeyCode::Char('n'));
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains("your edit to src/a.rs"),
        "n lands on the lost edit's row:\n{}",
        text(&frame)
    );
    let state = refused.press(&app, &state, "⏎", KeyCode::Enter);
    let state = app.pump(state, "the plan refused", |state| {
        matches!(state.status, graft::update::Status::Sticky { .. })
    });
    let frame = drawn(&state, 187, 50);
    let body = text(&frame);
    assert!(state.journal.shown && state.preview.is_none(), "J up, no box:\n{body}");
    let status_row = frame.len() - 3;
    assert!(
        frame[status_row].contains("src/a.rs has unsaved edits · c commits or S stashes them first"),
        "the refusal on J's status row, above the legend:\n{body}"
    );
    let state = app.key(&state, KeyCode::Escape);
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.journal.shown, "esc closed J");

    // The shell drops the discarded edit (setup, not counted): with the file
    // clean, the journey runs as the note counts it.
    git(repo, &["checkout", "--", lost_work::EDITED]);
    let mut route = Route::new("edit-in-undo-backup");
    let state = open_j(&mut route, &app, &state);
    let state = route.press(&app, &state, "n", KeyCode::Char('n'));
    let frame = drawn(&state, 187, 50);
    assert!(
        selected_row(&frame).contains("your edit to src/a.rs"),
        "n lands on the lost edit's row:\n{}",
        text(&frame)
    );
    let state = enter_to_the_box(&mut route, &app, &state);
    let state = confirm(&mut route, &app, &state, "15");
    route.within_count();
    let on_disk = std::fs::read(repo.join(lost_work::EDITED)).expect("src/a.rs is on disk");
    assert_eq!(on_disk, lost_work::LOST_EDIT, "his edit is back on disk");
    drop(state);
    app.close();
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

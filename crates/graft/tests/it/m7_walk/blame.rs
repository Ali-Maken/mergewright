//! Blame-place journeys (m7-power-tools/20f), walked on the session fixture
//! (`graft_fixtures::Shape::Session`): `src/auth/session.rs` at `main`, whose
//! line 22 (`if resp.status() == 401 && attempt == 0 {`) Lina last changed in
//! "Retry refresh once on 401".
//!
//! Setup keys (open the pane, pick the file, widen the diff, move the line
//! cursor) are not counted: the note's journeys start "the diff pane, cursor
//! on line 22" (`20-blame-place.md`, "Line cursor").

use std::path::Path;

use graft::update::AppState;
use graft::update::detail::{DetailSpec, Loading, Side, diff_cursor};
use graft::update::place::{BlameLoad, BlameState};
use graft_core::event::KeyCode;
use graft_core::oid::Oid;
use graft_git::blame::BlameLine;
use graft_ui::detail::DetailFocus;

use crate::app_harness::App;

/// The file every walk blames.
pub const FILE: &str = "src/auth/session.rs";

/// Lina's line at `main`.
pub const LINE: u32 = 22;

/// The trailer's lead glyph (`◂`).
pub const TRAILER: char = '\u{25c2}';

/// The blame cursor row's glyph (`▸`).
pub const CURSOR_ROW: char = '\u{25b8}';

/// The session fixture, writable, with a commit-graph file written so the
/// commit-graph offer never shows (it would take `B`, keymap §2.1).
pub fn session() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    crate::app_harness::git(scratch.path(), &["commit-graph", "write", "--reachable"]);
    scratch
}

/// A `git clone --depth 2` of the session fixture (20f's setup, 20j): its
/// history is cut at `main~1`, Priya's `0a27627`, and it has no
/// commit-graph. The clone is `<dir>/clone`; the source is kept alive too.
pub fn shallow_session() -> (graft_fixtures::Scratch, tempfile::TempDir) {
    let source = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let clone = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let url = format!("file://{}", source.path().display());
    crate::app_harness::git(
        clone.path(),
        &["clone", "--quiet", "--depth", "2", &url, "clone"],
    );
    (source, clone)
}

/// The commit-graph offer is not on screen and does not hold `B`.
pub fn assert_no_offer(state: &AppState) {
    assert_eq!(
        state.commit_graph_line(),
        None,
        "the walk fixture must not show the commit-graph offer"
    );
}

/// The frame at `columns`×`rows`, ages drawn against the corpus's clock.
pub fn drawn(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    crate::captures::frame_at(
        state,
        columns,
        rows,
        Some(graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    )
}

/// Open the app on `repo` at `columns`×`rows`, open `main`'s commit, select
/// [`FILE`], open its diff with one more line of context (`]`, so line 22
/// shows above the hunk's change at 26), and move the line cursor to
/// [`LINE`] on the new side.
pub fn on_line_22(repo: &Path, columns: u16, rows: u16) -> (App, AppState) {
    open_on_line_22(repo, columns, rows, false)
}

/// [`on_line_22`], with a compare base pinned first when `pin_base`: `j`
/// onto Priya's commit (`main~1`), `x`, `k` back to `main` — so the pane is
/// the compare `main~1…main`, whose diff of [`FILE`] is `main`'s own.
pub fn open_on_line_22(repo: &Path, columns: u16, rows: u16, pin_base: bool) -> (App, AppState) {
    let (app, state) = open_on_the_file(repo, columns, rows, pin_base);
    let state = app.key(&state, KeyCode::Enter);
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    let state = app.key(&state, KeyCode::Char(']'));
    let mut state = app.pump(state, "the wider diff", |state| {
        matches!(state.detail.diff, Loading::Ready(_))
    });
    for _ in 0..40 {
        let at = diff_cursor(&state).and_then(|cursor| cursor.at);
        if at.is_some_and(|at| at.side == Side::New && at.line == LINE) {
            return (app, state);
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!(
        "the diff cursor never reached new line {LINE}: {:?}",
        diff_cursor(&state)
    );
}

/// The pane open on `main`, the files cursor on [`FILE`]'s row and its diff
/// read: `F`'s start (m7-power-tools/21, "the detail pane on the file's
/// row"), and [`open_on_line_22`]'s first half.
pub fn open_on_the_file(repo: &Path, columns: u16, rows: u16, pin_base: bool) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, columns, rows);
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the repository's facts", |state| {
        state.repository.is_some() && state.system_git.is_some()
    });
    assert_no_offer(&state);
    let state = if pin_base {
        let state = app.key(&state, KeyCode::Char('j'));
        let state = app.key(&state, KeyCode::Char('x'));
        assert!(
            state.compare_base.is_some(),
            "x pinned a base: {:?}",
            state.status
        );
        app.key(&state, KeyCode::Char('k'))
    } else {
        state
    };
    // `⏎` opens the pane on the commit under the cursor (`main`).
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "main's file list", |state| {
        matches!(
            state.detail.spec,
            Some(DetailSpec::Commit(_) | DetailSpec::Compare { .. })
        ) && matches!(state.detail.files, Loading::Ready(_))
    });
    let state = file_row(&app, state, FILE);
    let state = app.pump(state, "the file's diff", |state| {
        matches!(state.detail.diff, Loading::Ready(_))
    });
    assert_eq!(state.detail.focus, DetailFocus::Files);
    (app, state)
}

/// `j` until the file list's cursor is on the row for `path`.
fn file_row(app: &App, mut state: AppState, path: &str) -> AppState {
    for _ in 0..12 {
        if diff_cursor(&state).is_some_and(|cursor| cursor.path == Path::new(path)) {
            return state;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("no file row for {path}: {:?}", state.detail.files);
}

/// Pump until the place on screen has its lines.
pub fn blame_ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the blame's lines", |state| {
        state
            .trail
            .on_screen()
            .is_some_and(|blame| matches!(blame.body, BlameLoad::Ready { .. }))
    })
}

/// `B`, and the blame's lines.
pub fn blame(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('B'));
    blame_ready(app, state)
}

/// The place on screen.
fn shown(state: &AppState) -> &BlameState {
    state
        .trail
        .on_screen()
        .unwrap_or_else(|| panic!("no place on screen: {:?}", state.status))
}

/// The line under the shown place's cursor.
pub fn cursor_line(state: &AppState) -> BlameLine {
    let blame = shown(state);
    match &blame.body {
        BlameLoad::Ready { blame: lines, .. } => lines.lines[blame.cursor].clone(),
        other => panic!("the blame has not arrived: {other:?}"),
    }
}

/// The author of the line under the shown place's cursor.
pub fn cursor_author(state: &AppState) -> String {
    let blame = shown(state);
    let line = cursor_line(state);
    match &blame.body {
        BlameLoad::Ready { blame: lines, .. } => {
            let id = line
                .commit
                .unwrap_or_else(|| panic!("line {} is not committed", line.final_line));
            lines.commits[&id].author.clone()
        }
        other => panic!("the blame has not arrived: {other:?}"),
    }
}

/// `git rev-parse <rev>` in `repo`.
pub fn oid(repo: &Path, rev: &str) -> Oid {
    let hex = crate::app_harness::git(repo, &["rev-parse", rev]);
    Oid::from_hex(hex.trim()).unwrap_or_else(|why| panic!("{rev}: {why:?}"))
}

/// Pump until `⏎`'s landing: `commit`'s own diff of `path`, the line cursor
/// on `line` (new side), the trail hidden under the graph.
pub fn landed(app: &App, state: AppState, commit: Oid, path: &str, line: u32) -> AppState {
    app.pump(state, "the landing on the commit's diff", |state| {
        diff_cursor(state).is_some_and(|cursor| {
            cursor.spec == DetailSpec::Commit(commit)
                && cursor.path == Path::new(path)
                && cursor
                    .at
                    .is_some_and(|at| at.side == Side::New && at.line == line)
        })
    })
}

/// The commit under the graph's cursor.
pub fn graph_cursor(state: &AppState) -> Option<Oid> {
    let history = state.history.as_ref()?;
    let window = history.window.as_ref()?;
    let row = window.rows.get(history.cursor.checked_sub(window.first)?)?;
    Some(row.commit.id)
}

/// A walk's counted route: the only way a walk body sends a key after setup
/// (review m7-18e-20f Minor 6), so a key can't be sent uncounted. The same
/// shape as `m5_walk`'s `Step::Press`, which drives a scenario runner this
/// harness doesn't use.
pub struct Route {
    id: &'static str,
    keys: Vec<&'static str>,
}

impl Route {
    pub fn new(id: &'static str) -> Route {
        Route {
            id,
            keys: Vec::new(),
        }
    }

    /// Send `code`, labelled `label`, and count it.
    pub fn press(
        &mut self,
        app: &App,
        state: &AppState,
        label: &'static str,
        code: KeyCode,
    ) -> AppState {
        self.keys.push(label);
        app.key(state, code)
    }

    /// The keys sent so far, in order (a promise scenario adds its
    /// journeys' routes up).
    pub fn keys(&self) -> &[&'static str] {
        &self.keys
    }

    /// The walk took its route; the note allows at most its count.
    pub fn within_count(&self) {
        let allowed = super::noted_keys(self.id);
        assert!(
            self.keys.len() <= allowed,
            "{} took {} inputs {:?}, the note allows {allowed}",
            self.id,
            self.keys.len(),
            self.keys
        );
    }
}

/// `blame-line-to-commit` (`B ⏎` = 2): from the diff cursor on line 22, the
/// commit that last touched it — Lina's — with its diff at that line. Lina's
/// commit numbers the line 21 (the later import moved it down one) and names
/// the file `src/session.rs` (before Priya's rename): `⏎` lands on
/// `LineRef { New, orig_line }` in the commit's own path.
#[test]
fn blame_line_to_commit() {
    let scratch = session();
    let repo = scratch.path();
    let lina = oid(repo, "main~3");
    let (app, state) = on_line_22(repo, 120, 40);

    let mut route = Route::new("blame-line-to-commit");
    let state = route.press(&app, &state, "B", KeyCode::Char('B'));
    let state = blame_ready(&app, state);
    assert_eq!(cursor_line(&state).final_line, LINE);
    assert_eq!(cursor_author(&state), "Lina");
    let state = route.press(&app, &state, "⏎", KeyCode::Enter);
    let state = landed(&app, state, lina, "src/session.rs", 21);
    route.within_count();

    assert!(state.trail.hidden(), "the trail waits under the graph");
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    assert_eq!(
        graph_cursor(&state),
        Some(lina),
        "the graph cursor is on Lina's commit"
    );
    assert_no_offer(&state);
    app.close();
}

/// `commit-back-to-blame` (`Esc` = 1): after `B ⏎`, focus in the diff body,
/// one `Esc` is blame again, on the same line.
#[test]
fn commit_back_to_blame() {
    back_to_blame(false);
}

/// The same walk with a compare base pinned in setup (20's pinned-base
/// ruling): still one `Esc`, and the base stays set aside while blame is up.
#[test]
fn commit_back_to_blame_with_a_base_pinned() {
    back_to_blame(true);
}

fn back_to_blame(pin_base: bool) {
    let scratch = session();
    let repo = scratch.path();
    let lina = oid(repo, "main~3");
    let (app, state) = open_on_line_22(repo, 120, 40, pin_base);
    if pin_base {
        assert!(
            matches!(state.detail.spec, Some(DetailSpec::Compare { .. })),
            "the pane is the compare: {:?}",
            state.detail.spec
        );
    }
    let state = blame(&app, &state);
    let state = app.key(&state, KeyCode::Enter);
    let state = landed(&app, state, lina, "src/session.rs", 21);
    assert_eq!(
        state.detail.focus,
        DetailFocus::Diff,
        "focus in the diff body"
    );

    let mut route = Route::new("commit-back-to-blame");
    let state = route.press(&app, &state, "Esc", KeyCode::Escape);
    route.within_count();

    assert_eq!(
        shown(&state).path,
        Path::new(FILE),
        "blame is back on screen"
    );
    assert_eq!(cursor_line(&state).final_line, LINE, "the same line");
    if pin_base {
        assert!(
            state.compare_base.is_none(),
            "nothing pinned while blame is up"
        );
        assert!(
            state
                .trail
                .origin
                .as_ref()
                .is_some_and(|origin| origin.base.is_some()),
            "the base is still set aside"
        );
    }
    assert_no_offer(&state);
    app.close();
}

/// `blame-who-at-line` (`j` = 1): in blame on Tom's line 21 (setup `B k`),
/// `j` moves to line 22 and the trailer names its author, Lina — on that row
/// only; Tom's trailer is gone.
#[test]
fn blame_who_at_line() {
    let scratch = session();
    let (app, state) = on_line_22(scratch.path(), 187, 50);
    let state = blame(&app, &state);
    let state = app.key(&state, KeyCode::Char('k'));
    assert_eq!(cursor_author(&state), "Tom");

    let mut route = Route::new("blame-who-at-line");
    let state = route.press(&app, &state, "j", KeyCode::Char('j'));
    route.within_count();

    assert_eq!(cursor_line(&state).final_line, LINE);
    let frame = drawn(&state, 187, 50);
    let trailers: Vec<&String> = frame.iter().filter(|row| row.contains(TRAILER)).collect();
    assert_eq!(trailers.len(), 1, "one trailer:\n{}", frame.join("\n"));
    assert!(
        trailers[0].contains(CURSOR_ROW) && trailers[0].contains(&format!("{TRAILER} Lina")),
        "the cursor row's trailer names Lina: {}",
        trailers[0]
    );
    assert!(
        !frame
            .iter()
            .any(|row| row.contains(&format!("{TRAILER} Tom"))),
        "Tom's trailer went with the cursor"
    );
    assert_no_offer(&state);
    app.close();
}

/// `blame-line-before` (`B ^` = 2): blame as it was before Lina's commit,
/// the cursor where her line came from — Tom's `if resp.status() == 401 {`,
/// line 20 in her parent.
#[test]
fn blame_line_before() {
    let scratch = session();
    let repo = scratch.path();
    let lina = oid(repo, "main~3");
    let (app, state) = on_line_22(repo, 187, 50);

    let mut route = Route::new("blame-line-before");
    let state = route.press(&app, &state, "B", KeyCode::Char('B'));
    let state = blame_ready(&app, state);
    let state = route.press(&app, &state, "^", KeyCode::Char('^'));
    let state = blame_ready(&app, state);
    route.within_count();

    assert_eq!(state.trail.places.len(), 2, "pushed, not replaced");
    let came_from = cursor_line(&state);
    assert_eq!(came_from.final_line, 20);
    assert_eq!(cursor_author(&state), "Tom");
    assert_eq!(
        String::from_utf8_lossy(&came_from.text).trim(),
        "if resp.status() == 401 {"
    );
    let frame = drawn(&state, 187, 50);
    let short = &lina.to_string()[..7];
    assert!(
        frame[0].contains(&format!("before {short}")),
        "the title: {}",
        frame[0]
    );
    assert_no_offer(&state);
    app.close();
}

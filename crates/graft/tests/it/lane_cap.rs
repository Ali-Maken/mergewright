//! The lane cap comes from the room the graph has, not from the screen's
//! width (m5-history-and-ghost-preview/21 item 4, ADR-0039's 2026-09-21
//! amendment).
//!
//! `view::tests` pins the numbers over scripted state. This draws the same
//! rule on a real repository with fourteen topic branches open at once, and
//! writes the capture the user judges by eye: the same 160-column terminal,
//! once with the detail pane beside the graph and once without.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame, frame_at};

/// The instant [`many_lanes`]'s first commit is stamped at, Unix seconds;
/// one minute per commit and per merge after it.
///
/// Without this the fixture was built at wall-clock time, so every commit
/// and merge object got a different id every run. The graph breaks a date
/// tie with the commit id (DESIGN.md §3.2), so the lane order of fifteen
/// columns opened in the same second was not stable either. After
/// `graft_fixtures::git::FIXED_DATE`, so these commits never appear to
/// predate the corpus history they are added to.
const LANE_EPOCH: i64 = 1_609_459_200;

/// The instant the capture's relative dates are drawn against: past the
/// newest of the 28 objects above, so an age reads the same forever.
const LANE_NOW: i64 = LANE_EPOCH + 60 * 60;

/// The size the capture is taken at. 160 columns so the screen's own
/// breakpoint is the twelve-lane one and the pane makes a visible
/// difference; 24 rows because a window holding *every* topic commit has no
/// foldable column at all — `Fold::plan` never hides a column with a node on
/// screen, and draws wider than the cap instead. The cap only decides
/// anything when some lanes are passing through.
const COLUMNS: u16 = 160;
const ROWS: u16 = 24;

/// Commits on `main` between the base and its tip: enough that the window,
/// scrolled to the oldest of them, holds no topic tip.
const MAIN_STEPS: i64 = 30;

/// The subject the cursor is put on: the oldest `main` commit, so the
/// window shows only rows the fourteen topic lanes pass straight through.
const SCROLLED_TO: &str = "main: step 01";

/// `main` with [`MAIN_STEPS`] commits of its own and fourteen **unmerged**
/// topic branches forked from its base, each newer than all of `main`
/// (ease-pass 06 D3, route 2: merged topics fold by default, so the lanes
/// come from branches still open). The topic tips sit at the top and their
/// lanes run down past every `main` commit to the base: scrolled to the
/// oldest `main` commit, fifteen columns are open with no topic node on
/// screen — more lanes than any breakpoint draws, which is what makes the
/// cap visible.
fn many_lanes() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    let base = git(path, &["rev-parse", "HEAD"]).trim().to_owned();

    // A column with a node on screen is never folded (`Fold::plan`,
    // m7-power-tools/36), so the window the cap decides must hold lanes
    // passing through rows with no node of theirs.
    for step in 1..=MAIN_STEPS {
        write(path, "main.rs", &format!("const STEP: u32 = {step};\n"));
        commit(path, &format!("main: step {step:02}"), step);
    }
    for topic in 1..=14 {
        let branch = format!("topic-{topic:02}");
        git(path, &["switch", "--quiet", "-c", &branch, &base]);
        write(path, &format!("{branch}.rs"), "fn topic() {}\n");
        commit(path, &format!("Work on {branch}"), MAIN_STEPS + topic);
    }
    git(path, &["switch", "--quiet", "main"]);
    scratch
}

fn write(repo: &Path, name: &str, body: &str) {
    std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
}

fn commit(repo: &Path, subject: &str, minute: i64) {
    git(repo, &["add", "--all"]);
    run_at(repo, &["commit", "-m", subject], minute);
}

/// One object-making git command, stamped `minute` minutes after
/// [`LANE_EPOCH`] so its id and its place in date order are the same on
/// every run.
fn run_at(repo: &Path, args: &[&str], minute: i64) {
    let when = format!("@{} +0000", LANE_EPOCH + minute * 60);
    graft_fixtures::git::run_at(repo, args, &when)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
}

/// The app opened on the fixture, with the graph on hand, the cursor on
/// [`SCROLLED_TO`] (the window scrolled past every topic tip), the detail
/// pane open on that row, and the repository path shortened to `/code/api`.
///
/// The path is shortened **before** any frame is drawn: the header lays
/// itself out against it, and the scratch repository's ~50-character
/// temporary path eats the room the header's `+N lanes` needs — the very
/// number this capture is about (the same rule m5/16 settled for the banner
/// captures).
fn opened_with_pane(app: &App, state: graft::update::AppState) -> graft::update::AppState {
    let state = app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    // The whole walk first: the rows `j` walks onto are all read.
    let mut state = crate::captures::at_rest(app, state);
    // `j` down from wherever the cursor landed to the oldest `main` commit;
    // bounded, so a fixture that never draws it fails here, not by timeout.
    for _ in 0..2 * MAIN_STEPS {
        if selected_subject(&state).as_deref() == Some(SCROLLED_TO) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert_eq!(
        selected_subject(&state).as_deref(),
        Some(SCROLLED_TO),
        "the cursor reaches the oldest main commit"
    );
    let state = app.key(&state, KeyCode::Enter);
    let mut state = app.pump(state, "the detail pane's file list", |state| {
        state.detail.shown && matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });
    if let Some(facts) = state.repository.as_mut() {
        facts.path = std::path::PathBuf::from("/code/api");
    }
    let state = app.resize(&state, COLUMNS, ROWS);
    // The pane's diff is read on its own job: a frame before it lands
    // differs from the next run's.
    app.pump(state, "the pane's diff read", |state| {
        frame(state, COLUMNS, ROWS)
            .iter()
            .all(|row| !row.contains("reading\u{2026}"))
    })
}

/// The selected row's subject.
fn selected_subject(state: &graft::update::AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
}

/// How many lanes the header says are folded away (`+N lanes`); 0 when the
/// header carries no such count, which is what it says when the whole graph
/// fits. Read off the drawn frame, not off the state: the number the user
/// sees is the one under test.
fn folded_lanes(rows: &[String]) -> usize {
    let Some(header) = rows.first() else {
        return 0;
    };
    let Some(at) = header.find(" lanes") else {
        return 0;
    };
    let Some(plus) = header[..at].rfind('+') else {
        return 0;
    };
    header[plus + 1..at].trim().parse().unwrap_or(0)
}

/// The defect, on a real repository: at 160 columns with the pane beside it
/// the graph stands in 92 columns, and it used to fold to the twelve lanes
/// of a wide screen — the breakpoint for a width it does not have. Fewer
/// lanes are drawn now, so more are folded behind the marker.
#[test]
fn the_pane_narrows_the_graph_so_the_graph_folds_more() {
    let scratch = many_lanes();
    let (app, state) = App::open(scratch.path());
    let state = opened_with_pane(&app, state);

    let with_pane = frame(&state, COLUMNS, ROWS);
    let folded_with_pane = folded_lanes(&with_pane);

    let mut alone = state.clone();
    alone.detail.shown = false;
    let full_width = frame(&alone, COLUMNS, ROWS);
    let folded_alone = folded_lanes(&full_width);

    assert!(
        folded_with_pane > folded_alone,
        "the pane leaves the graph 92 of the 160 columns, so it draws fewer \
         lanes and folds more of them: with the pane {folded_with_pane}, \
         without it {folded_alone}\n{}\n---\n{}",
        with_pane.join("\n"),
        full_width.join("\n")
    );

    drop(state);
    app.close();
}

/// Writes the lane-cap capture into `docs/shots/`.
///
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo test -p mergewright --test it lane_cap::capture_lane_cap_for_docs_shots`.
#[test]
fn capture_lane_cap_for_docs_shots() {
    let scratch = many_lanes();
    let (app, state) = App::open(scratch.path());
    let state = opened_with_pane(&app, state);

    let at = Some(Clock::Fixed(LANE_NOW));
    let with_pane = frame_at(&state, COLUMNS, ROWS, at);
    let mut alone = state.clone();
    alone.detail.shown = false;
    let full_width = frame_at(&alone, COLUMNS, ROWS, at);

    let body = format!(
        "{}\n== the pane open: the graph has 92 columns ==\n\n{}\n\n\
         == the same terminal, pane closed: the graph has all 160 ==\n\n{}\n",
        header(),
        with_pane.join("\n"),
        full_width.join("\n"),
    );
    check_shot("m5-lane-cap-160x24.txt", &body);

    drop(state);
    app.close();
}

/// The caption. What right looks like, in the user's terms, and what the
/// capture cannot show.
fn header() -> String {
    "# m5-lane-cap-160x24.txt: 160x24, drawn by `view::view` from a real\n\
     # AppState \u{2014} a scratch repository with fourteen unmerged topic\n\
     # branches forked from `main`'s base, scrolled to `main`'s oldest\n\
     # commits, so fifteen columns are open over the whole window (merged\n\
     # branches fold by default since ease-pass 06; crates/graft/tests/it/lane_cap.rs,\n\
     # capture_lane_cap_for_docs_shots, asserted). Not a pty capture: the\n\
     # terminal is ratatui's TestBackend, so this is a character grid with\n\
     # no colour attributes in it \u{2014} CLAUDE.md rule 5, colour ships\n\
     # unverified.\n\
     #\n\
     # Why: the lane cap was read off the **screen's** width while the\n\
     # detail pane stood beside the graph. At 160 columns the graph gets 92\n\
     # of them and folded to twelve lanes anyway \u{2014} the breakpoint for a\n\
     # width it does not have. ADR-0039's 2026-09-21 amendment, third\n\
     # instance: ask how much room this thing has, not how big the screen\n\
     # is (m5/21 item 4).\n\
     #\n\
     # What right looks like:\n\
     #  - **The two frames differ.** With the pane open the graph folds\n\
     #    more: the header says `+8 lanes` and a `\u{22ee}` marker column\n\
     #    stands where the folded lanes were. With the pane closed the same\n\
     #    repository on the same terminal says `+4 lanes`: more lanes\n\
     #    drawn. The two used to be identical, because both asked the\n\
     #    screen.\n\
     #  - The topic tips sit above the window, so their lanes pass through\n\
     #    rows with no node on them: those are what the cap folds. A column\n\
     #    with a node on screen is never folded (m7-power-tools/36; the\n\
     #    fixture reshaped again for the fold by ease-pass 06f).\n\
     #  - With the pane open the subject column starts two cells earlier\n\
     #    and every subject is still readable.\n\
     #  - Nothing reads as an error on either frame.\n\
     #\n\
     # What is **not** proven here: colour (rule 5), and whether the graph\n\
     # beside the pane reads well by eye \u{2014} that is yours to judge (rule 6).\n\
     # The counts themselves are pinned by\n\
     # `lane_cap::the_pane_narrows_the_graph_so_the_graph_folds_more` and\n\
     # `view::tests::the_lane_cap_comes_from_the_graphs_room_not_the_screens_width`.\n\
     #\n"
    .to_owned()
}

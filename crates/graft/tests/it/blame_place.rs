//! Blame-place captures (m7-power-tools/20f), under `docs/shots/m7/`. Real
//! `git`, the real job pool, the real `view::view`, on the session fixture
//! (`m7_walk::blame`). Ages are drawn against `CORPUS_NOW`, so Lina's commit
//! (210 days before it) reads `7 months ago`. The mock-ups' `a1b2c3d` is the
//! fixture's real `2f48ca3`. Colour unverified (rule 5): the heatmap, the
//! cursor row's tint and the gutter's dimming are not in a character grid.

use std::path::Path;

use graft::update::AppState;
use graft::update::detail::{DetailSpec, diff_cursor};
use graft_core::event::KeyCode;

use crate::app_harness::App;
use crate::m7_walk::blame::{
    CURSOR_ROW, LINE, TRAILER, assert_no_offer, blame, cursor_line, drawn, graph_cursor, landed,
    oid, on_line_22, open_on_line_22, session, shallow_session,
};

/// The diff's line cursor glyph (`▎`).
const DIFF_CURSOR: char = '\u{258e}';

/// Lina's trailer, whole.
const LINA: &str =
    "\u{25c2} Lina \u{b7} 7 months ago \u{b7} Retry refresh once on 401 \u{b7} 2f48ca3";

/// The row the blame cursor is on.
fn cursor_row(frame: &[String]) -> &String {
    let rows: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains(CURSOR_ROW))
        .collect();
    assert_eq!(rows.len(), 1, "one cursor row:\n{}", frame.join("\n"));
    rows[0]
}

/// The gutter of a blame row: everything before the `│` rule.
fn gutter(row: &str) -> &str {
    row.split('\u{2502}').next().unwrap_or(row)
}

/// Blame of [`FILE`] at `main`, the cursor on line 22, at `columns`×`rows`.
fn blame_on_22(columns: u16, rows: u16) -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = session();
    let (app, state) = on_line_22(scratch.path(), columns, rows);
    let state = blame(&app, &state);
    assert_eq!(cursor_line(&state).final_line, LINE);
    (scratch, app, state)
}

/// The frame, checked against `docs/shots/m7/<name>`.
fn shoot(state: &AppState, columns: u16, rows: u16, name: &str) -> Vec<String> {
    let frame = drawn(state, columns, rows);
    crate::captures::check_shot(&format!("m7/{name}"), &format!("{}\n", frame.join("\n")));
    frame
}

/// `docs/shots/m7/blame-187x50.txt`. Right: line 22 selected; its row reads
/// `▸ 2f48ca3 Lina …` and ends `◂ Lina · 7 months ago · Retry refresh once
/// on 401 · 2f48ca3`; no other row has a `◂` trailer.
#[test]
fn capture_blame_187x50() {
    let (_scratch, app, state) = blame_on_22(187, 50);
    let frame = shoot(&state, 187, 50, "blame-187x50.txt");
    let row = cursor_row(&frame);
    assert!(row.contains(LINA), "the trailer, whole: {row}");
    assert!(gutter(row).contains("2f48ca3 Lina"), "the gutter: {row}");
    assert_eq!(
        frame.iter().filter(|row| row.contains(TRAILER)).count(),
        1,
        "the trailer is on the cursor row only"
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/blame-140x40.txt`. Right: the gutter is 22 columns — sha,
/// name, age, no subject — and the cursor row's trailer reads `◂ Lina · 7
/// months ago · Retry refresh once on 401`: the fixture's line 22 is 57
/// columns, so the trailer's ` · 2f48ca3` (10 more) would pass 140 and is
/// shed. The spec's "the trailer whole" was an arithmetic slip: the
/// mock-up's line 22 is the same text, the design sheds the sha first, and
/// 134 + ` · 2f48ca3` (10) = 144 > 140.
#[test]
fn capture_blame_140x40() {
    let (_scratch, app, state) = blame_on_22(140, 40);
    let frame = shoot(&state, 140, 40, "blame-140x40.txt");
    let row = cursor_row(&frame);
    assert!(
        row.ends_with("\u{25c2} Lina \u{b7} 7 months ago \u{b7} Retry refresh once on 401"),
        "the trailer, the sha shed: {row}"
    );
    assert!(
        !gutter(row).contains("Retry"),
        "no subject in the gutter: {row}"
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/blame-100x30.txt`. Right: the gutter is 10 columns; the
/// cursor row's trailer starts `◂ Lina` and runs as far as it fits.
#[test]
fn capture_blame_100x30() {
    let (_scratch, app, state) = blame_on_22(100, 30);
    let frame = shoot(&state, 100, 30, "blame-100x30.txt");
    let row = cursor_row(&frame);
    assert!(
        row.contains(&format!("{TRAILER} Lina")),
        "the trailer, as far as it fits: {row}"
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/blame-long-line-187x50.txt`. Right: the cursor on line 27
/// (the 137-character line, Tom's); no trailer anywhere — the line leaves no
/// room — and Tom's name is in the cursor row's gutter.
#[test]
fn capture_blame_long_line_187x50() {
    let (_scratch, app, mut state) = blame_on_22(187, 50);
    for _ in 0..5 {
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert_eq!(cursor_line(&state).final_line, 27);
    let frame = shoot(&state, 187, 50, "blame-long-line-187x50.txt");
    let row = cursor_row(&frame);
    assert!(gutter(row).contains("Tom"), "the name in the gutter: {row}");
    assert!(
        !frame.iter().any(|row| row.contains(TRAILER)),
        "no trailer:\n{}",
        frame.join("\n")
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/blame-find-187x50.txt`. Right: after `/refresh⏎ n` the
/// title ends `· find "refresh" 8 of 8` and the cursor row `▸` is line 47
/// (`self.refresh.clear();`): `⏎` goes to the first match at or after the
/// cursor (line 22 has none, so 27, the 7th), `n` to the next. The spec's
/// `/retry` has one match in the fixture (line 27's string), so its "2 of N"
/// could not be shown; `refresh` is the fixture's word with several.
#[test]
fn capture_blame_find_187x50() {
    let (_scratch, app, state) = blame_on_22(187, 50);
    let mut state = app.key(&state, KeyCode::Char('/'));
    for letter in "refresh".chars() {
        state = app.key(&state, KeyCode::Char(letter));
    }
    let state = app.key(&state, KeyCode::Enter);
    assert_eq!(
        cursor_line(&state).final_line,
        27,
        "the first match from 22"
    );
    let state = app.key(&state, KeyCode::Char('n'));
    let frame = shoot(&state, 187, 50, "blame-find-187x50.txt");
    assert!(
        frame[0].ends_with("\u{b7} find \"refresh\" 8 of 8"),
        "the title: {}",
        frame[0]
    );
    let line = cursor_line(&state);
    assert_eq!(line.final_line, 47, "n: the next match");
    assert!(
        String::from_utf8_lossy(&line.text).contains("refresh"),
        "the cursor is on a match"
    );
    assert_no_offer(&state);
    app.close();
}

/// Open `repo` and pump until its facts are in.
fn facts_in(repo: &Path) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW),
    );
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the repository's facts", |state| {
        state.repository.is_some() && state.system_git.is_some()
    });
    (app, state)
}

/// 20j's ruling: git reads no commit-graph in a shallow repository
/// (`commit_graph_compatible`, 32's NOTES), so none is offered, and `B`
/// answers blame.
#[test]
fn a_shallow_clone_offers_no_commit_graph() {
    let (_source, dir) = shallow_session();
    let clone = dir.path().join("clone");
    let (app, state) = facts_in(&clone);
    assert_eq!(
        state.commit_graph_line(),
        None,
        "a shallow clone must not offer a commit-graph"
    );
    app.close();
    let (app, state) = on_line_22(&clone, 187, 50);
    let state = blame(&app, &state);
    assert_eq!(cursor_line(&state).final_line, LINE, "`B` is blame");
    assert_no_offer(&state);
    app.close();
}

/// A full clone with no commit-graph is one a slow sort would offer one
/// for, as before 20j. Since m7-power-tools/15 the offer waits on a sort of
/// 300 ms or more, which this small fixture never takes: the facts are what
/// is pinned here, the timing in `update::tests::commit_graph_offer`.
#[test]
fn a_full_clone_without_a_graph_still_offers_one() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
    let (app, state) = facts_in(scratch.path());
    assert_eq!(
        state.repository.as_ref().map(|facts| &facts.commit_graph),
        Some(&graft::repository::CommitGraphFact::Absent { declined: false })
    );
    app.close();
}

/// `docs/shots/m7/blame-shallow-187x50.txt` (20's "Ruling (planner): shallow
/// boundary"). A `--depth 2` clone, cut at Priya's `0a27627`; blame from line
/// 22, `k`×4 to line 18, a boundary line. Right: row 18 reads `▸ ^0a2762
/// shallow clone · can't say who`; its trailer `◂ can't say who · this clone
/// starts at 0a27627`; the boundary block's other rows carry brackets and no
/// name; `Priya` appears nowhere. (The ticket's "graph pane's last row" is not
/// on this screen: blame fills it at 187×50, so no graph pane is drawn.)
#[test]
fn blame_shallow_capture() {
    let (_source, dir) = shallow_session();
    let clone = dir.path().join("clone");
    let boundary = oid(&clone, "main~1");
    let short = &boundary.to_string()[..7];
    assert_eq!(short, "0a27627", "the fixture's boundary");
    let (app, state) = on_line_22(&clone, 187, 50);
    let mut state = blame(&app, &state);
    for _ in 0..4 {
        state = app.key(&state, KeyCode::Char('k'));
    }
    assert_eq!(cursor_line(&state).final_line, 18);
    assert_eq!(
        cursor_line(&state).commit,
        Some(boundary),
        "line 18 is a boundary line"
    );
    let frame = shoot(&state, 187, 50, "blame-shallow-187x50.txt");
    let all = frame.join("\n");
    let row = cursor_row(&frame);
    assert!(
        gutter(row).contains("\u{25b8} ^0a2762 shallow clone \u{b7} can't say who"),
        "row 18's gutter: {row}"
    );
    assert!(
        row.ends_with("\u{25c2} can't say who \u{b7} this clone starts at 0a27627"),
        "row 18's trailer: {row}"
    );
    assert!(!all.contains("Priya"), "no Priya anywhere:\n{all}");
    // The cursor's boundary block runs 4..=25: its rows below the cursor
    // are brackets, no name.
    for line in 19..=25 {
        let rule = format!("\u{2502} {line:>2}  ");
        let row = frame
            .iter()
            .find(|row| row.contains(&rule))
            .unwrap_or_else(|| panic!("line {line}:\n{all}"));
        // The gutter is what comes before the rule and line number; the
        // bracket itself is a `│`, so `gutter()` would stop at it.
        let mark = row.split(&rule).next().unwrap_or(row).trim();
        assert!(
            mark == "\u{2502}" || mark == "\u{2514}",
            "line {line}'s gutter is a bracket only: {row}"
        );
    }
    assert_no_offer(&state);
    app.close();
}

/// `B ⏎` from line 22, and the landing on Lina's commit.
fn then_commit(repo: &Path, pin_base: bool) -> (App, AppState) {
    let lina = oid(repo, "main~3");
    let (app, state) = open_on_line_22(repo, 120, 40, pin_base);
    let state = blame(&app, &state);
    let state = app.key(&state, KeyCode::Enter);
    let state = landed(&app, state, lina, "src/session.rs", 21);
    let state = crate::captures::settled_frame(&app, state);
    assert_eq!(graph_cursor(&state), Some(lina));
    (app, state)
}

/// The frame's hint bar. At 120×40 the pane is stacked under the graph, so
/// the bar is the row between them, not the last: the row offering `B`.
fn bar(frame: &[String]) -> &String {
    frame
        .iter()
        .find(|row| row.contains("B blame"))
        .unwrap_or_else(|| panic!("a bar offering B blame:\n{}", frame.join("\n")))
}

/// `docs/shots/m7/blame-then-commit-120x40.txt`. Right: after `B ⏎`, the
/// graph's cursor on Lina's `Retry refresh once on 401`; the pane on her
/// commit's `src/session.rs`, the diff's `▎` on `if resp.status() == 401 &&
/// attempt == 0 {` — line 21 in her version, the line that is 22 at `main`
/// (the ticket's "line 22" is the final number); the bar leads `esc back to
/// blame`.
#[test]
fn capture_blame_then_commit_120x40() {
    let scratch = session();
    let (app, state) = then_commit(scratch.path(), false);
    let frame = shoot(&state, 120, 40, "blame-then-commit-120x40.txt");
    let marked: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains(DIFF_CURSOR))
        .collect();
    assert_eq!(marked.len(), 1, "one diff cursor:\n{}", frame.join("\n"));
    assert!(
        marked[0].contains("attempt == 0"),
        "the cursor on Lina's line: {}",
        marked[0]
    );
    let bar = bar(&frame);
    assert!(
        bar.trim_start().starts_with("esc back to blame"),
        "the bar leads esc back to blame: {bar}"
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/blame-then-commit-with-base-120x40.txt` (20's pinned-base
/// ruling). Right: with a compare base pinned on Priya's `0a27627` before
/// `B ⏎`, the pane is Lina's commit's own (not `COMPARE`); the graph draws no
/// `x base` badge; the status row reads `compare base 0a27627 set aside · it
/// comes back when you leave blame`.
#[test]
fn capture_blame_then_commit_with_base_120x40() {
    let scratch = session();
    let lina = oid(scratch.path(), "main~3");
    let (app, state) = then_commit(scratch.path(), true);
    assert_eq!(
        diff_cursor(&state).map(|cursor| cursor.spec),
        Some(DetailSpec::Commit(lina))
    );
    assert!(state.compare_base.is_none(), "nothing pinned");
    let frame = shoot(&state, 120, 40, "blame-then-commit-with-base-120x40.txt");
    let all = frame.join("\n");
    assert!(!all.contains("COMPARE"), "not the compare pane:\n{all}");
    assert!(!all.contains("x base"), "no base badge:\n{all}");
    // `▏` marks the graph's cursor row (Lina's) in both landings; a pinned
    // base would draw a second one, on Priya's row.
    let priya = frame
        .iter()
        .find(|row| row.contains("Move session TTL to config"))
        .unwrap_or_else(|| panic!("Priya's row in the graph:\n{all}"));
    assert!(!priya.contains('\u{258f}'), "no base mark: {priya}");
    assert_eq!(all.matches('\u{258f}').count(), 1, "one `▏`:\n{all}");
    assert!(
        all.contains("compare base 0a27627 set aside \u{b7} it comes back when you leave blame"),
        "the set-aside words:\n{all}"
    );
    assert_no_offer(&state);
    app.close();
}

/// `docs/shots/m7/esc-back-to-blame-from-the-graph-120x40.txt` (20i, keymap
/// §2.1's pane clause). Two frames. (a) After `B ⏎` and `Tab` to the graph:
/// the pane is still open on Lina's commit, and the bar leads `esc back to
/// blame`. (b) After one `Esc`: blame of `src/auth/session.rs` again, the
/// `▸` on line 22 (`if resp.status() == 401 && attempt == 0 {`).
#[test]
fn capture_esc_back_to_blame_from_the_graph() {
    use graft_ui::detail::DetailFocus;
    let scratch = session();
    let (app, mut state) = then_commit(scratch.path(), false);
    for _ in 0..3 {
        if state.detail.focus == DetailFocus::Graph {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    assert_eq!(state.detail.focus, DetailFocus::Graph, "`Tab` to the graph");
    assert!(state.detail.shown, "the pane `⏎` opened is still open");
    assert!(state.trail.hidden(), "the trail waits");
    let graph = drawn(&state, 120, 40);
    let lead = graph
        .iter()
        .find(|row| row.contains(" esc "))
        .unwrap_or_else(|| panic!("a bar:\n{}", graph.join("\n")));
    assert!(
        lead.trim_start().starts_with("esc back to blame"),
        "(a) the bar leads esc back to blame: {lead}"
    );

    let state = app.key(&state, KeyCode::Escape);
    assert!(state.trail.on_screen().is_some(), "one `Esc` is blame");
    assert_eq!(cursor_line(&state).final_line, LINE, "the same line");
    let back = drawn(&state, 120, 40);
    assert!(
        cursor_row(&back).contains("attempt == 0"),
        "(b) the `▸` on line 22: {}",
        cursor_row(&back)
    );

    let body = format!(
        "Right: (a) `B ⏎` then `Tab` to the graph — the pane still open on \
         Lina's commit, the bar leads `esc back to blame`.\n{}\n\nRight: (b) \
         one `Esc` — blame again, `▸` on line 22 `if resp.status() == 401 && \
         attempt == 0 {{`.\n{}\n",
        graph.join("\n"),
        back.join("\n")
    );
    crate::captures::check_shot("m7/esc-back-to-blame-from-the-graph-120x40.txt", &body);
    assert_no_offer(&state);
    app.close();
}

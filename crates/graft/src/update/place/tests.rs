//! The blame place and its trail (m7-power-tools/20), through `update` as
//! the event loop delivers keys and job answers.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::{Blame, BlameCommit, BlameLine, IgnoreRevs, LineMark, Rev};
use graft_git::diff::Hunk;
use graft_ui::detail::DetailFocus;

use super::*;
use crate::update::events::{AppEvent, Command, Product};
use crate::update::tests::{commit_pane, key};
use crate::update::update;

/// Lina's commit: lines 1–30 but 5, which Mo wrote.
fn lina() -> Oid {
    Oid::from_bytes(&[0xa1; Oid::SHA1_BYTES]).expect("an oid")
}

fn mo() -> Oid {
    Oid::from_bytes(&[0xb2; Oid::SHA1_BYTES]).expect("an oid")
}

fn lina_parent() -> Oid {
    Oid::from_bytes(&[0xc3; Oid::SHA1_BYTES]).expect("an oid")
}

fn commit(id: Oid, author: &str, previous: Option<Oid>, shallow: bool) -> BlameCommit {
    BlameCommit {
        id,
        author: author.to_owned(),
        author_time: 0,
        summary: format!("{author}'s change"),
        path: "a.rs".into(),
        previous: previous.map(|parent| (parent, PathBuf::from("a.rs"))),
        boundary: previous.is_none(),
        shallow,
    }
}

/// `a.rs`, 30 lines: line 5 Mo's (a root), the rest Lina's (with a parent);
/// lines 22 and 27 say `retry`; with `uncommitted`, line 3 is not
/// committed yet.
fn a_blame(ignore_file: bool, uncommitted: bool) -> Blame {
    let lines = (1..=30)
        .map(|line: u32| BlameLine {
            commit: match line {
                3 if uncommitted => None,
                5 => Some(mo()),
                _ => Some(lina()),
            },
            orig_line: line,
            final_line: line,
            text: if line == 22 || line == 27 {
                format!("retry {line}").into_bytes()
            } else {
                format!("line {line}").into_bytes()
            },
            mark: LineMark::Plain,
        })
        .collect();
    let commits = BTreeMap::from([
        (lina(), commit(lina(), "Lina", Some(lina_parent()), false)),
        (mo(), commit(mo(), "Mo", None, false)),
    ]);
    Blame {
        path: "a.rs".into(),
        rev: Rev::Commit(lina()),
        lines,
        commits,
        ignore_revs: ignore_file.then(|| IgnoreRevs {
            file: ".git-blame-ignore-revs".into(),
            revisions: 2,
            applied: true,
        }),
    }
}

/// A place reading `a.rs` as of Lina's commit, ready, cursor on line 22.
fn ready_place(blame: Blame) -> BlameState {
    let rows: Arc<[graft_ui::blame::BlameRowView]> = rows::blame_rows(&blame).into();
    BlameState {
        path: "a.rs".into(),
        rev: Rev::Commit(lina()),
        title: graft_ui::blame::BlameRev::AsOf(short(lina())),
        options: graft_git::blame::BlameOptions::default(),
        body: BlameLoad::Ready {
            blame: Arc::new(blame),
            rows,
        },
        cursor: 21,
        top: 0,
        find: None,
        find_prompt: None,
        heatmap: false,
        last_click: None,
    }
}

/// A trail of one ready place over `state`, origin where `state` stands.
fn trail_over(state: &mut AppState, place: BlameState, shown: bool) {
    state.trail = Trail {
        places: vec![Place::Blame(Box::new(place))],
        shown,
        origin: Some(Origin {
            spec: state.detail.spec.unwrap_or(DetailSpec::WorkingTree),
            path: "a.rs".into(),
            at: None,
            focus: state.detail.focus,
            base: None,
            spot: Spot::of(state),
        }),
    };
}

/// `update::tests`' sweep: a place on screen with the find and the ignore
/// file as asked.
pub(crate) fn place_on_screen(mut state: AppState, finding: bool, ignore_file: bool) -> AppState {
    let mut place = ready_place(a_blame(ignore_file, false));
    if finding {
        place.find = Some("retry".to_owned());
    }
    trail_over(&mut state, place, true);
    state
}

/// `update::tests`' sweep: a trail waiting under `state`.
pub(crate) fn hide_a_trail(state: &mut AppState) {
    trail_over(state, ready_place(a_blame(false, false)), false);
}

/// `view::frame::tests`' overlay sweep: a place standing over the graph.
pub(crate) fn open_a_place(state: &mut AppState) {
    trail_over(state, ready_place(a_blame(false, false)), true);
}

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(state, key(code))
}

fn press_all(state: &AppState, codes: &[KeyCode]) -> AppState {
    codes
        .iter()
        .fold(state.clone(), |state, &code| press(&state, code).0)
}

/// The blame job `commands` asks for.
fn blame_job(commands: &[Command]) -> (JobId, BlameRequest) {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadBlame(job, request) => Some((*job, (**request).clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a blame job, got {commands:?}"))
}

fn answer(
    state: &AppState,
    job: JobId,
    answer: Result<BlameAnswer, BlameFailure>,
) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Produced(job, Product::Blame(answer.map(Arc::new))),
    )
}

/// The pane on the merge's diff, the cursor on body row `row`.
fn pane_at(row: usize) -> AppState {
    let mut state = commit_pane(JobId::new(1), false);
    on_a_file(&mut state);
    state.detail.body_cursor = row;
    state.size = crate::update::Size {
        columns: 120,
        rows: 40,
    };
    state
}

/// The files cursor off `commit_pane`'s `src/` row onto `a.rs`, whose diff
/// is the one loaded: `B` has a file to blame.
pub(crate) fn on_a_file(state: &mut AppState) {
    if let Loading::Ready(files) = &mut state.detail.files {
        let mut view = (**files).clone();
        view.cursor = 1;
        *files = Arc::new(view);
    }
}

/// `B` pressed on the pane at `row`, and the job's answer delivered.
fn blamed_at(row: usize) -> AppState {
    let state = pane_at(row);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    )
    .0
}

fn cursor_line(state: &AppState) -> u32 {
    let blame = state.trail.on_screen().expect("a place on screen");
    let BlameLoad::Ready { blame: lines, .. } = &blame.body else {
        panic!("not ready: {:?}", blame.body);
    };
    lines.lines[blame.cursor].final_line
}

// ---- Open ----

#[test]
fn b_on_a_diff_line_opens_blame_at_that_line() {
    let state = pane_at(22);
    let at = diff_cursor(&state)
        .and_then(|cursor| cursor.at)
        .expect("a line");

    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, request) = blame_job(&commands);
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert!(matches!(request.rev, Rev::Commit(_)), "{request:?}");
    assert!(opened.trail.on_screen().is_some(), "the place is on screen");

    let (read, _) = answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    assert_eq!(cursor_line(&read), at.line);
}

#[test]
fn b_with_the_graph_focused_is_unchanged() {
    let mut state = pane_at(22);
    state.detail.focus = DetailFocus::Graph;

    let (next, commands) = press(&state, KeyCode::Char('B'));
    assert!(!next.trail.is_live());
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadBlame(..))),
        "{commands:?}"
    );
}

#[test]
fn a_job_side_refusal_pops_and_says_so() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);

    let (refused, _) = answer(&opened, job, Err(BlameFailure::Binary));
    assert!(!refused.trail.is_live(), "the place is gone");
    assert!(
        matches!(&refused.status, Status::Sticky { reason, .. } if reason.contains("is binary · no blame")),
        "{:?}",
        refused.status
    );
}

// ---- Routing ----

#[test]
fn slash_in_blame_finds_in_the_file_not_the_graph() {
    let state = blamed_at(22);
    let typed = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(
        typed.trail.on_screen().and_then(|blame| blame.find.clone()),
        Some("re".to_owned())
    );
    assert!(typed.search.is_none(), "the graph search never opened");
}

#[test]
fn graph_verbs_are_inert_in_blame() {
    let state = blamed_at(22);
    for verb in ['m', 'r', 'a', 'c', 'o', 'u'] {
        let (next, commands) = press(&state, KeyCode::Char(verb));
        assert_eq!(next.trail, state.trail, "`{verb}` moved the place");
        assert!(
            next.preview.is_none() && next.commit_editor.is_none(),
            "`{verb}` reached the graph"
        );
        assert!(
            !commands.iter().any(is_write),
            "`{verb}` launched a write: {commands:?}"
        );
    }
}

#[test]
fn l_and_w_in_blame_never_reach_the_diff_pane() {
    let state = blamed_at(22);
    for code in [KeyCode::Char('l'), KeyCode::Char('w')] {
        let (next, _) = press(&state, code);
        assert_eq!(next.detail, state.detail, "{code:?} reached the pane");
    }
}

#[test]
fn esc_in_blame_never_cancels_a_running_write() {
    let mut state = blamed_at(22);
    state.status = Status::Busy("merging".to_owned());
    let (next, commands) = press(&state, KeyCode::Escape);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(_))),
        "a ready place has no job to stop, and the write is not the place's: {commands:?}"
    );
    assert_eq!(next.status, state.status, "the write is still running");
    assert!(!next.trail.is_live(), "Esc went back along the trail");
}

// ---- Navigation ----

#[test]
fn caret_opens_the_parent_and_esc_comes_back_to_the_same_line() {
    let state = blamed_at(22);
    let line = cursor_line(&state);
    let (parent, commands) = press(&state, KeyCode::Char('^'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(lina_parent()));
    assert_eq!(parent.trail.places.len(), 2);

    let (back, _) = press(&parent, KeyCode::Escape);
    assert_eq!(back.trail.places.len(), 1);
    assert_eq!(cursor_line(&back), line);
}

#[test]
fn caret_lands_where_the_line_came_from() {
    let state = blamed_at(22);
    let (parent, commands) = press(&state, KeyCode::Char('^'));
    let (job, request) = blame_job(&commands);
    assert!(request.came_from.is_some(), "asks where the line came from");
    let (read, _) = answer(
        &parent,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: Some(9),
        }),
    );
    assert_eq!(cursor_line(&read), 9);
}

fn hunk(old_start: u32, old_lines: u32, new_start: u32, new_lines: u32) -> Hunk {
    Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        heading: Vec::new(),
        lines: Vec::new(),
    }
}

#[test]
fn came_from_a_line_mid_hunk_is_the_hunks_first_old_line() {
    assert_eq!(came_from(&[hunk(10, 2, 10, 5)], 12), 10);
}

#[test]
fn came_from_a_pure_add_at_line_1_is_line_1() {
    assert_eq!(came_from(&[hunk(0, 0, 1, 3)], 2), 1);
}

#[test]
fn came_from_a_line_after_a_hunk_shifts_by_it() {
    assert_eq!(came_from(&[hunk(10, 2, 10, 5)], 20), 17);
}

#[test]
fn caret_at_the_root_says_nothing_before_it() {
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 4; // line 5, Mo's root commit
    }
    let (next, commands) = press(&state, KeyCode::Char('^'));
    assert_eq!(next.trail.places.len(), 1);
    assert!(!commands.iter().any(|c| matches!(c, Command::ReadBlame(..))));
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason.ends_with("added this file · nothing before it")),
        "{:?}",
        next.status
    );
}

#[test]
fn caret_on_an_uncommitted_line_says_so() {
    let mut state = pane_at(22);
    state.detail.body_cursor = 22;
    place_with(&mut state, a_blame(false, true), 2);
    let (next, _) = press(&state, KeyCode::Char('^'));
    assert!(
        matches!(&next.status, Status::Sticky { reason, .. } if reason == "line 3 isn't committed yet · nothing before it"),
        "{:?}",
        next.status
    );
}

/// A place over `state` reading `blame`, the cursor on row `cursor`.
fn place_with(state: &mut AppState, blame: Blame, cursor: usize) {
    let mut place = ready_place(blame);
    place.cursor = cursor;
    trail_over(state, place, true);
}

#[test]
fn enter_lands_on_the_commit_with_the_diff_at_this_line() {
    let state = blamed_at(22);
    let (landed, _) = press(&state, KeyCode::Enter);
    assert!(landed.trail.hidden(), "the trail waits under the graph");
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::Commit(lina()));
    assert_eq!(landing.at.map(|at| at.line), Some(22));
}

#[test]
fn enter_on_an_uncommitted_line_lands_in_the_changes_pane() {
    let mut state = pane_at(22);
    state.working_tree = Some(Arc::new(graft_git::status::WorkingTreeStatus {
        entries: vec![graft_git::status::StatusEntry {
            path: "a.rs".into(),
            original_path: None,
            staged: None,
            unstaged: Some(graft_git::status::Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: false,
        }],
    }));
    place_with(&mut state, a_blame(false, true), 2);
    let (landed, _) = press(&state, KeyCode::Enter);
    let landing = landed.line_landing.as_ref().expect("a landing");
    assert_eq!(landing.spec, DetailSpec::WorkingTree);
}

#[test]
fn esc_in_the_diff_after_enter_returns_to_blame_same_line() {
    let state = blamed_at(22);
    let line = cursor_line(&state);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Diff;
    landed.detail.shown = true;
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "blame is back");
    assert_eq!(cursor_line(&back), line);
}

#[test]
fn esc_from_blame_returns_where_b_was_pressed() {
    let before = pane_at(22);
    let state = blamed_at(22);
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert_eq!(back.detail.focus, before.detail.focus);
    assert_eq!(back.detail.body_cursor, before.detail.body_cursor);
    assert_eq!(
        back.history.as_ref().map(|history| history.cursor),
        before.history.as_ref().map(|history| history.cursor)
    );
}

#[test]
fn a_write_clears_the_trail() {
    let mut state = commit_pane(JobId::new(1), false);
    hide_a_trail(&mut state);
    let mut commands = vec![Command::Undo(JobId::new(9), false)];
    after_commands(&mut state, &mut commands);
    assert!(!state.trail.is_live());
}

#[test]
fn the_trail_holds_16() {
    let mut state = blamed_at(22);
    for _ in 0..TRAIL_MAX + 3 {
        let _commands = open_place(
            &mut state,
            Rev::Commit(lina()),
            "a.rs".into(),
            graft_ui::blame::BlameRev::Now,
            Want::Line(1),
            None,
        );
    }
    assert_eq!(state.trail.places.len(), TRAIL_MAX);
}

// ---- Keys ----

#[test]
fn find_and_n_move_between_matches() {
    let mut state = blamed_at(22);
    if let Some(blame) = state.trail.top_mut() {
        blame.cursor = 0;
    }
    let found = press_all(
        &state,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Char('t'),
            KeyCode::Enter,
        ],
    );
    assert_eq!(cursor_line(&found), 22);
    let next = press(&found, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&next), 27);
    let wrapped = press(&next, KeyCode::Char('n')).0;
    assert_eq!(cursor_line(&wrapped), 22);
    let back = press(&wrapped, KeyCode::Char('N')).0;
    assert_eq!(cursor_line(&back), 27);
}

#[test]
fn n_without_a_find_does_nothing() {
    let state = blamed_at(22);
    assert_eq!(
        press(&state, KeyCode::Char('n')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn esc_clears_the_find_before_popping() {
    let state = place_on_screen(pane_at(22), true, false);
    let (cleared, _) = press(&state, KeyCode::Escape);
    assert!(cleared.trail.is_live(), "still in blame");
    assert_eq!(
        cleared
            .trail
            .on_screen()
            .and_then(|blame| blame.find.clone()),
        None
    );
}

#[test]
fn i_toggles_ignore_revs_and_the_line_says_which() {
    let state = place_on_screen(pane_at(22), false, true);
    let (next, commands) = press(&state, KeyCode::Char('i'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_revs);
    assert!(!next.trail.top().expect("the place").options.ignore_revs);
}

#[test]
fn i_is_unbound_without_the_file() {
    let state = place_on_screen(pane_at(22), false, false);
    // Measured against a key nothing binds: `update` does ambient work on
    // every event (the evil-merge probe).
    assert_eq!(
        press(&state, KeyCode::Char('i')),
        press(&state, KeyCode::Function(12))
    );
}

#[test]
fn w_toggles_whitespace() {
    let state = blamed_at(22);
    let (next, commands) = press(&state, KeyCode::Char('w'));
    let (_, request) = blame_job(&commands);
    assert!(!request.options.ignore_whitespace);
    assert!(matches!(
        next.trail.top().expect("the place").body,
        BlameLoad::Reading {
            want: Want::Line(22),
            ..
        }
    ));
}

// ---- States ----

#[test]
fn blame_reading_title_then_lines() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    assert!(matches!(
        opened.trail.top().expect("the place").body,
        BlameLoad::Reading { .. }
    ));
    let (job, _) = blame_job(&commands);
    let (read, _) = answer(
        &opened,
        job,
        Ok(BlameAnswer {
            blame: a_blame(false, false),
            line: None,
        }),
    );
    assert_eq!(read.trail.top().expect("the place").row_count(), 30);
}

#[test]
fn esc_while_reading_cancels_the_job() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (back, commands) = press(&opened, KeyCode::Escape);
    assert!(!back.trail.is_live());
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
}

#[test]
fn blame_failed_shows_gits_words() {
    let state = pane_at(22);
    let (opened, commands) = press(&state, KeyCode::Char('B'));
    let (job, _) = blame_job(&commands);
    let (failed, _) = answer(
        &opened,
        job,
        Err(BlameFailure::Failed("fatal: no such path".to_owned())),
    );
    assert_eq!(
        failed.trail.top().expect("the place").body,
        BlameLoad::Failed("fatal: no such path".to_owned())
    );
}

// ---- is_write ----

#[test]
fn a_blame_read_is_not_a_write() {
    let (_, commands) = press(&pane_at(22), KeyCode::Char('B'));
    assert!(!commands.iter().any(is_write), "{commands:?}");
}

fn click(row: u16) -> AppEvent {
    use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    AppEvent::Core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 60,
        row,
        modifiers: Modifiers::NONE,
    }))
}

#[test]
fn mouse_double_click_is_enter() {
    let mut state = blamed_at(22);
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    // Row 0 is the title; screen row 5 is the fifth line (top 0).
    let (once, _) = update(&state, click(5));
    assert_eq!(cursor_line(&once), 5);
    assert!(once.trail.on_screen().is_some(), "one click only selects");
    let (twice, _) = update(&once, click(5));
    assert!(twice.trail.hidden(), "a double-click is `⏎`");
    assert_eq!(
        twice.line_landing.as_ref().map(|landing| landing.spec),
        Some(DetailSpec::Commit(mo()))
    );
}

#[test]
fn a_click_in_blame_never_reaches_the_graph() {
    let state = blamed_at(22);
    let cursor = |state: &AppState| state.history.as_ref().map(|history| history.cursor);
    for row in [0, 1, 5, 20, 39] {
        let (next, _) = update(&state, click(row));
        assert_eq!(cursor(&next), cursor(&state), "row {row} moved the graph");
        assert_eq!(next.detail, state.detail, "row {row} reached the pane");
    }
}

// ---- Hints ----

/// The keys the footer draws for `state`, at 200 columns.
fn bar(state: &AppState) -> Vec<(&'static str, &'static str)> {
    use graft_ui::graph_screen::{Divider, hints_for};
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(&context, Divider::Hidden, 200)
        .into_iter()
        .map(|hint| (hint.key, hint.verb))
        .collect()
}

#[test]
fn pane_bar_offers_b_blame() {
    assert!(
        bar(&pane_at(22)).contains(&("B", "blame")),
        "{:?}",
        bar(&pane_at(22))
    );
}

#[test]
fn pane_bar_hides_b_while_the_offer_is_up() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    assert!(state.commit_graph_actionable(), "the offer is up");
    assert!(!bar(&state).contains(&("B", "blame")), "{:?}", bar(&state));
}

#[test]
fn graph_bar_leads_esc_back_to_blame_while_hidden() {
    let state = blamed_at(22);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.trail.hidden());
    assert_eq!(
        bar(&landed).first(),
        Some(&("esc", "back to blame")),
        "{:?}",
        bar(&landed)
    );
}

#[test]
fn the_place_bar_offers_n_only_while_finding() {
    let finding = place_on_screen(pane_at(22), true, false);
    let not = place_on_screen(pane_at(22), false, false);
    assert!(bar(&finding).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("i", "ignore list")));
}

#[test]
fn a_live_global_passes_through_the_place() {
    assert!(live_global(KeyPress::plain(KeyCode::Char('q'))));
    assert!(!live_global(KeyPress::plain(KeyCode::Char('m'))));
}

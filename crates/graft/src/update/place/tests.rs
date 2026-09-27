//! The blame place and its trail (m7-power-tools/20), through `update` as
//! the event loop delivers keys and job answers. One file per box of 20's
//! acceptance; the builders every box shares live here.

mod esc_back;
mod hints;
mod keys;
mod mouse;
mod navigation;
mod open;
mod pinned_base;

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

/// The commit `pane_at`'s pane is on, and its first parent.
fn pane_commit(state: &AppState) -> (Oid, Oid) {
    let row = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .expect("a selected row");
    (row.commit.id, row.commit.parents[0])
}

/// A place over `state` reading `blame`, the cursor on row `cursor`.
fn place_with(state: &mut AppState, blame: Blame, cursor: usize) {
    let mut place = ready_place(blame);
    place.cursor = cursor;
    trail_over(state, place, true);
}

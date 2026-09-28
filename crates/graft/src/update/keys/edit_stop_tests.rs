//! `space` at a rewrite's `edit` stop (m7-power-tools/07d D1): stage-all on
//! the stopped row and the Uncommitted row, the fold everywhere else.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::oid::Oid;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ui::graph_screen::{Divider, Hint, LiveC, hints_for};

use super::edit_stop::space_stages_here;
use crate::update::events::Command;
use crate::update::state::AppState;
use crate::update::tests::{asked_view, merge_on_screen, operation_live_as};
use crate::update::{AppEvent, JobId, update};

/// Row 0 of `merge_on_screen`: a plain commit, `HEAD` here.
const HEAD_ROW: usize = 0;
/// Row 1: a merge.
const MERGE_ROW: usize = 1;

fn entry(path: &str, unstaged: Option<Change>, untracked: bool) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged,
        untracked,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

/// The two-row window with `HEAD` on row 0, the cursor on `cursor`, a
/// modified and an untracked file in the working tree; at an `edit` stop
/// of a rewrite Graft began when `edit` is set, nothing live otherwise.
fn screen(cursor: usize, edit: bool) -> AppState {
    let (mut state, _) = merge_on_screen(JobId::new(5), cursor);
    let head = Oid::from_bytes(&[1; Oid::SHA1_BYTES]).expect("20 bytes");
    state.refs = Some(Arc::new(crate::view::tests::refs_with_head(Some(head))));
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![
            entry("src/a.rs", Some(Change::Modified), false),
            entry("notes.txt", None, true),
        ],
    }));
    if edit {
        operation_live_as(&mut state, LiveC::ContinueAmends, true);
    }
    state
}

fn space(state: &AppState) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress {
            code: KeyCode::Char(' '),
            modifiers: Modifiers::NONE,
        })),
    )
}

/// What the changes pane's `Ctrl+A` sends from `state`: the one route,
/// `operate::stage_all`, run on a copy.
fn ctrl_a(state: &AppState) -> (AppState, Vec<Command>) {
    let mut next = state.clone();
    let mut commands = Vec::new();
    let mut local_jobs = next.local_jobs;
    assert!(
        crate::update::operate::stage_all(&mut next, &mut commands, &mut || local_jobs.next_id()),
        "Ctrl+A stages here"
    );
    next.local_jobs = local_jobs;
    (next, commands)
}

fn stages(commands: &[Command]) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::Operate(..)))
}

#[test]
fn space_on_the_stopped_row_stages_every_change() {
    let state = screen(HEAD_ROW, true);
    assert!(space_stages_here(&state));
    let (next, commands) = space(&state);
    let (want, sent) = ctrl_a(&state);
    assert_eq!(commands, sent, "the command Ctrl+A sends");
    assert_eq!(next.status, want.status, "Ctrl+A's words");
    assert_eq!(next.operating, want.operating);
}

#[test]
fn space_on_the_uncommitted_row_at_an_edit_stop_stages() {
    let mut state = screen(MERGE_ROW, true);
    state.on_working_tree = true;
    assert!(space_stages_here(&state));
    let (next, commands) = space(&state);
    let (want, sent) = ctrl_a(&state);
    assert_eq!(commands, sent, "the command Ctrl+A sends");
    assert_eq!(next.status, want.status, "Ctrl+A's words");
}

#[test]
fn space_on_a_merge_row_at_an_edit_stop_still_folds() {
    let state = screen(MERGE_ROW, true);
    assert!(!space_stages_here(&state));
    let (folded, commands) = space(&state);
    assert!(!stages(&commands), "{commands:?}");
    let merge = Oid::from_bytes(&[2; Oid::SHA1_BYTES]).expect("20 bytes");
    assert_eq!(asked_view(&commands).inclusion.folded, [merge].into());
    assert_eq!(
        folded.history.as_ref().map(|h| &h.view),
        Some(&asked_view(&commands))
    );
}

#[test]
fn space_outside_an_edit_stop_is_unchanged() {
    // The Uncommitted row, no stop: nothing staged, the fold's answer.
    let mut uncommitted = screen(MERGE_ROW, false);
    uncommitted.on_working_tree = true;
    assert!(!space_stages_here(&uncommitted));
    let (_, commands) = space(&uncommitted);
    assert!(!stages(&commands), "{commands:?}");

    // A merge row, no stop: the fold, as `space_folds_the_merge_under_the_cursor_and_unfolds_it`.
    let merge_row = screen(MERGE_ROW, false);
    let (_, commands) = space(&merge_row);
    let merge = Oid::from_bytes(&[2; Oid::SHA1_BYTES]).expect("20 bytes");
    assert_eq!(asked_view(&commands).inclusion.folded, [merge].into());

    // `HEAD`'s row, no stop: off a merge, `space` changes nothing.
    let head_row = screen(HEAD_ROW, false);
    let (next, commands) = space(&head_row);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next.history, head_row.history);
}

/// What the bar draws for `state` at 187 columns, where no hint goes bare.
fn bar(state: &AppState) -> Vec<Hint> {
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(&context, Divider::Hidden, 187)
}

/// The sweep (07d D1): edit stop, conflict stop, nothing live × the
/// stopped row, the Uncommitted row, a merge row × graph or banner focus.
/// The bar draws `space stage` exactly where `space` stages, and where it
/// does, `space` sends what `Ctrl+A` sends.
#[test]
fn the_bar_offers_space_stage_exactly_where_space_stages() {
    let mut swept = 0;
    let mut offered_anywhere = 0;
    for stop in [Some(LiveC::ContinueAmends), Some(LiveC::Continue), None] {
        for (row, uncommitted) in [(HEAD_ROW, false), (MERGE_ROW, true), (MERGE_ROW, false)] {
            for banner_focus in [false, true] {
                let mut state = screen(row, false);
                if let Some(c) = stop {
                    operation_live_as(&mut state, c, true);
                }
                state.on_working_tree = uncommitted;
                state.banner_focus = banner_focus;
                let what = (stop, row, uncommitted, banner_focus);
                let offered = bar(&state)
                    .iter()
                    .any(|hint| (hint.key, hint.verb) == ("space", "stage"));
                let (_, commands) = space(&state);
                assert_eq!(offered, stages(&commands), "{what:?}: {commands:?}");
                assert_eq!(offered, space_stages_here(&state), "{what:?}");
                if offered {
                    assert_eq!(commands, ctrl_a(&state).1, "{what:?}");
                    offered_anywhere += 1;
                }
                swept += 1;
            }
        }
    }
    assert_eq!(swept, 18);
    // The stopped row and the Uncommitted row, graph focused, edit stop.
    assert_eq!(offered_anywhere, 2);
}

#[test]
fn space_with_the_banner_focused_is_the_banners() {
    let mut on_head = screen(HEAD_ROW, true);
    on_head.banner_focus = true;
    assert!(!space_stages_here(&on_head), "the graph has no keyboard");
    let (next, commands) = space(&on_head);
    assert!(!stages(&commands), "{commands:?}");
    // The same as `space` with the banner focused at a stop that is not an
    // edit stop: the edit stop's arm takes nothing from the banner.
    let mut conflict_stop = screen(HEAD_ROW, false);
    operation_live_as(&mut conflict_stop, LiveC::Continue, true);
    conflict_stop.banner_focus = true;
    let (want, sent) = space(&conflict_stop);
    assert_eq!(commands, sent);
    assert_eq!(next.status, want.status);
    assert_eq!(next.history, want.history);
}

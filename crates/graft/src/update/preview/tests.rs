//! The Ghost Preview state machine, over scripted state.
//!
//! Nothing here opens a repository: what each key *means* is settled here,
//! and what a previewed operation *does* is `ops`' and is proved against
//! real git in `tests/it/`.

use graft_core::commit::{Identity, Timestamp};
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_ops::operation::{FastForward, MergeParams, RebaseParams};
use graft_ops::preview::{
    AlreadyIn, ConflictForecast, DropSuggestion, GhostCommit, HeadMove, RefMove, UndoNote,
    WorktreeEffect, ghost_oid,
};

use super::*;
use crate::history::{HistoryUpdate, Window};
use crate::update::{AppEvent, Progress, Size};

fn ids() -> impl FnMut() -> JobId {
    let mut next = 0;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn oid(n: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[19] = n;
    Oid::from_sha1(bytes)
}

fn who() -> Identity {
    Identity {
        name: b"Ada".to_vec(),
        email: b"ada@example.com".to_vec(),
        when: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
    }
}

fn row(id: u8, parents: &[u8]) -> Row {
    Row {
        commit: Commit {
            id: oid(id),
            parents: parents.iter().copied().map(oid).collect(),
            author: who(),
            committer: who(),
            message: format!("commit {id}").into_bytes(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    }
}

/// A screen big enough for the box, with three resident rows.
fn on_screen() -> AppState {
    let (state, _) = crate::update::opening(
        std::path::PathBuf::from("/tmp/graft-preview-tests"),
        Size {
            columns: 100,
            rows: 40,
        },
    );
    let id = state.awaiting.expect("opening mints the walk's id");
    // The walk announces itself before it sends rows; without this the
    // window has no `History` to land in.
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Started(id, crate::update::Task::ReadHistory),
    );
    let asked = state
        .history
        .as_ref()
        .expect("a walk that has started")
        .asked;
    let window = Window {
        request: asked,
        first: 0,
        rows: vec![row(3, &[2]), row(2, &[1]), row(1, &[0])].into(),
        laid_out: 3,
        done: true,
        promoted: false,
        fold: [].into(),
        anchor_behind: None,
    };
    let (state, _) = crate::update::update(
        &state,
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    );
    state
}

/// A plan that does something: one ghost, one branch moving onto it.
pub(in crate::update) fn a_plan() -> PreviewPlan {
    PreviewPlan {
        files: None,
        description: "rebase feature onto main".to_owned(),
        commands: vec!["git rebase --no-update-refs --no-autostash main".to_owned()],
        ghosts: vec![GhostCommit {
            oid: ghost_oid("replayed work", 0),
            parents: vec![Target::Real(oid(2))],
            subject: "replayed work".to_owned(),
            author: who(),
            replays: Some(oid(3)),
        }],
        ref_moves: vec![RefMove {
            name: "refs/heads/feature".to_owned(),
            from: Some(oid(3)),
            to: Some(Target::Ghost(0)),
        }],
        head: Some(HeadMove::ToBranch("refs/heads/feature".to_owned())),
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: vec![Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        }],
        set_aside: None,
        put_back: None,
        already_in: None,
        published: Vec::new(),
        left_behind: Vec::new(),
        left_behind_more: 0,
        forks: Vec::new(),
        forks_more: 0,
        messages: Vec::new(),
    }
}

/// A plan with nothing in it: a merge that is already up to date.
fn an_empty_plan() -> PreviewPlan {
    PreviewPlan {
        files: None,
        description: "merge main into feature".to_owned(),
        commands: vec!["git merge --no-edit --ff --no-edit main".to_owned()],
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
        already_in: None,
        published: Vec::new(),
        left_behind: Vec::new(),
        left_behind_more: 0,
        forks: Vec::new(),
        forks_more: 0,
        messages: Vec::new(),
    }
}

fn a_rebase() -> Operation {
    Operation::Rebase(RebaseParams::new(
        Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        },
        Source::Branch("main".to_owned()),
        oid(2),
    ))
}

fn subject() -> PreviewSubject {
    PreviewSubject::Operation {
        op: Box::new(a_rebase()),
        opened_by: "r",
    }
}

/// Opens a preview and answers its job with `plan`.
pub(in crate::update) fn opened_with(plan: PreviewPlan) -> (AppState, Vec<Command>) {
    opened_over(subject(), plan)
}

/// Opens a preview of `subject` and answers its job with `plan`.
fn opened_over(subject: PreviewSubject, plan: PreviewPlan) -> (AppState, Vec<Command>) {
    let mut state = on_screen();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject,
        false,
        &mut ids()
    ));
    let id = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        &mut state,
        id,
        Ok(Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: None,
        })),
    );
    (state, commands)
}

fn press(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::default(),
    }
}

fn labels(view: &PreviewBox) -> Vec<String> {
    view.choices()
        .iter()
        .map(|choice| choice.as_str().to_owned())
        .collect()
}

// ── opening and refusing ────────────────────────────────────────────────

/// A preview is modal: it claims every key, so nothing behind it moves.
#[test]
fn an_open_preview_claims_the_keys_behind_it() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let mut next = ids();
    for code in [
        KeyCode::Char('u'),
        KeyCode::Char('j'),
        KeyCode::Char('m'),
        KeyCode::Char('/'),
    ] {
        assert!(
            preview_key(&mut state, &mut commands, press(code), &mut next),
            "{code:?} must be claimed by the open preview"
        );
    }
    assert!(commands.is_empty(), "no key behind a preview starts a job");
    assert!(state.preview.is_some(), "and none of them closed it");
    assert!(state.search_prompt.is_none(), "`/` did not open a search");
}

/// `Ctrl+C` is the one key a preview does not claim.
#[test]
fn ctrl_c_is_not_claimed() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let key = KeyPress {
        code: KeyCode::Char('c'),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::default()
        },
    };
    assert!(!preview_key(&mut state, &mut commands, key, &mut ids()));
}

/// A write is running: the refusal is a problem line, never a `✓`.
#[test]
fn a_preview_refuses_while_a_write_is_running_and_says_so_as_a_problem() {
    let mut state = on_screen();
    state.operating = Some(JobId::new(9));
    let mut commands = Vec::new();
    assert!(!open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    assert!(state.preview.is_none());
    assert!(commands.is_empty());
    match &state.status {
        Status::Sticky { reason, .. } => assert!(reason.starts_with("wait for ")),
        other => panic!("a refusal must not read as a success: {other:?}"),
    }
}

/// git is part-way through a rebase: the refusal names it.
#[test]
fn a_preview_refuses_while_git_has_an_operation_in_progress() {
    let mut state = on_screen();
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Rebase,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    let mut commands = Vec::new();
    assert!(!open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    match &state.status {
        Status::Sticky { reason, .. } => {
            assert_eq!(reason, "finish or abort the rebase first");
        }
        other => panic!("expected a problem line, got {other:?}"),
    }
}

/// A second open over an open one changes nothing and says nothing: the
/// answer is already on screen.
#[test]
fn a_second_open_over_an_open_preview_is_silent() {
    let (mut state, _) = opened_with(a_plan());
    let was = state.status.clone();
    let mut commands = Vec::new();
    assert!(!open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    assert_eq!(state.status, was);
    assert!(commands.is_empty());
}

// ── the box ─────────────────────────────────────────────────────────────

/// While the plan is in flight the box says `predicting…` and the cursor
/// still rests on the safe choice (ADR-0038 §1).
#[test]
fn a_preview_in_flight_draws_the_planning_box_with_the_cursor_on_cancel() {
    let mut state = on_screen();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    let view = preview_box(&state).expect("a box while planning");
    assert!(view.planning);
    assert_eq!(view.selected(), 0);
    assert_eq!(labels(&view)[0], "Cancel");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::PlanPreview(..))),
        "{commands:?}"
    );
}

/// A plan that would do nothing offers no way to run it.
#[test]
fn a_plan_with_nothing_in_it_offers_close_only() {
    let (state, _) = opened_with(an_empty_plan());
    let view = preview_box(&state).expect("a box");
    assert_eq!(labels(&view), vec!["Close".to_owned()]);
    assert!(
        view.lines
            .iter()
            .any(|line| line.as_str() == "nothing to do"),
        "and it says why: {:?}",
        view.lines
    );
    // Review 08 Minor 7: it used to offer `undo: u takes this back` above
    // `nothing to do`, over a plan with nothing to take back.
    assert!(
        !view
            .lines
            .iter()
            .any(|line| line.as_str().starts_with("undo:")),
        "and offers no undo of a change it is not making: {:?}",
        view.lines
    );
}

/// The plan failed: the reason is on the box, and there is nothing to run.
#[test]
fn a_plan_that_failed_shows_the_reason_and_offers_close_only() {
    let mut state = on_screen();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    let id = state.preview.as_ref().expect("open").job;
    preview_produced(
        &mut state,
        id,
        Err(PreviewFailure::Failed(
            "main and feature have diverged".to_owned(),
        )),
    );
    let view = preview_box(&state).expect("a box");
    assert_eq!(labels(&view), vec!["Close".to_owned()]);
    assert_eq!(
        view.lines[0].as_str(),
        "preview failed: main and feature have diverged"
    );
}

/// A ghost whose real parent is neither resident nor an anchor: the box
/// says the picture cannot be drawn here, rather than drawing a shape the
/// user could misread (the conductor's binding condition; the slice's own
/// half is `slice::tests::slice_refuses_a_missing_ghost_parent`).
#[test]
fn a_preview_that_cannot_be_drawn_here_says_so_instead_of_drawing_it() {
    let mut plan = a_plan();
    plan.ghosts[0].parents = vec![Target::Real(oid(200))];
    let (state, _) = opened_with(plan);
    let preview = state.preview.as_ref().expect("open");
    assert_eq!(preview.undrawable, Some(oid(200)));
    assert!(preview.rows.is_empty(), "nothing is laid out");
    let view = preview_box(&state).expect("a box");
    assert!(
        view.lines[0]
            .as_str()
            .starts_with("preview cannot be drawn here:"),
        "{:?}",
        view.lines
    );
    assert_eq!(labels(&view), vec!["Close".to_owned()]);
}

/// Parents outside the slice are clipped, and the box says how many lanes
/// leave the view rather than letting them vanish.
#[test]
fn lanes_that_leave_the_view_are_counted_on_the_box() {
    let (state, _) = opened_with(a_plan());
    let preview = state.preview.as_ref().expect("open");
    assert!(
        preview.clipped > 0,
        "the oldest resident row's parent is outside the slice"
    );
    let view = preview_box(&state).expect("a box");
    assert!(
        view.lines
            .iter()
            .any(|line| line.as_str().ends_with("continue outside this view")),
        "{:?}",
        view.lines
    );
}

/// The plan's own `git` lines are what the box shows — never a second
/// rendering of the same command (`KNOWN_GAPS` row 229).
#[test]
fn the_box_shows_the_plans_own_command_lines() {
    let (state, _) = opened_with(a_plan());
    let view = preview_box(&state).expect("a box");
    let shown: Vec<String> = view
        .commands
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect();
    assert_eq!(shown, a_plan().commands);
}

// ── choosing ────────────────────────────────────────────────────────────

/// `→` moves to `Confirm` and `←` back; the cursor starts on the safe one.
#[test]
fn the_arrows_move_between_the_choices_and_start_on_the_safe_one() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let mut next = ids();
    assert_eq!(preview_box(&state).expect("a box").selected(), 0);
    preview_key(&mut state, &mut commands, press(KeyCode::Right), &mut next);
    assert_eq!(preview_box(&state).expect("a box").selected(), 1);
    preview_key(&mut state, &mut commands, press(KeyCode::Left), &mut next);
    assert_eq!(preview_box(&state).expect("a box").selected(), 0);
}

/// `Enter` on `Cancel` closes and runs nothing.
#[test]
fn enter_on_cancel_closes_and_runs_nothing() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    preview_key(&mut state, &mut commands, press(KeyCode::Enter), &mut ids());
    assert!(state.preview.is_none());
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
}

/// `Enter` on `Confirm` runs the **same** `Operation` the picture was drawn
/// from — the value is moved across, never rebuilt (design note hard
/// choice 3).
#[test]
fn confirm_runs_the_very_operation_the_picture_was_drawn_from() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let mut next = ids();
    preview_key(&mut state, &mut commands, press(KeyCode::Right), &mut next);
    preview_key(&mut state, &mut commands, press(KeyCode::Enter), &mut next);
    assert!(state.preview.is_none(), "the box closes as it acts");
    let operated = commands
        .iter()
        .find_map(|command| match command {
            Command::Operate(_, op, ..) => Some(op.as_ref()),
            _ => None,
        })
        .expect("confirm runs the operation");
    assert_eq!(operated, &a_rebase());
    assert!(state.operating.is_some(), "and it is a write in flight");
}

/// 05b: the key builds `r` with his branch alone; the job completes it (the
/// line, the drops) and answers it in `runs`. `y` runs **that** op — the
/// one the picture was drawn from — never the key's.
#[test]
fn the_confirmed_rebase_is_the_one_the_job_completed() {
    let mut state = on_screen();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut ids()
    ));
    let mut completed = a_rebase();
    if let Operation::Rebase(params) = &mut completed {
        params.line.branches.extend([
            Tip {
                name: "refs/heads/feature-b".to_owned(),
                oid: oid(4),
            },
            Tip {
                name: "refs/heads/feature-c".to_owned(),
                oid: oid(5),
            },
        ]);
    }
    let id = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        &mut state,
        id,
        Ok(Arc::new(Planned {
            plan: a_plan(),
            anchors: Vec::new(),
            runs: Some(Box::new(completed.clone())),
        })),
    );

    let mut next = ids();
    preview_key(
        &mut state,
        &mut commands,
        press(KeyCode::Char('y')),
        &mut next,
    );
    let operated = commands
        .iter()
        .find_map(|command| match command {
            Command::Operate(_, op, ..) => Some(op.as_ref()),
            _ => None,
        })
        .expect("y runs the operation");
    let Operation::Rebase(params) = operated else {
        panic!("a rebase: {operated:?}");
    };
    assert_eq!(params.line.branches.len(), 3, "the line the job completed");
    assert_eq!(operated, &completed);
}

/// A history operation cannot be stopped part-way, so the footer must not
/// offer `Esc` as a way out of one.
#[test]
fn a_confirmed_history_operation_is_not_offered_as_stoppable() {
    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let mut next = ids();
    preview_key(&mut state, &mut commands, press(KeyCode::Right), &mut next);
    preview_key(&mut state, &mut commands, press(KeyCode::Enter), &mut next);
    let running = state.remote.running.as_ref().expect("a running write");
    assert_eq!(
        running.stoppable,
        crate::update::remote::Stoppable::No(HISTORY_CANNOT_STOP)
    );
}

/// `Esc` closes, and cancels a plan that is still being made: nobody is
/// waiting for the answer any more.
#[test]
fn esc_closes_and_cancels_a_plan_still_in_flight() {
    let mut state = on_screen();
    let mut commands = Vec::new();
    let mut next = ids();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject(),
        false,
        &mut next
    ));
    let id = state.preview.as_ref().expect("open").job;
    commands.clear();
    preview_key(&mut state, &mut commands, press(KeyCode::Escape), &mut next);
    assert!(state.preview.is_none());
    assert!(
        commands.contains(&Command::CancelJob(id)),
        "{commands:?} must cancel the planning job"
    );
}

/// A plan that lands under a job id nobody is waiting for is dropped.
#[test]
fn a_plan_for_a_stale_job_is_ignored() {
    let (mut state, _) = opened_with(a_plan());
    let drawn = state.preview.as_ref().expect("open").rows.len();
    preview_produced(
        &mut state,
        JobId::new(4242),
        Err(PreviewFailure::Failed(
            "from a preview the user already closed".to_owned(),
        )),
    );
    let preview = state.preview.as_ref().expect("still open");
    assert_eq!(preview.rows.len(), drawn);
    assert!(matches!(preview.plan, Loading::Ready(_)));
}

// ── the small screen ────────────────────────────────────────────────────

/// Below `PREVIEW_MIN` the widget draws one line of text instead of a box.
/// `Enter` must not act on a picture nobody can see; `Esc` still closes.
#[test]
fn under_the_minimum_size_enter_does_nothing_and_esc_still_closes() {
    let (mut state, _) = opened_with(a_plan());
    state.size = Size {
        columns: 38,
        rows: 10,
    };
    assert!(!fits(&state));
    let mut commands = Vec::new();
    let mut next = ids();
    preview_key(&mut state, &mut commands, press(KeyCode::Right), &mut next);
    preview_key(&mut state, &mut commands, press(KeyCode::Enter), &mut next);
    assert!(state.preview.is_some(), "Enter is a no-op down here");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
    preview_key(&mut state, &mut commands, press(KeyCode::Escape), &mut next);
    assert!(state.preview.is_none(), "but Esc always closes");
}

/// The box takes rows out of the graph's viewport, so it covers rows the
/// graph never drew (the M5 group 04 review's Critical 1, from the other
/// side).
#[test]
fn an_open_preview_takes_its_rows_out_of_the_graph_viewport() {
    let plain = on_screen();
    let (open, _) = opened_with(a_plan());
    let taken = preview_rows_taken(&open);
    assert!(taken >= 7, "the box is at least a box: {taken}");
    assert_eq!(open.graph_rows() + usize::from(taken), plain.graph_rows());
}

// ── the mouse ───────────────────────────────────────────────────────────

/// The click targets agree with what the frame actually draws: the test
/// draws the whole frame with the box open, finds `[ Cancel ]` and
/// `[ Confirm ]` in the drawn grid, and clicks those cells through
/// `update`. At 40 (the box's narrowest), 80 and 120 columns.
///
/// The frame docks the box (`view/frame/boxes.rs`); `update` docks its own
/// copy for the hit test (`preview/rect.rs`). Reading the buttons off the
/// frame, not off `update`'s rect, is what lets this see the two disagree.
///
/// **Mutation:** dock `update`'s box one row off (`height - 1` in
/// `preview_box_rect`) and both clicks miss.
#[test]
fn a_click_lands_on_the_button_the_frame_drew() {
    for columns in [40, 80, 120] {
        let (mut state, _) = opened_with(a_plan());
        state.size.columns = columns;
        let grid = crate::view::tests::draw_cells(&state, columns, state.size.rows);
        let (row, cancel) = drawn_at(&grid, "[ Cancel ]");
        let (confirm_row, confirm) = drawn_at(&grid, "[ Confirm ]");
        assert_eq!(row, confirm_row, "{columns} columns: one buttons row");

        let (next, commands) = clicked(&state, confirm, row);
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..))),
            "{columns} columns: a click on Confirm runs it: {commands:?}"
        );
        assert!(next.preview.is_none(), "{columns} columns: Confirm closes");

        let (next, commands) = clicked(&state, cancel, row);
        assert!(
            next.preview.is_none(),
            "{columns} columns: a click on Cancel closes"
        );
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..))),
            "{columns} columns: Cancel runs nothing: {commands:?}"
        );
    }
}

/// A left click at `(column, row)`, through `update`.
fn clicked(state: &AppState, column: u16, row: u16) -> (AppState, Vec<Command>) {
    use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    crate::update::update(
        state,
        AppEvent::Core(Event::Mouse(MouseInput {
            kind: MouseKind::Down(MouseButton::Left),
            column,
            row,
            modifiers: Modifiers::NONE,
        })),
    )
}

/// The row `label` is drawn on, and a cell inside its word (past the
/// bracket and the space). Cells, not bytes: the border and the cursor's `▸`
/// are wider than one byte, so a byte offset would land right of the word.
fn drawn_at(grid: &[Vec<String>], label: &str) -> (u16, u16) {
    let letters: Vec<char> = label.chars().collect();
    let found = grid.iter().enumerate().find_map(|(y, cells)| {
        cells
            .windows(letters.len())
            .position(|window| {
                window
                    .iter()
                    .zip(&letters)
                    .all(|(cell, letter)| cell.chars().eq(std::iter::once(*letter)))
            })
            .map(|x| (y, x))
    });
    let (y, x) = found.unwrap_or_else(|| {
        let drawn: Vec<String> = grid.iter().map(|cells| cells.concat()).collect();
        panic!("{label} is drawn:\n{}", drawn.join("\n"))
    });
    let small = |n: usize| u16::try_from(n).expect("a small screen");
    (small(y), small(x + 2))
}

/// A click anywhere else is swallowed: the rows behind a modal picture are
/// not selectable, and a stray click must not dismiss a box being read.
#[test]
fn a_click_off_the_buttons_changes_nothing() {
    let (mut state, _) = opened_with(a_plan());
    let before = state.preview.clone();
    let mut commands = Vec::new();
    assert!(preview_click(&mut state, &mut commands, 0, 0, &mut ids()));
    assert_eq!(state.preview, before);
    assert!(commands.is_empty());
}

// ── `m` and `r` ─────────────────────────────────────────────────────────

/// Both keys refuse in words on a detached `HEAD` — there is no branch to
/// merge into or rebase.
#[test]
fn m_and_r_on_a_detached_head_say_what_is_missing() {
    for (which, verb) in [(HistoryKey::Merge, "m"), (HistoryKey::Rebase, "r")] {
        let mut state = on_screen();
        state.refs = Some(Arc::new(detached()));
        let mut commands = Vec::new();
        history_key(&mut state, &mut commands, which, &mut ids());
        assert!(state.preview.is_none());
        assert!(commands.is_empty());
        match &state.status {
            Status::Sticky { reason, .. } => {
                assert_eq!(reason, &format!("{verb} needs a branch checked out"));
            }
            other => panic!("expected a problem line, got {other:?}"),
        }
    }
}

/// On the uncommitted-changes row there is no commit to name.
#[test]
fn r_on_the_uncommitted_row_says_it_needs_a_commit() {
    let mut state = on_screen();
    state.on_working_tree = true;
    let mut commands = Vec::new();
    history_key(&mut state, &mut commands, HistoryKey::Rebase, &mut ids());
    assert!(state.preview.is_none());
    match &state.status {
        Status::Sticky { reason, .. } => assert_eq!(reason, "r needs a commit row"),
        other => panic!("expected a problem line, got {other:?}"),
    }
}

/// `m` is not `Ctrl+M` and not `Alt+m`.
#[test]
fn the_history_keys_are_the_plain_letters_only() {
    assert_eq!(
        is_history_key(press(KeyCode::Char('m'))),
        Some(HistoryKey::Merge)
    );
    assert_eq!(
        is_history_key(press(KeyCode::Char('r'))),
        Some(HistoryKey::Rebase)
    );
    assert_eq!(
        is_history_key(KeyPress {
            code: KeyCode::Char('r'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::default()
            },
        }),
        None,
        "Ctrl+R is refresh"
    );
    assert_eq!(is_history_key(press(KeyCode::Char('q'))), None);
}

/// `m` builds a merge of the row's branch into the current one, pinned to
/// the tip it was drawn against.
#[test]
fn m_builds_a_merge_of_the_selected_branch_into_the_current_one() {
    let (state, op) = history_key_over_two_branches(HistoryKey::Merge);
    assert!(state.preview.is_some());
    assert_eq!(
        op,
        Operation::Merge(MergeParams {
            source: Source::Branch("feature".to_owned()),
            into: Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            source_tip: oid(3),
            ff: FastForward::Default,
        })
    );
}

/// `r` builds a rebase of the current branch onto the selection.
#[test]
fn r_builds_a_rebase_of_the_current_branch_onto_the_selection() {
    let (_, op) = history_key_over_two_branches(HistoryKey::Rebase);
    assert_eq!(
        op,
        Operation::Rebase(RebaseParams::new(
            Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            Source::Branch("feature".to_owned()),
            oid(3),
        ))
    );
}

/// `main` checked out at `oid(2)`, `feature` at `oid(3)` under the cursor.
fn history_key_over_two_branches(which: HistoryKey) -> (AppState, Operation) {
    let mut state = on_screen();
    state.refs = Some(Arc::new(two_branches()));
    let mut commands = Vec::new();
    history_key(&mut state, &mut commands, which, &mut ids());
    let op = state
        .preview
        .as_ref()
        .and_then(|preview| preview.subject.operation())
        .cloned()
        .unwrap_or_else(|| panic!("{which:?} opened no preview: {:?}", state.status));
    (state, op)
}

fn name(full: &[u8]) -> RefName {
    RefName::new(full.to_vec()).expect("a valid ref name")
}

fn built(refs: &[(RefName, Oid)], head: &HeadState) -> RefBadges {
    crate::refs::badges(
        refs,
        head,
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here")
}

fn two_branches() -> RefBadges {
    built(
        &[
            (name(b"refs/heads/main"), oid(2)),
            (name(b"refs/heads/feature"), oid(3)),
        ],
        &HeadState::Branch(name(b"refs/heads/main")),
    )
}

/// m8/14: `m`/`r` on a stash row would merge or rebase onto the stash's
/// internal commit, named by sha. Refused in words; no preview opens.
#[test]
fn m_and_r_on_a_stash_row_are_refused_in_words() {
    for which in [HistoryKey::Merge, HistoryKey::Rebase] {
        let mut state = on_screen();
        let stash_row = state
            .history
            .as_ref()
            .and_then(crate::history::History::selected)
            .expect("on_screen selects a row")
            .commit
            .id;
        let refs = crate::refs::badges(
            &[(name(b"refs/heads/main"), oid(2))],
            &HeadState::Branch(name(b"refs/heads/main")),
            &std::collections::HashMap::new(),
            &[(stash_row, 0)],
            |_, _| Ok(graft_git::history::AheadBehind::default()),
        )
        .expect("no ahead/behind walk can fail here");
        state.refs = Some(Arc::new(refs));
        let mut commands = Vec::new();
        history_key(&mut state, &mut commands, which, &mut ids());
        assert!(state.preview.is_none(), "{which:?} opened a preview");
        assert!(commands.is_empty(), "{commands:?}");
        match &state.status {
            Status::Sticky { reason, .. } => {
                assert_eq!(reason, "a stash isn't a branch · ⏎ opens it");
            }
            other => panic!("expected a problem line, got {other:?}"),
        }
    }
}

fn detached() -> RefBadges {
    built(
        &[(name(b"refs/heads/feature"), oid(3))],
        &HeadState::Detached(oid(2)),
    )
}

/// Review 08 Minor 1 and the ticket's last acceptance line. The test above
/// calls [`preview_key`] directly, which proves the function is modal but
/// not that `key_event` reaches it before the global `u`. That ordering was
/// held by reading `keys.rs`; this holds it by pressing the key.
#[test]
fn u_over_an_open_preview_does_nothing_through_the_real_key_path() {
    let (state, _) = opened_with(a_plan());
    assert!(state.preview.is_some(), "a preview is open to type over");

    let (after, commands) = crate::update::update(
        &state,
        AppEvent::Core(graft_core::event::Event::Key(KeyPress::plain(
            KeyCode::Char('u'),
        ))),
    );

    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Undo(..) | Command::Redo(..))),
        "the global `u` must not fire behind a picture: {commands:?}"
    );
    assert!(after.preview.is_some(), "and the preview is still open");
    assert!(after.operating.is_none(), "nothing was launched");
}

// ── confirm keys: `y`, and `m m` / `r r` (m6-conflicts/11) ───────────────

fn opened_by(opener: &'static str) -> PreviewSubject {
    PreviewSubject::Operation {
        op: Box::new(a_rebase()),
        opened_by: opener,
    }
}

fn operated(commands: &[Command]) -> Vec<&Operation> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Operate(_, op, ..) => Some(op.as_ref()),
            _ => None,
        })
        .collect()
}

fn char_key(c: char) -> KeyPress {
    press(KeyCode::Char(c))
}

/// Presses `key` on a fresh `opener`-opened preview whose plan landed.
fn pressed_on(opener: &'static str, plan: PreviewPlan, key: KeyPress) -> (AppState, Vec<Command>) {
    let (mut state, _) = opened_over(opened_by(opener), plan);
    let mut commands = Vec::new();
    assert!(preview_key(&mut state, &mut commands, key, &mut ids()));
    (state, commands)
}

#[test]
fn the_opening_key_again_confirms() {
    let (state, _) = opened_over(opened_by("m"), a_plan());
    assert_eq!(
        preview_box(&state).expect("a box").selected(),
        0,
        "on Cancel"
    );
    let (state, commands) = pressed_on("m", a_plan(), char_key('m'));
    assert_eq!(operated(&commands), vec![&a_rebase()]);
    assert!(state.preview.is_none(), "the box closes as it acts");
}

#[test]
fn y_confirms_every_operation_preview() {
    for opener in ["m", "r", ".", "b"] {
        let (_, commands) = pressed_on(opener, a_plan(), char_key('y'));
        assert_eq!(operated(&commands), vec![&a_rebase()], "opened by {opener}");
    }
}

#[test]
fn y_does_not_confirm_a_journal_revert() {
    let subject = PreviewSubject::RevertTo(graft_core::journal::EntryId::new(1, [0; 10]));
    let (mut state, _) = opened_over(subject, a_plan());
    let mut commands = Vec::new();
    preview_key(&mut state, &mut commands, char_key('y'), &mut ids());
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
    let view = preview_box(&state).expect("a box");
    assert!(!view.confirm_key && view.again.is_none());
}

#[test]
fn b_again_does_not_confirm() {
    let (state, commands) = pressed_on("b", a_plan(), char_key('b'));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
}

#[test]
fn the_opening_key_waits_for_the_picture() {
    for key in ['m', 'y'] {
        let mut state = on_screen();
        let mut commands = Vec::new();
        assert!(open_preview(
            &mut state,
            &mut commands,
            opened_by("m"),
            false,
            &mut ids()
        ));
        let before = state.status.clone();
        commands.clear();
        preview_key(&mut state, &mut commands, char_key(key), &mut ids());
        assert!(commands.is_empty(), "{key}: {commands:?}");
        assert!(state.preview.is_some(), "{key}");
        assert_eq!(state.status, before, "{key}");
        assert!(
            preview_box(&state).expect("a box").planning,
            "{key}: predicting…"
        );
    }
}

#[test]
fn the_confirm_key_does_not_close_a_preview_that_cannot_act() {
    let (state, commands) = pressed_on("m", an_empty_plan(), char_key('y'));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
}

/// Review m6-amend-11-12 Minor 1 (`02-keymap.md:111`: a key the menu
/// displays must work): the box names `y confirm` and the opening verb only
/// where they act — a plan that landed and can act — or while planning,
/// where the key is waiting for the picture, not dead. A closing box (failed
/// or undrawable) and a can't-act plan draw neither. And every box that
/// draws `y confirm` over a landed plan acts on `y`.
#[test]
fn y_confirm_is_drawn_only_where_y_acts() {
    let failed = {
        let mut state = on_screen();
        let mut commands = Vec::new();
        assert!(open_preview(
            &mut state,
            &mut commands,
            opened_by("m"),
            false,
            &mut ids()
        ));
        let id = state.preview.as_ref().expect("open").job;
        preview_produced(
            &mut state,
            id,
            Err(PreviewFailure::Failed("diverged".to_owned())),
        );
        state
    };
    let undrawable = {
        let (mut state, _) = opened_over(opened_by("m"), a_plan());
        state.preview.as_mut().expect("open").undrawable = Some(oid(200));
        state
    };
    let (cannot_act, _) = opened_over(opened_by("m"), an_empty_plan());
    for (what, state) in [
        ("failed", &failed),
        ("undrawable", &undrawable),
        ("cannot act", &cannot_act),
    ] {
        let view = preview_box(state).expect("a box");
        assert!(!view.confirm_key, "{what}: `y confirm` drawn");
        assert!(view.again.is_none(), "{what}: `m merge` drawn");
        // And `y` really does nothing there: no drawn key is dead.
        let mut state = state.clone();
        let mut commands = Vec::new();
        preview_key(&mut state, &mut commands, char_key('y'), &mut ids());
        assert!(operated(&commands).is_empty(), "{what}: {commands:?}");
    }

    let (acting, _) = opened_over(opened_by("m"), a_plan());
    let view = preview_box(&acting).expect("a box");
    assert!(view.confirm_key && view.again.is_some(), "a plan that acts");
    let mut state = acting;
    let mut commands = Vec::new();
    preview_key(&mut state, &mut commands, char_key('y'), &mut ids());
    assert_eq!(operated(&commands), vec![&a_rebase()], "drawn, so it acts");

    let mut planning = on_screen();
    assert!(open_preview(
        &mut planning,
        &mut Vec::new(),
        opened_by("m"),
        false,
        &mut ids()
    ));
    let view = preview_box(&planning).expect("a box");
    assert!(
        view.planning && view.confirm_key,
        "while planning the key is waiting, not dead"
    );
}

#[test]
fn the_cursor_still_opens_on_cancel() {
    let (state, commands) = pressed_on("m", a_plan(), press(KeyCode::Enter));
    assert!(operated(&commands).is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
}

#[test]
fn the_other_verb_stays_swallowed() {
    let (state, commands) = pressed_on("m", a_plan(), char_key('r'));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
}

#[test]
fn confirm_key_refuses_modifiers() {
    let with = |c: char, control: bool, alt: bool| KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers {
            control,
            alt,
            ..Modifiers::default()
        },
    };
    assert!(!confirm_key(with('m', true, false), "m"));
    assert!(!confirm_key(with('r', false, true), "r"));
    assert!(!confirm_key(with('y', true, false), "m"));
    assert!(confirm_key(with('y', false, false), "."));
    assert!(confirm_key(with('r', false, false), "r"));
}

// ── a refused preview opens no box (m6-conflicts/67) ─────────────────────

/// Opens `subject`'s preview and answers its job with `failure`.
fn answered_with(subject: PreviewSubject, failure: PreviewFailure) -> (AppState, AppState) {
    let mut state = on_screen();
    let before = state.clone();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        subject,
        false,
        &mut ids()
    ));
    let id = state.preview.as_ref().expect("open").job;
    preview_produced(&mut state, id, Err(failure));
    (before, state)
}

/// `r` on a dirty tree: the job answers with a plan that sets the edits
/// aside (`graft_ops::preview::plan` no longer refuses a dirty rebase,
/// ADR-0045 §4), and the box opens with no refusal on the status line.
/// Flipped by m6-conflicts/73 from 67's refusal; 74 asserts the row's words.
#[test]
fn r_on_a_dirty_tree_opens_the_preview() {
    let plan = PreviewPlan {
        set_aside: Some(graft_ops::operation::SetAside { files: 1 }),
        ..a_plan()
    };
    let (state, _) = opened_over(opened_by("r"), plan);
    assert!(state.preview.is_some(), "the box is open");
    assert!(preview_box(&state).is_some(), "the box is drawn");
    assert!(
        !matches!(state.status, Status::Sticky { .. }),
        "no refusal: {:?}",
        state.status
    );
}

/// m6-conflicts/74: a plan that sets edits aside draws one row right under
/// `working tree:`, worded from its file count — whole at 120 columns,
/// `put back after` gone at 40, the file count kept at both. A plan that
/// sets nothing aside draws no such row.
#[test]
fn the_set_aside_row_sits_under_the_working_tree_line() {
    let plan = PreviewPlan {
        set_aside: Some(graft_ops::operation::SetAside { files: 3 }),
        ..a_plan()
    };
    let (mut state, _) = opened_over(opened_by("r"), plan);
    for (columns, want) in [
        (120, "your edits: 3 files set aside, put back after"),
        (40, "your edits: 3 files set aside"),
    ] {
        state.size = Size { columns, rows: 40 };
        let view = preview_box(&state).expect("a box");
        let lines: Vec<&str> = view.lines.iter().map(SanitizedText::as_str).collect();
        let tree = lines
            .iter()
            .position(|line| line.starts_with("working tree:"))
            .expect("a working tree line");
        assert_eq!(lines.get(tree + 1), Some(&want), "{columns}: {lines:?}");
    }

    let (clean, _) = opened_over(opened_by("r"), a_plan());
    let view = preview_box(&clean).expect("a box");
    assert!(
        view.lines
            .iter()
            .all(|line| !line.as_str().contains("your edits")),
        "{:?}",
        view.lines
    );
}

/// A merge the preview refuses by itself (`--ff-only` on diverged
/// branches) takes the same route: the status line, not the
/// `preview failed:` box.
#[test]
fn a_refused_merge_preview_says_why_on_the_status_line() {
    let refusal = graft_ops::operation::Refusal::NotFastForward {
        branch: "main".to_owned(),
    };
    let merge = PreviewSubject::Operation {
        op: Box::new(Operation::Merge(MergeParams {
            source: Source::Branch("refs/heads/feature".to_owned()),
            into: Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            source_tip: oid(3),
            ff: FastForward::Only,
        })),
        opened_by: "m",
    };
    let (_, state) = answered_with(merge.clone(), PreviewFailure::Refused(refusal.clone()));
    assert!(state.preview.is_none(), "a box is open");
    let PreviewSubject::Operation { op, .. } = merge else {
        unreachable!("built as an operation above")
    };
    assert_eq!(
        state.status,
        crate::update::operate::refusal_status(&op, &refusal, state.size.columns, &state.keymap)
    );
}

/// A read that failed is not a refusal: it keeps its `preview failed:` box.
#[test]
fn a_failed_preview_still_opens_the_failed_box() {
    let (_, state) = answered_with(
        opened_by("r"),
        PreviewFailure::Failed("git exited 128".to_owned()),
    );
    let view = preview_box(&state).expect("the failed box");
    assert!(
        view.lines
            .iter()
            .any(|line| line.as_str().contains("preview failed: git exited 128")),
        "{:?}",
        view.lines
    );
}

/// A preview opened from the journal gives the journal back when refused,
/// as `esc` would.
#[test]
fn a_refused_preview_from_the_journal_gives_the_journal_back() {
    let mut state = on_screen();
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        opened_by("r"),
        true,
        &mut ids()
    ));
    let id = state.preview.as_ref().expect("open").job;
    preview_produced(
        &mut state,
        id,
        Err(PreviewFailure::Refused(
            graft_ops::operation::Refusal::DirtyTree { count: 2 },
        )),
    );
    assert!(state.preview.is_none());
    assert!(state.journal.shown);
}

/// One wording: a dirty tree refused by the preview (before the box) and
/// refused by the write (after the confirm) leaves the same status line.
#[test]
fn the_preview_refusal_and_the_write_refusal_read_the_same() {
    let refusal = graft_ops::operation::Refusal::DirtyTree { count: 1 };
    let (_, previewed) = answered_with(opened_by("r"), PreviewFailure::Refused(refusal.clone()));

    let mut written = AppState::default();
    let mut commands = Vec::new();
    crate::update::operate::launch_unstoppable(
        &mut written,
        &mut commands,
        a_rebase(),
        HISTORY_CANNOT_STOP,
        Some("r"),
        &mut || JobId::new(9),
    )
    .expect("no write is running");
    let _refresh: Vec<Command> = crate::update::operate::operated(
        &mut written,
        JobId::new(9),
        crate::jobs::operate::Operated::Failed {
            error: "the sentence nobody should see here".into(),
            refresh: None,
            remote: None,
            refused: Some(refusal),
            edits: None,
        },
        &mut || JobId::new(10),
    );
    assert_eq!(previewed.status, written.status);
    assert!(
        matches!(&previewed.status, Status::Sticky { reason, .. } if reason == "rebase needs a clean tree"),
        "{:?}",
        previewed.status
    );
}

/// Review m6-final-fixes-74 M2: the conflict line folds the put-back's own
/// forecast, one row per line of the note's table ("Preview: the conflict
/// line folds the put-back").
#[test]
fn the_conflict_line_folds_the_put_back() {
    let clash = || ConflictForecast::Likely {
        paths: vec![std::path::PathBuf::from("base.txt")],
    };
    let unchecked = || ConflictForecast::NotChecked("the rebase may stop first".to_owned());
    let clean = ConflictForecast::Clean;
    // Clean, and nothing set aside or the put-back clean too.
    assert_eq!(conflict_line(&clean, None), "no conflicts expected");
    assert_eq!(
        conflict_line(&clean, Some(&ConflictForecast::Clean)),
        "no conflicts expected"
    );
    // Clean, the put-back may clash.
    assert_eq!(
        conflict_line(&clean, Some(&clash())),
        "your edits may clash when put back: base.txt"
    );
    // Clean, the put-back not checked.
    assert_eq!(
        conflict_line(&clean, Some(&unchecked())),
        "your edits' put-back: not checked (the rebase may stop first)"
    );
    // The operation may stop: its own line, then the put-back not checked.
    assert_eq!(
        conflict_line(&clash(), Some(&unchecked())),
        "may conflict: base.txt \u{b7} put-back not checked"
    );
    assert_eq!(
        conflict_line(
            &ConflictForecast::NotChecked("x".to_owned()),
            Some(&unchecked())
        ),
        "conflicts: not checked (x) \u{b7} put-back not checked"
    );
    // Nothing set aside: the operation's line as before.
    assert_eq!(conflict_line(&clash(), None), "may conflict: base.txt");
}

// ── the chain rows (m7/05) ──────────────────────────────────────────────

/// `a_plan` with `topic` built on `feature`: two ghosts, two branches.
fn a_line_plan() -> PreviewPlan {
    let mut plan = a_plan();
    plan.ghosts.push(GhostCommit {
        oid: ghost_oid("topic work", 1),
        parents: vec![Target::Ghost(0)],
        subject: "topic work".to_owned(),
        author: who(),
        replays: Some(oid(4)),
    });
    plan.ref_moves.push(RefMove {
        name: "refs/heads/topic".to_owned(),
        from: Some(oid(4)),
        to: Some(Target::Ghost(1)),
    });
    plan
}

fn drawn(plan: &PreviewPlan, op: &Operation, columns: u16) -> Vec<String> {
    plan_lines(
        plan,
        Some(op),
        false,
        0,
        columns,
        graft_ui::keymap::Keymap::shipped(),
    )
    .iter()
    .map(|line| line.as_str().to_owned())
    .collect()
}

#[test]
fn the_box_draws_one_row_per_branch() {
    let lines = drawn(&a_line_plan(), &a_rebase(), 187);
    let feature = lines
        .iter()
        .position(|line| line.starts_with("branches: ") && line.contains("feature"))
        .unwrap_or_else(|| panic!("a branches: row for feature in {lines:#?}"));
    let topic = &lines[feature + 1];
    assert!(
        topic.starts_with("          ") && topic.contains("topic"),
        "{lines:#?}"
    );
    assert!(topic.ends_with("built on yours \u{b7} follows"), "{topic}");
}

#[test]
fn a_one_branch_rebase_draws_as_before() {
    let lines = drawn(&a_plan(), &a_rebase(), 187);
    assert!(
        lines.iter().all(|line| !line.starts_with("branches: ")),
        "{lines:#?}"
    );
}

#[test]
fn already_in_row_says_dropped() {
    let mut plan = a_line_plan();
    plan.already_in = Some(DropSuggestion::Drop(AlreadyIn {
        branch: RefName::new("refs/heads/feature").expect("a ref name"),
        commits: vec![oid(3), oid(5), oid(6)],
        matches: oid(9),
        subject: "Add OAuth (#42)".to_owned(),
    }));
    let lines = drawn(&plan, &a_rebase(), 187);
    let row = lines
        .iter()
        .find(|line| line.starts_with("already in main: "))
        .unwrap_or_else(|| panic!("an already-in row in {lines:#?}"));
    assert_eq!(
        row,
        &format!(
            "already in main: feature's 3 commits match {} \"Add OAuth (#42)\" \u{b7} dropped",
            short(oid(9))
        )
    );
    // Not live here (drawn with `keeps` off): a row naming an unbound key
    // would be false.
    assert!(!row.contains(" R "), "{row}");
}

fn a_stop(plan: &mut PreviewPlan) {
    plan.conflicts = ConflictForecast::StopsAt {
        step: 2,
        of: 4,
        commit: oid(4),
        paths: vec![std::path::PathBuf::from("src/auth/session.rs")],
    };
}

#[test]
fn a_stop_forecast_names_the_step() {
    let mut plan = a_line_plan();
    a_stop(&mut plan);
    let lines = drawn(&plan, &a_rebase(), 187);
    let want = format!(
        "may stop at step 2 of 4 (pick {} topic work): src/auth/session.rs \u{b7} not checked past it",
        short(oid(4))
    );
    assert!(lines.contains(&want), "{want} in {lines:#?}");
}

/// At 80 columns the stop row loses its paths first; the step and the
/// branch names are never cut.
#[test]
fn cut_order_never_cuts_a_branch_name() {
    let mut plan = a_line_plan();
    a_stop(&mut plan);
    plan.published = vec![RefName::new("refs/heads/feature").expect("a ref name")];
    let lines = drawn(&plan, &a_rebase(), 80);
    let stop = lines
        .iter()
        .find(|line| line.starts_with("may stop at step 2 of 4"))
        .unwrap_or_else(|| panic!("a stop row in {lines:#?}"));
    assert!(!stop.contains("session.rs"), "paths cut first: {stop}");
    // The branch rows themselves, not any row: the stop row's subject
    // "topic work" would otherwise stand in for a missing topic row.
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("branches: feature ")),
        "{lines:#?}"
    );
    assert!(
        lines.iter().any(|line| line.starts_with("          topic ")
            && line.ends_with("built on yours \u{b7} follows")),
        "{lines:#?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line == "on origin: feature \u{b7} P will need force after"),
        "{lines:#?}"
    );
}

// ── `R` keeps a suggested drop (rebase-preview-keeps-drops) ──────────────

/// `feature` (at `oid(3)`) checked out, `main` at `oid(2)`.
fn feature_checked_out() -> RefBadges {
    built(
        &[
            (name(b"refs/heads/main"), oid(2)),
            (name(b"refs/heads/feature"), oid(3)),
        ],
        &HeadState::Branch(name(b"refs/heads/feature")),
    )
}

/// `a_rebase` as the job completed it: `oid(3)` suggested as a drop.
fn a_squash_merged_rebase() -> Operation {
    let mut op = a_rebase();
    if let Operation::Rebase(params) = &mut op {
        params.drops = vec![oid(3)];
    }
    op
}

fn a_drop_suggestion() -> DropSuggestion {
    DropSuggestion::Drop(AlreadyIn {
        branch: RefName::new("refs/heads/feature").expect("a ref name"),
        commits: vec![oid(3)],
        matches: oid(9),
        subject: "Add OAuth (#42)".to_owned(),
    })
}

/// A preview opened by `opener` over `op`, on `refs`, answered with a plan
/// carrying `suggestion` and `op` as the op `y` runs.
fn landed(
    refs: RefBadges,
    opener: &'static str,
    op: Operation,
    suggestion: Option<DropSuggestion>,
) -> AppState {
    let mut state = on_screen();
    state.refs = Some(Arc::new(refs));
    let mut commands = Vec::new();
    assert!(open_preview(
        &mut state,
        &mut commands,
        PreviewSubject::Operation {
            op: Box::new(op.clone()),
            opened_by: opener,
        },
        false,
        &mut ids()
    ));
    let mut plan = a_plan();
    plan.already_in = suggestion;
    let id = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        &mut state,
        id,
        Ok(Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: Some(Box::new(op)),
        })),
    );
    state
}

/// A preview over the rebase `op` opened on `state` (opener `r`) and
/// landed with [`a_drop_suggestion`] and `op` as its as-run op: where the
/// rebased branch is `HEAD`'s, `R` is live in it. For the rewrite editor's
/// open-race tests (30n Q2), which bring their own screen and ids.
pub(crate) fn land_a_keep_drops_preview(
    state: &mut AppState,
    op: Operation,
    next_id: &mut impl FnMut() -> JobId,
) {
    let mut commands = Vec::new();
    assert!(open_preview(
        state,
        &mut commands,
        PreviewSubject::Operation {
            op: Box::new(op.clone()),
            opened_by: "r",
        },
        false,
        next_id
    ));
    let mut plan = a_plan();
    plan.already_in = Some(a_drop_suggestion());
    let id = state.preview.as_ref().expect("a preview is open").job;
    preview_produced(
        state,
        id,
        Ok(Arc::new(Planned {
            plan,
            anchors: Vec::new(),
            runs: Some(Box::new(op)),
        })),
    );
}

fn opened_rewrites(commands: &[Command]) -> Vec<&graft_ops::rewrite::RewriteRequest> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::OpenRewrite { req, .. } => Some(req),
            _ => None,
        })
        .collect()
}

fn press_r(state: &mut AppState) -> Vec<Command> {
    let mut commands = Vec::new();
    assert!(preview_key(state, &mut commands, char_key('R'), &mut ids()));
    commands
}

#[test]
fn r_in_a_squash_merged_preview_opens_the_editor_with_drops_marked() {
    let op = a_squash_merged_rebase();
    let mut state = landed(
        feature_checked_out(),
        "r",
        op.clone(),
        Some(a_drop_suggestion()),
    );

    let commands = press_r(&mut state);

    let Operation::Rebase(params) = op else {
        panic!("a rebase");
    };
    assert_eq!(
        opened_rewrites(&commands),
        [&graft_ops::rewrite::RewriteRequest::FromRebase(params)],
        "the as-run params, drops and all: {commands:?}"
    );
    assert!(state.preview.is_none(), "the preview closes");
    assert!(state.rewrite.opening.is_some(), "the editor's read is out");
    assert!(
        matches!(&state.status, Status::Waiting(words) if words == "reading feature\u{2026}"),
        "{:?}",
        state.status
    );
}

#[test]
fn r_is_unbound_in_other_previews() {
    let merge = Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/main".to_owned()),
        into: Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        },
        source_tip: oid(2),
        ff: FastForward::Default,
    });
    let cases = vec![
        (
            "not checked",
            landed(
                feature_checked_out(),
                "r",
                a_squash_merged_rebase(),
                Some(DropSuggestion::NotChecked("an old version".to_owned())),
            ),
        ),
        (
            "no suggestion",
            landed(feature_checked_out(), "r", a_squash_merged_rebase(), None),
        ),
        (
            "a suggestion but no drops",
            landed(
                feature_checked_out(),
                "r",
                a_rebase(),
                Some(a_drop_suggestion()),
            ),
        ),
        (
            "a merge",
            landed(feature_checked_out(), "m", merge, Some(a_drop_suggestion())),
        ),
    ];
    for (case, mut state) in cases {
        let commands = press_r(&mut state);
        assert!(
            opened_rewrites(&commands).is_empty(),
            "{case}: {commands:?}"
        );
        assert!(state.preview.is_some(), "{case}: swallowed, the box stays");
        let view = preview_box(&state).expect("a box");
        assert!(!view.keep, "{case}: no `R keep` in the footer");
        assert!(
            view.lines
                .iter()
                .all(|line| !line.as_str().contains("R to keep")),
            "{case}"
        );
    }
}

/// The undrawable box (`preview cannot be drawn here: … · esc`) names no
/// `R`, so `R` does nothing there: key, row and footer agree (review
/// 08..rebase-keeps-drops m1).
#[test]
fn r_is_unbound_when_the_preview_cannot_be_drawn() {
    let mut state = landed(
        feature_checked_out(),
        "r",
        a_squash_merged_rebase(),
        Some(a_drop_suggestion()),
    );
    state.preview.as_mut().expect("open").undrawable = Some(oid(200));

    let commands = press_r(&mut state);

    assert!(opened_rewrites(&commands).is_empty(), "{commands:?}");
    assert!(state.preview.is_some(), "swallowed, the box stays");
    assert!(state.rewrite.opening.is_none());
    assert!(!preview_box(&state).expect("a box").keep);
}

#[test]
fn r_is_unbound_when_the_rebased_branch_is_not_head() {
    let main_checked_out = built(
        &[
            (name(b"refs/heads/main"), oid(2)),
            (name(b"refs/heads/feature"), oid(3)),
        ],
        &HeadState::Branch(name(b"refs/heads/main")),
    );
    let mut state = landed(
        main_checked_out,
        "r",
        a_squash_merged_rebase(),
        Some(a_drop_suggestion()),
    );

    let commands = press_r(&mut state);

    assert!(opened_rewrites(&commands).is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
    assert!(!preview_box(&state).expect("a box").keep);
}

#[test]
fn the_row_names_r_only_when_live() {
    let row_of = |state: &AppState| {
        preview_box(state)
            .expect("a box")
            .lines
            .iter()
            .map(|line| line.as_str().to_owned())
            .find(|line| line.starts_with("already in main: "))
            .expect("an already-in row")
    };
    let live = landed(
        feature_checked_out(),
        "r",
        a_squash_merged_rebase(),
        Some(a_drop_suggestion()),
    );
    let row = row_of(&live);
    assert!(
        row.ends_with("\u{b7} dropped \u{b7} R to keep them"),
        "{row}"
    );
    assert!(preview_box(&live).expect("a box").keep, "and the footer");

    let not_live = landed(
        feature_checked_out(),
        "r",
        a_rebase(),
        Some(a_drop_suggestion()),
    );
    let row = row_of(&not_live);
    assert!(row.ends_with("\u{b7} dropped"), "{row}");
    assert!(!preview_box(&not_live).expect("a box").keep);
}

// ── left behind (m7/37) ─────────────────────────────────────────────────

fn left(name: &str, on: u8) -> graft_ops::preview::LeftBehind {
    graft_ops::preview::LeftBehind {
        branch: RefName::new(format!("refs/heads/{name}").as_bytes()).expect("a ref name"),
        on: Oid::from_hex(&format!("dd21dca{on:033x}")).expect("an id"),
    }
}

/// One, two, ten: each rung's exact words, longest first.
#[test]
fn left_behind_rungs_table() {
    use super::line_rows::left_behind_rungs;

    assert_eq!(left_behind_rungs(&[], 0), Vec::<String>::new(), "none");
    assert_eq!(
        left_behind_rungs(&[left("feature-b", 1)], 0),
        [
            "left behind: feature-b stays on the old dd21dca",
            "left behind: feature-b",
        ],
        "one"
    );
    assert_eq!(
        left_behind_rungs(&[left("feature-b", 1), left("feature-c", 2)], 0),
        [
            "left behind: feature-b, feature-c stay on old commits",
            "left behind: feature-b, feature-c",
            "left behind: feature-b +1",
        ],
        "two"
    );
    let ten: Vec<_> = (0..8).map(|k| left(&format!("b{k}"), k)).collect();
    assert_eq!(
        left_behind_rungs(&ten, 2),
        [
            "left behind: b0, b1, b2, b3, b4, b5, b6, b7 +2 stay on old commits",
            "left behind: b0, b1, b2, b3, b4, b5, b6, b7 +2",
            "left behind: b0 +9",
        ],
        "ten: eight named, two counted"
    );
}

// ── fork rows (m8/14c, ADR-0059 §2) ─────────────────────────────────────

fn at(hex7: &str) -> Oid {
    Oid::from_hex(&format!("{hex7}{}", "0".repeat(33))).expect("an id")
}

fn branch(name: &str) -> RefName {
    RefName::new(format!("refs/heads/{name}").as_bytes()).expect("a ref name")
}

/// `a_line_plan` with two forks off it: `feature-x` follows (its ref moves
/// with the line), `feature-y` stays because it would conflict, one more
/// past the cap, and `backup` parked at a tip (no fork, left behind).
pub(in crate::update) fn a_forked_plan() -> PreviewPlan {
    use graft_ops::preview::{Fork, ForkFate, LeftBehind, WhyStays};
    let mut plan = a_line_plan();
    plan.ref_moves.push(RefMove {
        name: "refs/heads/feature-x".to_owned(),
        from: Some(oid(5)),
        to: Some(Target::Ghost(1)),
    });
    plan.forks = vec![
        Fork {
            branch: branch("feature-x"),
            at: at("a1b2c3d"),
            commits: 2,
            fate: ForkFate::Follows {
                onto: Target::Ghost(0),
            },
        },
        Fork {
            branch: branch("feature-y"),
            at: at("b2c3d4e"),
            commits: 1,
            fate: ForkFate::Stays(WhyStays::WouldConflict {
                commit: oid(6),
                paths: vec![std::path::PathBuf::from("src/a.rs")],
            }),
        },
    ];
    plan.forks_more = 1;
    plan.left_behind = vec![
        LeftBehind {
            branch: branch("feature-y"),
            on: at("b2c3d4e"),
        },
        LeftBehind {
            branch: branch("backup"),
            on: at("c3d4e5f"),
        },
    ];
    plan.left_behind_more = 1;
    plan
}

/// A rewrite (`a`, `R`, absorb, reword-head, the editor all run one).
fn a_rewrite() -> Operation {
    Operation::Rewrite(graft_ops::rewrite::RewriteParams {
        head: Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        },
        onto: oid(2),
        todo: Vec::new(),
        line: graft_ops::rewrite::BranchLine::default(),
        messages: std::collections::BTreeMap::new(),
        made: Vec::new(),
        index_base: None,
        forks: Vec::new(),
    })
}

/// A rewrite's box: one row per fork under the line's rows, follows then
/// stays, the one that would conflict led by `!` (R3), the branch parked
/// at the old tip last of the stays with no `left behind:` (R1), the rest
/// counted; a follower is not drawn as built on his.
#[test]
fn a_rewrite_box_draws_one_row_per_fork() {
    let lines = drawn(&a_forked_plan(), &a_rewrite(), 187);
    let indent = " ".repeat("branches: ".len());
    let topic = lines
        .iter()
        .position(|line| line.contains("topic"))
        .unwrap_or_else(|| panic!("topic's row in {lines:#?}"));
    assert_eq!(
        lines[topic + 1..topic + 5],
        [
            format!("{indent}feature-x follows \u{b7} 2 commits from a1b2c3d"),
            format!("{indent}! feature-y stays on b2c3d4e \u{b7} would conflict in src/a.rs"),
            format!("{indent}backup stays on c3d4e5f \u{b7} on the old tip"),
            format!("{indent}+1 more stays"),
        ],
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("left behind")),
        "a rewrite draws no left behind: {lines:#?}"
    );
    assert!(
        !lines
            .iter()
            .any(|line| line.contains("feature-x") && line.contains("built on yours")),
        "a fork is not built on his: {lines:#?}"
    );
}

/// `r` keeps `left behind:` for the branch at the old tip (R1).
#[test]
fn a_rebase_box_keeps_left_behind() {
    let lines = drawn(&a_forked_plan(), &a_rebase(), 187);
    assert!(
        lines
            .iter()
            .any(|line| line == "left behind: backup stays on the old c3d4e5f"),
        "{lines:#?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("on the old tip")),
        "{lines:#?}"
    );
}

/// The words and tones, one case at a time (R2): 0, 1 and N commits of a
/// follower; `+1 more stays`, `+N more stay`; a stays row muted; nothing
/// for no forks.
#[test]
fn fork_rows_table() {
    use super::line_rows::{ForkRow, fork_rows};
    use graft_ops::preview::{Fork, ForkFate, WhyStays};

    let follower = |name: &str, commits| Fork {
        branch: branch(name),
        at: at("a1b2c3d"),
        commits,
        fate: ForkFate::Follows {
            onto: Target::Ghost(0),
        },
    };
    let row = |text: &str, tone| ForkRow {
        text: text.to_owned(),
        tone,
    };
    let mut plan = a_plan();
    assert_eq!(fork_rows(&plan, true), Vec::<ForkRow>::new(), "none");
    plan.forks = vec![
        follower("none", 0),
        follower("one", 1),
        follower("two", 2),
        Fork {
            branch: branch("main"),
            at: at("b2c3d4e"),
            commits: 3,
            fate: ForkFate::Stays(WhyStays::DefaultBranch),
        },
    ];
    plan.forks_more = 1;
    assert_eq!(
        fork_rows(&plan, false),
        [
            row("none follows \u{b7} from a1b2c3d", ForkTone::Follows),
            row(
                "one follows \u{b7} 1 commit from a1b2c3d",
                ForkTone::Follows
            ),
            row(
                "two follows \u{b7} 2 commits from a1b2c3d",
                ForkTone::Follows
            ),
            row(
                "main stays on b2c3d4e \u{b7} the default branch",
                ForkTone::Muted
            ),
            row("+1 more stays", ForkTone::Muted),
        ]
    );
    plan.forks_more = 3;
    assert_eq!(
        fork_rows(&plan, false).last(),
        Some(&row("+3 more stay", ForkTone::Muted))
    );
}

/// ADR-0059 §6's reasons (14d) are no warning: muted, no `!`.
#[test]
fn a_fork_kept_by_a_safety_gate_is_muted() {
    use super::line_rows::{ForkRow, fork_rows};
    use graft_ops::preview::{Fork, ForkFate, WhyStays};

    for (why, words) in [
        (
            WhyStays::Carries {
                branch: branch("feature-w"),
                why: Box::new(WhyStays::Elsewhere {
                    worktree: std::path::PathBuf::from("../wt"),
                }),
            },
            "feature-w is checked out in ../wt",
        ),
        (
            WhyStays::WouldLose {
                commit: at("b2c3d4e"),
            },
            "would lose b2c3d4e",
        ),
        (
            WhyStays::WouldGain {
                commit: at("b2c3d4e"),
            },
            "would gain b2c3d4e",
        ),
    ] {
        let mut plan = a_plan();
        plan.forks = vec![Fork {
            branch: branch("feature-y"),
            at: at("b2c3d4e"),
            commits: 1,
            fate: ForkFate::Stays(why),
        }];
        assert_eq!(
            fork_rows(&plan, false).first(),
            Some(&ForkRow {
                text: format!("feature-y stays on b2c3d4e \u{b7} {words}"),
                tone: ForkTone::Muted
            })
        );
    }
}

/// A fork that would conflict leads with `!`, in the warning tone (R3).
#[test]
fn a_conflicting_fork_row_warns() {
    use super::line_rows::{ForkRow, fork_rows};
    use graft_ops::preview::{Fork, ForkFate, WhyStays};

    let mut plan = a_plan();
    plan.forks = vec![Fork {
        branch: branch("feature-y"),
        at: at("b2c3d4e"),
        commits: 1,
        fate: ForkFate::Stays(WhyStays::WouldConflict {
            commit: oid(6),
            paths: vec![std::path::PathBuf::from("src/a.rs")],
        }),
    }];
    assert_eq!(
        fork_rows(&plan, false).first(),
        Some(&ForkRow {
            text: "! feature-y stays on b2c3d4e \u{b7} would conflict in src/a.rs".to_owned(),
            tone: ForkTone::Warning
        })
    );
}

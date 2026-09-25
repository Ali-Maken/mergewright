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
    ConflictForecast, GhostCommit, HeadMove, RefMove, UndoNote, WorktreeEffect, ghost_oid,
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
    }
}

/// A plan with nothing in it: a merge that is already up to date.
fn an_empty_plan() -> PreviewPlan {
    PreviewPlan {
        description: "merge main into feature".to_owned(),
        commands: vec!["git merge --no-edit --ff --no-edit main".to_owned()],
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
    }
}

fn a_rebase() -> Operation {
    Operation::Rebase(RebaseParams {
        branch: Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(3),
        },
        onto: Source::Branch("main".to_owned()),
        onto_tip: oid(2),
    })
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
            Command::Operate(_, op) => Some(op.as_ref()),
            _ => None,
        })
        .expect("confirm runs the operation");
    assert_eq!(operated, &a_rebase());
    assert!(state.operating.is_some(), "and it is a write in flight");
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

/// The click targets agree with what the widget actually draws: the test
/// renders the real widget and reads the buttons back off the buffer.
#[test]
fn a_click_lands_on_the_button_the_widget_drew() {
    use ratatui::buffer::Buffer;
    use ratatui::widgets::Widget;

    let (mut state, _) = opened_with(a_plan());
    let view = preview_box(&state).expect("a box");
    let area = preview_box_rect(&state, &view).expect("somewhere to draw");
    // The one place the pure geometry and the real widget meet: the widget
    // is rendered into exactly the rect `update` worked out, and the
    // buttons are read back off the buffer.
    let rect = ratatui::layout::Rect {
        x: area.x,
        y: area.y,
        width: area.width,
        height: area.height,
    };
    let mut buffer = Buffer::empty(rect);
    let theme = graft_ui::theme::Theme::new(
        graft_ui::theme::ThemeKind::Dark,
        &crate::session::for_tests::capabilities(),
    );
    Widget::render(
        &graft_ui::preview_box::PreviewBoxWidget {
            view: &view,
            theme: &theme,
        },
        rect,
        &mut buffer,
    );
    let row = area.bottom() - 2;
    let drawn: String = (rect.x..rect.right())
        .map(|x| buffer[(x, row)].symbol().to_owned())
        .collect();
    let at_cancel = drawn.find("Cancel").expect("Cancel is drawn");
    let at_confirm = drawn.find("Confirm").expect("Confirm is drawn");

    let mut commands = Vec::new();
    let column = area.x + u16::try_from(at_confirm + 3).expect("a small screen");
    assert!(preview_click(
        &mut state,
        &mut commands,
        column,
        row,
        &mut ids()
    ));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "a click on Confirm runs it: {commands:?}"
    );

    let (mut state, _) = opened_with(a_plan());
    let mut commands = Vec::new();
    let column = area.x + u16::try_from(at_cancel + 3).expect("a small screen");
    assert!(preview_click(
        &mut state,
        &mut commands,
        column,
        row,
        &mut ids()
    ));
    assert!(state.preview.is_none(), "a click on Cancel closes");
    assert!(commands.is_empty(), "{commands:?}");
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
        Operation::Rebase(RebaseParams {
            branch: Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            onto: Source::Branch("feature".to_owned()),
            onto_tip: oid(3),
        })
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
            Command::Operate(_, op) => Some(op.as_ref()),
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
        ("failed", failed),
        ("undrawable", undrawable),
        ("cannot act", cannot_act),
    ] {
        let view = preview_box(&state).expect("a box");
        assert!(!view.confirm_key, "{what}: `y confirm` drawn");
        assert!(view.again.is_none(), "{what}: `m merge` drawn");
        // And `y` really does nothing there: no drawn key is dead.
        let mut state = state;
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

/// `r` on a dirty tree: the job's `Refused` answer closes the waiting box,
/// and the status line says why in the write's own words, naming `S`.
/// Group F (ADR-0045) flips this: the edits are set aside instead.
#[test]
fn r_on_a_dirty_tree_opens_no_preview_and_says_why() {
    let (before, state) = answered_with(
        opened_by("r"),
        PreviewFailure::Refused(graft_ops::operation::Refusal::DirtyTree { count: 1 }),
    );
    assert!(state.preview.is_none(), "a box is open");
    assert!(preview_box(&state).is_none(), "a box is drawn");
    assert_eq!(
        state.status,
        Status::Sticky {
            reason: "rebase needs a clean tree".to_owned(),
            keys: Some("S stash 1 changed file".to_owned()),
        }
    );
    // Focus is the graph's again: nothing overlays it, and the cursor is
    // where `r` was pressed.
    assert!(!state.journal.shown && !state.banner_focus);
    assert_eq!(
        state.history.as_ref().map(|it| (it.top, it.cursor)),
        before.history.as_ref().map(|it| (it.top, it.cursor))
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
        crate::update::operate::refusal_status(&op, &refusal)
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
    );
    let _refresh: Vec<Command> = crate::update::operate::operated(
        &mut written,
        JobId::new(9),
        crate::jobs::operate::Operated::Failed {
            error: "the sentence nobody should see here".into(),
            refresh: None,
            remote: None,
            refused: Some(refusal),
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

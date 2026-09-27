//! The stash overlay's keys, over scripted state. Nothing here touches a
//! repository: what each key *means* is settled here, and what a stash
//! *does* is `ops`' and is proved against real git in
//! `tests/it/stash_by_key.rs`.

use std::path::PathBuf;

use graft_core::commit::Timestamp;
use graft_core::event::{KeyPress, Modifiers};
use graft_core::oid::Oid;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};

use super::*;

fn ids() -> impl FnMut() -> JobId {
    let mut next = 0;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn entry(index: usize, message: &str, files: usize) -> StashEntry {
    StashEntry {
        index,
        commit: oid(u8::try_from(index).unwrap_or(0) + 10),
        message: message.to_owned(),
        at: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
        paths: (0..files)
            .map(|n| PathBuf::from(format!("f{n}.rs")))
            .collect(),
    }
}

/// Two stashes on screen, the overlay open, the cursor on `stash@{0}`.
fn open_with_two() -> AppState {
    let mut state = AppState::default();
    state.stash.shown = true;
    state.stash.entries = Loading::Ready(Arc::new(vec![
        entry(0, "WIP on main: token refresh", 3),
        entry(1, "quick fix", 1),
    ]));
    state
}

/// A working tree with one unstaged change in it, so a save is not refused.
fn dirty() -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus {
        entries: vec![StatusEntry {
            path: PathBuf::from("a.rs"),
            original_path: None,
            staged: None,
            unstaged: Some(Change::Modified),
            untracked: false,
            unmerged: false,
            unmerged_how: None,
            submodule: None,
        }],
    })
}

fn press(state: &mut AppState, code: KeyCode) -> (bool, Vec<Command>) {
    let mut commands = Vec::new();
    let claimed = stash_key(state, &mut commands, KeyPress::plain(code), &mut ids());
    (claimed, commands)
}

#[test]
fn capital_s_opens_the_overlay_and_asks_for_the_list() {
    let mut state = AppState::default();
    let (claimed, commands) = press(&mut state, KeyCode::Char('S'));
    assert!(claimed);
    assert!(state.stash.shown);
    assert!(matches!(commands.as_slice(), [Command::ReadStashes(_)]));
}

#[test]
fn escape_closes_it_and_cancels_a_read_still_in_flight() {
    let mut state = AppState::default();
    press(&mut state, KeyCode::Char('S'));
    let Loading::Pending(id) = state.stash.entries else {
        panic!("the open read is pending");
    };
    let (claimed, commands) = press(&mut state, KeyCode::Escape);
    assert!(claimed);
    assert!(!state.stash.shown);
    assert_eq!(commands, vec![Command::CancelJob(id)]);
}

#[test]
fn the_cursor_clamps_at_both_ends() {
    let mut state = open_with_two();
    press(&mut state, KeyCode::Char('k'));
    assert_eq!(state.stash.selected, 0);
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(state.stash.selected, 1);
    press(&mut state, KeyCode::Char('j'));
    assert_eq!(state.stash.selected, 1);
}

// ── ⏎ and p open a preview; y writes (m6-conflicts/86) ─────────────────

/// A key as `keys.rs` routes it: the open preview first, then the overlay.
fn key(state: &mut AppState, code: KeyCode) -> Vec<Command> {
    let mut commands = Vec::new();
    let mut next = ids();
    let pressed = KeyPress::plain(code);
    if !super::super::preview::preview_key(state, &mut commands, pressed, &mut next) {
        stash_key(state, &mut commands, pressed, &mut next);
    }
    commands
}

/// The overlay on a screen the box fits in.
fn open_on_screen() -> AppState {
    let mut state = open_with_two();
    state.size = crate::update::Size {
        columns: 100,
        rows: 40,
    };
    state
}

/// The preview `code` opened: the operation it pins, and its job.
fn previewed(state: &AppState) -> (Operation, JobId) {
    let preview = state.preview.as_ref().expect("a preview is open");
    let op = preview
        .subject
        .operation()
        .expect("an operation preview")
        .clone();
    (op, preview.job)
}

/// Answer the preview's job with a stash plan for `op`.
fn planned(state: &mut AppState, op: &Operation) {
    planned_with(
        state,
        op,
        graft_ops::preview::WorktreeEffect::Updated {
            paths: vec![PathBuf::from("f0.rs")],
            more: false,
        },
    );
}

/// [`planned`], with the plan's working-tree effect given.
fn planned_with(
    state: &mut AppState,
    op: &Operation,
    worktree: graft_ops::preview::WorktreeEffect,
) {
    let (_, job) = previewed(state);
    let plan = graft_ops::preview::PreviewPlan {
        files: None,
        description: op.describe(),
        commands: vec!["git stash apply --index stash@{1}".to_owned()],
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree,
        conflicts: graft_ops::preview::ConflictForecast::Clean,
        undo: graft_ops::preview::UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
    };
    super::super::preview::preview_produced(
        state,
        job,
        Ok(Arc::new(super::super::preview::Planned {
            plan,
            anchors: Vec::new(),
        })),
    );
}

#[test]
fn enter_opens_the_apply_preview_and_writes_nothing() {
    let mut state = open_on_screen();
    state.stash.selected = 1;
    let commands = key(&mut state, KeyCode::Enter);
    let (op, job) = previewed(&state);
    assert_eq!(
        op,
        Operation::StashApply {
            index: 1,
            commit: oid(11),
        }
    );
    assert!(
        matches!(commands.as_slice(), [Command::PlanPreview(id, _)] if *id == job),
        "a plan, no write: {commands:?}"
    );
    assert!(state.operating.is_none());
    assert!(state.stash.shown, "the list stays under the box");
}

#[test]
fn p_opens_the_pop_preview() {
    let mut state = open_on_screen();
    let commands = key(&mut state, KeyCode::Char('p'));
    let (op, _) = previewed(&state);
    assert_eq!(
        op,
        Operation::StashPop {
            index: 0,
            commit: oid(10),
        }
    );
    assert!(matches!(commands.as_slice(), [Command::PlanPreview(..)]));
    // `p` again is not a confirm (no `p p`).
    planned(&mut state, &op);
    let commands = key(&mut state, KeyCode::Char('p'));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_some());
}

#[test]
fn enter_twice_writes_nothing() {
    let mut state = open_on_screen();
    key(&mut state, KeyCode::Enter);
    let (op, _) = previewed(&state);
    planned(&mut state, &op);
    let commands = key(&mut state, KeyCode::Enter);
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "⏎ on Cancel wrote: {commands:?}"
    );
    assert!(state.preview.is_none(), "Cancel closed the box");
    assert!(state.operating.is_none());
}

#[test]
fn esc_from_the_preview_returns_to_the_same_stash() {
    let mut state = open_on_screen();
    key(&mut state, KeyCode::Char('j'));
    key(&mut state, KeyCode::Enter);
    let (op, _) = previewed(&state);
    planned(&mut state, &op);
    let commands = key(&mut state, KeyCode::Escape);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.preview.is_none());
    assert!(state.stash.shown, "back on the list");
    assert_eq!(state.stash.selected, 1, "on the same stash");
}

/// The box's lines for a stash plan whose working-tree effect is `worktree`.
fn box_lines(worktree: graft_ops::preview::WorktreeEffect) -> Vec<String> {
    let mut state = open_on_screen();
    key(&mut state, KeyCode::Enter);
    let (op, _) = previewed(&state);
    planned_with(&mut state, &op, worktree);
    let view = super::super::preview::preview_box(&state).expect("a box");
    view.lines
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect()
}

fn names(count: usize) -> Vec<PathBuf> {
    (0..count).map(|n| PathBuf::from(format!("p{n}"))).collect()
}

/// Review 86, m1: a stash whose change is only staged names those paths,
/// never an empty `working tree:` line.
#[test]
fn an_index_only_stash_names_its_staged_paths_in_the_box() {
    let lines = box_lines(graft_ops::preview::WorktreeEffect::IndexOnly {
        paths: vec![PathBuf::from("notes.md")],
        more: false,
    });
    assert_eq!(lines[0], "index only: notes.md", "{lines:?}");
    assert!(
        lines.iter().all(|line| !line.starts_with("working tree:")),
        "{lines:?}"
    );
}

/// Review 86, m2: past the cap the count is unknown, so no `(+n)`; under it
/// the count is exact.
#[test]
fn past_the_cap_the_box_says_and_more() {
    let lines = box_lines(graft_ops::preview::WorktreeEffect::Updated {
        paths: names(50),
        more: true,
    });
    assert_eq!(lines[0], "working tree: p0, p1, p2 and more", "{lines:?}");
    let lines = box_lines(graft_ops::preview::WorktreeEffect::Updated {
        paths: names(5),
        more: false,
    });
    assert_eq!(lines[0], "working tree: p0, p1, p2 (+2)", "{lines:?}");
    let lines = box_lines(graft_ops::preview::WorktreeEffect::IndexOnly {
        paths: names(50),
        more: true,
    });
    assert_eq!(lines[0], "index only: p0, p1, p2 and more", "{lines:?}");
}

/// Review 86, m3: a refused put-back has only `Close`; its footer is
/// `esc back`, not `←→ choose  ⏎ act`.
#[test]
fn a_refused_stash_box_offers_only_esc_back() {
    let mut state = open_on_screen();
    key(&mut state, KeyCode::Enter);
    let (_, job) = previewed(&state);
    super::super::preview::preview_produced(
        &mut state,
        job,
        Err(super::super::preview::PreviewFailure::Refused(
            graft_ops::operation::Refusal::StashIndexNoLongerApplies { index: 0 },
        )),
    );
    let view = super::super::preview::preview_box(&state).expect("a box");
    assert!(view.back_only, "{view:?}");
    assert_eq!(view.choices().len(), 1, "{view:?}");
    assert_eq!(
        view.lines[0].as_str(),
        "the staged part of stash@{0} no longer applies"
    );
}

#[test]
fn y_in_the_preview_applies() {
    let mut state = open_on_screen();
    state.stash.selected = 1;
    key(&mut state, KeyCode::Enter);
    let (op, _) = previewed(&state);
    planned(&mut state, &op);
    let commands = key(&mut state, KeyCode::Char('y'));
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("one operate, got {commands:?}");
    };
    assert_eq!(
        **operation,
        Operation::StashApply {
            index: 1,
            commit: oid(11),
        }
    );
    assert!(state.preview.is_none());
    // As `⏎` launched it before the preview: a write `Esc` may stop.
    let running = state.remote.running.as_ref().expect("a running write");
    assert!(
        matches!(running.stoppable, super::super::remote::Stoppable::Yes),
        "{:?}",
        running.stoppable
    );
}

#[test]
fn y_in_the_preview_pops() {
    let mut state = open_on_screen();
    key(&mut state, KeyCode::Char('p'));
    let (op, _) = previewed(&state);
    planned(&mut state, &op);
    let commands = key(&mut state, KeyCode::Char('y'));
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("one operate, got {commands:?}");
    };
    assert_eq!(
        **operation,
        Operation::StashPop {
            index: 0,
            commit: oid(10),
        }
    );
}

#[test]
fn d_asks_first_and_says_it_comes_back_at_stash_zero() {
    let mut state = open_with_two();
    let (claimed, commands) = press(&mut state, KeyCode::Char('d'));
    assert!(claimed);
    assert!(commands.is_empty(), "the box asks before anything runs");
    let pending = state.confirm.as_ref().expect("the box opened");
    assert_eq!(pending.view.title.as_str(), "Drop stash@{0}?");
    // The safe choice is under the cursor, and the two buttons say what the
    // question asks about.
    assert_eq!(pending.view.selected(), 0);
    assert_eq!(pending.view.choices[0].as_str(), "Keep");
    assert_eq!(pending.view.choices[1].as_str(), "Drop");
    assert_eq!(pending.view.command.as_str(), "git stash drop stash@{0}");
    let said = pending
        .view
        .message
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(said.contains("3 files"), "{said:?}");
    assert!(
        said.contains("as stash@{0}, not where it was"),
        "a restored stash re-lists at the top; the box must not imply otherwise: {said:?}"
    );
    assert_eq!(
        pending.then,
        ConfirmThen::Operate(Operation::StashDrop {
            index: 0,
            commit: oid(10),
        })
    );
}

#[test]
fn s_opens_the_prompt_and_typing_lands_in_the_message() {
    let mut state = open_with_two();
    state.working_tree = Some(dirty());
    press(&mut state, KeyCode::Char('s'));
    for character in "wip".chars() {
        press(&mut state, KeyCode::Char(character));
    }
    let prompt = state.stash.prompt.as_ref().expect("the prompt is open");
    assert_eq!(prompt.message.text(), "wip");
    assert!(!prompt.staged_only, "`-u` is the default scope");
}

#[test]
fn tab_flips_the_scope_and_enter_saves_what_is_chosen() {
    let mut state = open_with_two();
    state.working_tree = Some(dirty());
    // Something staged, so `--staged` is not refused either.
    Arc::make_mut(state.working_tree.as_mut().expect("set above")).entries[0].staged =
        Some(Change::Modified);
    press(&mut state, KeyCode::Char('s'));
    press(&mut state, KeyCode::Char('w'));
    press(&mut state, KeyCode::Tab);
    assert!(state.stash.prompt.as_ref().expect("still open").staged_only);
    let (_, commands) = press(&mut state, KeyCode::Enter);
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("one operate, got {commands:?}");
    };
    assert_eq!(
        **operation,
        Operation::StashPush(StashParams {
            message: Some("w".to_owned()),
            scope: StashScope::StagedOnly,
        })
    );
    assert!(state.stash.prompt.is_none(), "the prompt closes on save");
}

#[test]
fn an_empty_message_is_no_message_at_all_not_an_empty_one() {
    let mut state = open_with_two();
    state.working_tree = Some(dirty());
    press(&mut state, KeyCode::Char('s'));
    let (_, commands) = press(&mut state, KeyCode::Enter);
    let [Command::Operate(_, operation)] = commands.as_slice() else {
        panic!("one operate, got {commands:?}");
    };
    assert_eq!(
        **operation,
        Operation::StashPush(StashParams {
            message: None,
            scope: StashScope::All {
                include_untracked: true
            },
        })
    );
}

#[test]
fn a_clean_tree_refuses_the_save_and_enter_asks_for_nothing() {
    let mut state = open_with_two();
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: Vec::new(),
    }));
    press(&mut state, KeyCode::Char('s'));
    assert_eq!(save_refusal(&state), Some("nothing to stash"));
    let (_, commands) = press(&mut state, KeyCode::Enter);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.stash.prompt.is_some(), "the prompt stays up");
}

#[test]
fn a_tree_with_nothing_staged_refuses_only_the_staged_only_scope() {
    let mut state = open_with_two();
    state.working_tree = Some(dirty());
    press(&mut state, KeyCode::Char('s'));
    assert_eq!(save_refusal(&state), None);
    press(&mut state, KeyCode::Tab);
    assert_eq!(
        save_refusal(&state),
        Some("nothing to stash — nothing is staged")
    );
}

#[test]
fn an_unread_working_tree_refuses_nothing() {
    let mut state = open_with_two();
    state.working_tree = None;
    press(&mut state, KeyCode::Char('s'));
    assert_eq!(save_refusal(&state), None);
}

#[test]
fn space_asks_for_the_selected_stashs_files_and_a_second_space_drops_them() {
    let mut state = open_with_two();
    let (_, commands) = press(&mut state, KeyCode::Char(' '));
    let [Command::ListFiles(id, spec, _)] = commands.as_slice() else {
        panic!("one file list, got {commands:?}");
    };
    assert_eq!(
        *spec,
        DiffSpec::Commit {
            commit: oid(10),
            against: None,
        }
    );
    assert_eq!(state.stash.preview, Loading::Pending(*id));
    let asked = *id;
    let (_, commands) = press(&mut state, KeyCode::Char(' '));
    assert_eq!(commands, vec![Command::CancelJob(asked)]);
    assert_eq!(state.stash.preview, Loading::Idle);
}

#[test]
fn moving_the_cursor_takes_the_preview_with_it() {
    let mut state = open_with_two();
    press(&mut state, KeyCode::Char(' '));
    let (_, commands) = press(&mut state, KeyCode::Char('j'));
    assert!(
        commands.iter().any(|c| matches!(c, Command::CancelJob(_))),
        "{commands:?}"
    );
    assert_eq!(state.stash.preview, Loading::Idle);
}

#[test]
fn an_open_overlay_swallows_the_keys_it_does_not_use() {
    let mut state = open_with_two();
    let (claimed, commands) = press(&mut state, KeyCode::Char('q'));
    assert!(claimed, "`q` must not quit from under the overlay");
    assert!(commands.is_empty());
}

#[test]
fn a_write_already_running_makes_every_write_key_a_no_op() {
    let mut state = open_with_two();
    state.working_tree = Some(dirty());
    state.operating = Some(JobId::new(99));
    for code in [
        KeyCode::Enter,
        KeyCode::Char('p'),
        KeyCode::Char('d'),
        KeyCode::Char('s'),
    ] {
        let (_, commands) = press(&mut state, code);
        assert!(commands.is_empty(), "{code:?} asked for {commands:?}");
    }
    assert!(state.confirm.is_none());
    assert!(state.stash.prompt.is_none());
}

#[test]
fn a_shorter_list_pulls_the_cursor_back_and_drops_the_preview() {
    let mut state = open_with_two();
    state.stash.selected = 1;
    state.stash.preview = Loading::Failed("stale".to_owned());
    let id = JobId::new(7);
    state.stash.entries = Loading::Pending(id);
    let taken = stash_event(
        &mut state,
        id,
        &Product::Stashes(Ok(Arc::new(vec![entry(0, "only one", 1)]))),
    );
    assert!(taken);
    assert_eq!(state.stash.selected, 0);
    assert_eq!(state.stash.preview, Loading::Idle);
}

#[test]
fn an_answer_for_a_read_that_is_no_longer_wanted_changes_nothing() {
    let mut state = open_with_two();
    let before = state.stash.entries.clone();
    let taken = stash_event(
        &mut state,
        JobId::new(404),
        &Product::Stashes(Ok(Arc::new(Vec::new()))),
    );
    assert!(taken, "it is still the overlay's product");
    assert_eq!(state.stash.entries, before);
}

#[test]
fn a_file_list_nobody_here_asked_for_is_not_this_views() {
    let state = open_with_two();
    assert!(!claims(
        &state,
        JobId::new(5),
        &Product::Files(Err("never asked".to_owned()))
    ));
}

#[test]
fn control_and_alt_keys_are_not_the_overlays() {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let claimed = stash_key(
        &mut state,
        &mut commands,
        KeyPress {
            code: KeyCode::Char('S'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        },
        &mut ids(),
    );
    assert!(!claimed);
    assert!(!state.stash.shown);
}

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
            submodule: false,
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

#[test]
fn enter_applies_the_selected_stash_with_its_own_commit() {
    let mut state = open_with_two();
    state.stash.selected = 1;
    let (_, commands) = press(&mut state, KeyCode::Enter);
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
}

#[test]
fn p_pops_it() {
    let mut state = open_with_two();
    let (_, commands) = press(&mut state, KeyCode::Char('p'));
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

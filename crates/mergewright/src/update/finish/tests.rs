//! `update::finish` over plain data, driven through `update` the way the
//! loop drives it. The real-git half — a real merge, a real `--continue`, a
//! real hook — is `tests/it/finish.rs`.

use std::path::PathBuf;
use std::sync::Arc;

use mergewright_core::event::{Event, KeyCode, KeyPress, Modifiers};
use mergewright_core::job::JobId;
use mergewright_git::state::{InProgress, InProgressDetail};
use mergewright_git::status::{StatusEntry, WorkingTreeStatus};
use mergewright_ops::apply::Resume;
use mergewright_ops::finish::{ByGit, ByYou, Change, FinishSummary, ResultIs};
use mergewright_ui::finish_summary::HowSolved;

use super::*;
use crate::update::{AppEvent, Product, Size, Status, update};

fn detail(kind: InProgress) -> InProgressDetail {
    InProgressDetail {
        kind,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    }
}

fn unmerged(path: &str) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: None,
        submodule: None,
    }
}

/// A `kind` stopped with `paths` still conflicted, the banner focused, at
/// the harness' 120x40.
fn stopped(kind: InProgress, paths: &[&str]) -> AppState {
    AppState {
        in_progress: Some(detail(kind)),
        banner_focus: true,
        size: Size {
            columns: 120,
            rows: 40,
        },
        working_tree: Some(Arc::new(WorkingTreeStatus {
            entries: paths.iter().map(|path| unmerged(path)).collect(),
        })),
        ..AppState::default()
    }
}

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn plain(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn control(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }
}

fn key(state: &AppState, code: KeyCode) -> AppState {
    press(state, plain(code)).0
}

/// git's message for a merge that stopped, comments already gone.
const GITS_MESSAGE: &[u8] = b"Merge branch 'feature'\n\nwith a body\n";

/// One clashed file (`notes.txt`) and one git combined (`README.md`).
fn summary() -> Arc<FinishSummary> {
    Arc::new(FinishSummary {
        into: b"main".to_vec(),
        from: b"feature".to_vec(),
        by_you: vec![ByYou {
            path: PathBuf::from("notes.txt"),
            result: ResultIs::Theirs,
        }],
        by_git: vec![ByGit {
            path: PathBuf::from("README.md"),
            change: Change::Modified,
            added: 1,
            removed: 1,
        }],
        message: GITS_MESSAGE.to_vec(),
        recorded: true,
    })
}

/// `c` on a resolved merge's banner, and the review's read answered.
pub(crate) fn opened() -> AppState {
    let state = stopped(InProgress::Merge, &[]);
    review::landed(press(&state, plain(KeyCode::Char('c')))).0
}

fn finish(state: &AppState) -> &FinishState {
    state.finish.as_ref().expect("the review is open")
}

fn resumes(commands: &[Command]) -> Vec<&Resume> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::Resume(_, what, ..) => Some(what),
            _ => None,
        })
        .collect()
}

#[test]
fn a_rebase_c_still_continues_directly() {
    let mut state = stopped(InProgress::Rebase, &[]);
    state.live_entries = vec![mergewright_core::journal::EntryId::new(1, [0; 10])];
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    assert!(state.finish.is_none());
    assert_eq!(
        resumes(&commands),
        vec![&Resume::Continue { solved: Vec::new() }]
    );
}

/// The read lands: the cursor on the first clashed file (the mock's), the
/// view built from the read — one clashed row, one combined row, the
/// message's subject read-only.
#[test]
fn the_review_lands_with_the_cursor_on_the_first_clashed_file() {
    let state = opened();
    assert_eq!(finish(&state).cursor, 0, "on notes.txt");
    let view = crate::view::finish::finish_view(&state).expect("drawn");
    assert_eq!(
        (view.from.as_str(), view.into.as_str()),
        ("feature", "main")
    );
    assert_eq!(view.clashed.len(), 1);
    assert_eq!(view.clashed[0].how, HowSolved::ByHand, "no picks recorded");
    assert_eq!(view.combined, 1);
    assert_eq!(view.row_count(), 2);
    assert_eq!(view.message.as_str(), "Merge branch 'feature'");
    assert_eq!(view.clashes, 1);
}

#[test]
fn unread_review_has_no_names() {
    let state = review::reading();
    let view = crate::view::finish::finish_view(&state).expect("drawn");
    assert_eq!((view.from.as_str(), view.into.as_str()), ("", ""));
    assert_eq!(view.state, mergewright_ui::finish_summary::SummaryState::Loading);
}

#[test]
fn y_commits_with_gits_own_message() {
    let state = opened();
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(
        resumes(&commands),
        vec![&Resume::Continue { solved: Vec::new() }],
        "never `ContinueWith`: the message is git's, read-only"
    );
    assert_eq!(finish(&state).committing, state.operating);
    assert!(state.operating.is_some());
}

/// Decision 5: the message is read-only. A letter, `⇥`, `⇧⇥` and `⌃E` —
/// the old fields' keys — change nothing and send nothing.
#[test]
fn typing_and_tab_leave_the_review_unchanged() {
    let state = opened();
    for key in [
        plain(KeyCode::Char('x')),
        plain(KeyCode::Tab),
        plain(KeyCode::BackTab),
        control(KeyCode::Char('e')),
        control(KeyCode::Enter),
    ] {
        let (after, commands) = press(&state, key);
        assert_eq!(after, state, "{key:?} changed the state");
        assert!(commands.is_empty(), "{key:?} sent {commands:?}");
    }
}

/// Decision 2: keys by keymap id. With `confirm.yes` on `Y`, `Y` commits
/// and `y` does nothing.
#[test]
fn a_remapped_confirm_commits_and_the_old_key_does_not() {
    let (map, warnings) = mergewright_ui::keymap::Keymap::with_overrides(
        &[("confirm.yes".to_owned(), vec!["Y".to_owned()])],
        mergewright_ui::keymap::KeySwitches::default(),
    );
    assert!(warnings.is_empty(), "{warnings:?}");
    let mut state = opened();
    state.keymap = Arc::new(map);
    let (after, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty(), "y: {commands:?}");
    assert_eq!(after, state, "y does nothing");
    let (after, commands) = press(&state, plain(KeyCode::Char('Y')));
    assert_eq!(
        resumes(&commands),
        vec![&Resume::Continue { solved: Vec::new() }]
    );
    assert!(finish(&after).committing.is_some());
}

#[test]
fn y_while_reading_says_wait() {
    let (state, commands) = press(&review::reading(), plain(KeyCode::Char('y')));
    assert!(resumes(&commands).is_empty());
    assert_eq!(
        finish(&state).notice,
        Some(("still reading · wait".to_owned(), false))
    );
}

#[test]
fn an_unreadable_summary_commits_with_gits_message() {
    let state = stopped(InProgress::Merge, &[]);
    let (state, commands) = press(&state, plain(KeyCode::Char('c')));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            review::read_id(&commands),
            Product::FinishSummary(Err("1 still conflicted".to_owned().into())),
        ),
    );
    let (_, commands) = press(&state, plain(KeyCode::Char('y')));
    assert_eq!(
        resumes(&commands),
        vec![&Resume::Continue { solved: Vec::new() }]
    );
}

#[test]
fn esc_goes_back_to_the_banner() {
    let state = key(&opened(), KeyCode::Escape);
    assert!(state.finish.is_none());
    assert!(state.banner_focus, "back on the banner, focused");
}

/// `⏎`/`space` on a clashed row: its diff under it, again hides it; a stale
/// answer is dropped (decision 10: today's inline diff until 12b).
#[test]
fn space_expands_a_clashed_file() {
    let state = opened();
    let (state, commands) = press(&state, plain(KeyCode::Char(' ')));
    let reads: Vec<_> = commands
        .iter()
        .filter_map(|command| match command {
            Command::ReadFinishDiff(id, path) => Some((*id, path.clone())),
            _ => None,
        })
        .collect();
    let [(first, path)] = reads.as_slice() else {
        panic!("one diff read: {commands:?}");
    };
    assert_eq!(path, &PathBuf::from("notes.txt"));
    assert_eq!(finish(&state).expanded, Some((0, Loading::Pending(*first))));
    let collapsed = key(&state, KeyCode::Enter);
    assert_eq!(finish(&collapsed).expanded, None, "⏎ hides it");
    let (after, _) = update(
        &collapsed,
        AppEvent::Produced(*first, Product::FinishDiff(Err("late".to_owned()))),
    );
    assert_eq!(finish(&after).expanded, None, "a stale answer is dropped");
}

/// Decision 10: on the combined row `⏎` does nothing until 12b's look.
#[test]
fn enter_on_the_combined_row_does_nothing_yet() {
    let state = key(&opened(), KeyCode::Char('j'));
    assert_eq!(finish(&state).cursor, 1, "the combined row");
    let (after, commands) = press(&state, plain(KeyCode::Enter));
    assert_eq!(after, state);
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn keys_outside_2_6b_do_nothing() {
    let state = opened();
    for key in [
        plain(KeyCode::Home),
        plain(KeyCode::End),
        plain(KeyCode::Char('q')),
    ] {
        let (after, commands) = press(&state, key);
        assert_eq!(after, state, "{key:?} changed the state");
        assert!(commands.is_empty(), "{key:?} sent {commands:?}");
    }
}

#[test]
fn ctrl_c_passes_through() {
    let mut next = opened();
    let mut commands = Vec::new();
    assert!(!finish_key(
        &mut next,
        &mut commands,
        control(KeyCode::Char('c')),
        &mut || JobId::new(9)
    ));
}

#[test]
fn committing_swallows_every_key() {
    let state = key(&opened(), KeyCode::Char('y'));
    for code in [KeyCode::Escape, KeyCode::Char('j'), KeyCode::Enter] {
        let (after, commands) = press(&state, plain(code));
        assert_eq!(after, state, "{code:?}");
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
    }
}

#[test]
fn arrows_and_pages_move_the_file_cursor() {
    let state = opened();
    let down = key(&state, KeyCode::Down);
    assert_eq!(finish(&down).cursor, 1);
    assert_eq!(finish(&key(&down, KeyCode::Down)).cursor, 1, "stops");
    assert_eq!(finish(&key(&down, KeyCode::Up)).cursor, 0);
    assert_eq!(finish(&key(&state, KeyCode::PageDown)).cursor, 1);

    // A page is the list's visible height.
    let mut long = opened();
    let mut many = (*summary()).clone();
    many.by_you = (0..80)
        .map(|n| ByYou {
            path: PathBuf::from(format!("file{n}.txt")),
            result: ResultIs::Yours,
        })
        .collect();
    if let Some(open) = long.finish.as_mut() {
        open.summary = Loading::Ready(Arc::new(many));
    }
    let view = crate::view::finish::finish_view(&long).expect("drawn");
    let height =
        mergewright_ui::finish_summary::list_height(&view, crate::view::finish::summary_area(&long));
    assert!(height > 2, "{height}");
    let paged = key(&long, KeyCode::PageDown);
    assert_eq!(finish(&paged).cursor, height);
    let back = key(&paged, KeyCode::PageUp);
    assert_eq!(finish(&back).cursor, 0);
    assert_eq!(finish(&back).top, 0);
}

mod answers;
mod landing;
mod review;
mod words;
pub(crate) use review::landed;

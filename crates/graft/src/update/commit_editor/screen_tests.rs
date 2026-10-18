//! The commit screen of ease-pass/08: what goes in (C5–C9), the keys
//! between its parts (C10, C11), the journeys the note counts, the words
//! `esc` keeps (C15), the options (Q8 a), the amend, and what the commit
//! leaves behind ([`LastCommit`], Q9 a). Every test drives `update` or a
//! public function of the module, never its private helpers.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ops::operation::{Include, Operation};

use super::draft_tests::{head_at, oid};
use super::*;
use crate::update::{AppEvent, Product, update};

pub(super) fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

pub(super) fn control(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress {
        code,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }))
}

/// Each event in turn; the state after the last, and every command sent.
pub(super) fn walk(state: &AppState, events: Vec<AppEvent>) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut sent = Vec::new();
    for event in events {
        let (next, commands) = update(&state, event);
        state = next;
        sent.extend(commands);
    }
    (state, sent)
}

/// Typed text: one step, as the note counts it.
pub(super) fn text(words: &str) -> Vec<AppEvent> {
    words.chars().map(|c| key(KeyCode::Char(c))).collect()
}

pub(super) fn entry(path: &str, untracked: bool) -> StatusEntry {
    StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: (!untracked).then_some(Change::Modified),
        untracked,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    }
}

/// The mock's tree: three files edited, nothing staged, a new `.env`.
pub(super) fn mock_tree() -> AppState {
    let mut state = super::tests::ready(&[]);
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![
            entry("docs/toc.md", false),
            entry("README.md", false),
            entry("src/a.rs", false),
            entry(".env", true),
        ],
    }));
    state
}

pub(super) fn opened(state: &AppState) -> AppState {
    update(state, key(KeyCode::Char('c'))).0
}

pub(super) fn editor(state: &AppState) -> &CommitEditor {
    state.commit_editor.as_ref().expect("the screen is open")
}

/// The paths a commit in `commands` takes, in order.
pub(super) fn chosen(commands: &[Command]) -> Vec<String> {
    let op = commands.iter().find_map(|command| match command {
        Command::Operate(_, op, ..) => Some(&**op),
        _ => None,
    });
    let Some(Operation::Commit(params) | Operation::Amend(params)) = op else {
        panic!("no commit: {commands:?}");
    };
    let Include::Chosen(files) = &params.include else {
        panic!("{:?}", params.include);
    };
    files
        .iter()
        .map(|file| file.path.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn the_secrets_file_stays_out_marked_and_space_puts_it_in() {
    let open = opened(&mock_tree());
    let view = crate::view::commit_screen_view(&open).expect("open");
    let env = view.files.last().expect("rows");
    assert_eq!(env.path.as_str(), ".env");
    assert_eq!(env.group, graft_ui::commit_editor::FileGroup::StayingOut);
    assert!(env.secret, "marked `!`, why `looks like a secrets file`");
    // `⇥ ⇥` to the files, `j` three times to `.env`, `space`.
    let (moved, _) = walk(
        &open,
        vec![
            key(KeyCode::Tab),
            key(KeyCode::Tab),
            key(KeyCode::Char('j')),
            key(KeyCode::Char('j')),
            key(KeyCode::Char('j')),
            key(KeyCode::Char(' ')),
        ],
    );
    let view = crate::view::commit_screen_view(&moved).expect("open");
    assert!(
        view.files
            .iter()
            .all(|row| row.group == graft_ui::commit_editor::FileGroup::GoingIn),
        "{:?}",
        view.files
    );
}

#[test]
fn down_in_the_subject_moves_the_file_cursor_and_reads_its_diff() {
    let open = opened(&mock_tree());
    assert_eq!(editor(&open).file_cursor, 0);
    let (moved, commands) = update(&open, key(KeyCode::Down));
    assert_eq!(editor(&moved).focus, CommitFocus::Subject);
    assert_eq!(editor(&moved).file_cursor, 1);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFileDiff(..))),
        "{commands:?}"
    );
}

#[test]
fn tab_reaches_the_files_and_esc_and_tab_come_back() {
    let open = opened(&mock_tree());
    let (files, _) = walk(&open, vec![key(KeyCode::Tab), key(KeyCode::Tab)]);
    assert_eq!(editor(&files).focus, CommitFocus::Files);
    let (tabbed, _) = update(&files, key(KeyCode::Tab));
    assert_eq!(editor(&tabbed).focus, CommitFocus::Subject);
    let (escaped, _) = update(&files, key(KeyCode::Escape));
    assert_eq!(editor(&escaped).focus, CommitFocus::Subject, "not closed");
    let (body, _) = update(&open, key(KeyCode::Tab));
    let (escaped, _) = update(&body, key(KeyCode::Escape));
    assert_eq!(editor(&escaped).focus, CommitFocus::Subject);
}

/// Journey `commit-all-subject`: `c type ⏎`, 3 steps; `.env` stays out.
#[test]
fn commit_all_subject_takes_every_change_but_the_secret() {
    let steps = [
        vec![key(KeyCode::Char('c'))],
        text("fix toc"),
        vec![key(KeyCode::Enter)],
    ];
    assert_eq!(steps.len(), 3);
    let (_, commands) = walk(&mock_tree(), steps.concat());
    assert_eq!(chosen(&commands), ["docs/toc.md", "README.md", "src/a.rs"]);
}

/// Journey `commit-with-body`: `c type ⇥ type esc ⏎`, 6 steps.
#[test]
fn commit_with_body_is_six_steps() {
    let steps = [
        vec![key(KeyCode::Char('c'))],
        text("fix toc"),
        vec![key(KeyCode::Tab)],
        text("why it moved"),
        vec![key(KeyCode::Escape)],
        vec![key(KeyCode::Enter)],
    ];
    assert_eq!(steps.len(), 6);
    let (state, commands) = walk(&mock_tree(), steps.concat());
    let Some(Command::Operate(_, op, ..)) = commands
        .iter()
        .find(|command| matches!(command, Command::Operate(..)))
    else {
        panic!("{commands:?}");
    };
    let Operation::Commit(params) = &**op else {
        panic!("{op:?}");
    };
    assert_eq!(params.message, "fix toc\n\nwhy it moved\n");
    assert!(state.operating.is_some());
}

/// Journey `commit-leave-first`: `c type ⇥ ⇥ space ⏎`, 6 steps; the first
/// file is left out.
#[test]
fn commit_leave_first_is_six_steps() {
    let steps = [
        vec![key(KeyCode::Char('c'))],
        text("fix toc"),
        vec![key(KeyCode::Tab)],
        vec![key(KeyCode::Tab)],
        vec![key(KeyCode::Char(' '))],
        vec![key(KeyCode::Enter)],
    ];
    assert_eq!(steps.len(), 6);
    let (_, commands) = walk(&mock_tree(), steps.concat());
    assert_eq!(chosen(&commands), ["README.md", "src/a.rs"]);
}

#[test]
fn ctrl_enter_from_the_files_commits() {
    let open = opened(&mock_tree());
    let (files, _) = walk(
        &open,
        [text("fix toc"), vec![key(KeyCode::Tab), key(KeyCode::Tab)]].concat(),
    );
    let (_, commands) = update(&files, control(KeyCode::Enter));
    assert_eq!(chosen(&commands).len(), 3);
}

#[test]
fn every_file_left_out_refuses_put_a_file_in_first() {
    let (all_out, commands) = walk(
        &mock_tree(),
        [
            vec![key(KeyCode::Char('c'))],
            text("fix toc"),
            vec![key(KeyCode::Tab), key(KeyCode::Tab)],
            // `space` leaves the file under the cursor out and the cursor
            // follows it under `Staying out`; `k` back up to the next one.
            vec![
                key(KeyCode::Char(' ')),
                key(KeyCode::Char('k')),
                key(KeyCode::Char(' ')),
                key(KeyCode::Char('k')),
                key(KeyCode::Char('k')),
                key(KeyCode::Char(' ')),
            ],
            vec![key(KeyCode::Enter)],
        ]
        .concat(),
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..)))
    );
    assert_eq!(
        editor(&all_out).refusal(),
        Some(graft_ui::commit_editor::Refusal::NoFile)
    );
}

/// Q8 a: `⌃S` names itself in the header, and goes again on the second.
#[test]
fn ctrl_s_names_signing_off_in_the_header_and_again_takes_it_away() {
    let open = opened(&mock_tree());
    let (on, _) = update(&open, control(KeyCode::Char('s')));
    let header = crate::view::commit_screen_view(&on).expect("open").header;
    assert_eq!(header.options, ["signing off"]);
    let (off, _) = update(&on, control(KeyCode::Char('s')));
    let header = crate::view::commit_screen_view(&off).expect("open").header;
    assert!(header.options.is_empty());
}

/// The amend: header `amending <sha7> <subject>`, its message in place; an
/// untouched `⏎` takes the index as it is, a `space` makes it a choice.
#[test]
fn an_amend_names_its_commit_and_takes_the_index_until_space() {
    let mut state = mock_tree();
    state.refs = Some(Arc::new(head_at(oid(0xab))));
    let (asked, commands) = update(&state, key(KeyCode::Char('a')));
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (amend, _) = update(
        &asked,
        AppEvent::Produced(*id, Product::HeadMessage(Ok("old subject\n".into()))),
    );
    let header = crate::view::commit_screen_view(&amend)
        .expect("open")
        .header;
    assert_eq!(
        header
            .amending
            .map(|words| words.as_str().to_owned())
            .as_deref(),
        Some("abababa old subject")
    );
    assert_eq!(editor(&amend).subject.text(), "old subject");
    assert_eq!(editor(&amend).params().include, Include::Staged);
    assert_eq!(editor(&amend).refusal(), None, "a message-only amend");
    let (spaced, _) = walk(
        &amend,
        vec![
            key(KeyCode::Tab),
            key(KeyCode::Tab),
            key(KeyCode::Char(' ')),
        ],
    );
    let Include::Chosen(files) = editor(&spaced).params().include else {
        panic!("a choice once space is pressed");
    };
    assert_eq!(files.len(), 1);
}

/// m3, m10: the same files read again still re-ask the counts and the diff
/// on show, cancelling the count read that is still running.
#[test]
fn a_reread_while_open_reads_counts_and_diff_again() {
    let mut open = opened(&mock_tree());
    let old = editor(&open).counts_job.expect("counts asked on open");
    let mut commands = Vec::new();
    let mut next = 900;
    let mut next_id = || {
        next += 1;
        JobId::new(next)
    };
    tree_read(&mut open, &mut commands, &mut next_id);
    assert!(commands.contains(&Command::CancelJob(old)), "{commands:?}");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadCommitFiles(id, _) if *id != old)),
        "{commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadFileDiff(..))),
        "{commands:?}"
    );
}

/// M3: a partly staged file goes in as staged, so its counts and its diff
/// are read index against `HEAD`.
#[test]
fn a_staged_part_reads_the_staged_section() {
    let mut state = mock_tree();
    let mut partly = entry("src/a.rs", false);
    partly.staged = Some(Change::Modified);
    state.working_tree = Some(Arc::new(WorkingTreeStatus {
        entries: vec![partly, entry("README.md", false)],
    }));
    let (_, commands) = update(&state, key(KeyCode::Char('c')));
    let staged_parts = [std::path::PathBuf::from("src/a.rs")];
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::ReadCommitFiles(_, parts) if parts[..] == staged_parts
        )),
        "{commands:?}"
    );
    let section = commands.iter().find_map(|command| match command {
        Command::ReadFileDiff(_, key, _) => Some(key.section),
        _ => None,
    });
    assert_eq!(
        section,
        Some(Some(graft_ui::detail::WorkingTreeSection::Staged))
    );
}

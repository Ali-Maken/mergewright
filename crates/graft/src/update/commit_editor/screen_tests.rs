//! The commit screen of ease-pass/08: what goes in (C5–C9), the keys
//! between its parts (C10, C11), the journeys the note counts, the words
//! `esc` keeps (C15), the options (Q8 a), the amend, and what the commit
//! leaves behind ([`LastCommit`], Q9 a). Every test drives `update` or a
//! public function of the module, never its private helpers.

use std::collections::BTreeMap;
use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ops::operation::{Include, Operation};
use graft_ui::graph_screen::hints::RowBar;

use super::*;
use crate::update::{AppEvent, Product, update};

fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

fn control(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress {
        code,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }))
}

/// Each event in turn; the state after the last, and every command sent.
fn walk(state: &AppState, events: Vec<AppEvent>) -> (AppState, Vec<Command>) {
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
fn text(words: &str) -> Vec<AppEvent> {
    words.chars().map(|c| key(KeyCode::Char(c))).collect()
}

fn entry(path: &str, untracked: bool) -> StatusEntry {
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
fn mock_tree() -> AppState {
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

fn opened(state: &AppState) -> AppState {
    update(state, key(KeyCode::Char('c'))).0
}

fn editor(state: &AppState) -> &CommitEditor {
    state.commit_editor.as_ref().expect("the screen is open")
}

/// The paths a commit in `commands` takes, in order.
fn chosen(commands: &[Command]) -> Vec<String> {
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

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes")
}

fn last(commit: Oid) -> LastCommit {
    LastCommit {
        commit,
        message: Draft {
            subject: "fix toc".to_owned(),
            body: String::new(),
            amending: None,
        },
    }
}

#[test]
fn landed_bar_is_just_committed_only_on_the_commit() {
    assert_eq!(landed_bar(None, Some(oid(1))), None);
    assert_eq!(landed_bar(Some(&last(oid(1))), Some(oid(2))), None);
    assert_eq!(
        landed_bar(Some(&last(oid(1))), Some(oid(1))),
        Some(RowBar::JustCommitted)
    );
}

#[test]
fn c_opening_clears_the_last_commit() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(1))));
    assert!(opened(&state).last_commit.is_none());
}

/// A journal entry moving `main` from `before` to `after`.
fn moved(kind: OperationKind, before: Oid, after: Oid) -> JournalEntry {
    let main = |at: Oid| BTreeMap::from([("refs/heads/main".to_owned(), Some(at))]);
    JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 1,
        kind,
        description: "an entry".to_owned(),
        status: EntryStatus::Committed,
        touches: Touches::default(),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: main(before),
        refs_after: main(after),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: BTreeMap::new(),
        config_after: BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    }
}

#[test]
fn another_write_landing_clears_the_last_commit() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(1))));
    // An amend: a write that is not `c`'s commit, and lands on `HEAD`.
    let amend = Operation::Amend(graft_ops::operation::CommitParams::default());
    let words = commit_landed(
        &mut state,
        Some(&amend),
        &moved(OperationKind::Amend, oid(1), oid(3)),
    );
    assert_eq!(words, None, "an amend keeps its own words");
    assert!(state.last_commit.is_none());
}

#[test]
fn undoing_the_commit_gives_its_message_back_as_the_draft() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    undo_landed(&mut state, &moved(OperationKind::Undo, oid(2), oid(1)));
    assert!(state.last_commit.is_none());
    assert_eq!(
        state
            .commit_draft
            .as_ref()
            .map(|draft| draft.subject.as_str()),
        Some("fix toc")
    );
    let reopened = opened(&state);
    assert_eq!(editor(&reopened).subject.text(), "fix toc");
    assert_eq!(editor(&reopened).going_in(), 3, "files chosen afresh");
}

#[test]
fn undoing_something_else_leaves_the_draft_alone() {
    let mut state = mock_tree();
    state.last_commit = Some(Box::new(last(oid(2))));
    state.commit_draft = None;
    undo_landed(&mut state, &moved(OperationKind::Undo, oid(5), oid(4)));
    assert!(state.last_commit.is_none());
    assert!(state.commit_draft.is_none());
}

/// C15: `esc` keeps both fields; `c` picks them up with every option off.
#[test]
fn esc_keeps_subject_and_body_and_c_resumes_them_with_options_off() {
    let (closed, _) = walk(
        &mock_tree(),
        [
            vec![key(KeyCode::Char('c'))],
            text("half"),
            vec![control(KeyCode::Char('s')), key(KeyCode::Tab)],
            text("more"),
            vec![key(KeyCode::Escape), key(KeyCode::Escape)],
        ]
        .concat(),
    );
    assert!(closed.commit_editor.is_none());
    let reopened = opened(&closed);
    assert_eq!(editor(&reopened).subject.text(), "half");
    assert_eq!(editor(&reopened).body.text(), "more");
    assert!(!editor(&reopened).sign_off, "options start off");
}

#[test]
fn an_amend_draft_resumes_on_a_not_on_c() {
    let mut state = mock_tree();
    state.commit_draft = Some(Draft {
        subject: "amended words".to_owned(),
        body: String::new(),
        amending: Some(oid(7)),
    });
    let fresh = opened(&state);
    assert_eq!(editor(&fresh).subject.text(), "", "not on `c`");
    assert!(fresh.commit_draft.is_some(), "kept for `a`");
    let mut on_head = state.clone();
    on_head.refs = Some(Arc::new(head_at(oid(7))));
    let (asked, commands) = update(&on_head, key(KeyCode::Char('a')));
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (amend, _) = update(
        &asked,
        AppEvent::Produced(*id, Product::HeadMessage(Ok("old subject\n".into()))),
    );
    assert_eq!(editor(&amend).subject.text(), "amended words");
}

/// Refs with `main` at `head`, `HEAD` on it.
fn head_at(head: Oid) -> crate::refs::RefBadges {
    let main = graft_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a name");
    crate::refs::badges(
        &[(main.clone(), head)],
        &graft_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("counting never fails here")
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

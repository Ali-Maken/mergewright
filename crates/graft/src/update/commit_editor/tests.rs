//! The commit editor's keys, as plain function calls over plain data.
//!
//! Every test here drives `update` itself rather than the module's private
//! helpers, so what it proves is what a key press actually does.

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_git::status::{Change, StatusEntry, WorkingTreeStatus};
use graft_ops::operation::Operation;

use super::*;
use crate::update::{AppEvent, Product, Status, update};

/// A plain key press.
fn key(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress::plain(code)))
}

/// A key press with control held.
fn control(code: KeyCode) -> AppEvent {
    AppEvent::Core(Event::Key(KeyPress {
        code,
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }))
}

/// Type a run of characters into whatever has focus.
fn typed(state: &AppState, text: &str) -> AppState {
    let mut state = state.clone();
    for character in text.chars() {
        let (next, _) = update(&state, key(KeyCode::Char(character)));
        state = next;
    }
    state
}

/// A status with `staged` paths staged and nothing else.
fn staged(paths: &[&str]) -> Arc<WorkingTreeStatus> {
    Arc::new(WorkingTreeStatus {
        entries: paths
            .iter()
            .map(|path| StatusEntry {
                path: (*path).into(),
                original_path: None,
                staged: Some(Change::Modified),
                unstaged: None,
                untracked: false,
                unmerged: false,
                submodule: false,
            })
            .collect(),
    })
}

/// A state with a working tree read and room to draw.
fn ready(paths: &[&str]) -> AppState {
    AppState {
        working_tree: Some(staged(paths)),
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        ..AppState::default()
    }
}

/// The `Operate` command in `commands`, if there is one.
fn operate(commands: &[Command]) -> Option<&Operation> {
    commands.iter().find_map(|command| match command {
        Command::Operate(_, op) => Some(&**op),
        _ => None,
    })
}

#[test]
fn c_opens_the_editor_and_typing_lands_in_the_subject() {
    let (open, _) = update(&ready(&["a.rs", "b.rs"]), key(KeyCode::Char('c')));
    let editor = open.commit_editor.as_ref().expect("`c` opens the editor");
    assert_eq!(editor.focus, Field::Subject);

    let typed = typed(&open, "add a thing");
    let editor = typed.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.subject.text(), "add a thing");
    assert!(editor.body.is_empty());
}

#[test]
fn the_title_counts_the_staged_files() {
    let (open, _) = update(&ready(&["a.rs", "b.rs"]), key(KeyCode::Char('c')));
    let view = crate::view::commit_editor_view(&open).expect("the editor is open");
    assert!(
        view.title.as_str().contains("2 files staged"),
        "{}",
        view.title
    );
}

#[test]
fn enter_in_the_subject_moves_to_the_body_and_inserts_nothing() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (moved, _) = update(&open, key(KeyCode::Enter));
    let editor = moved.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.focus, Field::Body);
    assert_eq!(
        editor.subject.text(),
        "subject",
        "no newline in the subject"
    );
    assert!(editor.body.is_empty(), "and none in the body either");
}

#[test]
fn tab_walks_subject_body_options_button_and_back() {
    let (mut state, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let expected = [
        Field::Body,
        Field::Amend,
        Field::CommitButton,
        Field::Subject,
    ];
    for want in expected {
        let (next, _) = update(&state, key(KeyCode::Tab));
        state = next;
        assert_eq!(
            state.commit_editor.as_ref().expect("still open").focus,
            want
        );
    }
}

#[test]
fn enter_on_the_button_commits_the_subject_and_the_body() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (open, _) = update(&open, key(KeyCode::Enter)); // to the body
    let open = typed(&open, "body");
    // Tab from the body: options, then the button.
    let (open, _) = update(&open, key(KeyCode::Tab));
    let (open, _) = update(&open, key(KeyCode::Tab));
    assert_eq!(
        open.commit_editor.as_ref().expect("still open").focus,
        Field::CommitButton
    );
    let (fired, commands) = update(&open, key(KeyCode::Enter));
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("the button commits: {commands:?}");
    };
    assert_eq!(params.message, "subject\n\nbody\n");
    assert!(!params.sign_off);
    assert!(fired.operating.is_some(), "one write, and it is in flight");
}

#[test]
fn ctrl_enter_commits_from_anywhere() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(
        matches!(operate(&commands), Some(Operation::Commit(_))),
        "{commands:?}"
    );
}

#[test]
fn ctrl_s_turns_the_sign_off_on_and_it_reaches_the_params() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (signed, _) = update(&open, control(KeyCode::Char('s')));
    assert!(signed.commit_editor.as_ref().expect("open").sign_off);
    let (_, commands) = update(&signed, control(KeyCode::Enter));
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("{commands:?}");
    };
    assert!(params.sign_off);
}

#[test]
fn ctrl_shift_c_starts_a_co_author_trailer_and_puts_the_cursor_after_it() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (trailer, _) = update(&open, control(KeyCode::Char('C')));
    let editor = trailer.commit_editor.as_ref().expect("open");
    assert_eq!(editor.body.text(), "Co-authored-by: ");
    assert_eq!(editor.focus, Field::Body);
    assert_eq!(editor.body.cursor(), (0, "Co-authored-by: ".len()));
}

#[test]
fn nothing_staged_refuses_and_the_button_does_nothing() {
    let (open, _) = update(&ready(&[]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let view = crate::view::commit_editor_view(&open).expect("open");
    assert_eq!(
        view.refusal.as_deref().map(str::to_owned),
        Some("nothing staged".to_owned())
    );
    let (after, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
    assert!(after.operating.is_none());
    assert!(after.commit_editor.is_some(), "and the editor stays open");
}

#[test]
fn an_empty_subject_refuses() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let view = crate::view::commit_editor_view(&open).expect("open");
    assert_eq!(
        view.refusal.as_deref().map(str::to_owned),
        Some("subject empty".to_owned())
    );
}

#[test]
fn a_merge_in_progress_refuses_and_names_the_command() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(graft_git::state::InProgress::Merge);
    let (open, _) = update(&state, key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let view = crate::view::commit_editor_view(&open).expect("open");
    let refusal = view.refusal.as_deref().expect("a refusal").to_owned();
    assert!(refusal.contains("merge in progress"), "{refusal}");
    assert!(refusal.contains("git commit"), "{refusal}");
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
}

#[test]
fn allow_empty_lets_a_commit_through_with_nothing_staged() {
    let (open, _) = update(&ready(&[]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    // Tab to the options, then right three times to `allow empty`.
    let (open, _) = update(&open, key(KeyCode::Tab));
    let (open, _) = update(&open, key(KeyCode::Tab));
    let mut open = open;
    for _ in 0..3 {
        let (next, _) = update(&open, key(KeyCode::Right));
        open = next;
    }
    assert_eq!(
        open.commit_editor.as_ref().expect("open").focus,
        Field::AllowEmpty
    );
    let (toggled, _) = update(&open, key(KeyCode::Char(' ')));
    assert!(toggled.commit_editor.as_ref().expect("open").allow_empty);
    let (_, commands) = update(&toggled, control(KeyCode::Enter));
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("{commands:?}");
    };
    assert!(params.allow_empty);
}

#[test]
fn a_asks_for_heads_message_and_opens_on_the_answer() {
    let (asked, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('a')));
    assert!(
        asked.commit_editor.is_none(),
        "nothing on screen until the message lands"
    );
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (open, _) = update(
        &asked,
        AppEvent::Produced(
            *id,
            Product::HeadMessage(Ok("old subject\n\nold body\n".into())),
        ),
    );
    let editor = open.commit_editor.as_ref().expect("the editor opened");
    assert!(editor.amend);
    assert_eq!(editor.subject.text(), "old subject");
    assert_eq!(editor.body.text(), "old body");
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(
        matches!(operate(&commands), Some(Operation::Amend(_))),
        "{commands:?}"
    );
}

#[test]
fn a_says_why_when_heads_message_cannot_be_read() {
    let (asked, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('a')));
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (told, _) = update(
        &asked,
        AppEvent::Produced(
            *id,
            Product::HeadMessage(Err("nothing committed yet".into())),
        ),
    );
    assert!(told.commit_editor.is_none());
    let Status::WriteFailed(said) = &told.status else {
        panic!("{:?}", told.status);
    };
    assert!(said.contains("nothing committed yet"), "{said}");
}

#[test]
fn a_stale_head_message_is_dropped() {
    let (asked, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('a')));
    let (ignored, _) = update(
        &asked,
        AppEvent::Produced(JobId::new(9999), Product::HeadMessage(Ok("other".into()))),
    );
    assert!(ignored.commit_editor.is_none());
}

#[test]
fn a_template_fills_an_empty_body_and_drops_its_comment_lines() {
    let (open, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let Some(Command::ReadCommitTemplate(id)) = commands.first() else {
        panic!("`c` asks for the template: {commands:?}");
    };
    let (filled, _) = update(
        &open,
        AppEvent::Produced(
            *id,
            Product::CommitTemplate(Ok(Some(
                "# a comment git would strip\nWhy:\nHow:\n".to_owned(),
            ))),
        ),
    );
    assert_eq!(
        filled.commit_editor.as_ref().expect("open").body.text(),
        "Why:\nHow:"
    );
}

#[test]
fn a_template_never_overwrites_what_was_already_typed() {
    let (open, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let Some(Command::ReadCommitTemplate(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (open, _) = update(&open, key(KeyCode::Tab)); // to the body
    let open = typed(&open, "mine");
    let (after, _) = update(
        &open,
        AppEvent::Produced(*id, Product::CommitTemplate(Ok(Some("theirs".to_owned())))),
    );
    assert_eq!(
        after.commit_editor.as_ref().expect("open").body.text(),
        "mine"
    );
}

#[test]
fn esc_keeps_a_draft_and_the_next_c_picks_it_back_up() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "half a thought");
    let (closed, _) = update(&open, key(KeyCode::Escape));
    assert!(closed.commit_editor.is_none());
    assert!(closed.commit_draft.is_some(), "the draft is kept");

    let (reopened, _) = update(&closed, key(KeyCode::Char('c')));
    let editor = reopened.commit_editor.as_ref().expect("reopened");
    assert_eq!(editor.subject.text(), "half a thought");
    assert!(editor.from_draft);
    let view = crate::view::commit_editor_view(&reopened).expect("open");
    assert!(view.title.as_str().contains("draft kept"), "{}", view.title);
}

#[test]
fn esc_on_an_empty_editor_keeps_nothing() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let (closed, _) = update(&open, key(KeyCode::Escape));
    assert!(closed.commit_draft.is_none());
}

#[test]
fn ctrl_c_still_quits_from_inside_the_editor() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let (quit, _) = update(&open, control(KeyCode::Char('c')));
    assert!(quit.quit.is_some(), "Ctrl+C is never swallowed");
}

#[test]
fn ctrl_e_asks_for_the_message_to_go_out_to_an_editor() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (_, commands) = update(&open, control(KeyCode::Char('e')));
    let Some(Command::EditCommitMessage { message, .. }) = commands
        .iter()
        .find(|command| matches!(command, Command::EditCommitMessage { .. }))
    else {
        panic!("{commands:?}");
    };
    assert_eq!(message, "subject\n");
}

/// `Ctrl+E`'s id, and the state that is waiting on it.
fn sent_to_the_editor(subject: &str) -> (AppState, JobId) {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, subject);
    let (sent, commands) = update(&open, control(KeyCode::Char('e')));
    let Some(Command::EditCommitMessage { id, .. }) = commands
        .iter()
        .find(|command| matches!(command, Command::EditCommitMessage { .. }))
    else {
        panic!("{commands:?}");
    };
    (sent, *id)
}

#[test]
fn what_the_external_editor_saved_replaces_the_fields() {
    let (open, id) = sent_to_the_editor("typed here");
    let (back, _) = update(
        &open,
        AppEvent::Produced(
            id,
            Product::ExternalEditor(Ok("from vim\n\nand a body\n# stripped\n".to_owned())),
        ),
    );
    let editor = back.commit_editor.as_ref().expect("open");
    assert_eq!(editor.subject.text(), "from vim");
    assert_eq!(editor.body.text(), "and a body");
}

#[test]
fn an_external_editor_that_failed_keeps_the_old_text_and_says_so() {
    let (open, id) = sent_to_the_editor("typed here");
    let (back, _) = update(
        &open,
        AppEvent::Produced(
            id,
            Product::ExternalEditor(Err("editor exited 1, message unchanged".to_owned())),
        ),
    );
    assert_eq!(
        back.commit_editor.as_ref().expect("open").subject.text(),
        "typed here"
    );
    let Status::WriteFailed(said) = &back.status else {
        panic!("{:?}", back.status);
    };
    assert!(said.contains("message unchanged"), "{said}");
}

#[test]
fn while_the_commit_runs_only_esc_is_answered() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let id = running.operating.expect("a write is in flight");

    let (typed_at, _) = update(&running, key(KeyCode::Char('x')));
    assert_eq!(
        typed_at
            .commit_editor
            .as_ref()
            .expect("open")
            .subject
            .text(),
        "subject",
        "keys are ignored while it runs"
    );

    let (cancelled, commands) = update(&running, key(KeyCode::Escape));
    assert!(
        commands.contains(&Command::CancelJob(id)),
        "esc cancels the job: {commands:?}"
    );
    assert!(cancelled.commit_editor.is_some(), "and leaves it on screen");
}

#[test]
fn the_title_says_committing_while_the_write_runs() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let view = crate::view::commit_editor_view(&running).expect("open");
    assert!(view.busy);
    assert!(
        view.title.as_str().contains("committing…"),
        "{}",
        view.title
    );
}

/// `Esc` while `$EDITOR` is open means the draft is the user's again: what
/// the editor saves after that belongs to a screen that is gone.
#[test]
fn a_message_from_an_editor_the_user_has_closed_is_dropped() {
    let (open, id) = sent_to_the_editor("mine");
    let (closed, _) = update(&open, key(KeyCode::Escape));
    let (reopened, _) = update(&closed, key(KeyCode::Char('c')));
    let (after, _) = update(
        &reopened,
        AppEvent::Produced(id, Product::ExternalEditor(Ok("from vim".to_owned()))),
    );
    assert_eq!(
        after.commit_editor.as_ref().expect("open").subject.text(),
        "mine"
    );
}

/// `Ctrl+Z` is undo's key by default (ADR-0002) and must not be swallowed
/// here; with `[keys] suspend_on_ctrl_z` on it stops Graft instead.
#[test]
fn ctrl_z_suspends_only_when_the_config_says_so() {
    let quiet = ready(&["a.rs"]);
    let (_, commands) = update(&quiet, control(KeyCode::Char('z')));
    assert!(
        !commands.contains(&Command::Suspend),
        "left for undo: {commands:?}"
    );

    let mut asked = quiet;
    asked.suspend_on_ctrl_z = true;
    let (_, commands) = update(&asked, control(KeyCode::Char('z')));
    assert!(commands.contains(&Command::Suspend), "{commands:?}");
}

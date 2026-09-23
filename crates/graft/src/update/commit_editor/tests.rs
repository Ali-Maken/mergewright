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
                unmerged_how: None,
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

/// A merge Graft began: its journal entry is live, so the banner's `c` and
/// `a` end it.
#[test]
fn a_merge_in_progress_refuses_and_names_the_keys() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: None,
    });
    state.live_entries = vec![graft_core::journal::EntryId::new(1, [0; 10])];
    let (open, _) = update(&state, key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let view = crate::view::commit_editor_view(&open).expect("open");
    let refusal = view.refusal.as_deref().expect("a refusal").to_owned();
    assert!(refusal.contains("merge in progress"), "{refusal}");
    // m5-history-and-ghost-preview/15, piece 6: the refusal names the
    // banner's keys, not only the shell command ADR-0034's interim showed.
    // The banner is where a merge ends, either way, and it is two rows up.
    assert!(refusal.contains("c continue"), "{refusal}");
    assert!(refusal.contains("a abort"), "{refusal}");
    assert!(
        !refusal.contains("git commit"),
        "the shell command is no longer the only way out: {refusal}"
    );
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
}

/// **A merge begun in the shell names no key that refuses it** (m6-07
/// F12). No journal entry of Graft's is live, so the banner's `c` and `a`
/// both refuse it (`graft-ops/src/apply/resume.rs`, `resume_target`:
/// `Refusal::NotInProgress`, "an operation begun in the shell is finished
/// there"). The editor said `esc, then c continue  a abort` — two keys that
/// do nothing, the thing `resolver/answers.rs`'s last line refuses to do.
/// It names git's own command instead, as the bisect arm does.
#[test]
fn a_merge_begun_in_the_shell_names_no_key_that_refuses_it() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: None,
    });
    assert!(state.live_entries.is_empty(), "nothing of Graft's is live");
    let (open, _) = update(&state, key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let view = crate::view::commit_editor_view(&open).expect("open");
    let refusal = view.refusal.as_deref().expect("a refusal").to_owned();
    assert!(refusal.contains("merge"), "{refusal}");
    assert!(refusal.contains("shell"), "{refusal}");
    assert!(refusal.contains("git commit"), "{refusal}");
    for dead in ["c continue", "a abort", "esc, then"] {
        assert!(!refusal.contains(dead), "offers {dead:?}: {refusal}");
    }
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
}

/// A bisect draws no banner (M7's), so there are no keys to name and the
/// refusal keeps git's own command — a refusal must never offer a key that
/// is not live.
#[test]
fn a_bisect_in_progress_still_names_the_shell_command() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(graft_git::state::InProgressDetail {
        kind: graft_git::state::InProgress::Bisect,
        step: None,
        head_name: None,
        onto: None,
        theirs: None,
    });
    let (open, _) = update(&state, key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let view = crate::view::commit_editor_view(&open).expect("open");
    let refusal = view.refusal.as_deref().expect("a refusal").to_owned();
    assert!(refusal.contains("git bisect good/bad"), "{refusal}");
    assert!(!refusal.contains("c continue"), "{refusal}");
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
    let Status::Sticky { reason: said, .. } = &told.status else {
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
    let Status::Sticky { reason: said, .. } = &back.status else {
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

/// A pre-commit hook that says no is the common way a commit fails, and the
/// worst possible answer is an editor that closed with the message gone.
#[test]
fn a_commit_that_failed_keeps_the_editor_and_the_message() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let id = running.operating.expect("a write is in flight");

    let (after, _) = update(
        &running,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Failed {
                error: "pre-commit hook: 1 file would be reformatted".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
            })),
        ),
    );

    let editor = after.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.subject.text(), "subject");
    assert!(after.operating.is_none(), "the write is over");
    let Status::Sticky { reason: said, .. } = &after.status else {
        panic!("expected the reason, got {:?}", after.status);
    };
    assert!(said.contains("pre-commit hook"), "{said}");
}

/// Major 3 of the m3/05 review. Graft turns bracketed paste *on*, and the
/// editor dropped every paste on the floor — pasting a commit message into
/// it did nothing at all.
#[test]
fn a_paste_lands_in_the_subject() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));

    let (pasted, _) = update(
        &open,
        AppEvent::Core(Event::Paste("teach the editor to paste".to_owned())),
    );

    let editor = pasted.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.subject.text(), "teach the editor to paste");
}

/// The subject is one line, so a pasted message's newlines go; the body
/// keeps them, which is the whole point of pasting a message into it.
#[test]
fn the_subject_flattens_a_paste_and_the_body_keeps_its_lines() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let (into_subject, _) = update(&open, AppEvent::Core(Event::Paste("one\ntwo".to_owned())));
    assert_eq!(
        into_subject
            .commit_editor
            .as_ref()
            .expect("open")
            .subject
            .text(),
        "onetwo",
        "a newline is not typable in a one-line field"
    );

    let (on_body, _) = update(&open, key(KeyCode::Tab));
    let (into_body, _) = update(
        &on_body,
        AppEvent::Core(Event::Paste(
            "why this change\n\nand what it costs".to_owned(),
        )),
    );
    assert_eq!(
        into_body.commit_editor.as_ref().expect("open").body.text(),
        "why this change\n\nand what it costs"
    );
}

/// A paste with no editor open changes nothing and panics on nothing.
#[test]
fn a_paste_with_no_editor_open_is_ignored() {
    let quiet = ready(&["a.rs"]);

    let (after, commands) = update(
        &quiet,
        AppEvent::Core(Event::Paste("nowhere to put this".to_owned())),
    );

    assert!(after.commit_editor.is_none());
    assert!(commands.is_empty(), "{commands:?}");
}

/// Major 1 of the m3/05 review. A hook's refusal used to reach `AppState`
/// and stop there: the editor drew no status, so a rejected commit looked
/// like nothing had happened.
#[test]
fn a_refused_commits_reason_reaches_the_editors_own_view() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let id = running.operating.expect("a write is in flight");

    let (after, _) = update(
        &running,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Failed {
                error: "pre-commit hook: 1 file would be reformatted".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
            })),
        ),
    );

    let view = crate::view::commit_editor_view(&after).expect("the editor is still open");
    let said = view
        .status
        .expect("the reason is on the editor")
        .to_string();
    assert!(said.contains("pre-commit hook"), "{said}");
}

/// And `Esc` dismisses it without closing the editor — the same two-stage
/// `Esc` the graph screen has, so the message can be read and then got rid
/// of without throwing the typed subject away.
#[test]
fn esc_dismisses_the_reason_first_and_closes_the_editor_second() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let id = running.operating.expect("a write is in flight");
    let (failed, _) = update(
        &running,
        AppEvent::Produced(
            id,
            Product::Operated(Box::new(crate::jobs::operate::Operated::Failed {
                error: "pre-commit hook said no".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
            })),
        ),
    );

    let (dismissed, _) = update(&failed, key(KeyCode::Escape));
    assert_eq!(dismissed.status, Status::Ready, "the reason is gone");
    let editor = dismissed.commit_editor.as_ref().expect("the editor stayed");
    assert_eq!(editor.subject.text(), "subject", "and so did the message");

    let (closed, _) = update(&dismissed, key(KeyCode::Escape));
    assert!(closed.commit_editor.is_none(), "the second Esc closes it");
}

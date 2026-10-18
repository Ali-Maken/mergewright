//! The commit editor's keys, as plain function calls over plain data.
//!
//! Every test here drives `update` itself rather than the module's private
//! helpers, so what it proves is what a key press actually does.

use std::sync::Arc;

use mergewright_core::event::{Event, KeyCode, KeyPress, Modifiers};
use mergewright_core::job::JobId;
use mergewright_git::status::{Change, StatusEntry, WorkingTreeStatus};
use mergewright_ops::operation::Operation;

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
                submodule: None,
            })
            .collect(),
    })
}

/// A state with a working tree read and room to draw.
pub(crate) fn ready(paths: &[&str]) -> AppState {
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
        Command::Operate(_, op, ..) => Some(&**op),
        _ => None,
    })
}

#[test]
fn c_opens_the_editor_and_typing_lands_in_the_subject() {
    let (open, _) = update(&ready(&["a.rs", "b.rs"]), key(KeyCode::Char('c')));
    let editor = open.commit_editor.as_ref().expect("`c` opens the editor");
    assert_eq!(editor.focus, CommitFocus::Subject);

    let typed = typed(&open, "add a thing");
    let editor = typed.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.subject.text(), "add a thing");
    assert!(editor.body.is_empty());
}

#[test]
fn the_summary_counts_the_files_going_in() {
    let (open, _) = update(&ready(&["a.rs", "b.rs"]), key(KeyCode::Char('c')));
    let view = crate::view::commit_screen_view(&open).expect("the editor is open");
    assert!(
        matches!(
            view.summary,
            mergewright_ui::commit_editor::Summary::Goes {
                files: 2,
                out: 0,
                ..
            }
        ),
        "{:?}",
        view.summary
    );
}

/// `⏎` from the subject against `⌃⏎` from a clone of the same state: the
/// commands and what is left on screen must be the same (P16: `⏎` is `⌃⏎`
/// while the subject has focus).
fn enter_matches_ctrl_enter(open: &AppState) -> (AppState, Vec<Command>) {
    assert_eq!(
        open.commit_editor.as_ref().expect("open").focus,
        CommitFocus::Subject,
        "the press under test is from the subject"
    );
    let (entered, commands) = update(open, key(KeyCode::Enter));
    let (controlled, control_commands) = update(&open.clone(), control(KeyCode::Enter));
    assert_eq!(commands, control_commands, "⏎ sends what ⌃⏎ sends");
    assert_eq!(entered.operating, controlled.operating);
    (entered, commands)
}

#[test]
fn enter_in_the_subject_commits() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "fix: x");
    let (fired, commands) = enter_matches_ctrl_enter(&open);
    assert_eq!(
        commands
            .iter()
            .filter(|command| matches!(command, Command::Operate(..)))
            .count(),
        1,
        "{commands:?}"
    );
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("⏎ in the subject commits: {commands:?}");
    };
    assert_eq!(params.message, "fix: x\n");
    assert!(fired.operating.is_some(), "one write, and it is in flight");
}

#[test]
fn enter_in_the_subject_commits_the_body_too() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (open, _) = update(&open, key(KeyCode::Tab)); // to the body
    let open = typed(&open, "body");
    let (open, _) = update(&open, key(KeyCode::BackTab)); // back to the subject
    let (_, commands) = enter_matches_ctrl_enter(&open);
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("{commands:?}");
    };
    assert_eq!(params.message, "subject\n\nbody\n");
}

#[test]
fn enter_with_a_template_commits_what_ctrl_enter_would() {
    let (open, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let Some(Command::ReadCommitTemplate(id)) = commands.first() else {
        panic!("`c` asks for the template: {commands:?}");
    };
    let (filled, _) = update(
        &open,
        AppEvent::Produced(
            *id,
            Product::CommitTemplate(Ok(Some("# comment\nWhy:\n".to_owned()))),
        ),
    );
    let filled = typed(&filled, "subject");
    let (_, commands) = enter_matches_ctrl_enter(&filled);
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("{commands:?}");
    };
    assert_eq!(params.message, "subject\n\nWhy:\n");
}

#[test]
fn a_refused_enter_sends_nothing() {
    let mut merging = ready(&["a.rs"]);
    merging.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: mergewright_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    merging.live_entries = vec![mergewright_core::journal::EntryId::new(1, [0; 10])];
    let rows: Vec<(&str, AppState, &str, &str)> = vec![
        (
            "nothing staged",
            ready(&[]),
            "subject",
            "put a file in first",
        ),
        (
            "empty subject",
            ready(&["a.rs"]),
            "",
            "type a message first",
        ),
        (
            "a merge in progress",
            merging.clone(),
            "subject",
            "merge in progress",
        ),
    ];
    for (row, state, subject, words) in rows {
        let open = if state.in_progress.is_some() {
            opened_under(&state)
        } else {
            update(&state, key(KeyCode::Char('c'))).0
        };
        let open = typed(&open, subject);
        let (after, commands) = enter_matches_ctrl_enter(&open);
        assert!(commands.is_empty(), "{row}: {commands:?}");
        assert!(after.operating.is_none(), "{row}");
        let before = open.commit_editor.as_ref().expect("open");
        let editor = after.commit_editor.as_ref().expect("still open");
        assert_eq!(editor.focus, CommitFocus::Subject, "{row}: focus stays");
        assert_eq!(editor.subject.text(), before.subject.text(), "{row}");
        assert_eq!(editor.body.text(), before.body.text(), "{row}");
        let (controlled, _) = update(&open, control(KeyCode::Enter));
        let shown = shown_refusal(&after).expect("a refusal row");
        assert_eq!(Some(shown.clone()), shown_refusal(&controlled), "{row}");
        assert!(shown.contains(words), "{row}: {shown}");
    }
}

#[test]
fn enter_while_a_job_runs_is_eaten_like_ctrl_enter() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let id = running.operating.expect("a write is in flight");
    let (after, commands) = enter_matches_ctrl_enter(&running);
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(after.operating, Some(id));
    let before = running.commit_editor.as_ref().expect("open");
    let editor = after.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.focus, CommitFocus::Subject);
    assert_eq!(editor.subject.text(), before.subject.text());
    assert_eq!(editor.body.text(), before.body.text());
    let (_, commands) = update(&after, key(KeyCode::Escape));
    assert!(
        commands.contains(&Command::CancelJob(id)),
        "esc is still answered: {commands:?}"
    );
}

#[test]
fn enter_in_the_subject_amends_in_the_amend_editor() {
    let (asked, commands) = update(&ready(&["a.rs"]), key(KeyCode::Char('a')));
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let (open, _) = update(
        &asked,
        AppEvent::Produced(*id, Product::HeadMessage(Ok("old subject\n".into()))),
    );
    let (_, commands) = enter_matches_ctrl_enter(&open);
    let Some(Operation::Amend(params)) = operate(&commands) else {
        panic!("⏎ in the amend editor's subject amends: {commands:?}");
    };
    assert_eq!(params.message, "old subject\n");
}

#[test]
fn a_pasted_newline_does_not_commit() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let mut pasted = open.clone();
    assert!(paste(&mut pasted, "fix: x\n"));
    assert!(pasted.operating.is_none(), "a paste writes nothing");
    let editor = pasted.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.focus, CommitFocus::Subject);
    assert_eq!(editor.subject.text(), "fix: x");
}

#[test]
fn tab_still_moves_to_the_body() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (moved, commands) = update(&open, key(KeyCode::Tab));
    assert!(commands.is_empty(), "{commands:?}");
    let editor = moved.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.focus, CommitFocus::Body);
    assert_eq!(editor.subject.text(), "subject");
    assert!(editor.body.is_empty());
}

#[test]
fn enter_in_the_body_is_a_newline() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (open, _) = update(&open, key(KeyCode::Tab));
    let open = typed(&open, "one");
    let (open, commands) = update(&open, key(KeyCode::Enter));
    assert!(commands.is_empty(), "{commands:?}");
    let open = typed(&open, "two");
    let editor = open.commit_editor.as_ref().expect("still open");
    assert_eq!(editor.focus, CommitFocus::Body);
    assert_eq!(editor.body.text(), "one\ntwo");
    assert!(open.operating.is_none());
}

#[test]
fn tab_walks_subject_body_files_and_back() {
    let (mut state, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let expected = [CommitFocus::Body, CommitFocus::Files, CommitFocus::Subject];
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
fn enter_from_the_files_commits_the_subject_and_the_body() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (open, _) = update(&open, key(KeyCode::Tab)); // to the body
    let open = typed(&open, "body");
    let (open, _) = update(&open, key(KeyCode::Tab)); // to the files
    assert_eq!(
        open.commit_editor.as_ref().expect("still open").focus,
        CommitFocus::Files
    );
    let (fired, commands) = update(&open, key(KeyCode::Enter));
    let Some(Operation::Commit(params)) = operate(&commands) else {
        panic!("⏎ from the files commits: {commands:?}");
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
    assert_eq!(editor.focus, CommitFocus::Body);
    assert_eq!(editor.body.cursor(), (0, "Co-authored-by: ".len()));
}

#[test]
fn nothing_going_in_refuses_and_enter_does_nothing() {
    let (open, _) = update(&ready(&[]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    assert_eq!(shown_refusal(&open).as_deref(), Some("put a file in first"));
    let (after, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
    assert!(after.operating.is_none());
    assert!(after.commit_editor.is_some(), "and the editor stays open");
}

/// What the bar says `⏎` would not do, or the live operation's words in
/// its place (C9, m6-conflicts/48).
fn shown_refusal(state: &AppState) -> Option<String> {
    let view = crate::view::commit_screen_view(state).expect("open");
    view.live
        .map(|words| words.as_str().to_owned())
        .or_else(|| {
            view.refusal.map(|refusal| {
                match refusal {
                    mergewright_ui::commit_editor::Refusal::NoMessage => "type a message first",
                    mergewright_ui::commit_editor::Refusal::NoFile => "put a file in first",
                }
                .to_owned()
            })
        })
}

/// C9: before a word is typed the bar's `⏎` slot already says why it
/// would do nothing — a key's label, not a failure.
#[test]
fn a_fresh_commit_editor_says_type_a_message_first() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    assert_eq!(
        shown_refusal(&open).as_deref(),
        Some("type a message first")
    );
    assert_eq!(open.status, Status::Ready, "nothing on the status row");
}

#[test]
fn commit_with_empty_subject_commits_nothing() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    for press in [key(KeyCode::Enter), control(KeyCode::Enter)] {
        let (after, commands) = update(&open, press);
        assert!(operate(&commands).is_none(), "{commands:?}");
        assert_eq!(
            shown_refusal(&after).as_deref(),
            Some("type a message first")
        );
    }
}

#[test]
fn typing_clears_the_refusal() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let (typed_one, _) = update(&open, key(KeyCode::Char('x')));
    assert_eq!(shown_refusal(&typed_one), None);
    let (erased, _) = update(&typed_one, key(KeyCode::Backspace));
    assert_eq!(
        shown_refusal(&erased).as_deref(),
        Some("type a message first")
    );
}

/// The editor opened with `c` over `state`'s live operation. Since
/// m6-conflicts/48 `c` refuses mid-operation, so the editor is opened with the
/// operation hidden and the operation put back under it: an editor already
/// open when an operation began, the case its refusal row is kept for.
fn opened_under(state: &AppState) -> AppState {
    let mut hidden = state.clone();
    hidden.in_progress = None;
    hidden.live_entries = Vec::new();
    let (mut open, _) = update(&hidden, key(KeyCode::Char('c')));
    assert!(open.commit_editor.is_some(), "`c` opens the editor");
    open.in_progress.clone_from(&state.in_progress);
    open.live_entries.clone_from(&state.live_entries);
    open
}

/// A merge Mergewright began: its journal entry is live, so the banner's `c` and
/// `a` end it.
#[test]
fn a_merge_in_progress_refuses_and_names_the_keys() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: mergewright_git::state::InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    state.live_entries = vec![mergewright_core::journal::EntryId::new(1, [0; 10])];
    let open = opened_under(&state);
    let open = typed(&open, "subject");
    let refusal = shown_refusal(&open).expect("a refusal");
    assert!(refusal.contains("merge in progress"), "{refusal}");
    // m5-history-and-ghost-preview/15, piece 6: the refusal names the
    // banner's keys, not only the shell command ADR-0034's interim showed.
    // The banner is where a merge ends, either way, and it is two rows up.
    // A merge's `c` is `c finish` (m6-conflicts/70, n2).
    assert!(refusal.contains("c finish"), "{refusal}");
    assert!(refusal.contains("a abort"), "{refusal}");
    assert!(
        !refusal.contains("git commit"),
        "the shell command is no longer the only way out: {refusal}"
    );
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
}

/// The editor's refusal for an operation begun in the shell, `kind` given.
fn shell_refusal(kind: mergewright_git::state::InProgress) -> String {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    assert!(
        state.live_entries.is_empty(),
        "nothing of Mergewright's is live"
    );
    let open = opened_under(&state);
    let open = typed(&open, "subject");
    let refusal = shown_refusal(&open).expect("a refusal");
    let (_, commands) = update(&open, control(KeyCode::Enter));
    assert!(operate(&commands).is_none(), "{commands:?}");
    refusal
}

/// **A merge begun in the shell points at the banner's `c`** (m6-conflicts/39).
/// The banner's `c` finishes it since ticket 20; after `Esc` the pane's `c`
/// is Commit (keymap §2.4), so the words name `⇥ banner` to reach it. No
/// `a abort`: the banner's `a` still refuses a shell merge (M6/08).
#[test]
fn a_merge_in_progress_points_at_c() {
    let refusal = shell_refusal(mergewright_git::state::InProgress::Merge);
    assert_eq!(
        refusal,
        "merge begun in the shell — esc, ⇥ banner, then c finishes it"
    );
    for dead in ["git commit", "a abort"] {
        assert!(!refusal.contains(dead), "offers {dead:?}: {refusal}");
    }
    assert!(refusal.contains("⇥ banner"), "{refusal}");
}

/// **Every operation Mergewright began names the way to the banner**
/// (m6-conflicts/39): `esc, ⇥ banner, then `, never `esc, then c`, because
/// after `Esc` the pane's `c`/`a` are Commit/Amend (keymap §2.4).
#[test]
fn a_graft_begun_operation_names_the_banner() {
    use mergewright_git::state::InProgress;
    for kind in [
        InProgress::Merge,
        InProgress::Rebase,
        InProgress::CherryPick,
        InProgress::Revert,
    ] {
        let refusal = operation_refusal(
            (kind, Begun::InMergewright),
            crate::update::banner::BannerWay::tabs(1),
            mergewright_ui::keymap::Keymap::shipped(),
        )
        .editor_words();
        assert!(refusal.contains("esc, ⇥ banner, then "), "{refusal}");
        assert!(!refusal.contains("esc, then c"), "{refusal}");
    }
    assert_eq!(
        operation_refusal(
            (InProgress::Merge, Begun::InMergewright),
            crate::update::banner::BannerWay::tabs(1),
            mergewright_ui::keymap::Keymap::shipped(),
        )
        .editor_words(),
        "merge in progress — esc, ⇥ banner, then c finish  a abort"
    );
}

/// A rebase begun in the shell: `c` hands it to git (07c), so the words
/// name `c`. A revert the shell began still names git's own command.
#[test]
fn a_rebase_begun_in_the_shell_names_c_handing_it_to_git() {
    let refusal = shell_refusal(mergewright_git::state::InProgress::Rebase);
    assert_eq!(refusal, "rebase begun in the shell — c hands it to git");
    let refusal = shell_refusal(mergewright_git::state::InProgress::CherryPick);
    assert_eq!(
        refusal,
        "cherry-pick begun in the shell — c hands it to git"
    );
    let refusal = shell_refusal(mergewright_git::state::InProgress::Revert);
    assert_eq!(
        refusal,
        "revert begun in the shell — finish it there: git revert --continue"
    );
}

/// A bisect draws no banner (M7's), so there are no keys to name and the
/// refusal keeps git's own command — a refusal must never offer a key that
/// is not live.
#[test]
fn a_bisect_in_progress_still_names_the_shell_command() {
    let mut state = ready(&["a.rs"]);
    state.in_progress = Some(mergewright_git::state::InProgressDetail {
        kind: mergewright_git::state::InProgress::Bisect,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    let open = opened_under(&state);
    let open = typed(&open, "subject");
    let refusal = shown_refusal(&open).expect("a refusal");
    assert!(refusal.contains("git bisect good/bad"), "{refusal}");
    assert!(!refusal.contains("c continue"), "{refusal}");
}

#[test]
fn allow_empty_lets_a_commit_through_with_nothing_going_in() {
    let (open, _) = update(&ready(&[]), key(KeyCode::Char('c')));
    let mut open = typed(&open, "subject");
    assert!(toggle_from_palette(
        &mut open,
        mergewright_ui::keymap::ActionId("commit.toggle_allow_empty")
    ));
    assert!(open.commit_editor.as_ref().expect("open").allow_empty);
    let (_, commands) = update(&open, control(KeyCode::Enter));
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
    assert!(closed.drafts.commit.is_some(), "the draft is kept");

    let (reopened, _) = update(&closed, key(KeyCode::Char('c')));
    let editor = reopened.commit_editor.as_ref().expect("reopened");
    assert_eq!(editor.subject.text(), "half a thought");
    assert!(editor.from_draft);
}

#[test]
fn esc_on_an_empty_editor_keeps_nothing() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let (closed, _) = update(&open, key(KeyCode::Escape));
    assert!(closed.drafts.commit.is_none());
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
fn the_bar_says_committing_while_the_write_runs() {
    let (open, _) = update(&ready(&["a.rs"]), key(KeyCode::Char('c')));
    let open = typed(&open, "subject");
    let (running, _) = update(&open, control(KeyCode::Enter));
    let view = crate::view::commit_screen_view(&running).expect("open");
    assert!(view.busy);
    let bar = mergewright_ui::commit_editor::bar(&view, mergewright_ui::keymap::Keymap::shipped());
    assert!(bar.contains("committing…"), "{bar}");
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
/// here; with `[keys] suspend_on_ctrl_z` on it stops Mergewright instead.
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
                edits: None,
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

/// Major 3 of the m3/05 review. Mergewright turns bracketed paste *on*, and the
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
                edits: None,
            })),
        ),
    );

    let view = crate::view::commit_screen_view(&after).expect("the editor is still open");
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
                edits: None,
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

/// m6-conflicts/40: at 80 columns the bar's row has 79 columns after its
/// 1-column indent, and every live operation's words fit in them whole.
#[test]
fn every_refusal_fits_80_columns() {
    use mergewright_git::state::InProgress;
    use unicode_width::UnicodeWidthStr;

    let mut arms: Vec<String> = Vec::new();
    for kind in [
        InProgress::Merge,
        InProgress::Rebase,
        InProgress::CherryPick,
        InProgress::Revert,
        InProgress::Bisect,
    ] {
        for begun in [Begun::InMergewright, Begun::InShell] {
            arms.push(
                operation_refusal(
                    (kind, begun),
                    crate::update::banner::BannerWay::tabs(1),
                    mergewright_ui::keymap::Keymap::shipped(),
                )
                .editor_words(),
            );
        }
    }
    assert_eq!(arms.len(), 10, "every arm answered: {arms:#?}");
    let widest = arms.iter().max_by_key(|words| words.width()).expect("arms");
    for words in &arms {
        assert!(
            words.width() <= 78,
            "{} cols at 80 — {words}",
            words.width()
        );
    }
    assert_eq!(
        (widest.width(), widest.as_str()),
        (
            66,
            "revert begun in the shell — finish it there: git revert --continue"
        ),
        "the widest arm"
    );
}

// ------------------------------------------ the operation refusal (m6-conflicts/48)

/// The keys a Mergewright-begun operation's refusal names.
const MWR_KEYS: Option<&str> = Some("⇥ banner, then c continue  a abort");

/// Every `(InProgress, Begun)` pair: today's editor sentence, then the pane's
/// reason and keys.
type LiveRow = (
    (InProgress, Begun),
    &'static str,
    &'static str,
    Option<&'static str>,
);

/// See [`LiveRow`].
const EVERY_LIVE_OPERATION: [LiveRow; 10] = [
    // A merge's `c` opens the finish summary: `c finish` (m6-conflicts/70).
    (
        (InProgress::Merge, Begun::InMergewright),
        "merge in progress — esc, ⇥ banner, then c finish  a abort",
        "merge in progress",
        Some("⇥ banner, then c finish  a abort"),
    ),
    (
        (InProgress::Merge, Begun::InShell),
        "merge begun in the shell — esc, ⇥ banner, then c finishes it",
        "merge begun in the shell",
        Some("⇥ banner, then c finishes it"),
    ),
    (
        (InProgress::Rebase, Begun::InMergewright),
        "rebase in progress — esc, ⇥ banner, then c continue  a abort",
        "rebase in progress",
        MWR_KEYS,
    ),
    (
        (InProgress::Rebase, Begun::InShell),
        "rebase begun in the shell — c hands it to git",
        "rebase begun in the shell — c hands it to git",
        None,
    ),
    (
        (InProgress::CherryPick, Begun::InMergewright),
        "cherry-pick in progress — esc, ⇥ banner, then c continue  a abort",
        "cherry-pick in progress",
        MWR_KEYS,
    ),
    (
        (InProgress::CherryPick, Begun::InShell),
        "cherry-pick begun in the shell — c hands it to git",
        "cherry-pick begun in the shell — c hands it to git",
        None,
    ),
    (
        (InProgress::Revert, Begun::InMergewright),
        "revert in progress — esc, ⇥ banner, then c continue  a abort",
        "revert in progress",
        MWR_KEYS,
    ),
    (
        (InProgress::Revert, Begun::InShell),
        "revert begun in the shell — finish it there: git revert --continue",
        "revert begun in the shell — finish it there: git revert --continue",
        None,
    ),
    (
        (InProgress::Bisect, Begun::InMergewright),
        "bisect in progress — git bisect good/bad first",
        "bisect in progress — git bisect good/bad first",
        None,
    ),
    (
        (InProgress::Bisect, Begun::InShell),
        "bisect in progress — git bisect good/bad first",
        "bisect in progress — git bisect good/bad first",
        None,
    ),
];

/// The editor's row, over every live operation, is today's sentence byte
/// for byte (written before `OperationRefusal`, passing unchanged after).
#[test]
fn the_editor_words_are_unchanged() {
    for (live, words, _, _) in EVERY_LIVE_OPERATION {
        assert_eq!(
            operation_refusal(
                live,
                crate::update::banner::BannerWay::tabs(1),
                mergewright_ui::keymap::Keymap::shipped()
            )
            .editor_words(),
            words,
            "{live:?}"
        );
    }
}

/// The pane's answer is the editor's sentence split at its em dash, as the
/// `u` refusal's `Sticky { reason, keys }`.
#[test]
fn the_pane_status_is_the_editor_words_split() {
    for (live, _, reason, keys) in EVERY_LIVE_OPERATION {
        assert_eq!(
            operation_refusal(
                live,
                crate::update::banner::BannerWay::tabs(1),
                mergewright_ui::keymap::Keymap::shipped()
            )
            .status(),
            Status::Sticky {
                reason: reason.to_owned(),
                keys: keys.map(str::to_owned)
            },
            "{live:?}"
        );
    }
}

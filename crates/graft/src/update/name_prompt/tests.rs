//! The name prompt's keys and rules, over scripted state. What `b` does to
//! a real repository is `tests/it/name_prompt_by_key.rs`.

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;

use super::*;
use crate::update::events::AppEvent;
use crate::update::update;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is an oid")
}

fn name(full: &str) -> RefName {
    RefName::new(full.as_bytes().to_vec()).expect("a valid ref name")
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

fn control(character: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(character),
        modifiers: Modifiers::CONTROL,
    }
}

/// Three commits, `main` on row 1 and a tag `v1` on row 2, `main` checked
/// out and the cursor on row 0.
fn fixture() -> AppState {
    let refs = crate::refs::badges(
        &[
            (name("refs/heads/main"), oid(2)),
            (name("refs/tags/v1"), oid(1)),
        ],
        &HeadState::Branch(name("refs/heads/main")),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(graft_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("no ahead/behind walk can fail here");
    AppState {
        history: Some(crate::view::tests::history_of(3, 0)),
        refs: Some(std::sync::Arc::new(refs)),
        size: crate::update::Size {
            columns: 100,
            rows: 24,
        },
        ..AppState::default()
    }
}

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn type_in(state: &mut AppState, text: &str) {
    for character in text.chars() {
        let (next, _) = press(state, key(KeyCode::Char(character)));
        *state = next;
    }
}

fn typed(state: &AppState) -> &str {
    state
        .name_prompt
        .as_ref()
        .expect("the prompt is open")
        .field
        .text()
}

/// `b` on a commit row opens the prompt; every key after that is text.
fn prompt_open(tag: bool) -> AppState {
    let state = fixture();
    let (state, _) = press(&state, key(KeyCode::Char(if tag { 't' } else { 'b' })));
    assert!(state.name_prompt.is_some(), "the prompt opened");
    state
}

/// m3/06 Critical 1, the rule that cost a journaled undo: a modal text
/// field must claim every key, and must be in `text_field_open()`.
#[test]
fn name_prompt_swallows_bindings() {
    let mut state = prompt_open(false);
    assert!(
        state.text_field_open(),
        "the prompt is one of the text fields"
    );
    let before = state.clone();
    type_in(&mut state, "u.S");
    assert_eq!(typed(&state), "u.S", "every letter landed in the field");
    assert!(state.journal.shown == before.journal.shown, "`J` untouched");
    assert!(
        state.context_menu.is_none(),
        "`.` did not open the context menu"
    );
    assert!(state.stash.shown == before.stash.shown, "`S` did nothing");
    assert_eq!(
        state.history.as_ref().expect("a history").cursor,
        before.history.as_ref().expect("a history").cursor,
        "no motion key fired"
    );
    // `Ctrl+U` clears; `Esc` closes and runs nothing.
    let (state, _) = press(&state, control('u'));
    assert_eq!(typed(&state), "");
    let (state, commands) = press(&state, key(KeyCode::Escape));
    assert!(state.name_prompt.is_none(), "`Esc` closed it");
    assert!(commands.is_empty(), "and ran nothing");
}

/// The refusal is a preview drawn as the user types — and it says nothing
/// at all while there is nothing typed yet.
#[test]
fn name_prompt_refusal_previews_bad_and_existing_names() {
    let mut state = prompt_open(false);
    assert_eq!(
        name_refusal(&state),
        None,
        "an empty field is not a refusal: nothing has been named yet"
    );
    type_in(&mut state, "ok/name");
    assert_eq!(name_refusal(&state), None, "a name git would take");

    let mut state = prompt_open(false);
    type_in(&mut state, "bad name");
    assert_eq!(
        name_refusal(&state).as_deref(),
        Some("bad name: it contains a control character or a space"),
        "the same rule `Operation::check` applies, previewed"
    );

    let mut state = prompt_open(false);
    type_in(&mut state, "main");
    assert_eq!(name_refusal(&state).as_deref(), Some("exists"));

    // A tag name is checked against the tags, not the branches: `main` is
    // free as a tag and `v1` is not.
    let mut state = prompt_open(true);
    type_in(&mut state, "main");
    assert_eq!(name_refusal(&state), None, "no tag called `main`");
    let mut state = prompt_open(true);
    type_in(&mut state, "v1");
    assert_eq!(name_refusal(&state).as_deref(), Some("exists"));
}

/// `Enter` with a refusal showing keeps the box exactly as it is: pressing
/// harder does not make the name legal.
#[test]
fn enter_on_a_refused_name_does_nothing() {
    let mut state = prompt_open(false);
    type_in(&mut state, "main");
    let (after, commands) = press(&state, key(KeyCode::Enter));
    assert!(after.name_prompt.is_some(), "the box stayed open");
    assert_eq!(typed(&after), "main", "with the name still in it");
    assert!(commands.is_empty(), "and nothing ran");
}

/// An empty name is not a refusal to draw — it is one to say out loud, once
/// the user asks for it.
#[test]
fn enter_on_an_empty_name_says_so() {
    let state = prompt_open(false);
    let (after, commands) = press(&state, key(KeyCode::Enter));
    assert!(commands.is_empty(), "nothing ran");
    assert!(
        matches!(&after.status, Status::Sticky { reason, .. } if reason == "a branch needs a name"),
        "{:?}",
        after.status
    );
    let state = prompt_open(true);
    let (after, _) = press(&state, key(KeyCode::Enter));
    assert!(
        matches!(&after.status, Status::Sticky { reason, .. } if reason == "a tag needs a name"),
        "{:?}",
        after.status
    );
}

/// `⇥` flips the branch's checkout toggle, and is swallowed by the two
/// kinds that draw no toggle.
#[test]
fn tab_toggles_checkout_only_for_a_branch() {
    let state = prompt_open(false);
    assert_eq!(
        state.name_prompt.as_ref().map(|prompt| prompt.kind.clone()),
        Some(NameKind::Branch {
            at: oid(3),
            checkout: false
        }),
        "the cursor's row is the newest commit"
    );
    let (state, _) = press(&state, key(KeyCode::Tab));
    assert_eq!(
        state.name_prompt.as_ref().map(|prompt| prompt.kind.clone()),
        Some(NameKind::Branch {
            at: oid(3),
            checkout: true
        })
    );

    let tag = prompt_open(true);
    let (after, _) = press(&tag, key(KeyCode::Tab));
    assert_eq!(
        after.name_prompt.map(|prompt| prompt.kind),
        Some(NameKind::Tag { at: oid(3) }),
        "a tag has nothing to check out"
    );
}

/// A create moves no history, so it goes straight to the write — there is
/// no result to draw a picture of.
#[test]
fn enter_launches_the_create() {
    let mut state = prompt_open(false);
    type_in(&mut state, "release");
    let (after, commands) = press(&state, key(KeyCode::Enter));
    assert!(after.name_prompt.is_none(), "the box closed");
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Operate(_, op)
                if matches!(op.as_ref(), Operation::BranchCreate(params)
                    if params.name == "release" && params.at == oid(3) && !params.checkout)
        )),
        "{commands:?}"
    );

    let mut state = prompt_open(true);
    type_in(&mut state, "v2");
    let (_, commands) = press(&state, key(KeyCode::Enter));
    assert!(
        commands.iter().any(|command| matches!(
            command,
            Command::Operate(_, op)
                if matches!(op.as_ref(), Operation::TagCreate(params)
                    if params.name == "v2" && params.message.is_none())
        )),
        "a lightweight tag: {commands:?}"
    );
}

/// The working-tree row names no commit, so there is nothing to branch
/// from — and the app says which key it is refusing.
#[test]
fn b_on_the_working_tree_row_refuses_in_words() {
    let mut state = fixture();
    state.on_working_tree = true;
    let (after, _) = press(&state, key(KeyCode::Char('b')));
    assert!(after.name_prompt.is_none());
    assert!(
        matches!(&after.status, Status::Sticky { reason, .. } if reason == "b needs a commit row"),
        "{:?}",
        after.status
    );
    let (after, _) = press(&state, key(KeyCode::Char('t')));
    assert!(
        matches!(&after.status, Status::Sticky { reason, .. } if reason == "t needs a commit row"),
        "{:?}",
        after.status
    );
}

/// A bracketed paste is text like any other, and lands in the open field
/// rather than anywhere else.
#[test]
fn a_paste_lands_in_the_field() {
    let state = prompt_open(false);
    let (after, _) = update(
        &state,
        AppEvent::Core(Event::Paste("from-clipboard".to_owned())),
    );
    assert_eq!(typed(&after), "from-clipboard");
}

/// `git stash branch` moves `HEAD`, the index and the working tree, so it
/// gets the picture every other such write gets — and the overlay steps out
/// of the way as it opens.
#[test]
fn stash_b_opens_the_prompt_then_a_preview() {
    let mut state = fixture();
    state.stash.entries = crate::update::detail::Loading::Ready(std::sync::Arc::new(vec![
        graft_git::stash::StashEntry {
            index: 0,
            commit: oid(7),
            message: "WIP on main: 0202020 Commit 2".to_owned(),
            at: graft_core::commit::Timestamp {
                seconds: 1,
                offset_seconds: 0,
            },
            paths: vec![std::path::PathBuf::from("src/main.rs")],
        },
    ]));
    state.stash.shown = true;

    let (state, _) = press(&state, key(KeyCode::Char('b')));
    let prompt = state.name_prompt.as_ref().expect("the prompt opened");
    assert_eq!(
        prompt.kind,
        NameKind::BranchFromStash {
            index: 0,
            commit: oid(7)
        },
        "the entry's own `stash@{{n}}`, not the cursor's position"
    );
    assert!(state.stash.shown, "the overlay is still behind the box");

    let mut state = state;
    type_in(&mut state, "rescued");
    let (after, commands) = press(&state, key(KeyCode::Enter));
    assert!(after.name_prompt.is_none(), "the box closed");
    assert!(!after.stash.shown, "the overlay stepped out of the way");
    assert!(after.preview.is_some(), "a picture opened");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::PlanPreview(..))),
        "and it was planned, not written: {commands:?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "nothing ran: {commands:?}"
    );
}

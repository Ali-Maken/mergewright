//! The words kept between screens (ease-pass/08 C15; 08s M1, m7): the two
//! slots, each taken by its own way in and written by its own close, and
//! every landing path running the same hooks. Driven through `update`
//! wherever a key or an answer is how it happens; the helpers here serve
//! `landing_tests` too.

use std::collections::BTreeMap;
use std::sync::Arc;

use mergewright_core::journal::{
    EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
};
use mergewright_ops::apply::{Applied, Refresh};

use super::screen_tests::{editor, key, mock_tree, opened, text, walk};
use super::*;
use crate::jobs::operate::Operated;
use crate::jobs::undo::Undone;
use crate::update::{AppEvent, Product, update};

pub(super) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes")
}

pub(super) fn last(commit: Oid) -> LastCommit {
    LastCommit {
        commit,
        message: Draft {
            subject: "fix toc".to_owned(),
            body: String::new(),
            amending: None,
        },
    }
}

pub(super) fn words(subject: &str, body: &str) -> Draft {
    Draft {
        subject: subject.to_owned(),
        body: body.to_owned(),
        amending: None,
    }
}

/// Refs with `main` at `head`, `HEAD` on it.
pub(super) fn head_at(head: Oid) -> crate::refs::RefBadges {
    let main =
        mergewright_core::ref_name::RefName::new(b"refs/heads/main".to_vec()).expect("a name");
    crate::refs::badges(
        &[(main.clone(), head)],
        &mergewright_git::repo::HeadState::Branch(main),
        &std::collections::HashMap::new(),
        &[],
        |_, _| {
            Ok(mergewright_git::history::AheadBehind {
                ahead: 0,
                behind: 0,
            })
        },
    )
    .expect("counting never fails here")
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

fn applied(entry: JournalEntry) -> Applied {
    Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry,
        refresh: Refresh::WorkingTree,
        step_backup: None,
        replaced: mergewright_ops::undo::Replaced::default(),
    }
}

/// A write nobody on the screen launched (a stage, a fetch) lands.
pub(super) fn other_write_lands(state: &AppState, kind: OperationKind) -> AppState {
    let mut state = state.clone();
    let id = JobId::new(4242);
    state.operating = Some(id);
    let done = Operated::Done(applied(moved(kind, oid(1), oid(1))));
    update(
        &state,
        AppEvent::Produced(id, Product::Operated(Box::new(done))),
    )
    .0
}

/// An undo (or, `redo`, a redo) moving `main` from `before` to `after`
/// lands, complete or (`complete` off) not.
pub(super) fn undo_lands(
    state: &AppState,
    redo: bool,
    before: Oid,
    after: Oid,
    complete: bool,
) -> AppState {
    let mut state = state.clone();
    let id = JobId::new(4343);
    state.operating = Some(id);
    let kind = if redo {
        OperationKind::Redo
    } else {
        OperationKind::Undo
    };
    let applied = applied(moved(kind, before, after));
    let outcome = if complete {
        Undone::Done(applied)
    } else {
        Undone::Incomplete {
            applied,
            error: "the index could not be read".to_owned(),
        }
    };
    update(
        &state,
        AppEvent::Produced(id, Product::Undone(Box::new(outcome))),
    )
    .0
}

/// `a` on `HEAD` at `head`: the read asked, `HEAD`'s message answered.
fn amend_open(state: &AppState, head: Oid, message: &str) -> AppState {
    let mut state = state.clone();
    state.refs = Some(Arc::new(head_at(head)));
    let (asked, commands) = update(&state, key(KeyCode::Char('a')));
    let Some(Command::ReadHeadMessage(id)) = commands.first() else {
        panic!("{commands:?}");
    };
    let answer = Product::HeadMessage(Ok(message.to_owned()));
    update(&asked, AppEvent::Produced(*id, answer)).0
}

/// The subject cleared and `words` typed in its place.
fn retyped(state: &AppState, words: &str) -> AppState {
    let length = editor(state).subject.text().chars().count();
    let clear = vec![key(KeyCode::Backspace); length];
    walk(state, [clear, text(words)].concat()).0
}

pub(super) fn esc(state: &AppState) -> AppState {
    update(state, key(KeyCode::Escape)).0
}

#[test]
fn a_peek_at_the_last_commit_keeps_the_c_words() {
    let (typed, _) = walk(
        &mock_tree(),
        [vec![key(KeyCode::Char('c'))], text("half")].concat(),
    );
    let closed = esc(&amend_open(&esc(&typed), oid(7), "old subject\n"));
    assert!(closed.commit_editor.is_none());
    assert_eq!(editor(&opened(&closed)).subject.text(), "half");
}

#[test]
fn an_empty_c_screen_keeps_the_amend_words() {
    let amend = amend_open(&mock_tree(), oid(7), "old subject\n");
    let kept = esc(&retyped(&amend, "better"));
    let after_c = esc(&opened(&kept));
    assert!(
        after_c.drafts.commit.is_none(),
        "an empty `c` keeps nothing"
    );
    let again = amend_open(&after_c, oid(7), "old subject\n");
    assert_eq!(editor(&again).subject.text(), "better");
    assert!(editor(&again).from_draft);
}

#[test]
fn an_unchanged_amend_keeps_no_words() {
    let closed = esc(&amend_open(&mock_tree(), oid(7), "old subject\n"));
    assert!(closed.commit_editor.is_none());
    assert_eq!(closed.drafts.amend, None, "nothing typed over `HEAD`'s");
}

#[test]
fn amend_words_for_an_older_head_come_back_on_a_and_say_so() {
    let kept = esc(&retyped(
        &amend_open(&mock_tree(), oid(7), "old\n"),
        "better",
    ));
    assert_eq!(
        kept.drafts.amend.as_ref().and_then(|draft| draft.amending),
        Some(oid(7))
    );
    // A commit lands: `HEAD` moves on to `0808080`.
    let (running, _) = walk(
        &kept,
        [
            vec![key(KeyCode::Char('c'))],
            text("next"),
            vec![key(KeyCode::Enter)],
        ]
        .concat(),
    );
    let id = running.operating.expect("the commit is running");
    let done = Operated::Done(applied(moved(OperationKind::Commit, oid(7), oid(8))));
    let (landed, _) = update(
        &running,
        AppEvent::Produced(id, Product::Operated(Box::new(done))),
    );
    assert!(
        landed.drafts.amend.is_some(),
        "a commit landing keeps `a`'s words"
    );
    let again = amend_open(&landed, oid(8), "next\n");
    assert_eq!(editor(&again).subject.text(), "better");
    let header = crate::view::commit_screen_view(&again)
        .expect("open")
        .header;
    assert_eq!(
        header
            .amending
            .map(|words| words.as_str().to_owned())
            .as_deref(),
        Some("0808080 next")
    );
    let Status::Sticky { reason, .. } = &again.status else {
        panic!("{:?}", again.status);
    };
    assert_eq!(reason, "your words from amending 0707070");
}

#[test]
fn another_write_landing_keeps_the_words() {
    let kept = esc(&walk(
        &mock_tree(),
        [vec![key(KeyCode::Char('c'))], text("half")].concat(),
    )
    .0);
    let landed = other_write_lands(&kept, OperationKind::Stage);
    assert_eq!(landed.drafts, kept.drafts);
    assert_eq!(editor(&opened(&landed)).subject.text(), "half");
}

#[test]
fn another_write_landing_keeps_the_open_screen() {
    let (open, _) = walk(
        &mock_tree(),
        [vec![key(KeyCode::Char('c'))], text("half")].concat(),
    );
    let landed = other_write_lands(&open, OperationKind::Fetch);
    assert_eq!(
        editor(&landed).subject.text(),
        "half",
        "still open, words kept"
    );
}

#[test]
fn the_screens_own_commit_closes_it_and_keeps_nothing() {
    let (running, _) = walk(
        &mock_tree(),
        [
            vec![key(KeyCode::Char('c'))],
            text("fix toc"),
            vec![key(KeyCode::Enter)],
        ]
        .concat(),
    );
    let id = running.operating.expect("the commit is running");
    let done = Operated::Done(applied(moved(OperationKind::Commit, oid(1), oid(2))));
    let (landed, _) = update(
        &running,
        AppEvent::Produced(id, Product::Operated(Box::new(done))),
    );
    assert!(landed.commit_editor.is_none());
    assert_eq!(landed.drafts.commit, None);
    assert_eq!(
        landed.last_commit.as_ref().map(|last| last.commit),
        Some(oid(2))
    );
}

#[test]
fn every_landing_path_runs_the_same_hooks() {
    let (running, _) = walk(
        &mock_tree(),
        [
            vec![key(KeyCode::Char('c'))],
            text("fix toc"),
            vec![key(KeyCode::Enter)],
        ]
        .concat(),
    );
    let id = running.operating.expect("the commit is running");
    let entry = moved(OperationKind::Commit, oid(1), oid(2));
    let clean = Operated::Done(applied(entry.clone()));
    let partial = Operated::Incomplete {
        applied: applied(entry.clone()),
        error: "the index could not be read".to_owned(),
    };
    let not_kept = Operated::StagingNotKept {
        applied: applied(entry),
        paths: vec!["src/a.rs".into()],
    };
    for answer in [clean, partial, not_kept] {
        let what = format!("{answer:?}");
        let (landed, _) = update(
            &running,
            AppEvent::Produced(id, Product::Operated(Box::new(answer))),
        );
        assert!(landed.commit_editor.is_none(), "{what}");
        assert_eq!(
            landed.last_commit.as_ref().map(|last| last.commit),
            Some(oid(2)),
            "{what}"
        );
    }
}

#[test]
fn esc_keeps_subject_and_body_and_c_resumes_them_with_options_off() {
    let (closed, _) = walk(
        &mock_tree(),
        [
            vec![key(KeyCode::Char('c'))],
            text("half"),
            vec![
                super::screen_tests::control(KeyCode::Char('s')),
                key(KeyCode::Tab),
            ],
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
    state.drafts.amend = Some(Box::new(Draft {
        amending: Some(oid(7)),
        ..words("amended words", "")
    }));
    let fresh = opened(&state);
    assert_eq!(editor(&fresh).subject.text(), "", "not on `c`");
    assert!(fresh.drafts.amend.is_some(), "kept for `a`");
    let amend = amend_open(&state, oid(7), "old subject\n");
    assert_eq!(editor(&amend).subject.text(), "amended words");
    assert_eq!(
        amend.status,
        Status::Ready,
        "typed for this `HEAD`: nothing to say"
    );
}

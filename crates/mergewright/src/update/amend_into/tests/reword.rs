//! The reword editor (m8-polish-and-ship/14) after review m8-14: its mode
//! never carries into `c` (Major 1), and the text he typed is kept when the
//! plan refuses or the preview closes, so `a` reopens it (Major 2).

use super::*;

/// `HEAD`'s message, as the read answers it.
const OLD: &str = "old subject\n";

fn press(state: &AppState, code: KeyCode) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))))
}

fn typed(state: &AppState, text: &str) -> AppState {
    text.chars()
        .fold(state.clone(), |state, c| press(&state, KeyCode::Char(c)).0)
}

fn head_message_job(commands: &[Command]) -> JobId {
    commands
        .iter()
        .find_map(|command| match command {
            Command::ReadHeadMessage(id) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a HEAD-message read: {commands:?}"))
}

/// `a` on `HEAD` with `feature-b` built on it, answered `RewordHead`, and
/// `HEAD`'s message read: the reword editor, open.
fn reword_editor_from(state: &AppState) -> AppState {
    let (state, commands) = update(state, key('a'));
    let (id, _, _) = plan_job(&commands).expect("the dependents check");
    let (state, commands) = answer(&state, id, Ok(RewritePlanned::RewordHead));
    let read = head_message_job(&commands);
    let (state, _) = update(
        &state,
        AppEvent::Produced(read, Product::HeadMessage(Ok(OLD.to_owned()))),
    );
    let editor = state.commit_editor.as_ref().expect("the editor opened");
    assert!(editor.reword_line, "a reword editor");
    state
}

fn with_a_line() -> AppState {
    on_screen(0, &[(b"refs/heads/feature-b", 3)])
}

/// The reword's `⏎`: the plan job and the message it plans.
fn enter_plans(state: &AppState) -> (AppState, JobId, String) {
    let (state, commands) = press(state, KeyCode::Enter);
    let (id, request, _) = plan_job(&commands).expect("the reword's plan");
    let RewriteRequest::RewordHead { message } = request else {
        panic!("not a reword: {request:?}");
    };
    let message = message.clone();
    (state, id, message)
}

/// Major 1: `Esc` on a reword editor, then `c`: a plain commit of what he
/// staged — never a reword of `HEAD` planned from a stale mode.
#[test]
fn esc_on_a_reword_then_c_commits_the_staged_changes() {
    let state = reword_editor_from(&with_a_line());
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.commit_editor.is_none(), "Esc closed it");

    let (state, _) = update(&state, key('c'));
    let editor = state.commit_editor.as_ref().expect("c opened the editor");
    assert!(!editor.reword_line, "c is never a reword");
    assert!(!editor.amend, "c is a commit");
    let state = typed(&state, "add auth");
    let (_, commands) = press(&state, KeyCode::Enter);
    assert!(plan_job(&commands).is_none(), "no rewrite: {commands:?}");
    assert!(
        matches!(operate_job(&commands), Some((_, Operation::Commit(_)))),
        "{commands:?}"
    );
}

/// Major 1, the other half: the text `Esc` kept on a reword is `a`'s to
/// reopen, on the same `HEAD`.
#[test]
fn esc_on_a_reword_keeps_its_text_for_the_next_a() {
    let state = reword_editor_from(&with_a_line());
    let state = typed(&state, " edited");
    let (state, _) = press(&state, KeyCode::Escape);
    let state = reword_editor_from(&state);
    let editor = state.commit_editor.as_ref().expect("reopened");
    assert_eq!(editor.subject.text(), "old subject edited");
    assert!(editor.from_draft, "the title says it is his draft");
}

/// Major 2: the reword plan refused after `⏎`. The refusal is said, and
/// the next `a` reopens what he typed instead of `HEAD`'s old message.
#[test]
fn a_refused_reword_keeps_the_typed_text_for_the_next_a() {
    let state = reword_editor_from(&with_a_line());
    let state = typed(&state, " on refresh");
    let (state, id, message) = enter_plans(&state);
    assert_eq!(message, "old subject on refresh\n");
    let (state, _) = answer(
        &state,
        id,
        Ok(RewritePlanned::Refused(Refusal::RootCommit {
            commit: oid(3),
        })),
    );
    assert!(!sticky(&state).is_empty(), "the refusal is said");

    let state = reword_editor_from(&state);
    let editor = state.commit_editor.as_ref().expect("reopened");
    assert_eq!(editor.subject.text(), "old subject on refresh");
}

/// Major 2: the reword's preview closed with `Esc`; `a` reopens his text.
#[test]
fn a_closed_reword_preview_keeps_the_typed_text_for_the_next_a() {
    let state = reword_editor_from(&with_a_line());
    let state = typed(&state, " on refresh");
    let (state, id, _) = enter_plans(&state);
    let (state, _) = answer(&state, id, Ok(put_in(Vec::new(), PUT_IT_IN)));
    assert!(state.preview.is_some(), "the reword preview");
    let (state, _) = press(&state, KeyCode::Escape);
    assert!(state.preview.is_none(), "Esc closed it");

    let state = reword_editor_from(&state);
    let editor = state.commit_editor.as_ref().expect("reopened");
    assert_eq!(editor.subject.text(), "old subject on refresh");
}

/// A kept reword text names the `HEAD` it was typed for: once `HEAD` has
/// moved, `a` opens on the new `HEAD`'s own message.
#[test]
fn a_kept_reword_text_is_not_put_on_another_head() {
    let state = reword_editor_from(&with_a_line());
    let state = typed(&state, " edited");
    let (mut state, _) = press(&state, KeyCode::Escape);
    let mut moved = vec![(name(b"refs/heads/feature"), oid(4))];
    moved.push((name(b"refs/heads/feature-b"), oid(4)));
    state.refs = Some(Arc::new(
        crate::refs::badges(
            &moved,
            &HeadState::Branch(name(b"refs/heads/feature")),
            &std::collections::HashMap::new(),
            &[],
            |_, _| {
                Ok(mergewright_git::history::AheadBehind {
                    ahead: 0,
                    behind: 0,
                })
            },
        )
        .expect("badges"),
    ));
    // The new `HEAD` is not on the three rows drawn: `a` from the
    // uncommitted row takes the `HEAD` path all the same.
    state.on_working_tree = true;
    let state = reword_editor_from(&state);
    let editor = state.commit_editor.as_ref().expect("opened");
    assert_eq!(editor.subject.text(), "old subject");
    assert!(!editor.from_draft);
}

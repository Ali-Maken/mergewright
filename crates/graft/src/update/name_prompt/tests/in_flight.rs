//! One write at a time, inside the prompt (m6-conflicts/78, Major 3 and
//! Minor 2): `⏎` over a running write refuses **every kind** into the
//! prompt's own refusal, keeps the typed name, leaves `status` and
//! `operating` alone and plans nothing. The refusal goes with its cause: once
//! the write answers, the row is the prompt's keys again and `⏎` creates.

use graft_core::job::JobId;
use graft_git::state::{InProgress, InProgressDetail};

use super::*;

/// The drawn problem line under the open prompt, or `None` (its keys).
fn row_problem(state: &AppState) -> Option<String> {
    crate::view::status::bottom_row_problem(state).map(|line| line.reason.as_str().to_owned())
}

fn stash_entry() -> graft_git::stash::StashEntry {
    graft_git::stash::StashEntry {
        index: 0,
        commit: oid(7),
        message: "WIP on main: 0202020 Commit 2".to_owned(),
        at: graft_core::commit::Timestamp {
            seconds: 1,
            offset_seconds: 0,
        },
        paths: vec![std::path::PathBuf::from("src/main.rs")],
    }
}

/// A real `f` launched from the graph: the running write is the fetch.
fn fetching() -> (AppState, JobId) {
    let (state, commands) = press(&fixture(), key(KeyCode::Char('f')));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::Operate(id, op, ..) if matches!(op.as_ref(), Operation::Fetch(_)) => Some(*id),
            _ => None,
        })
        .expect("the fetch launched");
    (state, id)
}

/// The prompt of each kind, opened by its own key over the running fetch,
/// with `name` typed.
fn prompts(state: &AppState, name: &str) -> Vec<(&'static str, AppState)> {
    let mut out = Vec::new();
    let (mut branch, _) = press(state, key(KeyCode::Char('b')));
    type_in(&mut branch, name);
    out.push(("branch", branch.clone()));
    let (mut checkout, _) = press(state, key(KeyCode::Char('b')));
    type_in(&mut checkout, name);
    let (checkout, _) = press(&checkout, key(KeyCode::Tab));
    out.push(("branch + checkout", checkout));
    let (mut tag, _) = press(state, key(KeyCode::Char('t')));
    type_in(&mut tag, name);
    out.push(("tag", tag));
    let mut stash = state.clone();
    stash.stash.entries =
        crate::update::detail::Loading::Ready(std::sync::Arc::new(vec![stash_entry()]));
    stash.stash.shown = true;
    let (mut stash, _) = press(&stash, key(KeyCode::Char('b')));
    type_in(&mut stash, name);
    out.push(("stash branch", stash));
    for (kind, prompt) in &out {
        assert!(prompt.name_prompt.is_some(), "{kind}: the prompt opened");
    }
    out
}

/// The failing test first (review Major 3): `⏎` in the plain branch or tag
/// prompt launched a second write over the fetch and took `operating`.
#[test]
fn a_write_in_flight_refuses_inside_the_prompt() {
    let (state, fetch) = fetching();
    for (kind, prompt) in prompts(&state, "topic") {
        let (after, commands) = press(&prompt, key(KeyCode::Enter));
        assert!(
            after.name_prompt.is_some(),
            "{kind}: the prompt stayed open"
        );
        assert_eq!(typed(&after), "topic", "{kind}: the name is kept");
        let row = row_problem(&after).expect("refused on the row");
        assert!(
            row.starts_with("wait for fetch") && row.ends_with(" to finish"),
            "{kind}: {row}"
        );
        assert_eq!(after.status, prompt.status, "{kind}: status untouched");
        assert_eq!(after.operating, Some(fetch), "{kind}: the fetch keeps it");
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..) | Command::PlanPreview(..))),
            "{kind}: nothing planned or run: {commands:?}"
        );
    }
}

/// Minor 2: the refusal is recomputed each frame, so it goes when the fetch
/// ends — and `⏎` again creates, with nothing retyped. **Keys from the
/// fetch's end to created: 1.**
#[test]
fn the_prompts_refusal_goes_with_its_cause() {
    let (state, fetch) = fetching();
    for (kind, prompt) in prompts(&state, "topic") {
        let (refused, _) = press(&prompt, key(KeyCode::Enter));
        assert!(row_problem(&refused).is_some(), "{kind}: refused");
        let (ended, _) = update(&refused, AppEvent::Core(Event::JobCancelled(fetch)));
        assert!(ended.operating.is_none(), "{kind}: the fetch ended");
        assert!(
            ended.name_prompt.is_some(),
            "{kind}: the prompt is still up"
        );
        assert_eq!(row_problem(&ended), None, "{kind}: the row is its keys");

        let mut keys = 0;
        let (created, commands) = press(&ended, key(KeyCode::Enter));
        keys += 1;
        assert!(created.name_prompt.is_none(), "{kind}: the prompt closed");
        assert!(
            commands
                .iter()
                .any(|command| matches!(command, Command::Operate(..) | Command::PlanPreview(..))),
            "{kind}: created (or planned the stash branch): {commands:?}"
        );
        assert_eq!(keys, 1, "{kind}");
    }
}

/// Minor 6c: the stash branch's other refusal, git mid-operation
/// (`preview::preview_refusal` → `undo::live_refusal_words`), inside the
/// prompt: the name kept, nothing planned, the status line untouched.
#[test]
fn git_mid_operation_refuses_inside_the_stash_prompt() {
    let mut state = fixture();
    state.stash.entries =
        crate::update::detail::Loading::Ready(std::sync::Arc::new(vec![stash_entry()]));
    state.stash.shown = true;
    let (mut state, _) = press(&state, key(KeyCode::Char('b')));
    type_in(&mut state, "rescued");
    state.in_progress = Some(InProgressDetail {
        kind: InProgress::Merge,
        step: None,
        head_name: None,
        onto: None,
        theirs: Ok(None),
    });
    let before = state.status.clone();

    let (after, commands) = press(&state, key(KeyCode::Enter));
    assert_eq!(typed(&after), "rescued", "the name is kept");
    assert_eq!(
        row_problem(&after).as_deref(),
        Some(crate::update::undo::live_refusal_words(InProgress::Merge).as_str())
    );
    assert_eq!(after.status, before, "the status line is untouched");
    assert!(after.preview.is_none());
    assert!(commands.is_empty(), "nothing planned or run: {commands:?}");
}

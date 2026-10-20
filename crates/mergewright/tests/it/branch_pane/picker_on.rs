//! `C` on a teammate's row opens the picker on that branch (ease-pass 09b,
//! Q1): `C`, `⏎` and he is on it. On HEAD's own row `C` opens as before.

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use super::{MORE_RELEASE, TOC, cursor_on, drawn, opened, with_sibling};
use crate::app_harness::App;

/// `C`, then pumped until the picker's list has landed.
fn picker_up(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('C'));
    app.pump(state, "the picker's list", |state| {
        state.picker.as_ref().is_some_and(|picker| {
            matches!(picker.list, mergewright::update::detail::Loading::Ready(_))
        })
    })
}

fn selected(state: &AppState) -> String {
    state
        .picker
        .as_ref()
        .and_then(|picker| picker.selected_row())
        .map(|row| String::from_utf8_lossy(row.name.as_bytes()).into_owned())
        .unwrap_or_default()
}

#[test]
fn c_on_a_teammates_row_then_enter_switches_to_it_in_two_keys() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = picker_up(&app, &state);
    assert_eq!(selected(&state), "refs/heads/sara/fix-toc");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the switch to sara/fix-toc", |state| {
        state.picker.is_none()
            && drawn(state, 120)
                .iter()
                .any(|line| line.contains("sara/fix-toc (you)"))
    });
    assert!(state.picker.is_none());
    app.close();
}

#[test]
fn c_on_heads_own_row_opens_on_the_first_row_as_before() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, MORE_RELEASE);
    let state = picker_up(&app, &state);
    let picker = state.picker.as_ref().expect("the picker is up");
    assert_eq!(picker.selected, 0);
    assert_eq!(picker.wanted, None);
    app.close();
}

#[test]
fn capture_the_picker_on_a_teammates_branch() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = picker_up(&app, &state);
    let state = crate::captures::at_rest(&app, state);
    crate::captures::check_shot(
        "ease-pass/picker-on-branch-120.txt",
        &format!(
            "ease-pass 09b, drawn by `view::view` from a real AppState over the job pool \
             (crates/mergewright/tests/it/branch_pane/picker_on.rs): `main ← wallet-release \
             (HEAD) ← sara/fix-toc`, `ali/x` beside it. Not a pty capture; colour unverified \
             (rule 5).\n120x30. `C` pressed on sara/fix-toc's row. What right looks like: the \
             picker is up with an empty query and its cursor on `sara/fix-toc`, not on the \
             first row; `⏎` from here switches to it.\n\n{}\n",
            drawn(&state, 120).join("\n")
        ),
    );
    app.close();
}

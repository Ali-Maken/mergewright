//! The commit editor's in-progress words name keys that reach the banner
//! (m6-conflicts/39): `esc, ⇥ banner, then c`.
//!
//! Two walks. The pin: after `Esc` from the editor, the panes' `c`/`a` stay
//! commit/amend (`docs/design/02-keymap.md` §2.4, `:239`) — the route this
//! ticket must not change. And the words: `Esc`, `⇥` to the banner, `c` opens
//! the summary; `a` there is the banner's abort.

use super::*;

use graft::update::commit_editor::{Begun, live_operation};
use graft::update::confirm::ConfirmThen;
use graft::update::finish::ReturnTo;
use graft_git::state::InProgress;
use graft_ops::apply::Resume;

use super::shell_begun::{launched_into, resolved_shell_merge};

/// A merge begun **in the app** that stops on `shared.rs` and `notes.md`,
/// both then resolved (written and `git add`ed) and re-read: Graft's entry
/// is live, nothing is left unmerged, and the panes have the keyboard.
fn resolved_graft_merge() -> (Scratch, App, AppState) {
    let scratch = crate::conflict_interim::about_to_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = App::open(&repo);
    let state = crate::conflict_interim::stopped_on_the_conflict(&app, state);
    std::fs::write(repo.join("shared.rs"), "fn main() { both() }\n").expect("writable");
    std::fs::write(repo.join("notes.md"), "the agreed plan\n").expect("writable");
    crate::app_harness::git(&repo, &["add", "shared.rs", "notes.md"]);
    assert!(still_unmerged(&repo).is_empty(), "resolved");
    // The banner is modal while focused; `Esc` gives the panes `Ctrl+R`.
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.banner_focus);
    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the re-read after resolving", |state| {
        graft::update::banner::unmerged_paths(state).is_empty() && state.in_progress.is_some()
    });
    assert_eq!(
        live_operation(&state),
        Some((InProgress::Merge, Begun::InGraft)),
        "Graft's merge, still live"
    );
    (scratch, app, state)
}

/// The shell-begun leg's state, as [`resolved_graft_merge`] hands it.
fn resolved_shell() -> (Scratch, App, AppState) {
    let scratch = resolved_shell_merge();
    let (app, state) = launched_into(scratch.path());
    assert_eq!(
        live_operation(&state),
        Some((InProgress::Merge, Begun::InShell))
    );
    (scratch, app, state)
}

/// The commit editor opened from the changes pane, as ticket 20's repro:
/// `Tab` off the banner, the uncommitted-changes row, `c`.
pub(super) fn editor_open(app: &App, state: AppState) -> AppState {
    let mut state = state;
    let mut guard = 0;
    while state.banner_focus {
        assert!(guard < 6, "Tab never let the banner go");
        state = app.key(&state, KeyCode::Tab);
        guard += 1;
    }
    // The graph's cursor may sit on `feature` (where `m` was pressed):
    // `k` up to the uncommitted-changes row, which `changes_pane` opens.
    while !state.on_working_tree
        && state
            .history
            .as_ref()
            .is_some_and(|history| history.cursor > 0)
    {
        assert!(guard < 40, "never reached the uncommitted-changes row");
        state = app.key(&state, KeyCode::Char('k'));
        guard += 1;
    }
    let state = changes_pane(app, state);
    let state = app.key(&state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    state
}

/// What a key after `Esc` did: the editor (and whether it is an amend),
/// the summary, and the banner's focus.
#[derive(Debug, PartialEq, Eq)]
struct Landing {
    editor: Option<bool>,
    finish: bool,
    banner_focus: bool,
}

fn landing(state: &AppState) -> Landing {
    Landing {
        editor: state.commit_editor.as_ref().map(|editor| editor.amend),
        finish: state.finish.is_some(),
        banner_focus: state.banner_focus,
    }
}

/// `Esc` from the open editor, then `key`.
fn esc_then(app: &App, state: &AppState, key: char) -> Landing {
    let closed = app.key(state, KeyCode::Escape);
    assert!(closed.commit_editor.is_none(), "`Esc` closes the editor");
    assert!(
        !closed.banner_focus,
        "`Esc` lands in the pane, not the banner"
    );
    let after = app.key(&closed, KeyCode::Char(key));
    // The amend editor opens on `HEAD`'s message, read first
    // (`commit_editor::open` → `Command::ReadHeadMessage`).
    let after = if key == 'a' {
        app.pump(after, "HEAD's message for the amend editor", |state| {
            state.commit_editor.is_some()
        })
    } else {
        after
    };
    landing(&after)
}

/// Both walks — `Esc c`, `Esc a` — from the editor over `state`.
fn pin_the_panes_keys(app: &App, state: AppState) {
    let open = editor_open(app, state);
    assert_eq!(
        esc_then(app, &open, 'c'),
        Landing {
            editor: Some(false),
            finish: false,
            banner_focus: false
        },
        "`c` in the pane is Commit (§2.4)"
    );
    assert_eq!(
        esc_then(app, &open, 'a'),
        Landing {
            editor: Some(true),
            finish: false,
            banner_focus: false
        },
        "`a` in the pane is Amend (§2.4)"
    );
}

/// **Step 0, kept as a pin of keymap §2.4 (`02-keymap.md:239`):** after
/// `Esc` from the commit editor, the changes pane's `c` reopens the commit
/// editor and `a` opens the amend editor — on a merge Graft began and on
/// one begun in the shell. Neither reaches the banner; that is why the
/// words name `⇥ banner`.
#[test]
fn after_esc_the_panes_c_and_a_are_commit_and_amend() {
    let (graft_begun, app, state) = resolved_graft_merge();
    pin_the_panes_keys(&app, state);
    app.close();
    drop(graft_begun);

    let (shell_begun, app, state) = resolved_shell();
    pin_the_panes_keys(&app, state);
    app.close();
    drop(shell_begun);
}

/// `Esc` from the open editor, then `Tab` alone until the banner has the
/// keyboard. Returns the state and the number of `Tab`s.
pub(super) fn esc_then_tabs_to_the_banner(app: &App, state: &AppState) -> (AppState, usize) {
    let mut state = app.key(state, KeyCode::Escape);
    assert!(state.commit_editor.is_none(), "`Esc` closes the editor");
    let mut tabs = 0;
    while !state.banner_focus {
        assert!(tabs < 2, "`⇥` does not reach the banner within 2 presses");
        state = app.key(&state, KeyCode::Tab);
        tabs += 1;
    }
    (state, tabs)
}

/// The words the editor over `state` shows.
pub(super) fn editor_words(state: &AppState) -> String {
    state
        .commit_editor
        .as_ref()
        .expect("open")
        .refusal(state.working_tree.as_deref(), live_operation(state))
        .expect("a refusal")
}

/// **Step 2:** on a merge Graft began, the words name `⇥ banner`, and the
/// keys they name do what they say — `c` opens the summary, `a` the
/// banner's abort (its confirm box, `Resume::Abort`), not the amend editor.
#[test]
fn the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge() {
    let (scratch, app, state) = resolved_graft_merge();
    let open = editor_open(&app, state);
    assert_eq!(
        editor_words(&open),
        "merge in progress — esc, ⇥ banner, then c continue  a abort"
    );

    let (banner, tabs) = esc_then_tabs_to_the_banner(&app, &open);
    assert_eq!(tabs, 2, "Files → Diff → banner");
    let c = app.key(&banner, KeyCode::Char('c'));
    assert_eq!(
        c.finish.as_ref().map(|finish| finish.return_to),
        Some(ReturnTo::Banner),
        "`c` opens the summary: {:?}",
        c.status
    );

    let (banner, _) = esc_then_tabs_to_the_banner(&app, &open);
    let a = app.key(&banner, KeyCode::Char('a'));
    assert!(a.commit_editor.is_none(), "no amend editor");
    assert!(
        matches!(
            a.confirm.as_ref().map(|confirm| &confirm.then),
            Some(ConfirmThen::Resume(Resume::Abort))
        ),
        "`a` asks to abort: {:?}",
        a.status
    );
    app.close();
    drop(scratch);
}

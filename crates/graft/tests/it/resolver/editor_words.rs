//! The commit editor's in-progress words name keys that reach the banner
//! (m6-conflicts/39): `esc, ⇥ banner, then c`.
//!
//! Since m6-conflicts/48 the panes' `c`/`a` never open the editor while an
//! operation is live: they answer in the same words, as the `u` refusal's
//! `Sticky { reason, keys }`. The editor's own words stay for an editor that
//! was already open when the operation began, which these tests build by
//! construction ([`editor_open`]).

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

/// The changes pane focused, as ticket 20's repro: `Tab` off the banner,
/// the uncommitted-changes row.
pub(super) fn pane_focused(app: &App, state: AppState) -> AppState {
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
    changes_pane(app, state)
}

/// Runs `press` with the live operation hidden — `in_progress` and
/// `live_entries` cleared — then puts both back: the operation is live under
/// whatever `press` opened. Since m6-conflicts/48 the panes' `c`/`a` refuse
/// mid-operation, so this is how an editor already open when an operation
/// began (one begun in the shell, say) is reached: ticket 40's retained case.
fn with_the_operation_hidden(
    app: &App,
    state: AppState,
    press: impl FnOnce(&App, AppState) -> AppState,
) -> AppState {
    let mut state = state;
    let in_progress = state.in_progress.take();
    let live_entries = std::mem::take(&mut state.live_entries);
    let mut state = press(app, state);
    state.in_progress = in_progress;
    state.live_entries = live_entries;
    state
}

/// The commit editor open from the changes pane over a live operation, by
/// construction ([`with_the_operation_hidden`]); its template read answered.
pub(super) fn editor_open(app: &App, state: AppState) -> AppState {
    let state = pane_focused(app, state);
    let state = with_the_operation_hidden(app, state, |app, state| {
        let state = app.key(&state, KeyCode::Char('c'));
        app.pump(state, "commit.template for the editor", |state| {
            state.commit_template.is_none()
        })
    });
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    assert!(
        live_operation(&state).is_some(),
        "the operation is live again"
    );
    state
}

/// `⇥` once per `Tab` to the banner (m6-conflicts/70): two from the pane's
/// file list (the diff, then the banner), three from the graph with the
/// pane open.
fn tabs(count: usize) -> String {
    "⇥ ".repeat(count)
}

/// The pane's answer to `c`/`a` on a merge Graft began, `count` tabs from
/// the banner. A merge's `c` is `c finish` (m6-conflicts/70, n2).
fn graft_merge_refusal(count: usize) -> Status {
    Status::Sticky {
        reason: "merge in progress".to_owned(),
        keys: Some(format!("{}banner, then c finish  a abort", tabs(count))),
    }
}

/// The pane's answer to `c`/`a` on a merge begun in the shell, `count` tabs
/// from the banner.
pub(super) fn shell_merge_refusal(count: usize) -> Status {
    Status::Sticky {
        reason: "merge begun in the shell".to_owned(),
        keys: Some(format!("{}banner, then c finishes it", tabs(count))),
    }
}

/// `c` in `state` opens the finish summary, as the banner's `c` does
/// (m6-conflicts/52): no editor, and `Esc` from it goes to the banner.
fn c_opens_the_summary(state: &AppState) {
    let after = press_for_commands(state, KeyPress::plain(KeyCode::Char('c'))).0;
    assert!(after.commit_editor.is_none(), "`c` opened the editor");
    assert_eq!(
        after.finish.as_ref().map(|finish| finish.return_to),
        Some(ReturnTo::Banner),
        "`c` opens the summary: {:?}",
        after.status
    );
}

/// `key` in `state` answers in words: no editor, no read, no write, and
/// `status` is `refusal`.
fn answers_in_words(state: &AppState, key: char, refusal: &Status) {
    let (after, commands) = press_for_commands(state, KeyPress::plain(KeyCode::Char(key)));
    assert!(after.commit_editor.is_none(), "`{key}` opened the editor");
    assert!(
        after.head_message.is_none(),
        "`{key}` asked for HEAD's message"
    );
    assert!(
        !commands.iter().any(|command| matches!(
            command,
            Command::ReadCommitTemplate(_) | Command::ReadHeadMessage(_) | Command::Operate { .. }
        )),
        "`{key}` sent {commands:?}"
    );
    assert_eq!(&after.status, refusal, "`{key}`");
}

/// With a merge live — begun in Graft or in the shell — the changes pane's
/// `a` opens nothing and answers in the words the editor used to show
/// (m6-conflicts/48). Its `c` opened nothing either until m6-conflicts/52;
/// now it opens the finish summary, as the banner's `c` does. `a` is
/// unchanged: it never aborts.
#[test]
fn a_mid_operation_still_refuses_in_words() {
    let (graft_begun, app, state) = resolved_graft_merge();
    let pane = pane_focused(&app, state);
    answers_in_words(&pane, 'a', &graft_merge_refusal(2));
    c_opens_the_summary(&pane);
    app.close();
    drop(graft_begun);

    let (shell_begun, app, state) = resolved_shell();
    let pane = pane_focused(&app, state);
    answers_in_words(&pane, 'a', &shell_merge_refusal(2));
    c_opens_the_summary(&pane);
    app.close();
    drop(shell_begun);
}

/// **Every production way to open the editor** (`grep -rn
/// "commit_editor::open" crates/graft/src`: `update/keys.rs` `commit_key`
/// only, `c` and `a`, reached from the graph and from the changes pane)
/// opens no editor while an operation is live: `a` answers in words, and
/// on a merge `c` opens the finish summary instead (m6-conflicts/52).
#[test]
fn every_way_to_open_the_editor_refuses_mid_operation() {
    let (scratch, app, state) = resolved_shell();
    let mut graph = state;
    let mut guard = 0;
    while graph.banner_focus {
        assert!(guard < 6, "Tab never let the banner go");
        graph = app.key(&graph, KeyCode::Tab);
        guard += 1;
    }
    let pane = pane_focused(&app, graph.clone());
    // Graph → Files → Diff → banner is three `⇥`, but one `⇧⇥`: the
    // shorter way is named (m8/20d). The pane's files: two either way, so
    // forward.
    let from_the_graph = Status::Sticky {
        reason: "merge begun in the shell".to_owned(),
        keys: Some("⇧⇥ banner, then c finishes it".to_owned()),
    };
    for (where_, state, refusal) in [
        ("graph", &graph, from_the_graph),
        ("changes pane", &pane, shell_merge_refusal(2)),
    ] {
        eprintln!("{where_}");
        answers_in_words(state, 'a', &refusal);
        c_opens_the_summary(state);
    }
    app.close();
    drop(scratch);
}

/// A refused `c` leaves the draft `Esc` kept where it was.
#[test]
fn a_refused_c_keeps_the_draft() {
    let (scratch, app, state) = resolved_shell();
    let open = editor_open(&app, state);
    let typed = app.key(&open, KeyCode::Char('x'));
    let closed = app.key(&typed, KeyCode::Escape);
    let draft = closed.drafts.commit.clone();
    assert!(draft.is_some(), "`Esc` kept the draft");
    let refused = app.key(&closed, KeyCode::Char('c'));
    assert!(refused.commit_editor.is_none());
    assert_eq!(refused.drafts.commit, draft, "the draft is untouched");
    app.close();
    drop(scratch);
}

/// `a`'s read of `HEAD`'s message in flight, then the operation is live when
/// it lands: the amend editor does not open; the pane answers in words.
#[test]
fn head_message_arriving_mid_operation_does_not_open() {
    let (scratch, app, state) = resolved_shell();
    let pane = pane_focused(&app, state);
    let asked = with_the_operation_hidden(&app, pane, |app, state| {
        let state = app.key(&state, KeyCode::Char('a'));
        // The repo has another branch (`feature`), so `a` first asks
        // whether anything is built on his (m7-power-tools/09); nothing
        // is, and only then does it read HEAD's message.
        let state = app.pump(state, "what's built on the branch", |state| {
            !state.amend_into.checking()
        });
        assert!(state.head_message.is_some(), "`a` asked for HEAD's message");
        state
    });
    let landed = app.pump(asked, "HEAD's message", |state| {
        state.head_message.is_none()
    });
    assert!(landed.commit_editor.is_none(), "the amend editor opened");
    assert_eq!(landed.status, shell_merge_refusal(2));
    app.close();
    drop(scratch);
}

/// A cherry-pick Graft began, 80x24, `a` pressed (its `c` continues since
/// m6-conflicts/52): the footer holds the keys whole and no row is cut
/// with `…`. The state is the Graft-begun merge's with the kind
/// swapped to a cherry-pick — the words depend only on `(kind, begun)`, and
/// no fixture stops a Graft cherry-pick on a conflict.
#[test]
fn the_pane_words_fit_at_80() {
    let (scratch, app, state) = resolved_graft_merge();
    let mut pane = pane_focused(&app, state);
    pane.in_progress.as_mut().expect("live").kind = InProgress::CherryPick;
    let refused = app.key(&pane, KeyCode::Char('a'));
    assert_eq!(
        refused.status,
        Status::Sticky {
            reason: "cherry-pick in progress".to_owned(),
            keys: Some("⇥ ⇥ banner, then c continue  a abort".to_owned()),
        }
    );
    let sized = app.resize(&refused, 80, 24);
    let frame = crate::captures::frame(&sized, 80, 24);
    let body = frame.join("\n");
    assert!(
        frame
            .iter()
            .any(|row| row.contains("⇥ ⇥ banner, then c continue  a abort")),
        "the keys are not whole:\n{body}"
    );
    assert!(
        !frame
            .iter()
            .any(|row| row.contains("⇥ banner") && row.contains('…')),
        "a row is cut inside the keys:\n{body}"
    );
    app.close();
    drop(scratch);
}

/// After `Esc` from the editor over `state`, the pane's `a` answers in
/// words (m6-conflicts/48) and its `c` opens the finish summary
/// (m6-conflicts/52); neither reopens the editor.
fn pin_the_panes_keys(app: &App, state: AppState, refusal: &Status) {
    let open = editor_open(app, state);
    let closed = app.key(&open, KeyCode::Escape);
    assert!(closed.commit_editor.is_none(), "`Esc` closes the editor");
    assert!(
        !closed.banner_focus,
        "`Esc` lands in the pane, not the banner"
    );
    answers_in_words(&closed, 'a', refusal);
    c_opens_the_summary(&closed);
}

/// **Ticket 39's Step 0 pin, rewritten to m6-conflicts/48 and 52:** after
/// `Esc` from the commit editor, the changes pane's `a` answers in words and
/// its `c` opens the finish summary — on a merge Graft began and on one
/// begun in the shell. The words name `⇥ banner`.
#[test]
fn after_esc_the_panes_a_answers_in_words_and_c_opens_the_summary() {
    let (graft_begun, app, state) = resolved_graft_merge();
    pin_the_panes_keys(&app, state, &graft_merge_refusal(2));
    app.close();
    drop(graft_begun);

    let (shell_begun, app, state) = resolved_shell();
    pin_the_panes_keys(&app, state, &shell_merge_refusal(2));
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
    assert!(state.commit_editor.is_some(), "open");
    graft::update::commit_editor::live_words(state).expect("a refusal")
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
        "merge in progress — esc, ⇥ ⇥ banner, then c finish  a abort"
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

/// **The 80x24 check (ticket 40):** on a merge Graft began, the whole
/// sentence shows at the narrow breakpoint.
#[test]
fn the_graft_merge_words_fit_at_80_by_24() {
    let (scratch, app, state) = resolved_graft_merge();
    let open = editor_open(&app, state);
    super::shell_begun::the_words_fit(
        &open,
        "merge in progress — esc, ⇥ ⇥ banner, then c finish  a abort",
        80,
        24,
    );
    app.close();
    drop(scratch);
}

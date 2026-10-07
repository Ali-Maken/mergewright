//! The modals that can be on screen while a write that cannot stop runs
//! (m8/13b review fix, M2): `Ctrl+C` is the one key each passes through, so
//! it boxes there as on the graph, `y` sets the wait, and `Esc` drops it —
//! the write still running, no `CancelJob`. After the offer, `Ctrl+C` opens
//! the stop box. The finish summary's own case (the `--continue` commit it
//! launches) is `finish::tests::esc_on_the_summary_drops_the_quit_wait`;
//! here the summary is up over a merge, as are the resolver and the rewrite
//! editor.

use graft_core::event::{KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::operation::Operation;

use super::{not_cancelled, plain, press};
use crate::update::quit::waited_long;
use crate::update::state::AppState;

/// `state` with a merge launched under it, as `merging` does from the graph.
fn merge_under(mut state: AppState) -> (AppState, JobId) {
    let mut commands = Vec::new();
    let mut next = 900;
    let oid = |byte| graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes");
    crate::update::operate::launch_unstoppable(
        &mut state,
        &mut commands,
        Operation::Merge(graft_ops::operation::MergeParams {
            source: graft_ops::operation::Source::Branch("feature".to_owned()),
            into: graft_ops::operation::Tip {
                name: "refs/heads/main".to_owned(),
                oid: oid(2),
            },
            source_tip: oid(3),
            ff: graft_ops::operation::FastForward::Default,
        }),
        crate::update::preview::HISTORY_CANNOT_STOP,
        Some("m"),
        &mut || {
            next += 1;
            JobId::new(next)
        },
    )
    .expect("nothing else runs");
    let id = state.operating.expect("the merge runs");
    (state, id)
}

fn resolver() -> AppState {
    let mut state = crate::update::tests::resolver_on_screen(JobId::new(4_100));
    state.size = crate::update::Size {
        columns: 120,
        rows: 48,
    };
    state
}

fn rewrite_editor() -> AppState {
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        ..AppState::default()
    };
    crate::update::rewrite_editor::tests::open_an_editor(&mut state);
    state
}

/// The finish summary up, its commit not yet asked for.
fn finish_summary() -> AppState {
    crate::update::finish::tests::opened()
}

fn ctrl_c() -> KeyPress {
    KeyPress::control(KeyCode::Char('c'))
}

/// `Ctrl+C` → box, `y` → wait, `Esc` → no wait; the merge runs on and the
/// modal is still up (`up`).
fn esc_drops_the_wait(modal: &str, state: AppState, up: impl Fn(&AppState) -> bool) {
    let (state, id) = merge_under(state);
    assert!(up(&state), "{modal}: the modal is up over the merge");
    let (boxed, commands) = press(&state, ctrl_c());
    not_cancelled(&commands);
    assert!(boxed.confirm.is_some(), "{modal}: Ctrl+C boxes");
    let (waiting, commands) = press(&boxed, plain('y'));
    not_cancelled(&commands);
    assert!(waiting.quit_when_done.is_some(), "{modal}: the wait is set");
    let (next, commands) = press(&waiting, KeyPress::plain(KeyCode::Escape));
    not_cancelled(&commands);
    assert_eq!(next.quit_when_done, None, "{modal}: Esc drops the wait");
    assert_eq!(next.operating, Some(id), "{modal}: the merge runs on");
    assert_eq!(next.quit, None, "{modal}");
    assert!(up(&next), "{modal}: Esc spent on the wait, the modal stays");
}

/// After the offer, `Ctrl+C` over the modal opens the stop box.
fn ctrl_c_opens_the_stop_box(modal: &str, state: AppState) {
    let (state, _) = merge_under(state);
    let (boxed, _) = press(&state, ctrl_c());
    let (mut waiting, _) = press(&boxed, plain('y'));
    waited_long(&mut waiting);
    let (next, commands) = press(&waiting, ctrl_c());
    not_cancelled(&commands);
    assert_eq!(next.quit, None, "{modal}");
    let open = next.confirm.as_ref().expect("the stop box");
    assert_eq!(open.view.title.as_str(), "Quit now?", "{modal}");
}

#[test]
fn esc_drops_the_wait_over_the_finish_summary() {
    esc_drops_the_wait("finish summary", finish_summary(), |state| {
        state.finish.is_some()
    });
}

#[test]
fn esc_drops_the_wait_over_the_resolver() {
    esc_drops_the_wait("resolver", resolver(), |state| state.resolver.is_some());
}

#[test]
fn esc_drops_the_wait_over_the_rewrite_editor() {
    esc_drops_the_wait("rewrite editor", rewrite_editor(), |state| {
        state.rewrite.editor.is_some()
    });
}

#[test]
fn after_the_offer_ctrl_c_opens_the_stop_box_over_each_modal() {
    ctrl_c_opens_the_stop_box("finish summary", finish_summary());
    ctrl_c_opens_the_stop_box("resolver", resolver());
    ctrl_c_opens_the_stop_box("rewrite editor", rewrite_editor());
}

//! m6-conflicts/68: an abort begun with the resolver on screen reads
//! `aborted: …`.
//!
//! The resolver has no `a` of its own (keymap §2.6's table): it swallows the
//! key, as a modal pane does. The way to abort from it is the banner's:
//! `Esc` closes the resolver, `⇥` reaches the banner, `a y` aborts. That
//! route ends in `banner::resumed` like every other resume.

use graft_core::event::KeyCode;
use graft_core::journal::{EntryStatus, OperationKind};

use super::file_list::listing;
use super::press;
use super::words_and_c::applied;
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

fn resolver_over_a_rebase() -> AppState {
    let mut state = listing(&["gone.txt", "conflicted.rs"], &["conflicted.rs"]);
    state.in_progress = Some(crate::update::resolver::fixtures::operation(
        graft_git::state::InProgress::Rebase,
        None,
    ));
    state
}

fn sent_abort(commands: &[Command]) -> Option<graft_core::job::JobId> {
    commands.iter().find_map(|command| match command {
        Command::Resume(id, graft_ops::apply::Resume::Abort) => Some(*id),
        _ => None,
    })
}

#[test]
fn the_resolver_abort_reads_aborted() {
    let state = resolver_over_a_rebase();
    // The resolver claims `a` and sends no resume.
    let (after, commands) = press(&state, KeyCode::Char('a'));
    assert!(after.resolver.is_some(), "the resolver stays open");
    assert_eq!(sent_abort(&commands), None, "{commands:?}");
    assert!(after.confirm.is_none());

    let mut state = state;
    let mut sent = None;
    for code in [
        KeyCode::Escape,
        KeyCode::Tab,
        KeyCode::Char('a'),
        KeyCode::Char('y'),
    ] {
        let (next, commands) = press(&state, code);
        state = next;
        sent = sent.or(sent_abort(&commands));
    }
    let id = sent.unwrap_or_else(|| panic!("no abort was sent · {:?}", state.status));
    let mut aborted = applied("rebase main onto fix");
    aborted.entry.kind = OperationKind::Rebase;
    aborted.entry.status = EntryStatus::Aborted {
        reason: "aborted".to_owned(),
    };
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Done(aborted))),
        ),
    );
    assert_eq!(state.status, Status::done("aborted: rebase main onto fix"));
}

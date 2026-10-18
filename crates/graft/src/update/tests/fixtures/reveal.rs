//! Going to a commit the default fold hides, from any route (ease-pass 06c):
//! the walk's two answers a reveal takes — folded behind a merge, then the
//! commit's own row once the app has opened it — shared by every route's
//! test.

use std::sync::Arc;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_graph::Row;

use crate::history::{Found, HistoryUpdate, Sought, Wanted, Window};
use crate::update::events::{AppEvent, Command, Progress};
use crate::update::state::AppState;
use crate::update::update;

/// Commit `[byte; 20]`.
pub(crate) fn oid_of(byte: u8) -> Oid {
    Oid::from_sha1([byte; Oid::SHA1_BYTES])
}

fn row(byte: u8) -> Row {
    let who = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    Row {
        drawn_parents: vec![oid_of(byte.wrapping_add(1))],
        commit: Commit {
            id: oid_of(byte),
            parents: vec![oid_of(byte.wrapping_add(1))],
            author: who.clone(),
            committer: who,
            message: b"a commit".to_vec(),
        },
        lane: 0,
        incoming: Vec::new(),
        outgoing: Vec::new(),
        lanes: Vec::new(),
        marker: None,
    }
}

/// Walk `walk`'s answer to request `request`: commits `bytes` from row 0,
/// and — answering a find — how it was found.
pub(crate) fn answer(walk: JobId, request: u64, bytes: &[u8], found: Option<Found>) -> AppEvent {
    let window = Window {
        request,
        first: 0,
        rows: bytes.iter().map(|&byte| row(byte)).collect(),
        laid_out: bytes.len(),
        done: true,
        promoted: false,
        fold: Arc::from([]),
        sought: found.map(|found| {
            Box::new(Sought {
                anchor_behind: None,
                found: Some(found),
            })
        }),
    };
    AppEvent::Progress(walk, Progress::History(HistoryUpdate::Window(window)))
}

/// The find `commands` asks for `target`: its request id.
pub(crate) fn find_for(commands: &[Command], target: Oid) -> u64 {
    commands
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ShowRows(_, request)
                if matches!(request.wanted, Wanted::Find { commit, .. } if commit == target) =>
            {
                Some(request.id)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("a find for {target}: {commands:?}"))
}

/// The walk's first answer to find `find`: `target` folded behind `merge`,
/// commits `shown` on hand. The app opens `merge` — one row request, its
/// view `merge` opened, a find for `target` again — and the cursor waits on
/// the merge. Returns that state and the new find's id.
pub(crate) fn folded_answer(
    state: &AppState,
    walk: JobId,
    find: u64,
    (merge, target): (Oid, Oid),
    shown: &[u8],
) -> (AppState, u64) {
    let folded = Some(Found::Folded { behind: merge });
    let (state, commands) = update(state, answer(walk, find, shown, folded));
    let view = super::asked_view(&commands);
    assert!(
        view.inclusion
            .opened()
            .is_some_and(|opened| opened.contains(merge)),
        "the reveal opens the merge: {view:?}"
    );
    let again = find_for(&commands, target);
    let history = state.history.as_ref().expect("a walk");
    assert_eq!(history.seeking, Some(target), "the seek is out again");
    (state, again)
}

/// Both answers of a reveal: [`folded_answer`], then the opened view's
/// answer with `target`'s row among `opened`. Returns the landed state and
/// the second answer's commands.
pub(crate) fn revealed(
    state: &AppState,
    walk: JobId,
    find: u64,
    (merge, target): (Oid, Oid),
    (shown, opened): (&[u8], &[u8]),
) -> (AppState, Vec<Command>) {
    let (state, again) = folded_answer(state, walk, find, (merge, target), shown);
    update(&state, answer(walk, again, opened, Some(Found::Row)))
}

/// The commit the cursor is on.
pub(crate) fn on(state: &AppState) -> Option<Oid> {
    let history = state.history.as_ref()?;
    history.selected().map(|row| row.commit.id)
}

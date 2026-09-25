//! `Ctrl+O`/`Ctrl+T`: one side in every open block, one write, one queue
//! entry (m6-conflicts/16).

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::conflict::{ConflictFile, Take};
use graft_git::status::UnmergedHow;

use super::advance::{cursor, keys, landed, notice, sent, three, three_alone};
use super::{WALK, press, resolver_card_on_screen, sticky, takes};
use crate::jobs::conflict::{OpenedFile, Opening, TakeFailure};
use crate::update::resolver::{DoneOpen, DoneShape, TAKE_QUEUE_CAP};
use crate::update::{AppEvent, AppState, Command, Product, TakeAt, update};

fn ctrl(state: &AppState, key: char) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Core(Event::Key(KeyPress::control(KeyCode::Char(key)))),
    )
}

/// `state` in a merge of `feature` into `main`, so the sides have names.
fn merge(mut state: AppState) -> AppState {
    use crate::update::resolver::fixtures::{commit, oid, operation, refs};
    state.refs = Some(refs(false));
    state.in_progress = Some(operation(
        graft_git::state::InProgress::Merge,
        Some(commit(oid(3), "Add OAuth provider")),
    ));
    state
}

/// `three.txt` alone, in a merge.
fn three_merge() -> AppState {
    merge(three_alone())
}

/// Commands that would write: a take or a git write.
fn writes(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::TakeSide(..) | Command::Operate(..)))
        .count()
}

/// What every block of `three()` reads once each `(block, take)` is taken.
fn per_block(takes: &[(usize, Take)]) -> Vec<u8> {
    let mut file = three();
    for (block, take) in takes {
        file.take(*block, *take).expect("the block exists");
    }
    file.render()
}

fn document(state: &AppState) -> Arc<ConflictFile> {
    Arc::clone(
        state
            .resolver
            .as_ref()
            .and_then(|open| open.file.as_ref())
            .expect("a document"),
    )
}

#[test]
fn ctrl_t_waits_behind_a_queued_take() {
    let (state, first) = keys(&three_alone(), "o");
    let (state, second) = ctrl(&state, 't');
    assert_eq!(takes(&first) + takes(&second), 1, "one TakeSide out");
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(
        open.queued.iter().copied().collect::<Vec<_>>(),
        [(TakeAt::Every, Take::Theirs)]
    );

    let (state, commands) = landed(&state, &sent(&first)[0]);
    let out = sent(&commands);
    assert_eq!(out.len(), 1, "{commands:?}");
    assert_eq!((out[0].1, out[0].2), (TakeAt::Every, Take::Theirs));
    assert_eq!(
        out[0].3,
        document(&state),
        "sent against the landed document"
    );
    let (state, _) = landed(&state, &out[0]);
    assert_eq!(
        document(&state).render(),
        per_block(&[(0, Take::Ours), (1, Take::Theirs), (2, Take::Theirs)])
    );
}

/// Review m6-amend-16 M1: after a queued `o` then `Ctrl+T`, the footer's
/// count is the header's once the bulk take lands, with no key between.
#[test]
fn footer_count_follows_a_queued_bulk_take() {
    let (state, first) = keys(&three_alone(), "o");
    let (state, _) = ctrl(&state, 't');
    let (state, commands) = landed(&state, &sent(&first)[0]);
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(document(&state).open(), 0, "header: 0 open");
    // The stale `2 of 3 open` from `o` is gone: the footer is back to the
    // hints; the notice carries the bulk take's words.
    assert_eq!(state.status, crate::update::Status::Ready);
    let words = notice(&state).unwrap_or("");
    assert!(words.contains("in 2 blocks"), "{words}");
}

#[test]
fn a_bulk_take_is_one_queued_entry() {
    // One take in flight, 63 queued behind it.
    let (mut state, _) = keys(&three_alone(), "o");
    let open = state.resolver.as_mut().expect("open");
    assert!(open.in_flight.is_some());
    for _ in 0..TAKE_QUEUE_CAP - 1 {
        open.queued.push_back((TakeAt::Block(0), Take::Ours));
    }
    let (state, commands) = ctrl(&state, 't');
    assert_eq!(takes(&commands), 0);
    assert_eq!(
        state.resolver.as_ref().expect("open").queued.len(),
        TAKE_QUEUE_CAP,
        "Ctrl+T is one entry"
    );
    for refused in [press(&state, KeyCode::Char('t')), ctrl(&state, 'o')] {
        let (after, commands) = refused;
        assert_eq!(takes(&commands), 0);
        assert_eq!(sticky(&after).0, "still writing the last take");
        assert_eq!(
            after.resolver.as_ref().expect("open").queued.len(),
            TAKE_QUEUE_CAP
        );
    }

    block_and_every_dropped_by_a_changed_take();
}

/// `[Block, Every]` queued behind a take that comes back `Changed`: `2
/// queued takes dropped`, and the count survives the re-read.
fn block_and_every_dropped_by_a_changed_take() {
    // `[Block, Every]` queued behind a take that comes back `Changed`.
    let (state, first) = keys(&three_alone(), "tt");
    let (state, _) = ctrl(&state, 'o');
    assert_eq!(
        state
            .resolver
            .as_ref()
            .expect("open")
            .queued
            .iter()
            .map(|(at, _)| *at)
            .collect::<Vec<_>>(),
        [TakeAt::Block(1), TakeAt::Every]
    );
    let (state, commands) = update(
        &state,
        AppEvent::Produced(
            sent(&first)[0].0,
            Product::Taken(Err(TakeFailure {
                words: "three.txt changed on disk · read again".to_owned(),
                reread: true,
            })),
        ),
    );
    assert!(
        notice(&state).is_some_and(|words| words.ends_with(" · 2 queued takes dropped")),
        "{:?}",
        notice(&state)
    );
    let reread = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .expect("a Changed take re-reads");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            reread,
            Product::ConflictOpened(Ok(Opening::Unmerged(Arc::new(OpenedFile {
                file: Some(three()),
                how: UnmergedHow::BothModified,
                binary: false,
                on_disk: None,
                start_over: None,
            })))),
        ),
    );
    assert!(
        notice(&state).is_some_and(|words| words.contains("2 queued takes dropped")),
        "the count survives the re-read: {:?}",
        notice(&state)
    );
}

#[test]
fn ctrl_t_with_nothing_open_writes_nothing() {
    // Every block taken, the last two still pending.
    let (state, _) = keys(&three_alone(), "ttt");
    let queued = state.resolver.as_ref().expect("open").queued.clone();
    let (state, commands) = ctrl(&state, 't');
    assert_eq!(writes(&commands), 0, "{commands:?}");
    assert_eq!(notice(&state), Some("nothing open in three.txt"));
    assert_eq!(state.resolver.as_ref().expect("open").queued, queued);
}

#[test]
fn ctrl_t_notice_counts_the_blocks() {
    let mut state = three_merge();
    state.resolver.as_mut().expect("open").cursor_block = Some(1);
    let top = state.resolver.as_ref().expect("open").top;
    let (state, commands) = ctrl(&state, 't');
    assert_eq!(cursor(&state), Some(1), "the cursor stays at the press");
    assert_eq!(state.resolver.as_ref().expect("open").top, top);
    assert_eq!(notice(&state), None, "no notice at the press");
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(
        notice(&state),
        Some("took theirs · feature in 3 blocks · ⏎ marks three.txt done")
    );
    assert_eq!(cursor(&state), Some(1), "unchanged from before the press");

    // After `t` on block 0 has landed.
    let (state, commands) = keys(&three_merge(), "t");
    let (state, _) = landed(&state, &sent(&commands)[0]);
    let (state, commands) = ctrl(&state, 'o');
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(
        notice(&state),
        Some("took yours · main in 2 blocks · ⏎ marks three.txt done")
    );

    // One block open.
    let (state, commands) = keys(&three_merge(), "tt");
    let (state, second) = landed(&state, &sent(&commands)[0]);
    let (state, _) = landed(&state, &sent(&second)[0]);
    let (state, commands) = ctrl(&state, 't');
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(
        notice(&state),
        Some("took theirs · feature in 1 block · ⏎ marks three.txt done")
    );
}

#[test]
fn ctrl_t_notice_waits_while_takes_are_queued() {
    // `Ctrl+T` then `o` behind it: the bulk landing names only itself.
    let (state, commands) = ctrl(&three_merge(), 't');
    let (state, _) = keys(&state, "o");
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(notice(&state), Some("took theirs · feature in 3 blocks"));
}

#[test]
fn ctrl_t_on_a_card_is_one_choice() {
    for binary in [false, true] {
        let mut state = merge(resolver_card_on_screen(WALK));
        if let Some(card) = state
            .resolver
            .as_mut()
            .and_then(|open| open.whole_file.as_mut())
        {
            card.binary = binary;
        }
        for key in ['o', 't'] {
            let (after, commands) = ctrl(&state, key);
            assert_eq!(writes(&commands), 0, "{commands:?}");
            assert_eq!(
                sticky(&after).0,
                "this card is one choice · o yours  t theirs",
                "binary {binary}"
            );
        }
    }
}

#[test]
fn ctrl_t_on_a_done_file_is_refused() {
    for (start_over, words) in [
        (
            crate::jobs::conflict::StartOver::Can {
                kind: graft_ops::operation::RestartKind::Markers,
            },
            "three.txt is done · d starts it over",
        ),
        (
            crate::jobs::conflict::StartOver::Cannot {
                why: "no".to_owned(),
            },
            "three.txt is done",
        ),
    ] {
        let mut state = three_alone();
        state.resolver.as_mut().expect("open").done = Some(DoneOpen {
            shape: DoneShape::Removed,
            start_over,
        });
        for key in ['o', 't'] {
            let (after, commands) = ctrl(&state, key);
            assert_eq!(writes(&commands), 0, "{commands:?}");
            assert_eq!(sticky(&after).0, words);
        }
    }
}

#[test]
fn ctrl_alt_t_does_nothing() {
    let state = three_alone();
    let key = KeyPress {
        code: KeyCode::Char('t'),
        modifiers: Modifiers {
            alt: true,
            ..Modifiers::CONTROL
        },
    };
    let (after, commands) = update(&state, AppEvent::Core(Event::Key(key)));
    assert_eq!(writes(&commands), 0, "{commands:?}");
    assert_eq!(after.resolver, state.resolver);
    assert_eq!(after.status, state.status, "no words");
}

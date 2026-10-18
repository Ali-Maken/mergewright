//! A take moves the cursor on at the keypress, and takes typed faster than
//! the disk queue (m6-conflicts/15); what waits for that queue to drain —
//! `e`, `M`, `Esc` (m6-conflicts/28). `n`/`N` over the same fixtures are in
//! `cross.rs`.

use std::path::Path;
use std::sync::Arc;

use mergewright_core::event::KeyCode;
use mergewright_core::job::JobId;
use mergewright_git::conflict::{BlockState, ConflictFile, Take};
use mergewright_git::status::UnmergedHow;

use super::{WALK, base_loading, press, resolver_on_screen, sticky, takes};
use crate::jobs::conflict::{OpenedFile, Opening, TakeFailure};
use crate::update::resolver::{DoneOpen, DoneShape, TAKE_QUEUE_CAP};
use crate::update::{AppEvent, AppState, Command, Product, Status, TakeAt, update};

/// Three open blocks, one line a side each.
const THREE: &[u8] = b"a\n\
    <<<<<<< HEAD\nours 1\n=======\ntheirs 1\n>>>>>>> topic\n\
    b\n\
    <<<<<<< HEAD\nours 2\n=======\ntheirs 2\n>>>>>>> topic\n\
    c\n\
    <<<<<<< HEAD\nours 3\n=======\ntheirs 3\n>>>>>>> topic\n\
    d\n";

pub(super) fn three() -> ConflictFile {
    mergewright_git::conflict::parse(THREE).expect("three open blocks")
}

/// The resolver on `path` holding `file`, cursor on block 0, in a merge
/// whose list is `paths` — `unmerged` of them still unmerged, the rest done.
pub(super) fn on(path: &str, file: ConflictFile, paths: &[&str], unmerged: &[&str]) -> AppState {
    let mut state = super::file_list::listing(paths, unmerged);
    let open = state.resolver.as_mut().expect("open");
    open.path = path.into();
    open.file = Some(Arc::new(file));
    open.cursor_block = Some(0);
    state
}

/// `three.txt` alone.
pub(super) fn three_alone() -> AppState {
    on("three.txt", three(), &["three.txt"], &["three.txt"])
}

pub(super) fn cursor(state: &AppState) -> Option<usize> {
    state.resolver.as_ref().and_then(|open| open.cursor_block)
}

pub(super) fn notice(state: &AppState) -> Option<&str> {
    state
        .resolver
        .as_ref()
        .and_then(|open| open.notice.as_ref())
        .map(|(words, _)| words.as_str())
}

pub(super) fn open_path(state: &AppState) -> &Path {
    &state.resolver.as_ref().expect("open").path
}

/// Every `TakeSide` in `commands`, as `(id, block, take)`, plus the
/// document it was chosen against.
/// A take the resolver sent: its id, block, side and the document it was
/// chosen against.
pub(super) type Sent = (JobId, TakeAt, Take, Arc<ConflictFile>);

pub(super) fn sent(commands: &[Command]) -> Vec<Sent> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::TakeSide(id, request) => {
                Some((*id, request.at, request.take, Arc::clone(&request.file)))
            }
            _ => None,
        })
        .collect()
}

/// The worker's answer to one `TakeSide`: the take applied to the document
/// it carried.
pub(super) fn landed(state: &AppState, take: &Sent) -> (AppState, Vec<Command>) {
    let (id, at, side, file) = take;
    let mut file = (**file).clone();
    match at {
        TakeAt::Block(block) => file.take(*block, *side).expect("the block exists"),
        TakeAt::Every => {
            file.take_all(*side);
        }
    }
    update(
        state,
        AppEvent::Produced(*id, Product::Taken(Ok(Arc::new(file)))),
    )
}

/// `keys` pressed in order, every command collected.
pub(super) fn keys(state: &AppState, keys: &str) -> (AppState, Vec<Command>) {
    let mut state = state.clone();
    let mut all = Vec::new();
    for key in keys.chars() {
        let (next, commands) = press(&state, KeyCode::Char(key));
        state = next;
        all.extend(commands);
    }
    (state, all)
}

#[test]
fn a_take_moves_to_the_next_open_block() {
    let (state, _) = press(&three_alone(), KeyCode::Char('t'));
    assert_eq!(cursor(&state), Some(1));
    let (state, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(cursor(&state), Some(2));
    let (state, _) = press(&state, KeyCode::Char('t'));
    assert_eq!(
        cursor(&state),
        Some(2),
        "none open after 2: the cursor stays"
    );
    assert_eq!(
        notice(&state),
        Some("all blocks taken · y reviews the merge")
    );
}

/// `take` lands, and exactly one `TakeSide` goes out for `block`.
fn lands_and_sends(state: &AppState, take: &Sent, block: usize) -> (AppState, Sent) {
    let (state, commands) = landed(state, take);
    let mut out = sent(&commands);
    assert_eq!(out.len(), 1, "{commands:?}");
    let next = out.remove(0);
    assert_eq!((next.1, next.2), (TakeAt::Block(block), Take::Theirs));
    (state, next)
}

#[test]
fn takes_typed_faster_than_the_disk_are_queued_in_order() {
    let (state, commands) = keys(&three_alone(), "ttt");
    let out = sent(&commands);
    assert_eq!(out.len(), 1, "one TakeSide out: {commands:?}");
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.queued.len(), 2);
    assert_eq!(open.in_flight, Some((TakeAt::Block(0), Take::Theirs)));
    assert_eq!(cursor(&state), Some(2), "the cursor is already on 2");

    let (state, next) = lands_and_sends(&state, &out[0], 1);
    assert_eq!(
        next.3.blocks()[0].state,
        BlockState::Took(Take::Theirs),
        "chosen against the document that landed"
    );
    let (state, last) = lands_and_sends(&state, &next, 2);
    let (state, commands) = landed(&state, &last);
    assert_eq!(takes(&commands), 0);
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.in_flight, None);
    assert!(open.queued.is_empty());
    assert_eq!(open.job, None);
    assert_eq!(open.file.as_ref().expect("a document").open(), 0);
    assert_eq!(cursor(&state), Some(2), "the answers left the cursor alone");
    assert_eq!(
        notice(&state),
        Some("all blocks taken · y reviews the merge")
    );
}

#[test]
fn a_failed_take_drops_the_queue_and_says_so() {
    let (state, commands) = keys(&three_alone(), "ttt");
    let (id, ..) = sent(&commands)[0].clone();
    let (state, commands) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Taken(Err(TakeFailure {
                words: "could not write three.txt: disk full".to_owned(),
                reread: false,
            })),
        ),
    );
    assert_eq!(takes(&commands), 0, "{commands:?}");
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.in_flight, None);
    assert!(open.queued.is_empty());
    assert_eq!(open.job, None);
    assert_eq!(
        open.notice,
        Some((
            "could not write three.txt: disk full · 2 queued takes dropped".to_owned(),
            true
        ))
    );
    assert_eq!(cursor(&state), Some(2), "the cursor stays");
}

#[test]
fn a_take_while_another_job_runs_keeps_its_own_words() {
    // A base load in flight.
    let (state, _) = base_loading(&resolver_on_screen(WALK));
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "still loading the base");
    assert!(state.resolver.as_ref().expect("open").queued.is_empty());

    // A stage in flight.
    let mut state = resolver_on_screen(WALK);
    state.operating = Some(JobId::new(4_900));
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "a write is already running");
    assert!(state.resolver.as_ref().expect("open").queued.is_empty());

    // A read in flight.
    let mut state = resolver_on_screen(WALK);
    let open = state.resolver.as_mut().expect("open");
    open.job = Some(JobId::new(4_901));
    open.reading = true;
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "still reading the file");
    assert!(state.resolver.as_ref().expect("open").queued.is_empty());

    // A take in flight: queued, not refused.
    let (state, first) = press(&resolver_on_screen(WALK), KeyCode::Char('o'));
    assert_eq!(takes(&first), 1);
    let before = state.status.clone();
    let (state, second) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&second), 0);
    assert_eq!(state.status, before, "no refusal");
    assert_eq!(
        state.resolver.as_ref().expect("open").queued,
        [(TakeAt::Block(1), Take::Theirs)]
    );
}

#[test]
fn the_65th_queued_take_is_refused() {
    let (mut state, _) = press(&three_alone(), KeyCode::Char('t'));
    for _ in 0..TAKE_QUEUE_CAP {
        let (next, commands) = press(&state, KeyCode::Char('t'));
        assert_eq!(takes(&commands), 0);
        state = next;
    }
    assert_eq!(
        state.resolver.as_ref().expect("open").queued.len(),
        TAKE_QUEUE_CAP
    );
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "still writing the last take");
    assert_eq!(
        state.resolver.as_ref().expect("open").queued.len(),
        TAKE_QUEUE_CAP
    );
}

#[test]
fn a_take_on_a_done_file_is_refused_by_done_refusal() {
    let mut state = three_alone();
    state.resolver.as_mut().expect("open").done = Some(DoneOpen {
        shape: DoneShape::Removed,
        start_over: crate::jobs::conflict::StartOver::Can {
            kind: mergewright_ops::operation::RestartKind::Markers,
        },
    });
    let before = state.resolver.clone();
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    assert_eq!(sticky(&state).0, "three.txt is done · d starts it over");
    let open = state.resolver.as_ref().expect("open");
    let was = before.as_ref().expect("open");
    assert_eq!(open.in_flight, was.in_flight);
    assert_eq!(open.queued, was.queued);
    assert_eq!(open.cursor_block, was.cursor_block);
}

#[test]
fn a_block_the_document_lacks_is_refused_in_words() {
    let mut state = three_alone();
    state.resolver.as_mut().expect("open").cursor_block = Some(7);
    let (state, commands) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&commands), 0);
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(
        open.notice,
        Some(("no block 7 · re-read the file".to_owned(), true))
    );
    assert_eq!(open.in_flight, None, "nothing sent");
    assert_eq!(cursor(&state), Some(7), "the cursor stays");
    assert!(!matches!(state.status, Status::Done { .. }));
}

// ------------------------------------ what waits for the takes (ticket 28)

/// `keys` pressed on `three.txt` with nothing answered, then `code`: refused
/// in words, nothing sent, the takes in flight and queued exactly as they
/// were. Returns the state after, and the takes that were out.
fn waits_for_the_takes(keys_first: &str, code: KeyCode) -> (AppState, Vec<Sent>) {
    let (state, commands) = keys(&three_alone(), keys_first);
    let before = state.resolver.clone().expect("open");
    let (after, pressed) = press(&state, code);
    assert!(pressed.is_empty(), "nothing sent: {pressed:?}");
    assert_eq!(
        sticky(&after).0,
        "a take is still landing · try again in a moment"
    );
    let open = after.resolver.as_ref().expect("the resolver stays open");
    assert_eq!(
        open.in_flight, before.in_flight,
        "the take in flight is kept"
    );
    assert_eq!(open.queued, before.queued, "the queue is kept");
    (after, sent(&commands))
}

/// R6: `e` would hand the terminal over and re-read on return, dropping the
/// queue; it waits instead. Once the takes land, it hands over.
#[test]
fn e_waits_for_the_takes_in_flight() {
    let (state, out) = waits_for_the_takes("tt", KeyCode::Char('e'));
    let (state, next) = lands_and_sends(&state, &out[0], 1);
    let (state, _) = landed(&state, &next);
    let (_, commands) = press(&state, KeyCode::Char('e'));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenExternalEditor { .. })),
        "the takes landed, so `e` hands over: {commands:?}"
    );
}

/// R6: `M` the same.
#[test]
fn m_waits_for_the_takes_in_flight() {
    waits_for_the_takes("tt", KeyCode::Char('M'));
    waits_for_the_takes("t", KeyCode::Char('M'));
}

/// Review m6-amend-15 M1: `Esc` with takes queued dropped them silently
/// (`t t t Esc` sent one, dropped two, said nothing). It waits too — with
/// one take in flight and none queued as well.
#[test]
fn esc_waits_for_the_takes_in_flight() {
    waits_for_the_takes("ttt", KeyCode::Escape);
    waits_for_the_takes("t", KeyCode::Escape);
}

/// Review m6-amend-15 M2: a take answered `Changed` re-reads the file and
/// drops the queue. The count was on a problem notice, which the re-read's
/// landing clears; the count must outlive it.
#[test]
fn a_changed_reread_keeps_the_dropped_count() {
    let (state, commands) = keys(&three_alone(), "ttt");
    let (id, ..) = sent(&commands)[0].clone();
    let (state, commands) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Taken(Err(TakeFailure {
                words: "three.txt changed on disk · read again".to_owned(),
                reread: true,
            })),
        ),
    );
    let reread = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a Changed take re-reads: {commands:?}"));
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            reread,
            Product::ConflictOpened(Ok(Opening::Unmerged(Arc::new(OpenedFile {
                file: Some(three()),
                how: UnmergedHow::BothModified,
                binary: false,
                on_disk: None,
                unreadable: None,
                start_over: None,
            })))),
        ),
    );
    let open = state.resolver.as_ref().expect("open");
    assert!(open.file.is_some(), "the re-read landed");
    assert!(
        notice(&state).is_some_and(|words| words.contains("2 queued takes dropped")),
        "the count survives the landing: {:?}",
        open.notice
    );
}

//! A take moves the cursor on at the keypress, takes typed faster than the
//! disk queue, and `n`/`N` reach every block and cross files in the list's
//! order (m6-conflicts/15).

use std::path::Path;
use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_git::conflict::{BlockState, ConflictFile, Take};
use graft_git::status::UnmergedHow;

use super::{WALK, base_loading, press, resolver_on_screen, sticky, takes};
use crate::jobs::conflict::{OpenedFile, Opening, TakeFailure};
use crate::update::resolver::{DoneShape, TAKE_QUEUE_CAP};
use crate::update::{AppEvent, AppState, Command, Product, Status, update};

/// Three open blocks, one line a side each.
const THREE: &[u8] = b"a\n\
    <<<<<<< HEAD\nours 1\n=======\ntheirs 1\n>>>>>>> topic\n\
    b\n\
    <<<<<<< HEAD\nours 2\n=======\ntheirs 2\n>>>>>>> topic\n\
    c\n\
    <<<<<<< HEAD\nours 3\n=======\ntheirs 3\n>>>>>>> topic\n\
    d\n";

fn three() -> ConflictFile {
    graft_git::conflict::parse(THREE).expect("three open blocks")
}

/// The resolver on `path` holding `file`, cursor on block 0, in a merge
/// whose list is `paths` — `unmerged` of them still unmerged, the rest done.
fn on(path: &str, file: ConflictFile, paths: &[&str], unmerged: &[&str]) -> AppState {
    let mut state = super::file_list::listing(paths, unmerged);
    let open = state.resolver.as_mut().expect("open");
    open.path = path.into();
    open.file = Some(Arc::new(file));
    open.cursor_block = Some(0);
    state
}

/// `three.txt` alone.
fn three_alone() -> AppState {
    on("three.txt", three(), &["three.txt"], &["three.txt"])
}

fn cursor(state: &AppState) -> Option<usize> {
    state.resolver.as_ref().and_then(|open| open.cursor_block)
}

fn notice(state: &AppState) -> Option<&str> {
    state
        .resolver
        .as_ref()
        .and_then(|open| open.notice.as_ref())
        .map(|(words, _)| words.as_str())
}

fn open_path(state: &AppState) -> &Path {
    &state.resolver.as_ref().expect("open").path
}

/// Every `TakeSide` in `commands`, as `(id, block, take)`, plus the
/// document it was chosen against.
fn sent(commands: &[Command]) -> Vec<(JobId, usize, Take, Arc<ConflictFile>)> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::TakeSide(id, request) => {
                Some((*id, request.block, request.take, Arc::clone(&request.file)))
            }
            _ => None,
        })
        .collect()
}

/// The worker's answer to one `TakeSide`: the take applied to the document
/// it carried.
fn landed(
    state: &AppState,
    take: &(JobId, usize, Take, Arc<ConflictFile>),
) -> (AppState, Vec<Command>) {
    let (id, block, side, file) = take;
    let mut file = (**file).clone();
    file.take(*block, *side).expect("the block exists");
    update(
        state,
        AppEvent::Produced(*id, Product::Taken(Ok(Arc::new(file)))),
    )
}

/// `keys` pressed in order, every command collected.
fn keys(state: &AppState, keys: &str) -> (AppState, Vec<Command>) {
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
        Some("all blocks taken · ⏎ marks three.txt done")
    );
}

/// `take` lands, and exactly one `TakeSide` goes out for `block`.
fn lands_and_sends(
    state: &AppState,
    take: &(JobId, usize, Take, Arc<ConflictFile>),
    block: usize,
) -> (AppState, (JobId, usize, Take, Arc<ConflictFile>)) {
    let (state, commands) = landed(state, take);
    let mut out = sent(&commands);
    assert_eq!(out.len(), 1, "{commands:?}");
    let next = out.remove(0);
    assert_eq!((next.1, next.2), (block, Take::Theirs));
    (state, next)
}

#[test]
fn takes_typed_faster_than_the_disk_are_queued_in_order() {
    let (state, commands) = keys(&three_alone(), "ttt");
    let out = sent(&commands);
    assert_eq!(out.len(), 1, "one TakeSide out: {commands:?}");
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.queued.len(), 2);
    assert_eq!(open.in_flight, Some((0, Take::Theirs)));
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
        Some("all blocks taken · ⏎ marks three.txt done")
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
        [(1, Take::Theirs)]
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
#[allow(
    non_snake_case,
    reason = "`n` and `N` are two different keys, and the test is named for them"
)]
fn n_and_N_reach_a_taken_block() {
    let (state, commands) = press(&three_alone(), KeyCode::Char('t'));
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(cursor(&state), Some(1));
    let (state, _) = press(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(0), "N lands on the taken block");
    let (state, commands) = press(&state, KeyCode::Char('o'));
    let out = sent(&commands);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].1, out[0].2), (0, Take::Ours), "a re-take");
    assert_eq!(cursor(&state), Some(1), "and on to block 1");
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(2));
}

/// The opening read of the file the walk crossed to lands with `file`.
fn crossed_read_lands(state: &AppState, commands: &[Command], file: ConflictFile) -> AppState {
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no OpenConflict: {commands:?}"));
    update(
        state,
        AppEvent::Produced(
            id,
            Product::ConflictOpened(Ok(Opening::Unmerged(Arc::new(OpenedFile {
                file: Some(file),
                how: UnmergedHow::BothModified,
                binary: false,
                on_disk: None,
            })))),
        ),
    )
    .0
}

#[test]
fn n_at_the_last_block_crosses_in_list_order() {
    let mut state = on("c", three(), &["a", "b", "c"], &["a", "c"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, commands) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("a"), "wrapped, b skipped");
    assert_eq!(notice(&state), Some("▸ a"));
    let state = crossed_read_lands(&state, &commands, three());
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some("▸ a"));

    let (state, commands) = press(&state, KeyCode::Char('N'));
    assert_eq!(open_path(&state), Path::new("c"), "backward, wrapped");
    assert_eq!(notice(&state), Some("▸ c"));
    let state = crossed_read_lands(&state, &commands, three());
    assert_eq!(cursor(&state), Some(2), "N lands on the last block");
}

#[test]
fn n_crosses_before_it_wraps() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("b"));
    assert_eq!(notice(&state), Some("▸ b"));
}

/// `three.txt` alone, blocks 1 and 2 taken, block 0 still open, the
/// cursor on the last block.
fn only_block_0_open() -> AppState {
    let mut file = three();
    file.take(1, Take::Ours).expect("block 1");
    file.take(2, Take::Ours).expect("block 2");
    let mut state = on("three.txt", file, &["three.txt"], &["three.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    state
}

#[test]
fn n_wraps_when_no_other_file_is_open() {
    let (state, commands) = press(&only_block_0_open(), KeyCode::Char('n'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{commands:?}"
    );
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some("wrapped · 1 still open"));
}

#[test]
fn n_past_the_end_with_nothing_open_says_no_more() {
    let mut file = three();
    for block in 0..3 {
        file.take(block, Take::Ours).expect("the block");
    }
    let mut state = on("three.txt", file, &["three.txt"], &["three.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(2), "unmoved");
    assert_eq!(
        sticky(&state),
        ("no more conflicts", Some("⏎ marks three.txt done"))
    );
    let mut back = state.clone();
    back.resolver.as_mut().expect("open").cursor_block = Some(0);
    let (back, _) = press(&back, KeyCode::Char('N'));
    assert_eq!(cursor(&back), Some(0), "unmoved");
    assert_eq!(
        sticky(&back),
        ("no more conflicts", Some("⏎ marks three.txt done"))
    );
}

#[test]
fn a_cross_waits_for_the_take_in_flight() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    // Blocks 0 and 1 taken already, so `t` on 2 takes the last open one.
    {
        let open = state.resolver.as_mut().expect("open");
        let mut file = (**open.file.as_ref().expect("a document")).clone();
        file.take(0, Take::Ours).expect("block 0");
        file.take(1, Take::Ours).expect("block 1");
        open.file = Some(Arc::new(file));
    }
    let (state, first) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&first), 1);
    let (state, second) = press(&state, KeyCode::Char('n'));
    assert!(
        !second
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{second:?}"
    );
    assert_eq!(open_path(&state), Path::new("a"), "nothing selected");
    assert_eq!(sticky(&state).0, "still writing the last take");
}

#[test]
fn a_take_on_a_done_file_is_refused_by_done_refusal() {
    let mut state = three_alone();
    state.resolver.as_mut().expect("open").done = Some(DoneShape::Removed);
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
    assert!(!matches!(state.status, Status::Done(_)));
}

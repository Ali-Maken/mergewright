use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

use super::{FenceAnswers, InputMux, Inputs, ReplyWaiter, SEQUENCE_TIMEOUT};

/// Generous, because a loaded laptop must not fail these: every wait below
/// ends as soon as its answer arrives.
const PATIENCE: Duration = Duration::from_secs(5);

fn started() -> (UnixStream, InputMux, Inputs) {
    let (terminal, ours) = UnixStream::pair().unwrap();
    let (mux, inputs) = InputMux::start(OwnedFd::from(ours)).unwrap();
    (terminal, mux, inputs)
}

fn next_key(inputs: &Inputs) -> KeyCode {
    match inputs.recv_timeout(PATIENCE).unwrap().unwrap() {
        Event::Key(KeyPress { code, .. }) => code,
        other => panic!("expected a key, got {other:?}"),
    }
}

fn read_reply(waiter: &mut ReplyWaiter, expected: &[u8]) -> Vec<u8> {
    let deadline = Instant::now() + PATIENCE;
    let mut reply = Vec::new();
    let mut chunk = [0u8; 64];
    while reply.len() < expected.len() {
        let count = waiter.read(&mut chunk, deadline).unwrap();
        assert_ne!(count, 0, "the reply never arrived");
        reply.extend_from_slice(&chunk[..count]);
    }
    reply
}

#[test]
fn keys_typed_around_a_reply_arrive_in_order_and_the_reply_goes_to_its_waiter() {
    let (mut terminal, mux, inputs) = started();
    let mut waiter = mux.waiter();

    terminal.write_all(b"jk\x1b[?62;4cGq").unwrap();

    assert_eq!(read_reply(&mut waiter, b"\x1b[?62;4c"), b"\x1b[?62;4c");
    let codes: Vec<_> = (0..4).map(|_| next_key(&inputs)).collect();
    assert_eq!(
        codes,
        [
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char('G'),
            KeyCode::Char('q')
        ]
    );
}

#[test]
fn a_reply_split_across_two_reads_still_reaches_its_waiter() {
    let (mut terminal, mux, inputs) = started();
    let mut waiter = mux.waiter();

    terminal.write_all(b"\x1b[6;1").unwrap();
    // Well inside the sequence timeout, but far longer than one read takes,
    // so the two halves really are two reads.
    thread::sleep(SEQUENCE_TIMEOUT / 5);
    terminal.write_all(b"6;8t").unwrap();

    assert_eq!(read_reply(&mut waiter, b"\x1b[6;16;8t"), b"\x1b[6;16;8t");
    assert!(
        inputs.try_recv().is_err(),
        "no fragment of the reply became a key"
    );
}

#[test]
fn a_lone_escape_is_a_key_once_the_sequence_timeout_passes() {
    let (mut terminal, _mux, inputs) = started();
    let sent = Instant::now();

    terminal.write_all(b"\x1b").unwrap();

    let event = inputs.recv_timeout(PATIENCE).unwrap().unwrap();
    assert_eq!(event, Event::Key(KeyPress::plain(KeyCode::Escape)));
    assert!(
        sent.elapsed() >= SEQUENCE_TIMEOUT,
        "delivered before it could be told apart from a sequence"
    );
}

#[test]
fn a_reply_with_no_waiter_is_dropped_not_typed() {
    let (mut terminal, _mux, inputs) = started();

    terminal.write_all(b"\x1b[?1uq").unwrap();

    assert_eq!(next_key(&inputs), KeyCode::Char('q'));
}

#[test]
fn closed_input_is_reported_not_ignored() {
    let (terminal, _mux, inputs) = started();

    drop(terminal);

    let error = inputs.recv_timeout(PATIENCE).unwrap().unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::UnexpectedEof);
}

#[test]
fn a_control_chord_keeps_its_modifier() {
    let (mut terminal, _mux, inputs) = started();

    terminal.write_all(b"\x03").unwrap();

    assert_eq!(
        inputs.recv_timeout(PATIENCE).unwrap().unwrap(),
        Event::Key(KeyPress {
            code: KeyCode::Char('c'),
            modifiers: Modifiers::CONTROL,
        })
    );
}

// One reader per process (ticket m1-the-graph/35c): a wrapper that runs the
// loop more than once — the gallery — asks the same mux for a fresh `Inputs`
// each time, and every key goes to exactly one of them.

#[test]
fn a_fresh_inputs_takes_over_and_the_old_one_gets_nothing_more() {
    let (mut terminal, mux, first) = started();
    terminal.write_all(b"a").unwrap();
    assert_eq!(next_key(&first), KeyCode::Char('a'));

    let second = mux.inputs();
    terminal.write_all(b"b").unwrap();
    assert_eq!(next_key(&second), KeyCode::Char('b'));
    // Its sender was dropped by the swap, so a forwarder blocked on it ends.
    assert!(first.recv_timeout(PATIENCE).is_err());
}

#[test]
fn keys_typed_while_nobody_is_listening_go_to_the_next_inputs() {
    let (mut terminal, mux, first) = started();
    drop(first);
    terminal.write_all(b"]").unwrap();
    // Let the reader see the key with no receiver alive.
    thread::sleep(SEQUENCE_TIMEOUT * 2);
    let next = mux.inputs();
    assert_eq!(next_key(&next), KeyCode::Char(']'));
}

#[test]
fn hold_keeps_keys_for_the_next_inputs_and_ends_the_current_one() {
    let (mut terminal, mux, first) = started();
    mux.hold();
    assert!(first.recv_timeout(PATIENCE).is_err());
    terminal.write_all(b"q").unwrap();
    thread::sleep(SEQUENCE_TIMEOUT * 2);
    let next = mux.inputs();
    assert_eq!(next_key(&next), KeyCode::Char('q'));
}

#[test]
fn keys_given_back_come_before_anything_held_since() {
    let (mut terminal, mux, first) = started();
    mux.hold();
    drop(first);
    terminal.write_all(b"y").unwrap();
    thread::sleep(SEQUENCE_TIMEOUT * 2);
    // A run took `x` off its `Inputs` but stopped before handling it.
    mux.give_back([Event::Key(KeyPress::plain(KeyCode::Char('x')))]);
    let next = mux.inputs();
    assert_eq!(next_key(&next), KeyCode::Char('x'));
    assert_eq!(next_key(&next), KeyCode::Char('y'));
}

/// The point of `pause`, and the whole reason `hold` was not enough: while
/// the mux is paused the bytes a user types are still in the terminal's
/// buffer for whoever has the terminal now.
///
/// `second` is another handle on the same socket — the same relationship
/// `$EDITOR` has with stdin once Graft has stepped off it. If the reader
/// thread were still reading, this read would come back empty (or steal only
/// part), because the two would be racing for the same bytes.
#[test]
fn a_paused_mux_leaves_the_input_for_whoever_has_the_terminal() {
    let (mut terminal, ours) = UnixStream::pair().unwrap();
    let mut second = ours.try_clone().unwrap();
    let (mux, inputs) = InputMux::start(OwnedFd::from(ours)).unwrap();

    // A key first, so the reader is demonstrably running before it is asked
    // to stop.
    terminal.write_all(b"a").unwrap();
    assert_eq!(next_key(&inputs), KeyCode::Char('a'));

    mux.pause().unwrap();
    terminal.write_all(b"typed into the editor").unwrap();

    second
        .set_read_timeout(Some(PATIENCE))
        .expect("a read timeout");
    let mut taken = [0u8; 21];
    second
        .read_exact(&mut taken)
        .expect("the bytes are still there");
    assert_eq!(&taken[..], b"typed into the editor");

    // And nothing reached Graft: those keys were the editor's.
    assert!(inputs.recv_timeout(Duration::from_millis(100)).is_err());

    mux.resume();
    terminal.write_all(b"b").unwrap();
    assert_eq!(next_key(&inputs), KeyCode::Char('b'));
}

/// `pause` waits for the reader to park rather than returning on hope, so a
/// caller that gets `Ok(())` back can hand the terminal over at once.
#[test]
fn pause_returns_only_once_the_reader_has_stopped() {
    let (mut terminal, mux, inputs) = started();
    terminal.write_all(b"a").unwrap();
    assert_eq!(next_key(&inputs), KeyCode::Char('a'));

    let started = Instant::now();
    mux.pause().unwrap();
    assert!(
        started.elapsed() < PATIENCE,
        "pause waited out its park deadline instead of being answered"
    );
    mux.resume();
}

/// A fence listener that forwards each answer count down a channel.
fn listening(mux: &InputMux) -> (FenceAnswers, std::sync::mpsc::Receiver<usize>) {
    let (sender, answers) = std::sync::mpsc::channel();
    let guard = mux.answer_fences(Box::new(move |count| sender.send(count).is_ok()));
    (guard, answers)
}

#[test]
fn fence_answers_go_to_the_fence_listener_not_to_keys_or_waiters() {
    let (mut terminal, mux, inputs) = started();
    let mut waiter = mux.waiter();
    let (_listening, answers) = listening(&mux);

    terminal.write_all(b"j\x1b[0n\x1b[?62;4ck\x1b[0").unwrap();
    // Two reads: the second answer arrives split, as a slow pty delivers it.
    thread::sleep(SEQUENCE_TIMEOUT / 5);
    terminal.write_all(b"n").unwrap();

    assert_eq!(answers.recv_timeout(PATIENCE).unwrap(), 1);
    assert_eq!(
        answers.recv_timeout(PATIENCE).unwrap(),
        1,
        "the split one, whole"
    );
    assert_eq!(read_reply(&mut waiter, b"\x1b[?62;4c"), b"\x1b[?62;4c");
    assert_eq!(next_key(&inputs), KeyCode::Char('j'));
    assert_eq!(next_key(&inputs), KeyCode::Char('k'));
    assert!(inputs.try_recv().is_err(), "no answer became a key");
    let mut rest = [0u8; 16];
    assert_eq!(
        waiter.read(&mut rest, Instant::now()).unwrap(),
        0,
        "the waiter saw only its own reply"
    );
}

#[test]
fn with_no_fence_listener_a_fence_answer_is_an_ordinary_reply() {
    // The startup probe asks `CSI 5 n` before any listener exists, and reads
    // the answer through its waiter like every other reply.
    let (mut terminal, mux, inputs) = started();
    let mut waiter = mux.waiter();

    terminal.write_all(b"\x1b[0n").unwrap();

    assert_eq!(read_reply(&mut waiter, b"\x1b[0n"), b"\x1b[0n");
    assert!(inputs.try_recv().is_err());
}

#[test]
fn a_dropped_fence_listener_hands_answers_back_to_the_waiters() {
    let (mut terminal, mux, _inputs) = started();
    let mut waiter = mux.waiter();
    let (listening, answers) = listening(&mux);
    drop(listening);

    terminal.write_all(b"\x1b[0n").unwrap();

    assert_eq!(read_reply(&mut waiter, b"\x1b[0n"), b"\x1b[0n");
    assert!(answers.try_recv().is_err());
}

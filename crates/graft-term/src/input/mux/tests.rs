use std::io::Write;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::thread;
use std::time::{Duration, Instant};

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

use super::{InputMux, Inputs, ReplyWaiter, SEQUENCE_TIMEOUT};

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

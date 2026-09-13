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

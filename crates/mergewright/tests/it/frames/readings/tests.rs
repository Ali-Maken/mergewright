//! `Paced`'s readings with no fixture and no loop: which wakes nothing asked
//! for (ADR-0065 D1), and what a batched key's latency reads.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use mergewright_core::event::KeyCode;
use mergewright_core::job::JobId;

use super::super::paced::{Handed, Kind, Paced, Watching};
use super::super::sink::Sink;
use super::super::unasked::{Ask, AskedFrom, Role, unasked};
use super::LAID_OUT;
use crate::frame_trace::{KeyFeed, Posted};

const WINDOW: &str = "Progress::History::Window";

fn wake(kind: Kind, label: &'static str, job: Option<(JobId, Role)>) -> Handed {
    let now = Instant::now();
    Handed {
        kind,
        label,
        job,
        sent: now,
        at: now,
        due: None,
        marks: [None; 4],
        asked_before: 0,
        asked_after: 0,
        rows_out: false,
        wrote_before: Duration::ZERO,
        wrote_after: Duration::ZERO,
    }
}

fn key(index: usize) -> Handed {
    wake(Kind::Key(index, Instant::now()), "key", None)
}

fn job(id: u64, role: Role) -> Handed {
    let label = if role == Role::Started {
        "Started"
    } else {
        "Produced::FoldedCounted"
    };
    wake(Kind::Other, label, Some((JobId::new(id), role)))
}

fn ask(id: u64, from: AskedFrom) -> Ask {
    Ask {
        id: JobId::new(id),
        from,
    }
}

/// Key 0, its window (request 1), then `middle`, then the next key: the
/// unasked wakes between the two keys.
fn between_keys(middle: Vec<Handed>, asks: &[Ask]) -> Vec<&'static str> {
    let mut handed = vec![key(0), wake(Kind::Window(1), WINDOW, None)];
    handed.extend(middle);
    handed.push(key(1));
    let end = handed.len();
    unasked(&handed, 1..end, &[1], asks)
}

#[test]
fn unasked_a_count_asked_at_rest_is_asked() {
    let rest = wake(Kind::Rest, "Rested", None);
    let woke = between_keys(
        vec![rest, job(9, Role::Started), job(9, Role::Final)],
        &[ask(9, AskedFrom::Rest)],
    );
    assert_eq!(woke, Vec::<&str>::new());
}

#[test]
fn unasked_a_count_asked_by_the_keys_window_is_asked() {
    let woke = between_keys(
        vec![job(9, Role::Started), job(9, Role::Final)],
        &[ask(9, AskedFrom::InputWindow)],
    );
    assert_eq!(woke, Vec::<&str>::new());
}

#[test]
fn unasked_a_count_asked_by_an_answer_is_counted() {
    let woke = between_keys(
        vec![job(9, Role::Started), job(9, Role::Final)],
        &[ask(9, AskedFrom::Answer)],
    );
    assert_eq!(woke, ["Started", "Produced::FoldedCounted"]);
}

#[test]
fn unasked_repeats_strangers_and_later_windows_are_counted() {
    // A second final answer to one ask.
    let twice = between_keys(
        vec![job(9, Role::Final), job(9, Role::Final)],
        &[ask(9, AskedFrom::Rest)],
    );
    assert_eq!(twice, ["Produced::FoldedCounted"]);
    // An answer to an id never asked.
    let stranger = between_keys(vec![job(4, Role::Final)], &[ask(9, AskedFrom::Rest)]);
    assert_eq!(stranger, ["Produced::FoldedCounted"]);
    // A window after the first answering one request.
    let later = between_keys(vec![wake(Kind::Window(1), WINDOW, None)], &[]);
    assert_eq!(later, [WINDOW]);
}

/// `idle_gaps` counts only pauses that began after `LaidOut`, and in them
/// every wake nothing asked for. Fast: no fixture, no loop.
#[test]
fn idle_gaps_count_what_nothing_asked_for_after_laid_out() {
    let (post, events) = mpsc::channel::<Posted>();
    let feed = KeyFeed::start(Vec::new(), post).expect("an input thread with no keys");
    let watching = Watching::new(&(), vec![1, 2]);
    let keys = vec![(KeyCode::Char('j'), Duration::ZERO); 4];
    let mut wakes = Paced::new(&events, keys, &watching, Sink::timed(false), feed.log());
    wakes.handed = vec![
        key(0),
        wake(Kind::Window(1), WINDOW, None),
        wake(Kind::Other, LAID_OUT, None),
        // Key 1's pause began before `LaidOut`: left out.
        key(1),
        // Key 2's pause: key 1's rows (not counted), the same window again
        // and a job's answer (both counted).
        wake(Kind::Window(2), WINDOW, None),
        wake(Kind::Window(2), WINDOW, None),
        wake(Kind::Other, "Produced::Journal", None),
        key(2),
        // Key 3's pause: nothing.
        key(3),
    ];
    assert_eq!(
        wakes.idle_gaps(1),
        vec![vec![WINDOW, "Produced::Journal"], vec![]]
    );
    feed.finish().expect("the input thread ends");
}

/// A key that joined the next wake's turn (m7-power-tools/43) is shown by
/// that turn's frame: its latency runs to it, not to the look the loop took
/// between the two, when nothing had been drawn. Fast: no fixture, no loop.
#[test]
fn a_batched_key_reads_its_batch_frame() {
    let (post, events) = mpsc::channel::<Posted>();
    let feed = KeyFeed::start(Vec::new(), post).expect("an input thread with no keys");
    let watching = Watching::new(&(), Vec::new());
    let keys = vec![(KeyCode::Char('j'), Duration::ZERO); 3];
    let mut wakes = Paced::new(&events, keys, &watching, Sink::timed(false), feed.log());
    let sent = Instant::now();
    let at = |ms: u64| sent + Duration::from_millis(ms);
    // `drawn`: the `Phase::Drawn` mark, the last of `marks`.
    let key = |key: usize, drawn: Option<Instant>| {
        let mut handed = wake(Kind::Key(key, sent), "key", None);
        handed.sent = sent;
        handed.at = sent;
        handed.marks = [None, None, None, drawn];
        handed
    };
    // Keys 0 and 1 are one turn, drawn after key 1; key 2 is its own.
    wakes.handed = vec![key(0, None), key(1, Some(at(40))), key(2, Some(at(70)))];
    // The first frame; the look between keys 0 and 1 (nothing drawn); the
    // turn's frame; key 2's.
    wakes.drawn = vec![at(0), at(5), at(41), at(71)];
    assert_eq!(
        wakes.latencies().expect("every turn was drawn"),
        vec![
            Duration::from_millis(41),
            Duration::from_millis(41),
            Duration::from_millis(71)
        ]
    );
    feed.finish().expect("the input thread ends");
}

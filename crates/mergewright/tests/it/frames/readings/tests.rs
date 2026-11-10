//! `Paced`'s readings with no loop: which wakes nothing asked for (ADR-0065
//! D1), what a batched key's latency reads, and (on a small fixture) where
//! rows on hand are read.

use std::sync::mpsc;
use std::time::{Duration, Instant};

use mergewright::event_loop::Wake;
use mergewright::history::HistoryUpdate;
use mergewright::jobs::{Dispatch, JobRunner, Shutdown};
use mergewright::update::{AppEvent, AppState, Progress, update};
use mergewright_core::capabilities::RenderingTier;
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::job::JobId;
use mergewright_fixtures::synthetic::{self, Scale};

use super::super::paced::{Handed, Kind, Paced, Watching};
use super::super::run::ready_to_scroll_at;
use super::super::sink::Sink;
use super::super::unasked::{Ask, AskedFrom, Role, unasked};
use super::super::{PATIENCE, Refs};
use super::LAID_OUT;
use crate::frame_trace::{KeyFeed, Post, Posted};

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

/// Opens a small fixture at a 12-row terminal and takes every wake until the
/// walk has said `LaidOut`, as at key 262 of the 100k scroll: history laid
/// out, the worker answering windows only.
fn laid_out_small() -> (
    JobRunner<Post>,
    mpsc::Receiver<Posted>,
    mpsc::Sender<Posted>,
    AppState,
) {
    let repo = synthetic::borrow(Scale::of(2_000).with_commit_graph())
        .unwrap_or_else(|why| panic!("{why}"));
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, mut state, _) = ready_to_scroll_at(
        &repo,
        RenderingTier::Unicode,
        Refs::Fixture,
        Post(post.clone()),
        &events,
        (80, 12),
    )
    .unwrap_or_else(|why| panic!("{why}"));
    while state.history.as_ref().is_none_or(|h| h.laid_out.is_none()) {
        let posted = events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for `LaidOut`: {why}"));
        let Wake::Event(event) = posted.wake else {
            panic!("a wake no job sends");
        };
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
    (jobs, events, post, state)
}

/// `PageDown` from `state` until a key's viewport is off the rows on hand,
/// sending each key's `ShowRows`: the keys pressed, the state after the last,
/// and the window answering its newest request, as the worker sent it.
fn page_down_past_the_rows(
    jobs: &JobRunner<Post>,
    events: &mpsc::Receiver<Posted>,
    mut state: AppState,
) -> (Vec<AppEvent>, AppEvent) {
    let press = AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::PageDown)));
    let mut pressed = Vec::new();
    loop {
        assert!(
            pressed.len() < 40,
            "40 pages and the rows were still on hand"
        );
        let (next, commands) = update(&state, press.clone());
        state = next;
        pressed.push(press.clone());
        for command in commands {
            jobs.dispatch(command);
        }
        let height = state.graph_rows();
        let history = state.history.as_ref().expect("a walk");
        if history.visible(height).len() < height {
            break;
        }
    }
    let newest = state.history.as_ref().expect("a walk").asked;
    loop {
        let posted = events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for window {newest}: {why}"));
        let Wake::Event(event) = posted.wake else {
            continue;
        };
        if let AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) = &event
            && window.request == newest
        {
            return (pressed, event);
        }
    }
}

/// Rows on hand are read where a frame was drawn: at the end of each turn.
/// A key that joined its window's turn (m7-power-tools/43) was never drawn
/// without its rows — the turn's one frame had them — so it is not named;
/// the same key drawn on its own, before the window, is. Pumps a 2,000-commit
/// fixture to `LaidOut`; no timing in it: the turns are written by hand.
#[test]
fn rows_on_hand_are_read_at_the_frame_each_turn_drew() {
    let (jobs, events, _post, initial) = laid_out_small();
    let (pressed, window) = page_down_past_the_rows(&jobs, &events, initial.clone());
    let last = pressed.len() - 1;

    let (feed_post, feed_events) = mpsc::channel::<Posted>();
    let feed = KeyFeed::start(Vec::new(), feed_post).expect("an input thread with no keys");
    let watching = Watching::new(&(), Vec::new());
    let keys = vec![(KeyCode::PageDown, Duration::ZERO); pressed.len()];
    let mut wakes = Paced::new(
        &feed_events,
        keys,
        &watching,
        Sink::timed(false),
        feed.log(),
    );
    let drawn = Some(Instant::now());
    let turns = |last_drawn: bool| {
        let mut handed: Vec<Handed> = (0..pressed.len())
            .map(|index| {
                let mut handed = key(index);
                handed.marks[3] = if index < last || last_drawn {
                    drawn
                } else {
                    None
                };
                handed
            })
            .collect();
        let mut answer = wake(Kind::Window(0), WINDOW, None);
        answer.marks[3] = drawn;
        handed.push(answer);
        handed
    };
    wakes.replay = pressed.iter().cloned().chain([window]).collect();

    // The last key and its window in one turn: one frame, with the rows.
    wakes.handed = turns(false);
    assert_eq!(wakes.waited(initial.clone()), Vec::<String>::new());

    // The last key drawn on its own: a frame with none of its rows.
    wakes.handed = turns(true);
    let waited = wakes.waited(initial);
    assert_eq!(waited.len(), 1, "{waited:?}");
    assert!(
        waited[0].starts_with(&format!("key {last} (PageDown)")),
        "{waited:?}"
    );

    feed.finish().expect("the input thread ends");
    let shutdown = jobs.shutdown(
        Duration::from_secs(10),
        mergewright::jobs::EXIT_WRITE_WAIT,
        &(),
    );
    assert_eq!(shutdown, Shutdown::CLEAN, "the job pool stops cleanly");
}

//! What [`Paced`]'s stamps read as, once the scroll is over: latencies,
//! where each key's time went, the trace and startup lines, and the keys
//! drawn before their rows arrived.

use std::collections::HashSet;
use std::ops::Range;
use std::time::{Duration, Instant};

use mergewright::update::{AppState, update};

use super::paced::{Kind, Paced};

/// [`Wake::label`](mergewright::event_loop::Wake::label) of the walk's word that it
/// has laid history out to the end.
pub(super) const LAID_OUT: &str = "Progress::History::LaidOut";
use super::run::StartupWake;
use super::sink::Counts;

impl<D> Paced<'_, D> {
    /// What the frame after each wake wrote, summed by what woke it: the
    /// script's key, a window of rows, or another job event.
    /// `counted[i]` is `written[i]` parsed.
    pub(super) fn written_by_trigger(&self, counted: &[Counts]) -> Vec<(String, usize, Counts)> {
        let keys = &self.keys;
        let mut groups: Vec<(String, usize, Counts)> = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            // A wake that joined the next one's turn wrote no frame of its
            // own: the turn's is counted once, under the wake that ended it.
            if handed.drawn_at().is_none() {
                continue;
            }
            let (Some(before), Some(after)) = (counted.get(index), counted.get(index + 1)) else {
                continue;
            };
            let name = match handed.kind {
                Kind::Key(key, _) => format!("{:?}", keys[key].0),
                Kind::Window(_) => "Window".to_owned(),
                Kind::Other => "Other".to_owned(),
                Kind::Rest => "Rest".to_owned(),
            };
            let frame = after.minus(*before);
            match groups.iter_mut().find(|(group, ..)| *group == name) {
                Some((_, frames, sum)) => {
                    *frames += 1;
                    *sum = sum.plus(frame);
                }
                None => groups.push((name, 1, frame)),
            }
        }
        groups
    }

    /// Each key's latency: its send → the frame that showed it, the one
    /// drawn at the end of its turn (m7-power-tools/43). A key draws from the rows on hand ([`Paced::waited`] checks
    /// they were there); rows it asked for arrive later as their own frame.
    pub(super) fn latencies(&self) -> Result<Vec<Duration>, String> {
        let mut latencies = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(_, sent) = handed.kind else {
                continue;
            };
            let drawn = self
                .frame_after(index)
                .ok_or_else(|| format!("no frame was drawn after wake {index}"))?;
            latencies.push(drawn.saturating_duration_since(sent));
        }
        Ok(latencies)
    }

    /// Per key, in [`keys`] order, where its latency went: time queued
    /// behind the wake before it (and what that wake was), then the key's
    /// own frame and what it wrote. Printed for late keys.
    /// `counted[i]` is `written[i]` parsed.
    pub(super) fn breakdown(&self, counted: &[Counts]) -> Vec<String> {
        let mut lines = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(_, sent) = handed.kind else {
                continue;
            };
            let end = self.turn_end(index);
            let (Some(drawn), Some(before), Some(after)) = (
                self.frame_after(index),
                end.and_then(|end| counted.get(end)),
                end.and_then(|end| counted.get(end + 1)),
            ) else {
                lines.push("no frame drawn".to_owned());
                continue;
            };
            let previous = index.checked_sub(1).and_then(|at| {
                let wake = self.handed.get(at)?;
                let ended = self.frame_after(at)?;
                Some(format!(
                    "{} frame {:?}",
                    wake.kind_name(),
                    ended.saturating_duration_since(wake.at)
                ))
            });
            let frame = after.minus(*before);
            lines.push(format!(
                "queued {:?} behind {}; own frame {:?}, bytes={} transmits={} placements={}",
                handed.at.saturating_duration_since(sent),
                previous.unwrap_or_else(|| "nothing".to_owned()),
                drawn.saturating_duration_since(handed.at),
                frame.bytes,
                frame.transmits,
                frame.placements,
            ));
        }
        lines
    }

    /// Per key: one `MWR_FRAME_TRACE` line of its phases, in µs.
    /// `latency`: send → the key's frame. `oversleep`: the input thread's
    /// own wake past the due tick, before the send, so outside `latency`
    /// (kept so the OS timer slack stays visible). `queued`: send → handed
    /// to the loop (the channel hand-over, plus any frame ahead). Then the
    /// loop's phases: `drain`, `update`, `carry` (dispatch), `draw` (view,
    /// render, and `write`, the part inside the terminal's `write`), and
    /// `tail`, `Drawn` → the stamp the latency ends at. `asked`: the key
    /// sent a `ShowRows`; `rows_out`: one was unanswered when it was handed.
    /// Keys at or past `scrolled` are the idle keys.
    pub(super) fn trace(&self, scrolled: usize) -> Vec<String> {
        let micros = |from: Option<Instant>, to: Option<Instant>| match (from, to) {
            (Some(from), Some(to)) => to.saturating_duration_since(from).as_micros().to_string(),
            _ => "-".to_owned(),
        };
        let mut lines = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(key, sent) = handed.kind else {
                continue;
            };
            let drawn = self.frame_after(index);
            let [drained, updated, carried, painted] = handed.marks;
            let behind = index.checked_sub(1).and_then(|at| {
                let wake = self.handed.get(at)?;
                let ended = self.frame_after(at)?;
                Some(format!(
                    "{}:{}",
                    wake.kind_name().replace(' ', "_"),
                    ended.saturating_duration_since(wake.at).as_micros()
                ))
            });
            lines.push(format!(
                "key={key} code={:?} kind={} latency_us={} oversleep_us={} queued_us={} drain_us={} update_us={} carry_us={} draw_us={} write_us={} tail_us={} asked={} rows_out={} behind={}",
                self.keys[key].0,
                if key >= scrolled { "idle" } else { "scroll" },
                micros(Some(sent), drawn),
                micros(handed.due, Some(sent)),
                micros(Some(sent), Some(handed.at)),
                micros(Some(handed.at), drained),
                micros(drained, updated),
                micros(updated, carried),
                micros(carried, painted),
                handed
                    .wrote_after
                    .saturating_sub(handed.wrote_before)
                    .as_micros(),
                micros(painted, drawn),
                u8::from(handed.asked_after > handed.asked_before),
                u8::from(handed.rows_out),
                behind.unwrap_or_else(|| "nothing".to_owned()),
            ));
        }
        lines
    }

    /// The startup trace (m7-power-tools/12): every non-key wake from the
    /// first window of rows to key 1's frame — `before_loop` (taken in by
    /// [`ready_to_scroll`], not drawn) then the loop's own — one line each:
    /// `sent`/`recv` in µs after the first window was sent, then what the
    /// loop spent on it. Then one line: `burst_after_first_paint_us`, the
    /// first window's send to the last of these wakes' send (in the app the
    /// first window is painted as it lands, so a key pressed inside this span
    /// queues behind the rest of the burst), and `loop_start_us`, when this
    /// harness's loop drew its first frame (key 0 is sent then).
    pub(super) fn startup(&self, before_loop: &[StartupWake]) -> Vec<String> {
        let key_one = self
            .handed
            .iter()
            .position(|handed| matches!(handed.kind, Kind::Key(1, _)))
            .unwrap_or(self.handed.len());
        let span = |from: Option<Instant>, to: Option<Instant>| match (from, to) {
            (Some(from), Some(to)) => Some(to.saturating_duration_since(from)),
            _ => None,
        };
        let in_loop = self.handed[..key_one]
            .iter()
            .filter(|handed| !matches!(handed.kind, Kind::Key(..)))
            .map(|handed| {
                let [drained, updated, carried, drawn] = handed.marks;
                StartupWake {
                    label: handed.label,
                    sent: handed.sent,
                    recv: handed.at,
                    update: span(drained, updated),
                    draw: span(carried, drawn),
                }
            });
        let mut wakes: Vec<StartupWake> = Vec::new();
        wakes.extend(before_loop.iter().copied());
        wakes.extend(in_loop);
        let Some(first) = wakes
            .iter()
            .position(|wake| wake.label == "Progress::History::Window")
        else {
            return vec!["startup no window of rows was taken".to_owned()];
        };
        let wakes = &wakes[first..];
        let origin = wakes[0].sent;
        let micros = |at: Instant| at.saturating_duration_since(origin).as_micros();
        let took =
            |spent: Option<Duration>| spent.map_or("-".to_owned(), |d| d.as_micros().to_string());
        let mut lines: Vec<String> = wakes
            .iter()
            .enumerate()
            .map(|(index, wake)| {
                format!(
                    "startup i={index} kind={} sent_us={} recv_us={} update_us={} draw_us={}",
                    wake.label,
                    micros(wake.sent),
                    micros(wake.recv),
                    took(wake.update),
                    took(wake.draw),
                )
            })
            .collect();
        let last_sent = wakes.iter().map(|wake| wake.sent).max().unwrap_or(origin);
        lines.push(format!(
            "startup burst_after_first_paint_us={} wakes={} loop_start_us={}",
            micros(last_sent),
            wakes.len(),
            self.drawn
                .first()
                .map_or("-".to_owned(), |at| micros(*at).to_string()),
        ));
        lines
    }

    /// The wakes in `handed[range]` that nothing asked for (m7-power-tools/12,
    /// `idle_gap_wakes`): every job event but a key's rows, the first window
    /// answering each `ShowRows` the loop sent. A window after its first, or
    /// one nobody asked for, counts. Keys never count.
    pub(super) fn unasked_wakes(&self, range: Range<usize>) -> Vec<&'static str> {
        let asked = self.watching.asked();
        let mut answered: HashSet<u64> = self.handed[..range.start]
            .iter()
            .filter_map(|handed| match handed.kind {
                Kind::Window(id) => Some(id),
                _ => None,
            })
            .collect();
        let mut woke = Vec::new();
        for handed in &self.handed[range] {
            match handed.kind {
                // Neither is a job's wake: a key, and the loop's own rest.
                Kind::Key(..) | Kind::Rest => {}
                Kind::Window(id) if asked.contains(&id) && answered.insert(id) => {}
                Kind::Window(_) | Kind::Other => woke.push(handed.label),
            }
        }
        woke
    }

    /// Per idle key whose pause began after the walk said `LaidOut`: the
    /// wakes that came in that pause, from the key before's hand-over to
    /// this one's, that nothing asked for ([`Paced::unasked_wakes`]). With
    /// history laid out and the worker parked, a quiet app has nothing to
    /// say, so each should be empty. Keys at or past `scrolled` are the idle
    /// keys; a pause that began before `LaidOut` is left out, so the count
    /// of pauses is part of the reading.
    pub(super) fn idle_gaps(&self, scrolled: usize) -> Vec<Vec<&'static str>> {
        let Some(laid_out) = self
            .handed
            .iter()
            .position(|handed| handed.label == LAID_OUT)
        else {
            return Vec::new();
        };
        let keys: Vec<(usize, usize)> = self
            .handed
            .iter()
            .enumerate()
            .filter_map(|(index, handed)| match handed.kind {
                Kind::Key(key, _) => Some((key, index)),
                _ => None,
            })
            .collect();
        keys.windows(2)
            .filter(|pair| pair[1].0 >= scrolled && pair[0].1 > laid_out)
            .map(|pair| self.unasked_wakes(pair[0].1 + 1..pair[1].1))
            .collect()
    }

    /// The keys whose own frame was drawn before their rows arrived: replays every
    /// wake through `update` from the state the loop started with, and after
    /// each key checks that the cursor's row and the whole viewport were
    /// already on hand. `update` is pure, so this is the state each key's
    /// frame was drawn from. The text tiers draw exactly those rows; the
    /// pixel tier's strips widen them (`History::drawn`), not checked here.
    pub(super) fn waited(&self, mut state: AppState) -> Vec<String> {
        let keys = &self.keys;
        let mut waited = Vec::new();
        for (handed, event) in self.handed.iter().zip(&self.replay) {
            state = update(&state, event.clone()).0;
            let Kind::Key(key, _) = handed.kind else {
                continue;
            };
            let height = state.graph_rows();
            let Some(history) = state.history.as_ref() else {
                waited.push(format!("key {key}: no history on screen"));
                continue;
            };
            let on_screen = height.min(history.rows.saturating_sub(history.top));
            if history.selected().is_none() || history.visible(height).len() < on_screen {
                waited.push(format!(
                    "key {key} ({:?}): cursor {}, top {}, {} of {on_screen} rows on hand",
                    keys[key].0,
                    history.cursor,
                    history.top,
                    history.visible(height).len()
                ));
            }
        }
        waited
    }
}

/// `idle_gaps` counts only pauses that began after `LaidOut`, and in them
/// every wake but a key's rows: the first window answering a request the
/// loop sent. Fast: no fixture, no loop.
#[test]
fn idle_gaps_count_what_nothing_asked_for_after_laid_out() {
    use std::sync::Mutex;
    use std::sync::mpsc;

    use mergewright_core::event::KeyCode;

    use super::paced::{Handed, Watching};
    use super::sink::Sink;
    use crate::frame_trace::{KeyFeed, Posted};

    const WINDOW: &str = "Progress::History::Window";
    let (post, events) = mpsc::channel::<Posted>();
    let feed = KeyFeed::start(Vec::new(), post).expect("an input thread with no keys");
    let watching = Watching {
        inner: &(),
        asked: Mutex::new(vec![1, 2]),
    };
    let keys = vec![(KeyCode::Char('j'), Duration::ZERO); 4];
    let mut wakes = Paced::new(&events, keys, &watching, Sink::timed(false), feed.log());
    let now = Instant::now();
    let wake = |kind: Kind, label: &'static str| Handed {
        kind,
        label,
        sent: now,
        at: now,
        due: None,
        marks: [None; 4],
        asked_before: 0,
        asked_after: 0,
        rows_out: false,
        wrote_before: Duration::ZERO,
        wrote_after: Duration::ZERO,
    };
    wakes.handed = vec![
        wake(Kind::Key(0, now), "key"),
        wake(Kind::Window(1), WINDOW),
        wake(Kind::Other, LAID_OUT),
        // Key 1's pause began before `LaidOut`: left out.
        wake(Kind::Key(1, now), "key"),
        // Key 2's pause: key 1's rows (not counted), the same window again
        // and a job's answer (both counted).
        wake(Kind::Window(2), WINDOW),
        wake(Kind::Window(2), WINDOW),
        wake(Kind::Other, "Produced::Journal"),
        wake(Kind::Key(2, now), "key"),
        // Key 3's pause: nothing.
        wake(Kind::Key(3, now), "key"),
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
    use std::sync::Mutex;
    use std::sync::mpsc;

    use mergewright_core::event::KeyCode;

    use super::paced::{Handed, Watching};
    use super::sink::Sink;
    use crate::frame_trace::{KeyFeed, Posted};

    let (post, events) = mpsc::channel::<Posted>();
    let feed = KeyFeed::start(Vec::new(), post).expect("an input thread with no keys");
    let watching = Watching {
        inner: &(),
        asked: Mutex::new(Vec::new()),
    };
    let keys = vec![(KeyCode::Char('j'), Duration::ZERO); 3];
    let mut wakes = Paced::new(&events, keys, &watching, Sink::timed(false), feed.log());
    let sent = Instant::now();
    let at = |ms: u64| sent + Duration::from_millis(ms);
    // `drawn`: the `Phase::Drawn` mark, the last of `marks`.
    let key = |key: usize, drawn: Option<Instant>| Handed {
        kind: Kind::Key(key, sent),
        label: "key",
        sent,
        at: sent,
        due: None,
        marks: [None, None, None, drawn],
        asked_before: 0,
        asked_after: 0,
        rows_out: false,
        wrote_before: Duration::ZERO,
        wrote_after: Duration::ZERO,
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

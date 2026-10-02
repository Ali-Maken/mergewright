use std::io::{self, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseInput, MouseKind};

use super::*;
use crate::event_loop::Wake;
use crate::update::{AppEvent, AppState};

/// A trace file kept in memory, readable while the trace owns it.
#[derive(Clone, Default)]
pub(crate) struct Shared(pub(crate) Arc<Mutex<Vec<u8>>>);

impl Shared {
    pub(crate) fn text(&self) -> String {
        let bytes = self.0.lock().expect("no test panics while holding this");
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

impl Write for Shared {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut bytes = self.0.lock().expect("no test panics while holding this");
        bytes.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn key_wake() -> Wake {
    Wake::Event(AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(
        'j',
    )))))
}

#[test]
fn an_unset_or_empty_variable_opens_nothing() {
    assert!(FrameTrace::from_var(None).expect("no file").is_none());
    assert!(
        FrameTrace::from_var(Some(OsString::new()))
            .expect("no file")
            .is_none()
    );
    let opened = Tracer::open(Ok(None), "-".to_owned());
    assert!(opened.meter.is_none() && opened.stamps.is_none());
    let mut tracer = opened.tracer;
    assert!(tracer.clock().is_none(), "off reads no clock");
    assert!(tracer.unreported().is_none());
}

#[test]
fn a_path_that_cannot_be_created_is_an_error_the_tracer_reports_once() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("no-such-dir").join("trace.txt");
    let opened = FrameTrace::from_var(Some(path.into_os_string()));
    assert!(opened.is_err());
    let mut tracer = Tracer::open(opened, "-".to_owned()).tracer;
    let reason = tracer.unreported().expect("the reason waits for the row");
    assert!(!reason.is_empty());
    assert!(tracer.unreported().is_none(), "said once");
    assert!(tracer.clock().is_none(), "and tracing is off");
}

#[test]
fn the_term_word_keeps_only_safe_characters() {
    assert_eq!(term_word(Some("ghostty")), "ghostty");
    assert_eq!(term_word(Some("vscode 1.9/x")), "vscode_1.9_x");
    assert_eq!(term_word(None), "-");
    assert_eq!(term_word(Some("")), "-");
    assert_eq!(term_word(Some(&"a".repeat(80))).len(), 32);
}

#[test]
fn a_frame_line_is_key_value_numbers_with_a_dash_for_not_measured() {
    let file = Shared::default();
    let mut trace = FrameTrace::writing_to(Box::new(file.clone()));
    trace
        .record(&FrameRecord {
            at_us: 7,
            wakes: 3,
            motions: 2,
            queued: 1,
            update_us: 40,
            commands: 4,
            draw_us: 900,
            bytes: 5120,
            write_us: 88,
            fence_us: None,
            input_age_us: Some(1032),
        })
        .expect("memory");
    assert_eq!(
        file.text(),
        "frame at_us=7 wakes=3 motions=2 queued=1 update_us=40 commands=4 draw_us=900 \
         bytes=5120 write_us=88 fence_us=- input_age_us=1032\n"
    );
}

#[test]
fn the_meter_counts_what_reaches_the_inner_writer() {
    let meter = Meter::default();
    let inner = Shared::default();
    let mut out = io::LineWriter::new(Metered::new(inner.clone(), meter.clone()));
    out.write_all(&[b'x'; 3000]).expect("memory");
    out.flush().expect("memory");
    let (bytes, _spent) = meter.take();
    assert_eq!(bytes, 3000);
    assert_eq!(inner.text().len(), 3000);
    assert_eq!(meter.take().0, 0, "taken once");
}

#[test]
fn a_stamped_key_gives_its_frame_an_input_age_and_the_rest_count_as_queued() {
    let file = Shared::default();
    let (stamps, received) = mpsc::channel();
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(file.clone())),
        "-".to_owned(),
        None,
        Some(received),
    );
    // Three keys forwarded, each stamp ahead of its key; the loop takes one.
    for _ in 0..3 {
        stamps.send(Instant::now()).expect("receiver alive");
    }
    tracer.took(&key_wake());
    let clock = tracer.clock();
    tracer.updated(clock, 2);
    tracer.drew(tracer.clock(), &AppState::default());
    let text = file.text();
    let line = text.lines().nth(1).expect("header, then the frame");
    assert!(line.contains(" wakes=1 motions=1 queued=2 "), "{line}");
    assert!(line.contains(" commands=2 "), "{line}");
    assert!(!line.ends_with("input_age_us=-"), "{line}");
}

#[test]
fn a_wheel_scroll_is_a_motion_and_a_click_is_not() {
    let file = Shared::default();
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(file.clone())),
        "-".to_owned(),
        None,
        None,
    );
    let mouse = |kind| {
        Wake::Event(AppEvent::Core(Event::Mouse(MouseInput {
            kind,
            column: 0,
            row: 0,
            modifiers: Modifiers::NONE,
        })))
    };
    tracer.took(&mouse(MouseKind::ScrollDown));
    tracer.took(&mouse(MouseKind::Moved));
    tracer.drew(tracer.clock(), &AppState::default());
    let text = file.text();
    assert!(text.contains(" wakes=2 motions=1 "), "{text}");
    assert!(text.contains("input_age_us=-"), "no stamps sent: {text}");
}

#[test]
fn the_header_says_the_tier_size_counts_and_term_and_repeats_only_on_change() {
    let file = Shared::default();
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(file.clone())),
        "ghostty".to_owned(),
        None,
        None,
    );
    let mut state = AppState {
        settled_tier: Some(RenderingTier::Pixel),
        size: crate::update::Size {
            columns: 187,
            rows: 50,
        },
        history: Some(crate::view::tests::history_of(12, 0)),
        ..AppState::default()
    };
    tracer.drew(tracer.clock(), &state);
    tracer.drew(tracer.clock(), &state);
    state.size.rows = 49;
    tracer.drew(tracer.clock(), &state);
    let text = file.text();
    let headers: Vec<&str> = text.lines().filter(|l| l.starts_with("header ")).collect();
    assert_eq!(
        headers,
        [
            "header tier=pixel columns=187 rows=50 history_rows=12 refs=0 term=ghostty",
            "header tier=pixel columns=187 rows=49 history_rows=12 refs=0 term=ghostty",
        ]
    );
    assert_eq!(text.lines().filter(|l| l.starts_with("frame ")).count(), 3);
}

/// A trace file whose writes all fail.
struct Failing;

impl Write for Failing {
    fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("disk full"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("disk full"))
    }
}

#[test]
fn a_failed_write_stops_the_trace_and_says_why_once() {
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(Failing)),
        "-".to_owned(),
        None,
        None,
    );
    tracer.drew(tracer.clock(), &AppState::default());
    assert_eq!(tracer.unreported().as_deref(), Some("disk full"));
    assert!(tracer.clock().is_none(), "tracing stopped");
    tracer.drew(tracer.clock(), &AppState::default());
    assert!(tracer.unreported().is_none(), "said once");
}

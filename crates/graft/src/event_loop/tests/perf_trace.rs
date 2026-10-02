//! The frame trace (`crate::trace`, m8-polish-and-ship G-trace) through the
//! loop itself: what `pump_with` writes to the trace for a run of keys.

use std::io::LineWriter;

use super::*;
use crate::trace::tests::Shared;
use crate::trace::{FrameTrace, Meter, Metered, Tracer};

const COLUMNS: u16 = 80;
const ROWS: u16 = 24;

/// A terminal writing through the trace's meter into memory, as the shipped
/// binary's does into `Stdout` when tracing.
fn metered_terminal(
    meter: Meter,
) -> (
    Terminal<CrosstermBackend<LineWriter<Metered<Recorder>>>>,
    Recorder,
) {
    let recorder = Recorder::default();
    let terminal = Terminal::with_options(
        CrosstermBackend::new(LineWriter::new(Metered::new(recorder.clone(), meter))),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, COLUMNS, ROWS)),
        },
    )
    .expect("a fixed viewport never asks the backend for its size");
    (terminal, recorder)
}

/// 100 commits whose subjects say `Commit <n>`, in a repository at a path
/// that holds a marker word.
fn on_history() -> AppState {
    AppState {
        repo_path: std::path::PathBuf::from("/code/zebracorn"),
        history: Some(crate::view::tests::history_of(100, 0)),
        size: Size {
            columns: COLUMNS,
            rows: ROWS,
        },
        ..AppState::default()
    }
}

fn presses(count: usize) -> Vec<Wake> {
    let mut wakes: Vec<Wake> = (0..count)
        .map(|_| key(KeyPress::plain(KeyCode::Char('j'))))
        .collect();
    wakes.push(quit_key());
    wakes
}

/// Runs `wakes` over `tracer`; returns what reached the terminal.
fn run(wakes: ScriptedWakes, tracer: &mut Tracer, meter: Meter) -> (Exit, String) {
    let (mut terminal, recorder) = metered_terminal(meter);
    let mut session = session();
    let mut graphics = Graphics::new(NoReplies, &session.capabilities);
    let mut wakes = wakes;
    let exit = pump_with(
        &mut terminal,
        &mut wakes,
        &Recorded::default(),
        &mut session,
        on_history(),
        &mut graphics,
        Outside::default(),
        tracer,
    )
    .expect("writing to memory cannot fail");
    (exit, recorder.written())
}

/// Every `frame` line's fields, by name; panics on a field that is not a
/// number or `-`.
fn frames(trace: &str) -> Vec<std::collections::BTreeMap<String, Option<u64>>> {
    trace
        .lines()
        .filter_map(|line| line.strip_prefix("frame "))
        .map(|fields| {
            fields
                .split(' ')
                .map(|field| {
                    let (name, value) = field.split_once('=').expect("key=value");
                    let value = if value == "-" {
                        None
                    } else {
                        Some(
                            value
                                .parse()
                                .unwrap_or_else(|_| panic!("a number: {field}")),
                        )
                    };
                    (name.to_owned(), value)
                })
                .collect()
        })
        .collect()
}

fn field(frame: &std::collections::BTreeMap<String, Option<u64>>, name: &str) -> Option<u64> {
    *frame.get(name).unwrap_or_else(|| panic!("no {name} field"))
}

const FIELDS: [&str; 11] = [
    "at_us",
    "wakes",
    "motions",
    "queued",
    "update_us",
    "commands",
    "draw_us",
    "bytes",
    "write_us",
    "fence_us",
    "input_age_us",
];

#[test]
fn fifty_j_presses_write_one_line_per_drawn_frame_every_field_parseable() {
    for (label, wakes) in [
        ("one frame each", ScriptedWakes::spaced(presses(50))),
        // The `q` comes after a pause: queued behind the presses it would
        // join their turn and quit before it drew.
        (
            "all queued, sharing frames",
            ScriptedWakes::idle(presses(50), [vec![0; 50], vec![1]].concat()),
        ),
    ] {
        let file = Shared::default();
        let meter = Meter::default();
        let mut tracer = Tracer::on(
            FrameTrace::writing_to(Box::new(file.clone())),
            "test".to_owned(),
            Some(meter.clone()),
            None,
        );
        let (exit, written) = run(wakes, &mut tracer, meter);
        assert_eq!(exit, Exit::Quit);
        let trace = file.text();
        let frames = frames(&trace);
        let moved: Vec<_> = frames
            .iter()
            .filter(|frame| field(frame, "motions").is_some_and(|m| m > 0))
            .collect();
        assert!(
            (1..=50).contains(&moved.len()),
            "{label}: {} frames drew the presses",
            moved.len()
        );
        let motions: u64 = frames.iter().filter_map(|f| field(f, "motions")).sum();
        assert_eq!(motions, 50, "{label}: every press counted once");
        for frame in &frames {
            let names: Vec<&str> = frame.keys().map(String::as_str).collect();
            let mut expected = FIELDS.to_vec();
            expected.sort_unstable();
            assert_eq!(names, expected, "{label}");
            assert!(field(frame, "fence_us").is_none(), "no fence before G-pace");
        }
        // One line per drawn frame: as many lines as synchronized updates
        // ended on the terminal.
        let ended = written
            .matches(&*String::from_utf8_lossy(END_SYNCHRONIZED_UPDATE))
            .count();
        assert_eq!(frames.len(), ended, "{label}");
        let bytes: u64 = frames.iter().filter_map(|f| field(f, "bytes")).sum();
        assert_eq!(
            usize::try_from(bytes).expect("small"),
            written.len(),
            "{label}: the meter counted every byte the terminal got"
        );
        let header = trace.lines().next().expect("a header first");
        assert_eq!(
            header,
            "header tier=unicode columns=80 rows=24 history_rows=100 refs=0 term=test"
        );
    }
}

#[test]
fn the_trace_holds_no_subject_and_no_path() {
    let file = Shared::default();
    let meter = Meter::default();
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(file.clone())),
        "test".to_owned(),
        Some(meter.clone()),
        None,
    );
    let (_, written) = run(ScriptedWakes::spaced(presses(10)), &mut tracer, meter);
    assert!(written.contains("Commit 9"), "the subjects were on screen");
    let trace = file.text();
    for marker in ["Commit", "zebracorn", "/code", "test@example"] {
        assert!(!trace.contains(marker), "{marker} in the trace:\n{trace}");
    }
    // Every value is a number or `-`, but the header's two words.
    for line in trace.lines() {
        for field in line.split(' ').skip(1) {
            let (name, value) = field.split_once('=').expect("key=value");
            if matches!(name, "tier" | "term") {
                continue;
            }
            assert!(
                value == "-" || value.bytes().all(|b| b.is_ascii_digit()),
                "{field} in {line}"
            );
        }
    }
}

#[test]
fn a_pump_run_with_the_variable_unset_writes_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    let opened = Tracer::open(FrameTrace::from_var(None), "test".to_owned());
    assert!(opened.meter.is_none() && opened.stamps.is_none());
    let mut tracer = opened.tracer;
    let (exit, written) = run(
        ScriptedWakes::spaced(presses(5)),
        &mut tracer,
        Meter::default(),
    );
    assert_eq!(exit, Exit::Quit);
    assert!(!written.contains("trace:"), "nothing to report: no status");
    let left = std::fs::read_dir(dir.path()).expect("dir").count();
    assert_eq!(left, 0);
}

#[test]
fn an_unwritable_path_says_so_on_the_status_row_and_the_app_runs_on() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("missing").join("trace.txt");
    let opened = Tracer::open(
        FrameTrace::from_var(Some(path.into_os_string())),
        "test".to_owned(),
    );
    let mut tracer = opened.tracer;
    let (exit, written) = run(
        ScriptedWakes::spaced(presses(3)),
        &mut tracer,
        Meter::default(),
    );
    assert_eq!(exit, Exit::Quit, "kept running to the q");
    assert!(written.contains("trace: "), "status row says why");
    assert!(
        written.contains("No such file"),
        "with the reason: {written}"
    );
}

/// A trace file that takes the first `ok` writes, then fails.
struct FailsAfter {
    ok: usize,
}

impl Write for FailsAfter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.ok == 0 {
            return Err(io::Error::other("disk full"));
        }
        self.ok -= 1;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn a_trace_write_that_fails_mid_run_stops_the_trace_and_the_app_runs_on() {
    let mut tracer = Tracer::on(
        FrameTrace::writing_to(Box::new(FailsAfter { ok: 3 })),
        "test".to_owned(),
        None,
        None,
    );
    let (exit, written) = run(
        ScriptedWakes::spaced(presses(6)),
        &mut tracer,
        Meter::default(),
    );
    assert_eq!(exit, Exit::Quit);
    assert_eq!(written.matches("trace: disk full").count(), 1, "{written}");
    assert!(tracer.clock().is_none(), "tracing stopped");
}

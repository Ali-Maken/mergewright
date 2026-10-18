//! The stash put-back pass inside the input-to-frame budget
//! (m6-conflicts/75; PROMPT.md §6.3 "input to frame: under 16 ms";
//! ADR-0021): the `.` menu opening with its fixed Checkout and Cherry-pick
//! slots, and the checkout preview with the `your edits:` row, at 120x40 and
//! 187x50.
//!
//! Timed as `resolver/frame_budget.rs` times the resolver: `update` through
//! `view::view`, ratatui's diff and crossterm's encoding into an in-memory
//! sink; the terminal's own parsing and painting are left out. The preview's
//! plan is a job's answer, not a key's, so the preview is timed as the frame
//! drawn whole over its planned state — every cell painted, the most a frame
//! of it can cost. Heavy tier only (`heavy-tests`, ADR-0018); the budget
//! fails only in a release build (ADR-0021), and a debug build prints what
//! it measured.

use std::sync::PoisonError;
use std::time::{Duration, Instant};

use mergewright::config::Loaded;
use mergewright::session::Session;
use mergewright::update::AppState;
use mergewright_core::event::KeyCode;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::{cursor_to, main_and_fix, planned, rev};
use crate::app_harness::{App, git};
use crate::heavy::MEASURING;
use crate::preview::opened;

/// PROMPT.md §6.3.
const INPUT_TO_FRAME: Duration = Duration::from_millis(16);

/// How many times each frame is timed per size.
const ROUNDS: usize = 40;

type Sink = Terminal<CrosstermBackend<std::io::Sink>>;

fn terminal(columns: u16, rows: u16) -> Sink {
    Terminal::with_options(
        CrosstermBackend::new(std::io::sink()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, columns, rows)),
        },
    )
    .expect("an in-memory terminal opens")
}

fn draw(terminal: &mut Sink, session: &Session, state: &AppState) {
    terminal
        .draw(|frame| mergewright::view::view(session, state, frame))
        .expect("drawing into a sink cannot fail");
}

/// Prints the summary line and returns the frames at or over the budget.
fn summed(what: &str, columns: u16, rows: u16, latencies: &[Duration]) -> Vec<String> {
    let mut sorted = latencies.to_vec();
    sorted.sort();
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    println!(
        "stash_frames: what={what} size={columns}x{rows} frames={} p50_us={} p99_us={} max_us={} budget_us={}",
        sorted.len(),
        at(50).as_micros(),
        at(99).as_micros(),
        sorted.last().map_or(0, Duration::as_micros),
        INPUT_TO_FRAME.as_micros(),
    );
    latencies
        .iter()
        .enumerate()
        .filter(|(_, spent)| **spent >= INPUT_TO_FRAME)
        .map(|(n, spent)| format!("{what} {columns}x{rows} #{n}: {spent:?}"))
        .collect()
}

/// `.` on `fix`'s row, timed key through frame, then `Esc` (untimed) —
/// [`ROUNDS`] times.
fn menu_opens(app: &App, state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    let session = Session::settle(&Loaded::default(), crate::captures::capabilities());
    let mut terminal = terminal(columns, rows);
    let mut state = app.resize(state, columns, rows);
    draw(&mut terminal, &session, &state);
    let mut latencies = Vec::new();
    for _ in 0..ROUNDS {
        let started = Instant::now();
        state = app.key(&state, KeyCode::Char('.'));
        draw(&mut terminal, &session, &state);
        latencies.push(started.elapsed());
        assert!(state.context_menu.is_some(), "`.` opened no menu");
        state = app.key(&state, KeyCode::Escape);
        draw(&mut terminal, &session, &state);
    }
    summed("menu-open", columns, rows, &latencies)
}

/// The planned checkout preview with its `your edits:` row, drawn whole
/// [`ROUNDS`] times.
fn preview_frames(app: &App, state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    let state = app.resize(state, columns, rows);
    let body = crate::captures::frame(&state, columns, rows).join("\n");
    assert!(
        body.contains("your edits:"),
        "not the set-aside preview at {columns}x{rows}:\n{body}"
    );
    let session = Session::settle(&Loaded::default(), crate::captures::capabilities());
    let mut latencies = Vec::new();
    for _ in 0..ROUNDS {
        // A fresh terminal each round: its first frame paints every cell.
        let mut terminal = terminal(columns, rows);
        let started = Instant::now();
        draw(&mut terminal, &session, &state);
        latencies.push(started.elapsed());
    }
    summed("preview-frame", columns, rows, &latencies)
}

/// `. ⏎` on `fix`'s row over the edited `base.txt`: a checkout preview that
/// sets the edit aside, planned.
fn dirty_preview(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    planned(app, state)
}

#[test]
fn the_menu_and_the_set_aside_preview_keep_input_to_frame_under_16_ms() {
    let scratch = main_and_fix();
    let path = scratch.path();
    std::fs::write(path.join("base.txt"), "my edit\n").expect("the scratch repo is writable");
    git(path, &["add", "--", "base.txt"]);
    let (app, state) = App::open(path);
    let on_fix = cursor_to(&app, opened(&app, state), &rev(path, "fix"));
    let preview = dirty_preview(&app, &on_fix);

    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let late: Vec<String> = [(120u16, 40u16), (187, 50)]
        .into_iter()
        .flat_map(|(columns, rows)| {
            let mut late = menu_opens(&app, &on_fix, columns, rows);
            late.extend(preview_frames(&app, &preview, columns, rows));
            late
        })
        .collect();
    drop(turn);
    if cfg!(debug_assertions) {
        // ADR-0021: debug records the budget, release enforces it.
        println!(
            "stash_frames: debug build, budget recorded not asserted: {} frames at or over {INPUT_TO_FRAME:?}: {}",
            late.len(),
            late.join(", ")
        );
    } else {
        assert!(
            late.is_empty(),
            "{} frames missed the {INPUT_TO_FRAME:?} input-to-frame budget: {}",
            late.len(),
            late.join(", ")
        );
    }
    drop(preview);
    drop(on_fix);
    app.close();
}

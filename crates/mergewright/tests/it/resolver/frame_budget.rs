//! The resolver's three-pane frame inside the input-to-frame budget at the
//! user's widths (m6-conflicts/63 Scope 5; PROMPT.md §6.3 "input to frame:
//! under 16 ms"; ADR-0021). The band (m6-conflicts/55) styles cells one by
//! one on the resolver's draw path, so the frame most at risk is the widest
//! one with the most blocks on screen: three panes at 187x50 and 200x50, a
//! long file whose screen is all blocks, some taken (the band drawn) and
//! some open (the `?` separators), at the true-colour tier with tints on.
//!
//! Each key is timed from `update` to the frame written: the state's
//! update, `view::view`, ratatui's diff and crossterm's encoding into an
//! in-memory sink. What it leaves out, as `frames.rs` does: the terminal's
//! own parsing and painting. Heavy tier only (`heavy-tests`, ADR-0018); the
//! budget fails only in a release build (ADR-0021), and a debug build
//! prints what it measured.

use std::sync::PoisonError;
use std::time::{Duration, Instant};

use mergewright::config::Loaded;
use mergewright::session::Session;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use super::*;
use crate::heavy::MEASURING;

/// PROMPT.md §6.3.
const INPUT_TO_FRAME: Duration = Duration::from_millis(16);

/// Lines in the long file; every [`EVERY`]th line differs on both sides.
/// Five, not fewer: git joins two conflicts with three or fewer unchanged
/// lines between them into one block (xdiff's `simplify_non_conflicts`).
const LINES: usize = 600;
const EVERY: usize = 5;

/// Blocks taken theirs before the timed keys, so the band is on screen.
const TAKEN: usize = 12;

/// A merge of `feature` into `main` stopped on `long.txt`: [`LINES`] lines,
/// one conflicting line in every [`EVERY`] — 120 one-line blocks, so every
/// screen of the three panes is mostly blocks.
fn long_merge() -> Scratch {
    use crate::app_harness::{git, git_as_mergewright};
    let scratch = mergewright_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let lines = |side: &str| -> String {
        (1..=LINES)
            .map(|n| {
                if n % EVERY == 0 {
                    format!("line {n} as {side} wrote it, long enough to fill a pane's width\n")
                } else {
                    format!("line {n}\n")
                }
            })
            .collect()
    };
    let write = |text: &str| std::fs::write(repo.join("long.txt"), text).expect("writable");
    write(&lines("base"));
    git(repo, &["add", "--all"]);
    git_as_mergewright(repo, &["commit", "-m", "a long file"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write(&lines("feature"));
    git(repo, &["add", "--all"]);
    git_as_mergewright(repo, &["commit", "-m", "feature's lines"]);
    git(repo, &["checkout", "-q", "main"]);
    write(&lines("main"));
    git(repo, &["add", "--all"]);
    git_as_mergewright(repo, &["commit", "-m", "main's lines"]);
    git_stops(repo, &["merge", "feature"]);
    scratch
}

#[test]
fn three_panes_at_187_and_200_columns_keep_input_to_frame_under_16_ms() {
    let scratch = long_merge();
    let (app, state) = opened_on(&scratch);
    let state = taken_and_open_on_screen(&app, &state);

    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let late: Vec<String> = [187u16, 200]
        .into_iter()
        .flat_map(|columns| late_keys_at(&app, &state, columns))
        .collect();
    drop(turn);
    if cfg!(debug_assertions) {
        // ADR-0021: debug records the budget, release enforces it.
        println!(
            "resolver_frames: debug build, budget recorded not asserted: {} keys at or over {INPUT_TO_FRAME:?}: {}",
            late.len(),
            late.join(", ")
        );
    } else {
        assert!(
            late.is_empty(),
            "{} keys missed the {INPUT_TO_FRAME:?} input-to-frame budget in the resolver: {}",
            late.len(),
            late.join(", ")
        );
    }
    drop(state);
    app.close();
    drop(scratch);
}

/// The resolver on `long.txt` at 187x50 with the first [`TAKEN`] blocks
/// taken theirs and the cursor back among them, so taken blocks (the band)
/// and open ones (the `?` separators) share the screen.
fn taken_and_open_on_screen(app: &App, state: &AppState) -> AppState {
    let state = app.resize(state, 187, 50);
    let mut state = resolver_on(app, state, "long.txt");
    // Each take moves the cursor on to the next open block (m6-conflicts/15).
    for _ in 0..TAKEN {
        state = app.key(&state, KeyCode::Char('t'));
        state = settled(app, state);
    }
    assert_eq!(open(&state).cursor_block, Some(TAKEN), "{:?}", state.status);
    for _ in 0..TAKEN / 2 {
        state = app.key(&state, KeyCode::Char('N'));
    }
    state
}

/// Checks the frame at `columns`x50 is the one worth timing, times the keys,
/// prints the summary line and returns the keys at or over the budget.
fn late_keys_at(app: &App, state: &AppState, columns: u16) -> Vec<String> {
    let sized = app.resize(state, columns, 50);
    let frame = crate::captures::frame(&sized, columns, 50);
    let body = frame.join("\n");
    // Three panes: a taken block's row shows yours, then theirs twice
    // (theirs and the result that took it).
    assert!(
        frame.iter().any(|row| row.contains("as main wrote it")
            && row.matches("as feature wrote it").count() == 2),
        "not three panes at {columns}x50:\n{body}"
    );
    assert!(
        body.contains("◀"),
        "no taken block's band on screen:\n{body}"
    );
    assert!(body.contains('?'), "no open block on screen:\n{body}");
    let latencies = timed_keys(app, sized, columns);
    let mut sorted = latencies.clone();
    sorted.sort();
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    println!(
        "resolver_frames: size={columns}x50 keys={} p50_us={} p99_us={} max_us={} budget_us={}",
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
        .map(|(key, spent)| format!("{columns}x50 key {key}: {spent:?}"))
        .collect()
}

/// `n` ten times, then `N` ten times, four rounds: each key's `update`
/// through to its frame written into an in-memory terminal, timed.
fn timed_keys(app: &App, mut state: AppState, columns: u16) -> Vec<Duration> {
    let session = Session::settle(&Loaded::default(), crate::captures::capabilities());
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(std::io::sink()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, columns, 50)),
        },
    )
    .expect("an in-memory terminal opens");
    // The first frame paints every cell; the timed ones paint what changed,
    // as the real loop does.
    terminal
        .draw(|frame| mergewright::view::view(&session, &state, frame))
        .expect("drawing into a sink cannot fail");
    let keys =
        (0..4).flat_map(|_| std::iter::repeat_n('n', 10).chain(std::iter::repeat_n('N', 10)));
    let mut latencies = Vec::new();
    for key in keys {
        let started = Instant::now();
        state = app.key(&state, KeyCode::Char(key));
        terminal
            .draw(|frame| mergewright::view::view(&session, &state, frame))
            .expect("drawing into a sink cannot fail");
        latencies.push(started.elapsed());
    }
    latencies
}

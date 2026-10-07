# Load flakes: two tests that failed once each in a loaded workspace run

Status: done
Build with: debugger (Opus).
**Crate set: graft only.** Files: `crates/graft/src/event_loop/{tests.rs,tests/pace.rs,places.rs}`,
`crates/graft/tests/it/{pty.rs,outside_repo.rs}`, `crates/graft/MAP.md`.
Source: conductor, 2026-10-07. Each failed once in a workspace nextest on main while two builders loaded the machine,
and passed alone every time. No failure output was kept.

## NOTES (debugger, 2026-10-07)

No compaction. No scout: two tests and the code under each, read directly.

**How load was made.** `taskpolicy -b` (background QoS: E-cores, low priority) on the test process. That slows the
test without loading the machine for the other builder. Load average 10–17 from the other builder throughout.

### 1. `event_loop::tests::queued_keys_draw_once`. Cause in the test (its clock), not the product

- **Red.** 121 of 200 runs failed under `taskpolicy -b`; 0 of 200 failed without it. Symptom:
  `tests.rs:916 frames: left 3|4|5, right 2`.
- **Mechanism, observed** (`[DEBUG-13h]` probe on the draw branch, `running/turn.rs:158`, since removed). The
  extra frames were drawn with `elapsed` 16–169 ms.
  - The turn takes queued keys only while `elapsed < BATCH_BUDGET` (8 ms, `running/turn.rs:32`, checked at :147).
  - `elapsed` is read from `Wakes::now`. `ScriptedWakes` never overrode it, so it read the default `Instant::now()`
    (`event_loop/mod.rs:165`).
  - Under load, 20 debug-build updates take more than 8 ms. Once the budget is spent, the loop draws as soon as the
    pacer allows: 16 ms after the last frame, or 8 ms on a terminal that never answers.
  - That is the designed behaviour (`turn.rs:28-31`: "a burst longer than this still draws as it goes"). The test's
    `== 2` holds only if the machine is fast enough.
- **Fix (cause).** `ScriptedWakes` keeps its own clock (`tests.rs`, field `now`).
  - Time moves only where the loop waits with a deadline. `next_wake_by` advances the clock to the deadline instead
    of sleeping until it.
  - Updates take no time, so the test checks the batching rule, not CPU speed.
  - `pace.rs`'s `Stamped` wrapper forwards `now` and stamps frames with it. The "≥ 16 ms apart" check now runs on
    the clock the pacer actually reads.
  - Cost to the user: none. The change is test-only.
- **Green.** 0 of 200 runs failed under `taskpolicy -b`. The whole `event_loop::` module (150 tests) passed 5 of 5
  times under `taskpolicy -b` with `--test-threads 4`.
- The same latent flake was in its siblings: `a_resize_starts_its_own_turn`, `a_click_waits_for_the_frame_he_saw`
  and `a_wheel_joins_the_frame` count frames the same way. The same fix covers them.

### 2. `it::outside_repo::outside_repo_captures`. Cause in the product (places screen) and the harness

- **The earlier debugger's hypothesis holds, now observed.** `Pty::wait_for` (`tests/it/pty.rs:210`) returned a
  half-drawn frame.
- **Red.** A temporary probe test (removed) waited the same way, `has("RECENT")`, but polled every 200 µs instead of
  20 ms, then compared that screen with the screen 500 ms later. They differed in **5 of 70 runs**, with no extra
  load.
  - Each time, the first screen ended part-way through the `─` rule, with no bar row.
  - The real test loops (40 under `taskpolicy -b`, 60 in 4 parallel lanes) did not go red: its 20 ms poll rarely
    lands inside the window.
- **Cause, product.** `event_loop/places.rs:292` (`outside_loop`) drew each empty-state frame with a bare
  `screen.draw`, with no DEC 2026 bracket.
  - Every repository frame has the bracket (`running/frame.rs:98,115`). ADR-0054 §3 / DESIGN.md §4.1 say every
    frame is atomic.
  - The frame reaches the pty in pieces. A terminal, or the harness, can show the top of a new frame over the
    bottom of the old one.
  - Not verified: whether the pieces come from std's `Stdout` buffering or from the pty's own chunking. It does not
    change the fix.
- **Cause, harness.** `Screen::replay` showed every byte as it came, so even a bracketed frame could be taken
  half-drawn.
- **Fix (cause).**
  1. `places::draw_whole` wraps each empty-state frame in `SYNC_BEGIN`/`SYNC_END`. The end marker goes out even when
     the draw fails, as `draw_frame` does it.
  2. `Screen::replay` leaves out a synchronized update that has begun and not ended (`pty.rs` `ended`), which is
     what a DEC 2026 terminal shows.
  - Cost to the user: none visible. A 2026 terminal (Ghostty, VS Code, Zed) now shows the empty state's frames
    whole; others ignore the sequence.
- **Green.** The same probe found **0 of 120** differing after the fix, which would be about 0.01 % likely at the
  old rate.
- **Regression tests**, red before the fix and green after:
  - `it::pty::a_frame_not_yet_ended_is_not_on_the_screen` (the harness)
  - `it::outside_repo::the_empty_state_draws_each_frame_as_one_synchronized_update` (the product: begin and end
    counts match, and `RECENT` and `quit` are drawn inside an open update)

### Hypotheses ruled out for 2

- **A later, different frame** (re-detection or age text): ruled out. Every difference was a truncated first frame;
  the settled screens matched the capture.
- **`quit_cleanly`** (exit code or stderr) under load: not observed in 100 loop runs.

**Touches.**
- `Screen::replay` is used by every pty test: `outside_repo`, `not_a_repo`, `keymap_config`, `palette_find`,
  `quit_waits::{signal,hangup}`. It only holds back an unended update, and all of them passed in the workspace
  nextest.
- `Exit` and the places loop's wakes are unchanged.
- Captures: the text grids are unchanged, since the bracket is invisible to `Screen`. `outside_repo_captures` and
  every `check_shot` passed.
- `ALT_SCREEN` and `PROBE_FIRST` counts are unaffected (`open_recent_outside_…` passed).
- The places-screen hang-up path (13g finding) is unchanged: a failed write still returns its error.
- `ScriptedWakes` users: `tests.rs`, `tests/{pace,perf_trace,pixel_scroll}.rs`. `Stamped` is the only wrapper, and
  it now forwards `now`.
- MAP: the `places::drive` row gains `draw_whole`.

**Gates.** All workspace-wide: `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.
`cargo nextest run --test-threads 4`: 5848 passed, 10 skipped (620 s, beside the other builder).
`cargo test --doc`: ok.

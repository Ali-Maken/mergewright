# 43-scroll-coalesce-input — queued keys are applied together and drawn once

Status: blocked — loop built and green; Unicode scenario B red because held_key.rs (42's file) counts J's own frame as a post-release j frame. See NOTES continuation 1.
Build with: Sonnet — one loop rule in two files, every case named below with its test; the scout mapped every
seam (`.scratch/m7-power-tools/scouts/43.md`).
Needs: `42-scroll-held-key-gate` landed (its red run recorded first). No signature from another ticket.
Blocked by: 42
File set (crate `graft`): `src/event_loop/running.rs`, `src/event_loop/mod.rs`, `src/event_loop/tests.rs`,
`tests/it/frames/paced.rs`, `tests/it/frames/readings.rs`, `crates/graft/MAP.md`,
`docs/perf/fix-scroll-held-key.txt` (append).
Overlap and order:
- 42 before (planner ruling 2026-10-01, scout 42): 42 may widen visibility and add read-only accessors in
  `paced.rs` (per-`Drawn` `Instant` and sink offset), and adds `Sink::draining` (`frames/sink.rs`) and
  `ready_to_scroll_at` (`frames/run.rs`). 43 owns `paced.rs` from 42's merge on: keep 42's accessors working with the
  same meaning (`held_key` reads them). `paced.rs` line numbers below are from HEAD 552efc7f; re-locate by name.
  `sink.rs`, `run.rs`: not in this file set.
- 45 also edits crate `graft` (literal sites: `event_loop/graphics.rs`, `session.rs`, `examples/kitty_check/
  driver.rs`, some `tests/it/*`): disjoint files, may run concurrently (ADR-0051: two builders in the crate).
  If 45 lands first and adds a field, re-run the gates after merge; no file is shared.
- 46 edits `running.rs` (prefetch call, `:115`), `crates/graft/MAP.md` and the perf file: **46 after 43**
  (46 Blocked by 43).
- 41, 44: none.

## Read first
- `.scratch/m7-power-tools/scouts/43.md` (whole; 1 page).
- `42-scroll-held-key-gate.md` §Need, §Prior art, §Journeys (the pipeline sections for this fix), §Planner ruling.
- `.scratch/m7-power-tools/debug-scroll-lag.md` cause 2 and "Proposed fix" 2.
- `crates/graft/MAP.md` `## event_loop` (`:888-909`; `job_event_waiting` ~`:900`), `event_loop::input`
  (`:936-937`), `event_loop::outside` (`:911-928`).
- `crates/graft-core/src/event.rs:20-21` (key doc: releases never reach `update`) and `:195-202` (`MouseKind`).

## Why
Today `frames` (`running.rs:91-174`) takes one wake, runs `update` + `carry_out`, and draws (`:116`, `:170`); only
job answers share a frame (`:163-169`, "A key is never taken this way"). When a frame leaves slower than keys arrive
(a big Pixel frame, a busy terminal), keys pile up in the channel and each is drawn after the last one arrived.
Ratatui's recipe (research §Terminal techniques): input updates state, one draw. Take that; reject the fixed render
tick (see gate spec, Prior art).

m7/12's "a key is always drawn in its own pass" gives no reason beyond the parenthetical
(`12-idle-layout-advance.md:53`, `:325`; scout 43 item 6). No key ends a batch on that ground.

## Decided
1. **A turn** takes one wake, then keeps taking queued wakes while the next one **joins the frame**, running
   `update` + `carry_out` for each in arrival order, then draws once.
2. **Joins the frame:** job events (`said_by_a_job`, `mod.rs:230`, the predicate `running.rs` uses today; if it and
   the public `is_job_event`, `mod.rs:220`, differ, say how), **key events** (there is no release variant,
   `event.rs:20-21`: every `Event::Key` the loop sees is a press or repeat; `update` is pure and each key's commands
   still run in order, so the result equals drawing between them) and **wheel scrolls** (`MouseKind::Scroll{Up,Down,
   Left,Right}`). **Does not join** (the turn draws first, the wake starts the next turn): every other mouse event
   (buttons, drags, `Moved` hover: he pointed at what he saw), paste, resize, focus, signals, terminal wakes,
   shutdown.
3. **The batch ends early** when `carry_out` hands the terminal over — any of the six commands at `running.rs:212-217`
   (incl. `EditCommitMessage`, `OpenExternalEditor`, `OpenConflictEditor`) — or the loop ends (`Exit`): the rest stays
   queued, exactly as today after such a key (`ChannelWakes::unhandled_input` keeps what follows a quit).
   `carry_out` returns only `AppState` today, so it gains the signal (below).
4. **Cap:** the existing 8 ms of updates per turn (`BATCH_BUDGET`, `running.rs:88`) now counts keys too; past it the
   turn draws and the rest starts the next turn.
5. **No new state, no timer.** When nothing is queued the turn is one wake, as today: a lone key's latency does not
   change.
6. **Trait:** `job_event_waiting` is replaced by a **required** method, so no `impl Wakes` can silently skip
   batching (today's default `false`, `mod.rs:106-108`, is how a harness would quietly stop coalescing).

## Budget, defined on arrival
A release never reaches the loop (`event.rs:20-21`), so nothing here is timed from it. Every bound starts at a key's
**arrival**: the moment `next_wake` can return it (in tests, its send). The last key the loop sees is the OS's last
repeat, which precedes the release, so a bound from the last arrival is at least as strict as the user's "after I let
go".
- **Graft's share:** a key that arrives while a frame is being written is applied in the next turn and shown in that
  turn's frame — never later, unless the 8 ms cap cut the batch (≈ 300 keys at `update`'s 25 µs). So from the last
  arrival: at most the frame in flight + ≤ 8 ms of updates + one draw.
- **The gate** (42, release): `frames_after_release ≤ 2` (frames drawn after the last `j`'s arrival), `tail_ms ≤ 100`
  (last `j` arrival → the drain passing the last frame's last byte), `j_ms ≤ 100` (`J` arrival → first byte of `J`'s
  frame drained).
- The 8 ms cap itself is not timed by a test (no clock seam; no sleeps in tests): argued in the report and recorded
  as **unverified** in NOTES. The release held-key run is its only measurement.

## Signatures
```rust
// crates/graft/src/event_loop/mod.rs
/// Can this wake be applied in the same turn as the one before it, sharing one draw?
/// Job events, key events (press/repeat; no release reaches here), wheel scrolls. Nothing else.
pub fn joins_frame(wake: &Wake) -> bool;

pub trait Wakes {
    // removed: fn job_event_waiting(&mut self) -> bool   (had a default `false`)
    /// Never blocks. Is a wake queued, and does it `joins_frame`?
    fn joining_waiting(&mut self) -> bool;      // required, no default; &mut as today (ChannelWakes keeps `seen`)
    // waiting, next_wake, mark, … unchanged
}

// crates/graft/src/event_loop/running.rs (private)
enum Carried { Continue, EndsBatch }           // EndsBatch: a handover command ran, or the loop is ending
fn carry_out(/* as today */) -> (AppState, Carried);
```
`is_job_event` stays (the startup trace and tests use it). `pump`'s signature is unchanged.
Impls to change (method only): `ChannelWakes` (`mod.rs:186`), `ScriptedWakes` (`event_loop/tests.rs:94`, method
`:111`), `Paced` (`tests/it/frames/paced.rs:302` at 552efc7f). None in `tests/it/frame_trace.rs` (scout grep).
`DrainSink` (42) is not a `Wakes` impl (42 ruling 1).

## Acceptance (each box names its test; red first)
- [x] 20 `j` queued before the turn → exactly one frame drawn, cursor 20 rows down.
      — `event_loop::tests::queued_keys_draw_once`.
- [x] `j j <resize> j` queued → two frames: one for `j j`, one for the resize turn (with the last `j` joining it);
      final cursor 3 rows down, size the new one. — `event_loop::tests::a_resize_starts_its_own_turn`.
- [x] `j j <mouse click> j` → the click is handled after a frame showing `j j`.
      — `event_loop::tests::a_click_waits_for_the_frame_he_saw`.
- [x] `j j <wheel down> j` → one frame. — `event_loop::tests::a_wheel_joins_the_frame`.
- [x] `j j q j` → loop ends `Exit::Quit`; the trailing `j` is in `unhandled_input`; no frame drawn after the startup
      draw (the batch ends on `Exit` before drawing). — `event_loop::tests::a_quit_ends_the_batch`.
- [x] A command that hands the terminal over ends the batch; the wake after it is still queued. No scripted `Outside`
      exists (tests use `Outside::default()`, which refuses, `running.rs:238-258`), so: a `carry_out` unit test per
      handover command returning `Carried::EndsBatch`, plus a loop test that `EndsBatch` stops taking wakes.
      — `event_loop::tests::{every_handover_ends_the_batch, a_handover_ends_the_batch}`.
- [x] A lone key still draws in its own turn (latency unchanged). — `event_loop::tests::a_lone_key_draws_at_once`.
- [x] A two-key prefix (`m m`) and a modal prompt queued together behave as when drawn between.
      — `event_loop::tests::a_queued_prefix_runs_as_typed`.
- [x] **The two m7/12 tests this rule changes**, rewritten, not deleted (scout 43 item 5):
  - `queued_wakes_draw_once` (`tests.rs:672`): its trailing quit key now joins the job batch and the batch ends on
    `Exit` before drawing (1 frame, not 2). Change: put one non-joining wake (focus gained, or `Moved` hover — one that
    changes no state the test reads) between the last job and the quit. Assertion **unchanged: 2 frames** — the
    startup draw and one shared draw for every job. Not weaker: what it proves (queued jobs share one draw) is
    asserted at the same count; only the script's terminator moved.
  - `a_key_behind_queued_wakes_is_drawn_in_its_own_pass` (`tests.rs:688`, 4 frames) asserts the rule removed here.
    Renamed `a_key_behind_queued_wakes_joins_their_frame`; same script; the new count derived in a comment from
    Decided 1–3; plus a new assertion that the frame which shows the jobs also shows the key's effect. Not weaker: it
    still pins exactly where the key is drawn, now to the new rule, and adds what the old one never checked.
  - Every other m7/12 test unchanged and green; name them in the report.
- [x] The `frames::` harness measures a key's latency from its send to the frame that showed it; a batched key's
      latency is to the batch's frame. Today `readings.rs:50-63` takes `drawn[index + 1]` per wake and `paced.rs`
      (`:114-116`, `:174-178` at 552efc7f) stamps one `drawn` per `next_wake`: both change so each wake maps to the
      frame drawn at the end of its turn. — new `frames::readings::a_batched_key_reads_its_batch_frame`; existing
      `frames::` tests green with unchanged budgets.
- [x] A `Drawn` mark still means one frame drawn, and 42's `paced.rs` accessors (per-`Drawn` `Instant` and sink
      offset) keep that meaning (`held_key` reads them). — checked by the release run below.
- [ ] Unicode half of the gate goes green in release: `held_key::held_j_stops_on_release_unicode` and
      `held_key::j_then_journal_opens_at_once_unicode`, run by `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` (`# rev coalesce`). Pixel stays red until `46-scroll-strips-wiring`; record
      its numbers (expect `tail_ms` bounded near two Pixel frames, `bytes_per_frame_held` still MB).
- [x] `crates/graft/MAP.md`: `joins_frame`, `Wakes::joining_waiting`; the `job_event_waiting` line removed.

## Touches (report each on the **Touches:** line)
- Every `job_event_waiting` caller and impl (grep; all replaced; three impls above).
- Idle prefetch in `frames` (`prefetch` asks `waiting()`, `running.rs:115`): unchanged here; 46 rewires it later.
  Say how checked.
- `undrawn_since` (`:111`, `:147`, resets `:140`, `:171`): still the clock for the cap; say how the reset points move.
- Journal/undo: no new operation; `u` queued behind `j`s runs once (test or argument).
- Captures: none change (same frames, fewer of them only under backlog); fast-tier capture tests stay green.
- `scripts/m7-perf-key.sh`: reads per-key lines; check a batched key still prints a line. Its `frames::` filter does
  not select `held_key::` (42 ruling 2).
- 42's `held_key` readings and `paced.rs` accessors: read `Drawn` marks, unchanged meaning (box above).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
Medium (~45k).

## NOTES (builder, 2026-10-01)
Scout used (`scouts/43.md`; paced.rs re-located by name at 0d38d93d). No compaction.

Ticks: every box names its test above; the `Drawn`-meaning box is checked by the release run (held_key read its frames;
`frames_while_held`/`frames_after_release` counts are per `Drawn`). **Unicode box NOT ticked**: release run
(`# rev coalesce` in `docs/perf/fix-scroll-held-key.txt`):
- Unicode A: green (frames_after_release 2, tail 73 ms; was 109 / 3260).
- Unicode B: red — frames_after_release 3, tail 105 ms; j_ms 25 (was 3210), frames_before_J 0. One run only; not
  diagnosed. Suspect: `J`'s journal answer (a job event) arrives after `J`'s frame and draws a third frame.
- Pixel A: tail 1391 ms (was 215976), frames_after_release 2. Pixel B: tail 372, j_ms 318. bytes_per_frame_held=0
  because frames_while_held=0 (every repeat joined one turn behind the slow first frame) — 46's job.

Deviations / test changes:
- `a_key_behind_queued_wakes_joins_their_frame`: same script now asserts 1 frame (quit ends the turn undrawn);
  the "frame shows the key" check runs the script without the quit and with `j` (unbound key has no effect).
- Seven pre-existing tests assumed a frame per key (`[key, quit]` scripts): they now use `ScriptedWakes::spaced`
  (each wake arrives after the frame before it) — `pixel_run`, f3 silent-terminal, `every_frame_is_wrapped…`,
  `a_job_result_is_just_another_wake`, `copy_run`. Assertions unchanged.
- Loop handover test needs `NoTty` backend: ratatui `clear` asks the tty for the cursor (ENXIO in tests).
- 8 ms cap: unverified (no clock seam), as specified.
- Not in file set, not changed: `held_key.rs` reads `written[k+1]` per wake — still valid since stamps stay one per
  `next_wake`, and a batched wake writes 0 bytes; `after_idle.rs` uses `drawn[shown+1]` (look, not frame) if `G`'s
  window joins a turn — possible small misread, unverified.

## NOTES (continuation 1, 2026-10-01)
No scout (43's scout already covers the loop; this was a measurement question, answered by a temporary print). No compaction.

- Repeated: `scripts/fix-scroll-held-key.sh` 3x (to scratch) + 3 direct runs: Unicode B red **every time**:
  frames_after_release=3 each run, tail_ms 99/100/101/106/110/112/114. Unicode A green each run (2 frames, 76-79 ms).
- Diagnosis (temporary eprintln in `held_key::read`, reverted): B's post-release frames are, by handed index,
  k=278 j#186 (in flight when the last j arrived, +13-17 ms), k=279 j#187 (+34-48 ms), **k=282 = J's frame**:
  J handed at k=280 (sent +49-56 ms after last j), its journal answers k=281-282 joined the turn, one frame drawn,
  stamped on k=282 (+66-77 ms). So J + its journal answer draw **one** frame; last builder's guess (journal answer
  draws an extra frame) is wrong. A shows the same first two frames and no third.
- Cause: the B metrics include J. `crates/graft/tests/it/held_key.rs:342-349` counts every frame drawn after the
  last j as "after release" — J's frame is necessarily one (J comes THEN_J = 50 ms later, `:51`). `:364-368` takes
  tail to `written.last()`, J's frame end, so tail = 50 ms wait + J's frame (~32 ms at 500 KB/s) + jitter.
  The loop is behaving as specified; j_ms 11-26, frames_before_J 0.
- Fix belongs in held_key.rs (42's test, not in 43's file set): in B, count frames/tail only up to J's handed index
  (J is judged by j_ms/j_done_ms). Not done here — changing a gate's definition needs the conductor/spec owner.
  Box stays open.

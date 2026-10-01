# 43-scroll-coalesce-input — queued keys are applied together and drawn once

Status: ready
Build with: (blank: scout first — `event_loop::running` is existing code)
Needs: `42-scroll-held-key-gate` landed (its red run recorded first). No signature from another ticket.
Blocked by: 42

## Read first
- `42-scroll-held-key-gate.md` §Need, §Prior art, §Journeys (the pipeline sections for this fix).
- `.scratch/m7-power-tools/debug-scroll-lag.md` cause 2 and "Proposed fix" 2.
- `crates/graft/MAP.md` `## event_loop` (`:888-909`), `event_loop::input` (`:936-937`), `event_loop::outside`
  (`:911-928`).
- `.scratch/m7-power-tools/issues/12*.md` Decided sections only: why "a key is always drawn in its own pass" was
  chosen (m7/12). The scout states that reason; if it still holds for some key, that key ends the batch.

## Why
Today `pump` takes one wake, runs `update` + `carry_out`, and draws (`running.rs:116`, `:170`); only job answers
share a frame (`running.rs:162-168`, "A key is never taken this way"). When a frame leaves slower than keys arrive
(a big Pixel frame, a busy terminal), keys pile up in the channel and each is drawn after release. Ratatui's recipe
(research §Terminal techniques): input updates state, one draw. Take that; reject the fixed render tick (see gate
spec, Prior art).

## Decided
1. **A turn** takes one wake, then keeps taking queued wakes while the next one **joins the frame**, running
   `update` + `carry_out` for each in arrival order, then draws once.
2. **Joins the frame:** job events (`is_job_event`, unchanged) and **key presses and repeats** (any key: `update` is
   pure and each key's commands still run in order, so the result equals drawing between them) and **wheel scrolls**.
   **Does not join** (the turn draws first, the wake starts the next turn): mouse buttons and drags (he clicked what
   he saw), paste, resize, focus, signals, terminal wakes, and key releases if the decoder sends them.
3. **The batch ends early** when a carried-out command hands the terminal over (`$EDITOR`, mergetool, suspend,
   `--continue` in git) or ends the loop (`Exit`): the rest stays queued, exactly as today after such a key
   (`ChannelWakes::unhandled_input` keeps what follows a quit).
4. **Cap:** the existing 8 ms of updates per turn (m7/12) now counts keys too.
5. **No new state, no timer.** When nothing is queued the turn is one wake, as today: a lone key's latency does not
   change.
6. **Trait:** `job_event_waiting` is replaced by a **required** method, so no `impl Wakes` can silently skip
   batching (a default `false` is how the harness would quietly stop coalescing).

## Signatures
```rust
// crates/graft/src/event_loop/mod.rs
/// Can this wake be applied in the same turn as the one before it, sharing one draw?
/// Job events, key presses/repeats, wheel scrolls. Nothing else.
pub fn joins_frame(wake: &Wake) -> bool;

pub trait Wakes {
    // removed: fn job_event_waiting(&self) -> bool
    /// Never blocks. Is a wake queued, and does it `joins_frame`?
    fn joining_waiting(&self) -> bool;          // required, no default
    // waiting, next_wake, mark, … unchanged
}
```
`is_job_event` stays (the startup trace and tests use it). `pump`'s signature is unchanged.

## Acceptance (each box names its test; red first)
- [ ] 20 `j` queued before the turn → exactly one frame drawn, cursor 20 rows down.
      — `event_loop::tests::queued_keys_draw_once`.
- [ ] `j j <resize> j` queued → two frames: one for `j j`, one for the resize turn (with the last `j` joining it);
      final cursor 3 rows down, size the new one. — `event_loop::tests::a_resize_starts_its_own_turn`.
- [ ] `j j <mouse click> j` → the click is handled after a frame showing `j j`.
      — `event_loop::tests::a_click_waits_for_the_frame_he_saw`.
- [ ] `j j q j` → loop ends `Exit::Quit`; the trailing `j` is in `unhandled_input`; one frame drawn at most before
      exit. — `event_loop::tests::a_quit_ends_the_batch`.
- [ ] A key whose command hands the terminal over ends the batch; the key after it is still queued when the handover
      runs (scripted `Outside` seam if one exists; scout names it, else a `carry_out` unit test on the ending rule).
      — `event_loop::tests::a_handover_ends_the_batch`.
- [ ] A lone key still draws in its own turn (latency unchanged). — `event_loop::tests::a_lone_key_draws_at_once`.
- [ ] Job answers still share one frame (m7/12 tests unchanged and green). — name the existing tests in the report.
- [ ] The `frames::` harness still measures a key's latency from its send to the frame that showed it; a batched
      key's latency is to the batch's frame. — `frames::` readings unit test the scout names, or new
      `frames::readings::tests::a_batched_key_reads_its_batch_frame`.
- [ ] Unicode half of the gate goes green in release: `frames::held_key::held_j_stops_on_release_unicode` and
      `j_then_journal_opens_at_once_unicode`, run by `scripts/fix-scroll-held-key.sh`, appended to
      `docs/perf/fix-scroll-held-key.txt` (`# rev coalesce`). Pixel stays red until `46-scroll-strips-wiring`; record
      its numbers (expect `tail_ms` bounded near two Pixel frames, `bytes_per_frame_held` still MB).
- [ ] `crates/graft/MAP.md`: `joins_frame`, `Wakes::joining_waiting`; the `job_event_waiting` line removed.

## Files (exact)
- `crates/graft/src/event_loop/running.rs`, `crates/graft/src/event_loop/mod.rs`, the event loop's existing tests
  module (scout names the file), `crates/graft/MAP.md`.
- Every `impl Wakes for` (grep; expected: `ChannelWakes` in `event_loop`, the scripted list in its tests,
  `tests/it/frames/paced.rs`, maybe `tests/it/frame_trace.rs`) — the method only.
- `tests/it/frames/readings.rs` / `verdict.rs` only if latency attribution needs it (scout says).
- `docs/perf/fix-scroll-held-key.txt` (append).
- **Not** `event_loop/graphics.rs`, `view/strips.rs` (`46-scroll-strips-wiring` holds them).

## Touches (report each on the **Touches:** line)
- Every `job_event_waiting` caller and impl (grep; all replaced).
- Idle prefetch in `pump` (asks `waiting()` before each strip): unchanged; say how checked.
- Prefix keys (`m m`, `r r`) and modal keys (name prompt claims every key): the batch runs the same `update` in the
  same order — prove with one test of a two-key prefix queued together.
- Journal/undo: no new operation; `u` queued behind `j`s runs once (test or argument).
- Captures: none change (same frames, fewer of them only under backlog); `GRAFT_UPDATE_SHOTS` run not needed — the
  fast-tier capture tests stay green.
- `scripts/m7-perf-key.sh`: reads per-key lines; check a batched key still prints a line.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; plus `cargo clippy --all-targets --features heavy-tests -- -D warnings`.

## Size
Medium (~45k).

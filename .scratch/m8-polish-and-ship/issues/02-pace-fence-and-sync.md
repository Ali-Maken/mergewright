# G-pace — the loop never writes ahead of the terminal

Status: done
Build with: Fable (signature) (conductor, after scout `scouts/frame-loop.md`)
Scout: `.scratch/m8-polish-and-ship/scouts/frame-loop.md` — note: `CSI 0 n` is NOT routed as a reply today (decoder drops it, csi.rs:28/97).
Blocked by: 01
Needs: `trace-product-frame-trace` (`FrameRecord::fence_us`).

## Read first
- `docs/adr/0054-the-loop-never-writes-ahead-of-the-terminal.md` (whole; short)
- `docs/design/04-terminal-capability-detection.md` §4.1 (DSR, DEC 2026 bullets), §4.7
- `docs/design-notes/m8-polish-and-ship.md`: Prior art rows 3–5, Interfaces (graft-term, `Pacer`), Journeys J1–J3
- `crates/graft-term/MAP.md`, `crates/graft/MAP.md` (`event_loop`, `Wake`, `joins_frame`, `ReplyWaiter`, `DrainSink`,
  `held_key` tests), `crates/graft-core/MAP.md` (`Capabilities`)
- KNOWN_GAPS rows 1089, 1095 (read the two lines only)

## Scope
In:
- graft-term: `SYNC_BEGIN`/`SYNC_END`; `write_fence`; `fence_answers`; startup probe adds `CSI 5 n`, result in
  `Capabilities::answers_dsr`.
- Every frame: `SYNC_BEGIN` · text · image commands · `SYNC_END` · fence (fence only when `answers_dsr` and not given up).
- graft: `Pacer` (one frame in flight); `Wake::Fence`; the input decoder routes `CSI 0 n` to the reply path, never to
  keys; inputs arriving while a frame is in flight are applied to state, not drawn; the answer releases one frame.
- Fallbacks: 500 ms unanswered → draw anyway; 3 timeouts in a row → stop fencing this run, cap one frame per 16 ms.
- Wheel events in one turn sum into one scroll; per-event step unchanged.
- `DrainSink` (test harness) answers fences as it drains, so `held_key` measures the paced loop.
- `FrameRecord::fence_us` filled.
Out:
- Per-row work at rest (G-settle). Pixel memory (G-pixel-mem). Wheel step change (his scene 1 verdict).

## Signatures
```rust
// graft-term
pub const SYNC_BEGIN: &[u8] = b"\x1b[?2026h";
pub const SYNC_END: &[u8] = b"\x1b[?2026l";
pub fn write_fence(out: &mut impl std::io::Write) -> std::io::Result<()>;
pub fn fence_answers(bytes: &[u8]) -> usize;
// graft-core::Capabilities
pub answers_dsr: bool,
// graft::event_loop (pub(crate))
pub(crate) struct Pacer { /* sent_at: Option<Instant>, misses: u8, fenced: bool, last_drawn: Option<Instant> */ }
impl Pacer {
    pub(crate) fn new(fenced: bool, fence_timeout: Duration) -> Pacer;
    pub(crate) fn may_draw(&self, now: Instant) -> bool;
    pub(crate) fn drew(&mut self, now: Instant);
    pub(crate) fn answered(&mut self, now: Instant);
    pub(crate) fn deadline(&self) -> Option<Instant>;
}
// enum Wake { …, Fence }
```

## Acceptance
- [x] `Pacer` unit tests: answered → may draw; unanswered < 500 ms → may not; 500 ms → may; 3 misses → unfenced and
      `may_draw` false within 16 ms of the last frame; an answer after give-up does not re-enable —
      `event_loop::pace::tests::{an_answered_fence_lets_the_next_frame_draw_at_once,
      an_unanswered_fence_holds_the_next_frame_until_its_timeout, three_timeouts_in_a_row_stop_fencing_and_cap_frames_at_16_ms,
      an_answer_after_giving_up_does_not_fence_again}` (+ `a_late_answer_is_the_oldest_fences_not_the_newest`,
      `an_answer_in_between_resets_the_run_of_timeouts`, `forgetting_after_a_handover_waits_for_nothing`).
- [x] `fence_answers` counts `CSI 0 n` amid other replies and split across reads — `graft_term::pace::tests::
      {fence_answers_are_counted_amid_other_replies_and_keys, an_answer_cut_off_at_the_end_is_not_counted_until_it_is_whole}`;
      split across reads in the decoder: `input::decode::tests::a_fence_answer_split_across_reads_is_one_reply`.
- [x] A `CSI 0 n` reaching the input decoder is never a key — `input::decode::tests::a_fence_answer_is_a_reply_never_a_key`;
      routed to the fence listener, not keys or waiters: `input::mux::tests::fence_answers_go_to_the_fence_listener_not_to_keys_or_waiters`.
- [x] Frame bytes: `SYNC_BEGIN` … images … `SYNC_END` then `CSI 5 n` when fenced, over a Pixel (Kitty) fixture frame —
      `event_loop::tests::pace::a_fenced_pixel_frame_is_bracketed_with_its_images_inside_then_fenced`. iTerm2 bytes
      not asserted by this test (same `draw_frame` path).
- [x] ≤ 1 outstanding fence over 300 held `j`, sink answering only on demand —
      `event_loop::tests::pace::three_hundred_held_keys_never_put_two_frames_in_the_pipe` (also: every key applied in order,
      < 150 frames, every frame fenced, a frame after the last key).
- [x] Wheel: 10 wheel events in one turn → `top` moves 10 × 3 lines, one frame —
      `event_loop::tests::pace::ten_wheel_events_in_one_turn_scroll_ten_steps_in_one_frame`. See NOTES: applied per event
      in `update`, drawn once; not summed inside `update`.
- [x] J1 / J3 — `scripts/m8-pace.sh` → `docs/perf/m8-pace.txt` (release, HEAD d127565a, mains power):
      `J1 Unicode 40MB/s tail 1 ms, free 1 ms`; `J1 Pixel (Kitty+deflate) 40MB/s 29 ms, free 8 ms` — all pass;
      `J3 g_after_idle 13.2 ms` pass. Tests: `held_key::held_j_stops_on_release_{unicode_40mb,unicode_free,pixel_deflate,
      pixel_deflate_free}`, `frames::after_idle::g_after_idle`. Row verdicts in the file: **1089 still open**,
      **1095 stays a question** (NOTES).
- [x] Silent terminal → no fence written, frames ≥ 16 ms apart —
      `event_loop::tests::pace::a_silent_terminal_gets_no_fence_and_frames_at_least_16_ms_apart`; detection:
      `detect::tests::a_terminal_silent_on_device_status_is_never_fenced`; pacer:
      `pace::tests::a_silent_terminal_is_capped_from_the_start_and_never_fenced`.
- [x] Review m8-01 Major 1: `queued` counts keys behind a frame's first job wake —
      `trace::tests::keys_waiting_behind_a_job_answer_count_as_queued` (failed before the fix: `queued=0`; passes after).
- [x] Review m8-01 Major 2: no `too_many_arguments` / `too_many_lines` allow left in `event_loop/` (`grep -rn
      "too_many" crates/graft/src/event_loop/` empty); `running::Screen { terminal, graphics }` takes `pump_with`/`frames`
      to 7 args; `startup::run` split into `listen`, `outside`, `wind_down`. Verified by the gate
      `cargo clippy --workspace --all-targets -- -D warnings` (clean) and the 5161-test nextest run.

## Touches
- `event_loop::pump`, `joins_frame` (turn window kept), `ReplyWaiter` / reply routing, input decoder, graphics writes
  (`event_loop/graphics.rs`), `DrainSink`, `held_key` and `pixel_scroll` tests.
- Capability probe (startup budget §4.4: the DSR rides the same round; no new round).
- tmux path: tmux answers DSR itself — note it; the passthrough re-probe unchanged.
- `TESTING.md` held-key rows; `docs/perf/`; DESIGN §4.7 (already written); MAP.md of graft-term, graft-core, graft.
- G-trace's `FrameRecord` (`fence_us`).

## Scout questions
- Is every frame (text + Kitty/iTerm2/Sixel commands) inside DEC 2026 today? file:line of begin/end.
- How do terminal replies reach the loop mid-session (`ReplyWaiter` → `Wake`)? Is `CSI 0 n` routed as a reply or would
  it reach the key decoder?
- Wheel: step per event in the graph; is a turn's wheel applied per event or summed?
- Does `pump` block in `write` on a full pty?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens. If the scout finds DEC 2026 not wrapping image commands, that fix stays here.

## NOTES (builder, 2026-10-02)
- **Compacted once** mid-ticket. Nothing remains open from the ticket's scope except the wheel item below.
- Scouts: `scouts/frame-loop.md` (given) and `scouts/pace-harness.md` (mine, harness).
- Deviations from Signatures: `Pacer::answered` returns `Option<Duration>` (feeds `fence_us`); added `fenced`,
  `owed`, `forget`; `sent_at` is a `VecDeque` (a timeout can leave two fences owed, answered oldest first).
  `Wakes` gains required `next_wake_by(deadline) -> Waited`. `pump_with` now takes a `Screen` (Major 2).
- Wheel: **not summed inside `update`** — that is `update/mouse.rs`, held by ticket 13. The observable (one frame,
  10× step) holds because wheel wakes join one turn. If per-event `update` cost matters, the change is
  `update::mouse::wheel` taking a notch count; blocked on ticket 13's files.
- Row 1089 (Kitty without deflate): still open, cause is bytes, not write-ahead. Raw at 40 MB/s: `tail 2134 ms`,
  `frames_while_held=0` — one raw frame is ~28 MB (`bytes_per_key` 298k), and the ticked `DrainSink` moves at most
  16 KiB per 1 ms tick (≤ 16 MB/s whatever the rate), so one frame takes ~2 s to drain. At a free drain raw tails in
  55 ms but fails `bytes_per_frame_held<=512KiB`. The three red `held_key` tests are all raw Kitty (the known gap):
  `held_j_stops_on_release_pixel`, `held_j_stops_on_release_pixel_free`, `j_then_journal_opens_at_once_pixel`. Pacing cannot shrink a frame; that is G-pixel-mem / 46d.
- Row 1095: stays a question. The fixture tails pass; his feel in Ghostty / VS Code / Zed is his to judge.
- Harness finding: the ticked drain's "40 MB/s" is ≤ 16 MB/s in fact (pipe/tick). Not changed: it would move
  every earlier reading. Noted in TESTING.md.
- `g_after_idle` now runs fenced with an in-memory sink that answers at once (his terminals answer DSR). Unfenced,
  the 16 ms cap delays a frame that follows within 16 ms (measured 24–36 ms before the change). Baseline at the
  parent commit, same day: 12.5–14.6 ms (3 runs); this branch 10.8–13.4 ms (5 runs).
- Unverified: tmux (answers DSR itself — documented on `answers_dsr`); a fence answer reaching `$EDITOR` (mitigated
  by settling owed fences before a handover, no pty test); feel in his terminals.
- Test change: `the_ui_thread_never_waits_on_a_job` now allows `recv_timeout` only as `self.receiver.recv_timeout`
  (the wake channel, used by `next_wake_by`), not on any job.

## Review fixes (builder, 2026-10-02; `docs/reviews/m8-02-pace.md`)
- **M1, cause fixed.** `Pacer::answered` (`event_loop/pace.rs`) now ends the run of timeouts on *any* answer, not only
  on catching up; "three timeouts in a row" = three with no answer between (ADR-0054 wording holds, no spec change).
  A terminal answering 600 ms late stays fenced, one frame per timeout. Tests:
  `pace::tests::a_terminal_answering_every_fence_late_stays_fenced` (reproduced the review's 0/500/1000/1500/1516…
  before the fix) and, through the loop, `tests::pace::a_terminal_answering_every_fence_600_ms_late_stays_fenced`.
  Module doc corrected. Cost to him: a terminal slower than 500 ms keeps up to ~delay/500 + 1 frames in flight
  instead of falling to 16 ms frames.
- **M2, tests added through the real loop** on a virtual timeline (`tests/pace.rs` `Line`: keys at instants, each
  written fence answered after a set delay or never, `Wakes::now` = the line's clock; no sleeping, no seam needed):
  `a_handover_takes_the_owed_fence_answer_first_and_holds_what_came_meanwhile` (order: Ctrl+Z, j held, fence@100,
  handover@100, draw at once, then j), `a_handover_with_no_answer_coming_goes_ahead_at_the_fence_timeout` (handover
  at exactly 500 ms), `a_fenced_terminal_that_never_answers_is_drawn_per_timeout_then_unfenced` (frames at
  0/500/1000/1500 fenced, then unfenced ≥ 16 ms apart). Checked red: removing `settle_fences` fails both handover
  tests; reverting M1 fails the 600 ms one. The handover runs with `Outside::default()` (refused in words), so the
  real `$EDITOR` path past `carry_out` is still not exercised.
- **M3, split.** `running.rs` 791 → 201 lines: `running/turn.rs` (389, `Run`; `turn` split into `idle`/`paced`, `take`
  into `fence_answered`/`terminal_changed`/`input`, the dead `Wake::Fence` arm now just the exhaustive one),
  `running/carry.rs` (171), `running/frame.rs` (126). `mux.rs` 458 → 389: fence slot, lease and routing in
  `input/mux/fence.rs` (86). Child modules, so `event_loop/mod.rs` and `input/mod.rs` are untouched; old paths
  re-exported. Behaviour unchanged: the whole suite is the check. `startup.rs` (564) not split — outside this file set.
- **Paused at** (2026-10-02, conductor's request, CPU). Done: M1, M2 (committed `47fbbf96`), M3 split (this WIP
  commit). fmt ok; workspace clippy `-D warnings` ok. Workspace nextest: 4457/5287 run, 4452 passed, **5 failed**
  (only one name captured by my filter: `graft::it remote_by_key::capture_fix_plain_box_after_rebase`; unknown
  whether pre-existing or from this diff — the split moves no behaviour, but not verified). In flight: nothing.
  Next: rerun `cargo nextest run --workspace --no-fail-fast` capturing every FAIL name; check each against `main`
  (8cd1e93f); then `cargo test --doc`; then final commit and report.

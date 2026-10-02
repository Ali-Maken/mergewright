# G-pace — the loop never writes ahead of the terminal

Status: ready
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
- [ ] `Pacer` unit tests: answered → may draw; unanswered < 500 ms → may not; 500 ms → may; 3 misses → unfenced and
      `may_draw` false within 16 ms of the last frame; an answer after give-up does not re-enable (tests named at tick).
- [ ] `fence_answers` counts `CSI 0 n` amid other replies and split across reads (test).
- [ ] A `CSI 0 n` reaching the input decoder is never a key (test: no `Msg` for it).
- [ ] Frame bytes: every frame starts `SYNC_BEGIN`, ends `SYNC_END` then `CSI 5 n` when fenced; image commands sit
      inside the pair (test over a Pixel fixture frame).
- [ ] Never more than one unanswered frame: a sink that answers only on demand sees ≤ 1 outstanding fence over 300 held
      `j` (test).
- [ ] Wheel: 10 wheel events in one turn → one scroll of 10× step, one frame (test).
- [ ] J1: `held_key` tails ≤ 100 ms after release, Unicode and Pixel, at 40 MB/s drain and free; J3 `g_after_idle`
      still ≤ 16 ms. Script `docs/perf/m8-pace.txt` (summary lines); row 1089/1095 verdicts stated there.
- [ ] Silent terminal (no DSR answer at startup) → no fence written, behaviour as before plus the 16 ms cap (test).

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

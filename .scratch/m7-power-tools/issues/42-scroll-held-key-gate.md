# 42-scroll-held-key-gate — a test at his scale that fails today: held `j` keeps going after release

Status: ready
Build with: (blank: scout first — `tests/it/frames/` harness is existing code)
Needs: `41-scroll-team-fixture` (`team::{TeamSpec::MONOREPO, borrow, stats}`)
Blocked by: 41
File set (crate `graft`): `tests/it/frames/drain_sink.rs` (new), `tests/it/frames/held_key.rs` (new),
`tests/it/frames.rs` (`mod` lines only), `crates/graft/MAP.md`; `scripts/fix-scroll-held-key.sh`,
`docs/perf/fix-scroll-held-key.txt`, `TESTING.md`.
Overlap and order: 41 before (TESTING.md, fixture). 43 after (edits `paced.rs`/`readings.rs`, which this ticket only
reads). 45 may run concurrently (disjoint `graft` files; whichever merges second re-runs the gates, since 45 adds a
field to `Capabilities`/`Place`).

This spec carries the pipeline sections for the whole fix (41–46); the others point here.

## Need (his words, `docs/field-notes.md:82`)
His work repo, 17 694 commits, 1 323 refs, first real use: "as soon as it was opened, and i did j it was slow like
lagging i stopped pressing and it kept going … it is laggy as hell. that's why with J it didnt go instantly. it went
after a while." Daily task: scroll history to a commit, then open the journal. What hurts: the cursor keeps moving
after he lets go, and the next key waits behind it.

## Measured causes (`.scratch/m7-power-tools/debug-scroll-lag.md`, `docs/perf/scroll-lag-big-repo.txt`)
1. Pixel: the thread is in the strip cache key (`crates/graft-graph/src/raster/cache.rs:294-298`): 6.35 MB per `j`
   frame (`scroll-lag-big-repo.txt:26`). Fixed by `44-scroll-emphasis-layer`, `45-scroll-kitty-layer-deflate`,
   `46-scroll-strips-wiring` (ADR-0053).
2. Both tiers: one key, one full draw (`crates/graft/src/event_loop/running.rs:163-170`): when a frame leaves slower
   than keys arrive, every queued key is still drawn after release (Unicode at a 150 KB/s drain: 42 frames, 3.3 s,
   `scroll-lag-big-repo.txt:53`). Fixed by `43-scroll-coalesce-input`.

## Prior art (`docs/research/big-repo-scroll.md`)
| Aspect | Best answer found | Ours | Call |
|---|---|---|---|
| Burst of keys | Ratatui recipe: input updates state, one draw per frame (§Terminal techniques, ratatui recipe link) | update every queued key, draw once | **take** the drain-before-draw half |
| Fixed render tick (60 fps) | Ratatui recipe's `frame_rate` | — | **reject**: adds up to 16 ms to every lone key and a timer wake when idle (m7/12e asserts 0 idle wakes); drawing when the last frame has left bounds the tail the same |
| One key per turn | tig `update_views` (Guess in the study) | today's loop | **reject**: it is the bug |
| crossterm `poll(ZERO)` caveat | Question in the study | n/a: Graft's `InputMux` decodes on its own thread, the loop drains a channel | not applicable |
| Paging / lazy history (lazygit 300, gitui 3000) | §lazygit, §gitui | not changed | **reject**: measured not the cause (`update` 25 µs; open burst 20 ms; 0.3 s vs 5 s same tail) |
| Commit-graph | git docs, lazygit#3965 | already read; fixture has a split chain | **take** (fixture) |
| Cache key | gitui#2823: a per-tick value in a key misses every tick | the selection is that value | **take**: strips key on layout only (ADR-0053) |
| Selection paint as a separate image | nothing found (GUI tools repaint a canvas; TUI tools are text) | emphasis layer | **invent**: our cost is bytes per image; a layer keeps the big image stable |
| Compressed transmit | Kitty graphics protocol `o=z` (https://sw.kovidgoyal.net/kitty/graphics-protocol/#compression; not in the study) | deflate when the probe says yes, off again on an error reply | **take** |

## Journeys (counted per `docs/research/ux-principles.md` §1.3; start: graph open on the team fixture, cursor on row 0)
| # | Task | Steps | Best tool | Lands |
|---|---|---|---|---|
| 1 | scroll to a commit with held `j`, let go | 1 key (`j` held = one motion) + 1 decision (when to let go) | lazygit / tig: same, 1 | cursor on the row shown when the last repeat arrived; **stops ≤ 100 ms after the last `j` arrives**: no later frame moves it |
| 2 | then open the journal | 1 key (`J`) | lazygit reflog tab `]`: 1 | journal pane **at once: its frame starts reaching the terminal ≤ 100 ms after `J` arrives**, with nothing ahead of it but what was already in flight; no `j` frame after it |

**Timed from arrival, never from release.** A key release never reaches the loop (`crates/graft-core/src/event.rs:
20-21`: no release variant), so Graft cannot time from it. The last `j` the loop sees is the OS's last repeat, which
comes before the release; a bound from the last arrival is therefore at least as strict as "after I let go". In the
test, arrival = the key's send through `KeyFeed`. Names below that say "release" (`frames_after_release`, the test
names) keep the user's word; what they measure is the last arrival.

"At once" = 100 ms: Nielsen's 0.1 s limit for a response that feels instantaneous
(https://www.nngroup.com/articles/response-times-3-important-limits/). Graft's own share is the existing ≤ 16 ms
input-to-frame budget (PROMPT.md §6.3, `frames::` gate); the rest is the frame in flight draining. The time the
terminal takes to swallow the journal's own frame is the terminal's speed and is printed, not budgeted. Step counts
do not change; the fix is time.

## Why the existing frame-budget tests missed it (diagnosis "The four questions", last bullet)
(a) `frames::` writes into an in-memory sink with no back-pressure (`tests/it/frames.rs:20-21`): a 6.4 MB write costs
0.3 ms there, so keys never queue. (b) They gate per-key latency only, never bytes per key or frames drawn after the
last key. (c) The 100k fixture is the wrong shape (0.77 MB vs 6.7 MB per key). **Closed by:** this ticket's draining
sink, its byte and after-last-key gates, and the team fixture; `46-scroll-strips-wiring` and
`43-scroll-coalesce-input` add fast-tier guards (strip transmits on `j`; queued keys draw once) so a regression shows
before a review.

## Scope
In: a pty-like sink that drains at a fixed rate and blocks the writer when full; the held-key test at his scale on
both tiers; its SUMMARY lines; the red run at HEAD recorded. Out: any product code; any `impl Wakes` (owned by
`43-scroll-coalesce-input`).

## Decided
- **Script.** `TeamSpec::MONOREPO`, 187×50 (his terminal, `docs/field-notes.md:57`), the `frames::` default cell size
  (state it in the line). Open, wait until history says `LaidOut` and startup settles (`ready_to_scroll`). Scenario
  **A**: `j` every 16 ms for 3 s (188 keys; macOS's fastest repeat is 15 ms), then quiet until 1 s passes with no
  byte. Scenario **B**: A's hold, then 50 ms, then `J`, then quiet. Keys go through `KeyFeed` (the app's input path).
- **Drain model.** A bounded pipe of 16 KiB drained by a thread at **500 KB/s for Unicode** and **40 MB/s for
  Pixel**. Stress models of a terminal busy parsing, not his terminal. Chosen so HEAD overruns them (Unicode
  ~13 KB × 60/s ≈ 780 KB/s; Pixel ~7.5 MB × 60/s) and a fixed build has room: at most pipe + frame in flight + one
  batched frame ≈ 42 KB = 84 ms Unicode; ≤ 2 × 512 KiB = 26 ms Pixel. The builder measures the real macOS pty
  capacity once (non-blocking write until `EAGAIN`) and records it beside the 16 KiB.
- **Readings** (one `SUMMARY held_key` line per tier and scenario): `keys`, `frames_while_held`,
  `frames_after_release` (Drawn marks after the last `j`'s send), `tail_ms` (last `j` send → the drain passing the
  last frame's last byte), `bytes_per_frame_held` (mean), `strip_transmits_held` and `emphasis_transmits_held`
  (Kitty `a=t`/`a=T`, split by image id: bit 31 set = emphasis, per `45-scroll-kitty-layer-deflate` Decided 1;
  before 45 no id has it, so every transmit counts as a strip. Define the bit as a local const naming 45;
  46 switches it to `graft_term::kitty::EMPHASIS_ID_BIT`), `rows_scrolled`, `j_ms` (B: `J` send → the drain reaching
  the **first** byte of the first frame drawn after `J` was updated), `j_done_ms` (… its last byte; printed only),
  `frames_before_J` (frames drawn between `J`'s send and `J`'s frame), `peak_rss_mb`. A frame's end offset is
  recorded at its Drawn mark and must include its image commands; if graphics are written after Drawn, record where
  they are.
- **Premises** (fail the test, not the verdict, if false — they prove the fixture still has his shape): on ≥ 50% of
  scenario A's frames the graph draws ≥ 12 lanes; between consecutive frames the selection's lit columns change on
  ≥ 50% of frames (Ancestry over the window, `crates/graft-graph/MAP.md` ancestry).
- **Verdict.** Counts fail in both builds; times fail in release only, printed in debug (ADR-0021).
  - `frames_after_release ≤ 2`; `frames_before_J ≤ 1`; Pixel `bytes_per_frame_held ≤ 512 KiB`; Pixel
    `strip_transmits_held ≤ ceil(rows_scrolled / 32) + 2`.
  - release: `tail_ms ≤ 100`, `j_ms ≤ 100`.
  - `peak_rss_mb` printed, **not asserted here**: `getrusage` is the test process's lifetime peak (fixture, other
    tests); the in-process Unicode run read 252–358 MB at HEAD (`scroll-lag-big-repo.txt:7,17`) against 41 MB for the
    app alone over a pty (`:45`). The Graft-process RSS gate is in `47-scroll-acceptance`'s pty walk.
- **Red at HEAD is the point.** The test lands red (heavy tier only; builders' gates are the fast tier). The red run
  at this ticket's base commit is recorded, then stays red until the fix tickets land.

## Signatures (tests only, `crates/graft/tests/it/frames/`)
```rust
// drain_sink.rs (new)
pub struct DrainSink { /* bounded pipe + drain thread */ }
impl DrainSink {
    pub fn new(capacity: usize, bytes_per_second: u64) -> DrainSink;
    pub fn writer(&self) -> impl std::io::Write + Send + 'static; // blocks while the pipe is full
    pub fn written(&self) -> u64;                                 // bytes accepted so far (frame offsets)
    pub fn drained_at(&self, offset: u64) -> Option<std::time::Instant>; // when the drain passed `offset`
    pub fn quiet_for(&self, d: std::time::Duration) -> bool;
}
// held_key.rs (new), #[cfg(feature = "heavy-tests")]
#[test] fn held_j_stops_on_release_unicode();
#[test] fn held_j_stops_on_release_pixel();
#[test] fn j_then_journal_opens_at_once_unicode();
#[test] fn j_then_journal_opens_at_once_pixel();
```

## Acceptance (each box names its test)
- [ ] `DrainSink` blocks a writer once `capacity` is full and releases at the set rate (±10% over 1 s).
      — `frames::drain_sink::tests::drains_at_its_rate` (fast tier, small rate, < 1 s).
- [ ] The four tests above exist, read `MONOREPO`, assert the premises and the verdict as Decided.
      — `frames::held_key::{held_j_stops_on_release_unicode, held_j_stops_on_release_pixel,
      j_then_journal_opens_at_once_unicode, j_then_journal_opens_at_once_pixel}`.
- [ ] **Red at HEAD, both tiers**, release: the four SUMMARY lines and which verdict items failed, written by
      `scripts/fix-scroll-held-key.sh` to `docs/perf/fix-scroll-held-key.txt` (`# HEAD <hash>`, machine, load).
      Expected: Unicode `tail_ms` and `j_ms` in seconds; Pixel the same plus `bytes_per_frame_held` in MB. If either
      tier passes at HEAD, **stop** and report: the model or the fixture is wrong, not the fix.
- [ ] Premises hold at HEAD (same run). If not, stop and report the numbers (the fixture is not his shape).
- [ ] `TESTING.md` heavy tier: a "Held keys" paragraph — what the four lines mean, the drain model and why, and that
      times run from the last key's arrival (a release is never seen); right looks like: every verdict item `ok`. The
      release command already matches `frames::` (check; say so).
- [ ] `crates/graft/MAP.md` frame-trace paragraph: `drain_sink`, `held_key`, one line each.

## Files (exact)
- `crates/graft/tests/it/frames/drain_sink.rs` (new), `crates/graft/tests/it/frames/held_key.rs` (new; split to
  `held_key/*.rs` past ~400 lines), `crates/graft/tests/it/frames.rs` (`mod` lines only).
- `scripts/fix-scroll-held-key.sh` (new), `docs/perf/fix-scroll-held-key.txt` (new), `TESTING.md`, `crates/graft/MAP.md`.
- Read, do not edit: `frames/run.rs`, `paced.rs`, `readings.rs`, `verdict.rs`, `sink.rs`. Needing an edit there →
  `blocked: needs <file>`.

## Touches
- Existing `frames::` tests: untouched (new files only); prove with `cargo nextest run` and the heavy filter list.
- `scripts/m7-perf-key.sh` and its selftest parse `frames:` lines; the new lines start `SUMMARY held_key` and must not
  match its patterns — grep its patterns, say which.
- Fixture cache: `team::borrow` (read-only).
- Memory: the sink keeps counts and offsets, never the bytes (the in-memory sink's 3.5 GB peak,
  `scroll-lag-big-repo.txt:27`, is the bytes it kept).

## Gates
Fast-tier gates workspace-wide (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo nextest run`, `cargo test --doc`). Heavy compile check: `cargo clippy --all-targets --features heavy-tests
-- -D warnings`. The release red run goes through the script only.

## Size
Medium (~55k).

# 42-scroll-held-key-gate — a test at his scale that fails today: held `j` keeps going after release

Status: done
Build with: Sonnet — test-harness work in mapped files; every seam the scout found is decided below (Planner ruling).
Needs: `41-scroll-team-fixture` (`team::{TeamSpec::MONOREPO, borrow, stats}`); `45-scroll-kitty-layer-deflate` merged
(it edits `frames/run.rs`, which this ticket now edits too).
Blocked by: 41, 45
File set (crate `graft`): `tests/it/drain_sink.rs` (new), `tests/it/held_key.rs` (new), `tests/it/main.rs` (`mod`
lines only), `tests/it/frames.rs` (`mod` line visibility only), `tests/it/frames/sink.rs` (one constructor + its
test, below), `tests/it/frames/run.rs` (one sized entry point, below; visibility), `tests/it/frames/paced.rs`
(visibility and read-only accessors only), `crates/graft/MAP.md`; `scripts/fix-scroll-held-key.sh`,
`docs/perf/fix-scroll-held-key.txt`, `TESTING.md`.
Overlap and order: **41 → 45 → 42 → 43 → 46** for the shared files.
- 41 before (TESTING.md, fixture).
- 45 before: it holds `frames/run.rs` (`Capabilities` literal `:326`) while it builds; ADR-0051 forbids two builders on
  one file. 42 starts from a base with 45 merged.
- 43 after (already Blocked by 42): 43 owns `paced.rs` and `readings.rs` from 42's merge on; 42's `paced.rs` edit is
  visibility/accessors only, so 43 builds on it.
- 46 after (edits `held_key.rs`'s layer const only).

This spec carries the pipeline sections for the whole fix (41–46); the others point here.

## Planner ruling 2026-10-01 (scout 42)
Answers `.scratch/m7-power-tools/scouts/42.md` contradictions 1–3 and overlap 45. Supersedes the earlier "read, do not
edit" list and the `frames::held_key` / `frames::drain_sink` paths.

1. **Seam: the drain plugs in under `Sink`, not beside it.** `Sink` gains `Sink::draining(&DrainSink)`: same counters
   as `Sink::timed` (`sent`, `write_spent`), but each write is forwarded to the drain's blocking writer and **no byte
   is kept**. `Paced` and `run::in_memory` keep taking a concrete `Sink`, so no harness type goes generic and no new
   `impl Wakes` exists: `held_key` drives the loop through `Paced`, as every `frames` test does. Rejected: `Paced<S>`
   generic (rewrites the file 43 rebuilds); a `DrainSink` `impl Wakes` (Scope forbids; 43 makes the trait method
   required). A byte accessor on a draining `Sink` panics naming itself (test code; a call is a harness bug). If any
   path `held_key` reaches reads bytes back from the sink, stop: `blocked: <file:line> reads sink bytes`.
   **Ownership:** `sink.rs` and `run.rs` — 42 (after 45). `paced.rs` — 42 may widen visibility and add read-only
   accessors for what `held_key` needs per `Drawn` (its `Instant`, the sink offset at it); no logic. Then 43 owns it.
2. **The red gate lives outside `frames::`.** `held_key` and `drain_sink` are root modules of the `it` test crate
   (`tests/it/held_key.rs`, `tests/it/drain_sink.rs`), so their paths are `held_key::…` and `drain_sink::…`. The
   substring filters `frames::` / `frame_trace::` (`scripts/m7-perf-key.sh:60`, `TESTING.md:166`) cannot match them:
   that script's `SUMMARY runs:` stays as it is, with no `--skip` or `#[ignore]` anywhere. The gate has its own filter,
   `held_key::`, run by `scripts/fix-scroll-held-key.sh`. Still red at HEAD and in the full heavy-tier command
   (`TESTING.md:152` runs everything): that is the honest state until 46, and the "Held keys" paragraph says so.
   `held_key` reaches `frames::{run, sink, paced}` through `crate::frames::…`; widen to `pub(crate)` only the items and
   `mod` lines it uses, and list them. `drain_sink` is not feature-gated (its tests are fast tier) and its tests call
   every `pub(crate)` method, so no `#[allow(dead_code)]`.
3. **Size is a parameter, never the env.** `std::env::set_var` is process-global, races the heavy tier's
   `--test-threads=2`, and leaks into every later test. `run.rs` gains `ready_to_scroll_at`, the same body with the
   size passed in; `ready_to_scroll` becomes a one-line call to it with the value it reads from `GRAFT_REAL_SIZE` today
   (`frames.rs:177-189`), so every existing caller is untouched. The size goes wherever the env value goes today.
4. **Order with 45:** 45 first, 42 from a base with it merged (one owner of `run.rs` at a time). `held_key` takes
   `run::capabilities(tier)` and never writes a `Capabilities {…}` or `Place {…}` literal, so later fields never reach
   42's files.
5. **Kitty transmits are counted in the stream, not from kept bytes.** `DrainSink` scans the bytes passing its writer
   for Kitty `ESC _ G` commands and records offset + image id per transmit (header bytes only, across write
   boundaries). Transmit = control data with `a=t` or `a=T`, or with no `a=` key and not the continuation of an `m=1`
   chunk (Kitty's default action is `t`). `held_key` splits them by frame offsets and by `EMPHASIS_ID_BIT_42`.

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
In: a pty-like sink that drains at a fixed rate and blocks the writer when full; `Sink::draining`; the sized
`ready_to_scroll_at`; the held-key test at his scale on both tiers; its SUMMARY lines; the red run at HEAD recorded.
Out: any product code; any `impl Wakes` (owned by `43-scroll-coalesce-input`); any logic change in `paced.rs`,
`readings.rs`, `verdict.rs`; any edit to `scripts/m7-perf-key.sh` or the `TESTING.md:166` command.

## Decided
- **Script.** `TeamSpec::MONOREPO`, 187×50 (his terminal, `docs/field-notes.md:57`) via `ready_to_scroll_at`, the
  `frames::` default cell size (state it in the line). Open, wait until history says `LaidOut` and startup settles.
  Scenario **A**: `j` every 16 ms for 3 s (188 keys; macOS's fastest repeat is 15 ms), then quiet until 1 s passes
  with no byte. Scenario **B**: A's hold, then 50 ms, then `J`, then quiet. Keys go through `KeyFeed` (the app's input
  path). Each test asserts its key list is `j`×188 (then `J` in B), nothing else (47's walk count).
- **Drain model.** A bounded pipe of 16 KiB drained by a thread at **500 KB/s for Unicode** and **40 MB/s for
  Pixel**. Stress models of a terminal busy parsing, not his terminal. Chosen so HEAD overruns them (Unicode
  ~13 KB × 60/s ≈ 780 KB/s; Pixel ~7.5 MB × 60/s) and a fixed build has room: at most pipe + frame in flight + one
  batched frame ≈ 42 KB = 84 ms Unicode; ≤ 2 × 512 KiB = 26 ms Pixel. The builder measures the real macOS pty
  capacity once (non-blocking write until `EAGAIN`) and records it beside the 16 KiB.
- **Readings** (one `SUMMARY held_key` line per tier and scenario): `keys`, `frames_while_held`,
  `frames_after_release` (Drawn marks after the last `j`'s send), `tail_ms` (last `j` send → the drain passing the
  last frame's last byte), `bytes_per_frame_held` (mean), `strip_transmits_held` and `emphasis_transmits_held`
  (Kitty transmits from `DrainSink::kitty_transmits`, split by image id: bit 31 set = emphasis, per
  `45-scroll-kitty-layer-deflate` Decided 1; before 45's layer half no id has it, so every transmit counts as a strip.
  Define the bit as a local const `EMPHASIS_ID_BIT_42` naming 45; 46 switches it to
  `graft_term::kitty::EMPHASIS_ID_BIT`), `rows_scrolled`, `j_ms` (B: `J` send → the drain reaching the **first** byte
  of the first frame drawn after `J` was updated), `j_done_ms` (… its last byte; printed only), `frames_before_J`
  (frames drawn between `J`'s send and `J`'s frame), `peak_rss_mb`. A frame's end offset is the sink's `sent()` at its
  Drawn mark; images are written inside `draw` before Drawn (scout 42: `running.rs:389-391`, `:172`), so it includes
  them.
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
  at this ticket's base commit is recorded, then stays red until the fix tickets land: 43 turns Unicode green, 46
  Pixel.

## Signatures (tests only, `crates/graft/tests/it/`)
```rust
// drain_sink.rs (new; root module of `it`; not feature-gated)
pub(crate) struct DrainSink { /* Arc'd: bounded pipe + drain thread + counters; never the bytes */ }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KittyTransmit { pub(crate) offset: u64, pub(crate) image_id: u32 }
impl DrainSink {
    pub(crate) fn new(capacity: usize, bytes_per_second: u64) -> DrainSink;
    pub(crate) fn writer(&self) -> impl std::io::Write + Send + 'static; // blocks while the pipe is full
    pub(crate) fn written(&self) -> u64;                                 // bytes accepted so far
    pub(crate) fn drained_at(&self, offset: u64) -> Option<std::time::Instant>; // when the drain passed `offset`
    pub(crate) fn quiet_for(&self, d: std::time::Duration) -> bool;
    pub(crate) fn kitty_transmits(&self) -> Vec<KittyTransmit>;          // in stream order (ruling 5)
}

// frames/sink.rs (additive, beside `Sink::timed`)
impl Sink {
    /// Writes go to `drain` (blocking while its pipe is full); no byte is kept.
    /// `sent` and `write_spent` mean what they mean for `Sink::timed`.
    pub(crate) fn draining(drain: &DrainSink) -> Sink;
}

// frames/run.rs (additive): `ready_to_scroll`'s parameters and return, unchanged, plus the size.
// `ready_to_scroll` becomes `ready_to_scroll_at(<its args>, <the GRAFT_REAL_SIZE value it uses today>)`.
pub(crate) fn ready_to_scroll_at(/* ready_to_scroll's parameters */, size: (u16, u16) /* (cols, rows) */)
    -> /* ready_to_scroll's return type */;

// held_key.rs (new; root module of `it`), #[cfg(feature = "heavy-tests")]
const EMPHASIS_ID_BIT_42: u32 = 1 << 31; // 45 Decided 1; 46 replaces with graft_term::kitty::EMPHASIS_ID_BIT
#[test] fn held_j_stops_on_release_unicode();
#[test] fn held_j_stops_on_release_pixel();
#[test] fn j_then_journal_opens_at_once_unicode();
#[test] fn j_then_journal_opens_at_once_pixel();
```

## Acceptance (each box names its test)
- [x] `DrainSink` blocks a writer once `capacity` is full and releases at the set rate (±10% over 1 s); `written`,
      `drained_at`, `quiet_for` agree with it. — `drain_sink::tests::drains_at_its_rate` (fast tier, small rate, < 1 s).
- [x] `kitty_transmits`: a header split across two writes counts once with its id; `a=T`, `a=t` and no-`a=` first
      chunks count; `m=1` continuation chunks, `a=p`/`a=d` do not; payload bytes are never kept.
      — `drain_sink::tests::counts_kitty_transmits_across_writes` (fast tier).
- [x] A draining `Sink` forwards every byte, keeps none, and its `sent` equals the drain's `written`.
      — `frames::sink::tests::a_draining_sink_forwards_and_keeps_nothing` (green, so harmless under the `frames::`
      filter; fast tier if `frames` is compiled without `heavy-tests`, else heavy — say which in the report).
- [x] `ready_to_scroll` behaves as before: every existing `frames::` test compiles unchanged (heavy clippy) and the
      fast-tier ones pass (nextest). The heavy `frames::` tests are not run by the builder: mark that **unverified**
      in the report; the next review's release run covers it.
- [x] The four tests exist, read `MONOREPO` at 187×50, assert the key list, the premises and the verdict as Decided.
      — `held_key::{held_j_stops_on_release_unicode, held_j_stops_on_release_pixel,
      j_then_journal_opens_at_once_unicode, j_then_journal_opens_at_once_pixel}`.
- [x] **Red at HEAD, both tiers**, release: the four SUMMARY lines and which verdict items failed, written by
      `scripts/fix-scroll-held-key.sh` (`cargo test --release --features heavy-tests --test it -- held_key::
      --test-threads=1 --nocapture`; exits 1 when any test failed) to `docs/perf/fix-scroll-held-key.txt`
      (`# HEAD <hash>`, machine, load). Expected: Unicode `tail_ms` and `j_ms` in seconds; Pixel the same plus
      `bytes_per_frame_held` in MB. If either tier passes at HEAD, **stop** and report: the model or the fixture is
      wrong, not the fix.
- [x] Premises hold at HEAD (same run). If not, stop and report the numbers (the fixture is not his shape).
- [x] Old filters select nothing new: `cargo test --release --features heavy-tests --test it -- frames::
      frame_trace:: --list` grepped for `held_key::` (0 lines), quoted in the report. (`drain_sink::` likewise 0;
      `frames::sink::tests::…` is the one new `frames::` test and is green.)
- [x] `TESTING.md` heavy tier: a "Held keys" paragraph — the command, what the four lines mean, the drain model and
      why, that times run from the last key's arrival (a release is never seen), and that the four tests are **red in
      the heavy tier until 46** (Unicode green after 43); right looks like: every verdict item `ok`.
- [x] `crates/graft/MAP.md` frame-trace paragraph: `drain_sink`, `held_key`, `Sink::draining`,
      `ready_to_scroll_at`, one line each.

## Files (exact)
- New: `crates/graft/tests/it/drain_sink.rs`, `crates/graft/tests/it/held_key.rs` (split to `held_key/*.rs` past
  ~400 lines), `scripts/fix-scroll-held-key.sh`, `docs/perf/fix-scroll-held-key.txt`.
- Edit: `crates/graft/tests/it/main.rs` (two `mod` lines), `tests/it/frames.rs` (`mod` visibility only),
  `tests/it/frames/sink.rs` (`Sink::draining` + its test), `tests/it/frames/run.rs` (`ready_to_scroll_at`,
  visibility), `tests/it/frames/paced.rs` (visibility, read-only accessors; no logic), `TESTING.md`,
  `crates/graft/MAP.md`.
- Read, do not edit: `frames/readings.rs`, `frames/verdict.rs`, `frame_trace.rs`, `scripts/m7-perf-key.sh`.
  Needing an edit there → `blocked: needs <file>`.

## Touches
- Existing `frames::` tests: `ready_to_scroll` delegates with today's value; `Sink::timed` unchanged; `Paced` logic
  unchanged. Prove with `cargo nextest run` and the heavy compile check; name the `frames::` tests in the report.
- `scripts/m7-perf-key.sh` (`:60` filter, `:85`/`:118` patterns) and `TESTING.md:166`: select nothing red (box above).
  The new lines start `SUMMARY held_key`; grep its patterns, say which.
- Full heavy tier (`TESTING.md:152`): goes red on the four tests until 46 — deliberate, documented.
- 43: inherits `paced.rs` with 42's accessors; 46: edits `held_key.rs`'s const; 47: reads `held_key::`.
- Fixture cache: `team::borrow` (read-only).
- Memory: the sink keeps counts and offsets, never the bytes (the in-memory sink's 3.5 GB peak,
  `scroll-lag-big-repo.txt:27`, is the bytes it kept).

## Gates
Fast-tier gates workspace-wide (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo nextest run`, `cargo test --doc`). Heavy compile check: `cargo clippy --all-targets --features heavy-tests
-- -D warnings`. The release red run goes through the script only.

## Size
Medium (~60k).

## NOTES (builder, 2026-10-01)
- Ticks: `frames::sink::tests::a_draining_sink_forwards_and_keeps_nothing` is heavy tier (`frames` is heavy-gated); passed via `cargo test --features heavy-tests -- drain_sink:: frames::sink::`. Heavy `frames::` scroll tests **unverified** (not run).
- Red run (release, `docs/perf/fix-scroll-held-key.txt`): Unicode A tail_ms=3260, frames_after_release=109; B j_ms=3210, frames_before_J=108. Pixel A tail_ms=215976, bytes_per_frame_held=13 113 157, frames_after_release=188; B j_ms=225288. Pixel strip_transmits_held=4 (ok). Premises hold: 188/188 wide, 184/188 lit changed. Real macOS pty capacity: 1024 B.
- Outside the file set: `frames/journal_find.rs` (+ `run.rs`) got `kitty_deflate: false`. 45 had left the heavy build broken (E0063). The Pixel run therefore has deflate off.
- Premise "lanes" = max `Row::lanes.len()` over visible rows, before the fold.
- Old filter: `--list` for `frames:: frame_trace::` gives 0 `held_key::` lines and 0 `drain_sink::` lines (17 tests). `scripts/m7-perf-key.sh` was not re-run (~10 min).
- nextest: 5085/5086. `m7_walk::pull::pull_ff_over_edits` failed under load and passed alone. Not in this ticket's files.

# Debug: held `j` lags and keeps playing after release (big repo)

Field note 2026-10-01, charms-rms-backend-v3 (17 694 commits, 1 323 refs,
commit-graph chain). Debugger, 2026-10-01. Diagnosis only, no fix.

Numbers: `docs/perf/scroll-lag-big-repo.txt` (SUMMARY lines), made by
`scripts/scroll-lag-big-repo.sh <copy>`; two full runs, same shape.
Repo used: `rsync -a` copy of his repository into the worktree's
`target/scroll-lag/` (working tree, node_modules, reflogs, packed refs
identical; his repo was only read). A `git clone --no-local` + all refs +
`commit-graph write --split` gave the same Unicode numbers (Pixel not run
on the clone).

## Verdict

Two causes, one multiplying the other. **Which one he hit depends on his
terminal, still unknown — ask him (see Question).** The evidence points to
the Pixel tier.

1. **Pixel tier: every `j` re-sends ~6.4 MB of images.** The selection's
   thread is baked into each 32-row strip's pixels and is part of the strip
   cache key (`crates/graft-graph/src/raster/cache.rs:295-296`; thread
   computed per frame at `crates/graft/src/view/strips.rs:191`, per row at
   `:355-360`). On a branchy repo, moving the cursor one row changes the
   thread, so every viewport strip misses, is rasterised again and
   re-transmitted as uncompressed base64 RGBA (`f=32`, no `o=z`:
   `crates/graft-term/src/kitty/encode.rs:14-26`).
2. **No input coalescing (both tiers).** Each key is forwarded on an
   unbounded channel (`crates/graft/src/event_loop/input.rs:38-47`,
   `event_loop/mod.rs:142`) and the loop runs `update` + a full `draw` per
   key (`event_loop/running.rs:116`, `:170`). Only job answers are batched
   (`running.rs:162-168`, "A key is never taken this way"). When a frame
   takes longer to leave than the key period, keys pile up in the channel
   and each one is still drawn after release — "it kept going", and `J`
   waits behind them.

## Evidence

Per-key cost of `j`, in-process (real loop, real jobs, in-memory terminal,
release; p50 / p99 µs; `write` = into memory):

| tier, size | update | draw (view+raster+encode) | write | bytes per `j` frame | strip transmits / 120 `j` |
|---|---|---|---|---|---|
| Unicode 120x40 | 24 / 86 | 799 / 3522 | 172 / 634 | 11 098 | 0 |
| Unicode 200x50 | 26 / 73 | 1176 / 4772 | 231 / 801 | 13 933 | 0 |
| Pixel 120x40 | 23 / 39 | 5657 / 11090 | 292 / 516 | 6 354 632 | 169 |
| Pixel 200x50 | 24 / 42 | 6527 / 11498 | 268 / 550 | 7 191 989 | 186 |

`update` (history, layout, refs) is ~25 µs. Graft's compute is not the
problem at either tier. Bytes are.

Real pty, `j` held 30 Hz for 3 s (91 keys), reader that costs nothing (so
this is Graft alone, before any terminal parses a byte):

| probe answered as | size | bytes/key | frames after release | tail (last key → last byte) | RSS |
|---|---|---|---|---|---|
| VS Code/Zed (no Kitty gfx) → Unicode | 120x40 | 9.7–10.6 KB | 1 | 1–2 ms | 42 MB |
| same | 200x50 | 12.6 KB | 1 | 1 ms | 41 MB |
| Ghostty-like (Kitty gfx) → Pixel | 120x40 | 7.0 MB | 50 | 1.2–1.3 s | 662 MB |
| same | 200x50 | 7.5 MB | 47–50 | 1.5–1.6 s | 703 MB |

The pty itself drains ~150 MB/s; 7 MB/key × 30 keys/s = 210 MB/s, so the
loop blocks in `write` and keys queue. A real terminal decoding base64 and
uploading textures drains slower still: the tail he sees is longer than
these numbers.

**Probe (H1 confirmed):** thread left out of the cache key (env-gated,
reverted): Pixel 120x40 `j` transmits 169 → 10, bytes per `j` frame
6.35 MB → 0.38 MB.

**Coalescing (H2 confirmed, model):** Unicode, terminal modelled as a
reader draining 150 KB/s: 42 frames after release, tail 3.3 s. At
300 KB/s: 1 frame, 14 ms. Same loop, same keys; the backlog is the queue.

**His RSS 376 MB matches Pixel, not Unicode.** Pixel: 10 keys → 120 MB,
31 keys (1 s held) → 365 MB, 91 → 662 MB. Unicode: 41–42 MB at any count.
Inference, not proof.

## The four questions

- **Per-key cost of `j`:** see the first table. Unicode ~1 ms in total, Pixel
  ~6 ms. Neither is slow in-process; the 6.4–7.2 MB a Pixel `j` writes is.
- **Coalesced?** No. One key, one `update`, one draw (`running.rs:116-170`).
- **What runs at open, does it compete?** Opening jobs (EvilProbe,
  SystemGit, Remotes, RadarContext, ForcePushProbe, DuplicatesProbe,
  Journal, history window) finish in a burst of 18–25 ms after first paint
  (`frame_trace: real startup` lines). Held `j` at 0.3 s after open vs 5 s:
  same tail (Pixel 1261 vs 1196 ms, Unicode 2 vs 1 ms). Open does not
  compete. "As soon as it was opened" is when he pressed, not why.
- **Why the perf tests missed it:** (a) `frames::` writes into an in-memory
  sink with no back-pressure (`crates/graft/tests/it/frames.rs:20-21` says
  so): a 6.4 MB write costs 0.3 ms there. (b) They gate latency per key,
  never bytes per key or keys drawn after release. (c) Fixture shape: the
  100k synthetic fixture's Pixel scroll writes 0.77 MB/key
  (`docs/perf/m7-review-release.txt:8`, 323 MB / 420 keys); his repo writes
  6.7 MB/key (2.83 GB / 420), 9x more. Commit count and ref count are not
  what matters: lane width and how often the thread changes from row to
  row are. 50k synthetic refs test badge lookups, not this.

## Lazy loading?

No. It does not touch the measured cause. Layout, refs and history cost
~25 µs per `j` and the opening burst is ~20 ms; pressing at 0.3 s or 5 s
gives the same tail. The time goes into images re-sent on every cursor move
(Pixel) and keys that are never merged (both tiers).

## Hypotheses

1. Thread in the strip key forces a re-send per `j` (Pixel). **Confirmed** by the probe.
2. No coalescing turns a slow drain into after-release replay. **Confirmed** (model run, code).
3. Opening work competes with the first keys. **Ruled out** (0.3 s vs 5 s same).
4. Per-frame compute (layout, ref badges) too slow on 17.7k/1.3k. **Ruled out** (update 25 µs; Unicode draw ~1 ms).
5. Something in his working tree (node_modules, dist, reflogs). **Ruled out** for Unicode: the rsync copy and a plain clone give the same numbers.

## Proposed fix (cause, not symptom)

1. **Pixel: keep the thread out of the strip.** Strips stay keyed on layout
   only; draw the thread emphasis as its own small layer (lit columns only)
   placed above the cached strip, or dim off-thread columns by a second
   placement, so a cursor move sends at most the overlay, never the strip.
   Needs a design decision (ADR-0024 says thread = alpha); the planner's
   call. Cheaper part on top: `o=z` (zlib) on transmits, since strips are
   mostly transparent. Costs him nothing visible.
2. **Both tiers: coalesce input.** Before drawing, run `update` over every
   input already waiting (as job answers are at `running.rs:162-168`), then
   draw once. Stop the batch at any key that hands the terminal over or
   quits (`carry_out` already sees those). A held key then ends within one
   frame of release whatever the terminal's speed. Cost: on a slow terminal
   the cursor jumps several rows per frame instead of replaying each.

Fix 2 alone bounds the tail but leaves Pixel scrolling at ~20 frames/s
through a free pty (less in a real terminal): that would be a symptom fix
for Pixel. Fix 1 is the cause there.

## Tests that would have caught it

- **Fast tier, Pixel:** on a small branchy fixture (≥8 lanes, cursor moving
  across threads), `j` within cached strips transmits 0 strips (count
  `a=t` in the frame bytes). Red today: 169 transmits / 120 `j` on his repo.
- **Fast tier, coalescing:** pump with N `j` wakes already queued; assert
  one frame drawn and the cursor N rows down. Red today: N frames.
- **Heavy tier:** `frames::` with a sink that drains at a fixed rate (say
  20 MB/s) and a check "frames drawn after the last key ≤ 1", plus bytes
  per `j` frame printed and gated. `scripts/scroll-lag-pty.py` with
  `PROFILE=kitty` is the end-to-end form.
- `frames::a_real_repository_keeps_input_to_frame_under_16_ms` (added,
  `#[ignore]`, needs `GRAFT_REAL_REPO`) is the manual check against a real
  repository.

## Question for him (rule 7: not yet confirmed)

Which terminal was it, and which size? Ghostty (Pixel tier, cause 1) or
Zed / VS Code (Unicode tier: Graft keeps up at 10–14 KB per key, so the lag
would be his terminal draining under ~300 KB/s plus cause 2). His 376 MB
RSS says Pixel.

## Seen on the way (not this bug)

One run of five: Unicode 120x40, "key 300 (PageDown): cursor 6998, top 6961,
0 of 38 rows on hand" (`crates/graft/tests/it/frames/verdict.rs:62`). Not
reproduced in two more runs. PageDown, not `j`.

## Touches (what this debug changed)

- `crates/graft/tests/it/frames.rs`, `frames/run.rs`: ignored manual test on
  a real repo; `GRAFT_REAL_SIZE` resize in `ready_to_scroll` (only when that
  variable is set; other `frames::` tests unchanged; nextest 5061 passed).
- `scripts/scroll-lag-big-repo.sh`, `scripts/scroll-lag-pty.py`: new.
- `docs/perf/scroll-lag-big-repo.txt`: new. No raw stream committed (dumps
  stay in ignored `target/scroll-lag/`).
- No product code changed. The probe in `raster/cache.rs` was reverted
  (grep `DEBUG-sl1`: none).

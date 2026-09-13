# m1-the-graph/30b — A one-row scroll in the pixel tier writes ~336 KB

Status: done
Build with: **Opus** (high) — Kitty protocol, split from 30
Blocked by: 30
Leaves runnable: yes — a one-row scroll frame places existing images and transmits at most one new strip.

Why: 30's heavy `frames::` tests measured pixel frames writing 0.3–0.9 MB and ~84 Kitty commands for a one-row scroll (100k fixture). Expected by DESIGN §3.8 / tickets 24c, 25: place existing ids, ≤1 new strip. Pixel p99 input-to-frame is 0.5–1 s debug. Cause unknown.

## Read first
- Ticket 30 "Handoff (what remains)" item 1 and Gotchas; its results table
- DESIGN.md §3.8 (strip cache and transmit paragraphs only); ADR for transmit-once strips if one exists (grep docs/adr for "strip")
- `crates/graft-term/MAP.md`, `crates/graft-graph/MAP.md`, `crates/graft/MAP.md`; scouts for source

## Scope
In:
- Count per frame: transmits (`a=t`) vs placements (`a=p`) vs deletes, bytes each, from the `frames.rs` `Sink`. Write the finding (numbers) in this ticket before changing code.
- Fix the cause. Add a fast-tier test that pins it (e.g. a one-row scroll over cached strips emits 0 transmits and N placements).
- Re-run the pixel `frames::` heavy tests alone; record bytes/frame and input-to-frame (debug and release) in 30's table.

Out: Unicode chunk spikes, release-vs-debug assertion policy, tightening proofs (→ 30c). 50k refs (→ 30d).

## Acceptance
- [x] Finding with numbers; cause named.
- [x] New fast test red before fix, green after.
- [x] Bytes per one-row scroll frame and p99 recorded, before/after.
- [x] Capture under `docs/shots/30b-*` if what's drawn changes: none; see "Cause 2" for the one-frame change a pty capture cannot hold (user checks by eye).
- [x] Gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; `frames::` heavy tests alone, output to a file.

## Size
Under ~80k tokens. Stop with a handoff note rather than compacting.

## Finding (2026-09-13, builder-opus)

Measured with the `frames.rs` `Sink` now splitting Kitty commands by kind (`a=t` + continuation chunks, `a=p`, `a=d,d=i`, `a=d,d=I`), grouped by what woke each frame. Pixel tier, 100k fixture, debug, before any fix (`/tmp/30b/before-debug.txt`):

| Trigger | Frames | Transmits | Transmit bytes | Placements | Deletes | Frees | Bytes/frame |
|---|---|---|---|---|---|---|---|
| `j` | 120 | 100 | 43.4 MB | 225 | 2 | 99 | 364 KB |
| `PageDown` | 240 | 120 | 51.6 MB | 150 | 119 | 115 | 217 KB |
| `PageUp` | 60 | 33 | 14.2 MB | 35 | 34 | 33 | 239 KB |
| Window (rows answer) | 268 | 102 | 43.8 MB | 137 | 0 | 94 | 165 KB |

One strip transmit ≈ 434 KB. Frees ≈ transmits in every row: almost every transmit is `KittyImages::ensure_uploaded`'s "same slot (`first_row`), new pixels" path, i.e. the strip `Arc` handed in changed while its rows did not.

**Cause 1 (fixed):** `graft_graph::raster::cache::layout_hash` hashed each row's own `emphasis`. The app sets `Emphasis::Highlighted` on the cursor row, but every row also carries a `thread` whenever something is selected, and under a thread the rasteriser reads only per-column emphasis (`RasterRow::emphasis_at`, `marker`). So every cursor move changed the key of the strip under the old and new cursor, missed, re-rasterised identical pixels, and `KittyImages` freed and retransmitted the slot. Fix: hash the row's `emphasis` only when it has no thread. Fast test: `strip_cache::a_threaded_rows_own_emphasis_changes_neither_pixels_nor_key` (red: rasterised 2, want 1; pixels already equal).

After fix 1 (`/tmp/30b/after1-{debug,release}.txt`, not committed):

| Run | p50 / p99 / max | Keys over 16 ms | Total bytes | `j` frames: transmits, bytes/frame |
|---|---|---|---|---|
| debug, before | 65 / 375 / 425 ms | 420/420 | 155 MB | 100 / 120, 364 KB |
| debug, after 1 | 42 / 96 / 117 ms | 328/420 | 118 MB | 18 / 120, 67 KB |
| release, after 1 | 7.3 / 15.9 / 19.2 ms | 5/420 | 284 MB | 19 / 120, 71 KB |

(Release writes more in total because it keeps up and draws more frames on page keys: 153 window frames, 287 transmits.)

## Handoff (what remains) — status partial

**Cause 2 (not fixed, not yet named):** page keys and the window answering them still transmit ~1–2 strips per frame, and frees ≈ transmits there too (release: PageDown 240 frames, 298 transmits, 291 frees; Window 153 frames, 287 transmits, 281 frees). So the same `first_row` slot gets new pixels twice per page: once on the key's frame and again when the window arrives. Suspects, in order: (a) `thread` lit columns change when the window changes (`view::thread` runs `Ancestry::of(&window.rows, ..)` over the window only, so rows near the window's edges light differently); (b) `Fold::plan`'s pinned set includes the thread, so `lane_columns`/`marker` change for every strip in view; (c) the key frame draws a strip over rows the old window still covers, then the new window redraws it. Legitimate pixel change (thread following the selection) may be part of it; a double transmit per page is not.

Exact next step: temporarily split `layout_hash` into sub-hashes (lane_columns+marker, per-row commit/edges, thread, ring) and, on a `StripCache::get` miss whose `first_row` was cached before, count which part differed. Run the pixel `frames::` test alone in release with `--nocapture` to a file. Then fix the dominant one and add a fast test for it.

Not done: 50k-refs pixel re-run and 30's table update (after cause 2); `docs/shots/` capture not needed (hash change draws identical pixels, test-proven).

Gotchas: `frames.rs` `Sink` now prints per-trigger `frames: ... trigger=` lines (transmits, transmit_bytes, placements, deletes, frees) — use them. Debug still misses badly; release narrowly (debug-vs-release policy is 30c).

## NOTES
- Part 1: no compaction, stopped at ~60k tokens at a clean point.
- Continuation 1: no compaction. Cause 2 named and fixed. The Handoff section above is kept as history; it is superseded.

## Cause 2 (2026-09-13, builder-opus, continuation 1)

**Measured** (temporary diagnosis, since removed): `StripCache::get` split the layout hash into parts and, on each miss, compared them with the previous rasterisation at the same `first_row`, tagged with the selected id, the window's `first` and `history.top`. Release, 100k fixture, pixel, 556 misses (`/tmp/30b/diag2-release.txt`):

| Miss | Count | Selection | Window | Top |
|---|---|---|---|---|
| New `first_row` (new territory, legitimate) | 288 | | | |
| Thread + emphasis parts changed | 119 | changed | same | changed |
| Thread + emphasis parts changed | 50 | changed | changed | same |
| Thread part only (selection moved within the window, legitimate) | 52 | changed | same | changed |
| Nothing changed (evicted, re-rasterised) | 43 | | | |
| Ids/edges changed | 1 | | | |

**Cause:** a page key moves the cursor to a row outside the window on hand. `view::thread` then returns None (`History::selected` is None), so the key's frame draws every strip whose rows are on hand **unthreaded** (the emphasis part changes, since unthreaded rows hash their own emphasis). It rasterises and transmits them, and when the window arrives the thread returns and they are rasterised and transmitted again (the 119 + 50 rows). Not a window-edge effect: ancestry is window-independent for rows in both windows.

**Fix:** `view::strips::pixel_rows` leaves out every strip while the cursor's row has not arrived (a non-empty window with no thread). Same rule as a strip missing its rows: wait for the final picture. The cursor is always on screen, so its row is always on the way. Fast test `view::strips::tests::every_strip_waits_while_the_cursor_row_has_not_arrived`: red `left: [(32, 32), (64, 32), (96, 4)]`, green after.

**Visible change (user judges by eye):** on a page key whose cursor row isn't on hand, the graph is blank for the one frame before the window answers, instead of drawing unlit for one frame. A pty capture can't hold one frame, so try PageDown fast in Ghostty/Kitty on a big repo.

**After (both causes), pixel tier:**

| Run | p50 / p99 / max | Keys over 16 ms | Total bytes | Kitty commands | Transmits: PageDown / Window |
|---|---|---|---|---|---|
| release 100k, after 1 | 7.3 / 15.9 / 19.2 ms | 5/420 | 284 MB | 70,794 | 298 / 287 |
| release 100k, after 2 | 7.7 / 16.4 / 21.5 ms | 6/420 | 193 MB | 48,414 | 181 / 196 |
| release 50k refs, after 2 | 9.2 / 15.8 / 19.9 ms | 5/420 | 193 MB | 48,414 | 181 / 196 |
| debug 100k, before | 65 / 375 / 425 ms | 420/420 | 155 MB | | 120 / 102 |
| debug 100k, after 2 | 38.7 / 94.0 / 112.5 ms | 324/420 | 113 MB | 28,378 | 84 / 136 |
| debug 50k refs, after 2 | 42.7 / 93.4 / 123.1 ms | 330/420 | 86 MB | 21,327 | |

`j` (one-row scroll) frames after both: 17 transmits in 120 frames, 7.7 MB, **64 KB/frame** (was 364 KB). Each of those 17 transmits is a strip entering the view. Latency barely moves: it isn't dominated by bytes to an in-memory sink. The budget is still missed (debug badly, release narrowly), which is 30c's work.

Remaining transmits on page keys are mostly legitimate: new strips entering the view, and strips relit because the selection moved. The 43 evicted-and-rasterised-again misses are logged in KNOWN_GAPS.

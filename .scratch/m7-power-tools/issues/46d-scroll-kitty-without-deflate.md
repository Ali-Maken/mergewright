# scroll-kitty-without-deflate: Kitty images without compression, held `j` on a big repo

Status: parked — user chose ship M7 with this as a known gap (2026-10-01); gap M7/46-kitty-no-deflate, expiry M8. Research comes
before any design; there is no build spec yet.
Build with: left blank. The conductor sets it after step 2 below.
Needs: 46 merged; `46c-scroll-strips-fold-churn` merged (its fast-tier guard and the restated bound).
Blocked by: the user's answer; `46c-scroll-strips-fold-churn`

## Need
- **Who:** anyone on a Kitty-protocol terminal that rejects `o=z`, or any session whose first deflated image got an
  error reply (45 Decided 4 falls back to raw).
- **Not the user's daily path:** Ghostty inflates `o=z` (45 NOTES Step 0), and Zed runs the Unicode tier.
- **What hurts:** held `j` on MONOREPO at 187×50 with a 40 MB/s drain: A tail 2775 ms, B tail/j 1483/1436 ms
  (`docs/perf/fix-scroll-held-key.txt`, `# rev strips`). Budget 100 ms (42 Journeys).

## Why 46 couldn't fix it (46 ruling (b))
One raw layer over the graph column is about 5.8 MB, or 7.7 MB base64. At 40 MB/s that is about 190 ms, and any `j`
that moves the thread sends at least one emphasis. These are 46 NOTES 2's numbers, **unverified**: step 2 re-measures
them.

## Steps (each produces a file; the next refuses to start without it)
1. **`researcher`** writes `docs/research/kitty-without-deflate.md`, covering:
   - Which terminals implement Kitty graphics (`a=T`, `f=32`) but not `o=z`, with source links per terminal:
     WezTerm, Konsole, Warp, Rio, and any others found.
   - What raw throughput each one sustains.
   - **Exit:** if no terminal in common use lacks `o=z`, the conductor proposes won't-fix to the user. The KNOWN_GAPS
     row stays open with the reason and the runtime-fallback case.
2. **Measure, by script, into `docs/perf/scroll-kitty-without-deflate.txt`:** per held frame, the bytes in backdrops
   vs emphasis, and the emphasis's lit bounding box (rows × cells) against its full size. This decides the shape.
   Candidate shapes:
   - Crop to the lit box (44 ruling item 3).
   - One emphasis image per row band.
   - Send only the rows whose `Lit` changed.
3. **Planner, spec-only pass:** the chosen shape and its signatures. If it changes `Emphasised` or `StripCache`'s
   keys, it is a signature ticket (Fable).

## Acceptance (for the eventual build; restated in step 3's spec)
- [ ] `held_key::{held_j_stops_on_release_pixel, j_then_journal_opens_at_once_pixel}` (deflate off): every verdict
      item `ok` in release. Limits unchanged.
- [ ] The deflate-on, iTerm2 and Unicode lines are not worse than `# rev fold-churn`.
- [ ] The KNOWN_GAPS row is closed with the test names.

## Touches
- `graft-graph` `raster::{emphasis, cache}`, `graft` `event_loop::graphics`, `graft-term` `kitty`.
- 47's deflate-off box, and the pty walk's `PROFILE=kitty`.

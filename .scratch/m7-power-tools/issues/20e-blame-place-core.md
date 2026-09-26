# 20e blame-place-core — 20's build half (20 compacted in orientation)

Status: ready
Blocked by: none (pinned-base ruling appended to 20, "Ruling (planner): a pinned compare base and the trail").
Crates: graft + graft-ui (graft-ui for the hints and shallow Part B only, as 20 says).
Build with: Opus (unchanged from 20).
Scout: done (`scouts/blame-place.md`), plus 20's builder NOTES (:301), which already list every seam with file:line. **Don't re-orient: start from those NOTES.**
Split by: conductor, 2026-09-26, along 20's builder's seam (20-a). No new decision.

## Scope
The spec is `20-blame-place.md`. Build its "20-a" list from the NOTES: blame_from, `B`, Place/Trail/`GraphOverlay::Place`, place_key, trail_back, is_write, came_from, the blame job, mouse, the Esc rungs, find/n/N, w/i, the heatmap, the shallow ruling's Parts B and C, and the hint contexts. Also the pinned-base ruling.

Tick these boxes **in 20's file**, in place, each naming its test: **Open, Routing, Navigation, Keys, States, Hints, MAP.md**. Leave Walks and Captures to 20f.

Your context is the risk: 20 compacted once. Commit at each green seam. If you compact, say where you stopped.

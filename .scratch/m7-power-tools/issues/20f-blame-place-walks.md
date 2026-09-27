# 20f blame-place-walks — 20's walks and captures

Status: ready
Blocked by: 20g (same files), 20h (the pane bar loses `space fold` on a file row; the re-shoots and hint sweeps must see it — planner, 2026-09-27).
Crates: graft (tests, `docs/shots/`).
Build with: Opus (unchanged from 20).
Scout: none. Read 20's NOTES and 20e's report.
Split by: conductor, 2026-09-26, along 20's builder's seam (20-b). No new decision.

## Scope
From `20-blame-place.md`: the m7_walk journeys (plus the pinned-base walk from the ruling), the 7 captures + `blame-then-commit-with-base`, the pane-bar re-shoots, the hint sweep tests, and the shallow capture. Tick 20's **Walks** and **Captures** boxes in place, each naming its test. Captures ≤ 200 KB, each with its "Right:" line. They are judged by eye at the M7 stop.

The sweep form is 20h's, not this ticket's: the hint sweeps here run over every `PaneFocused` form, none left out.

# 22 — The Ghostty capture test skips the tall row the real frame applies

Status: ready · Build with: Sonnet · Blocked by: none
Source: KNOWN_GAPS ease/21-fix-scroll-rows-misalign; scout `.scratch/ease-pass/scouts/21q-fix-scroll-rows.md`.

## Need
`docs/shots/m7/fix-scroll-ghostty.png` and the Unicode grid beside it in `docs/shots/m7/fix-scroll-contact.md` don't line
up row for row, so the user can't compare them. Scout: `ghostty_shot.rs:186` calls `pixel_rows` directly without the
TallRow/make_room step the real frame applies (`event_loop/graphics.rs:215`); its pane height is `graph_pane_rows`
(`:210-211`), one more than `graph_rows` (`update/state/queries.rs:28-37`, which reserves the second line); the crop
`at-10..pane.end` (`:213`) takes an extra row 189. Not proven by a run.

## Scope
1. Red first: a test showing the shot path's rows differ from the real frame's rows for the same state. If it is green,
   the scout's premise is wrong — stop and report.
2. Make the shot draw through the same row path as the real frame (no copy of the logic). Re-shoot; PNG and grid agree
   row for row (or the caption says exactly why not, with the line). Also explain the cursor's pane row 44 vs 45.
3. Fix the stale lines: `scripts/47-scroll-acceptance.sh:291`, `scroll-lag-pty.py:34, 301-314`, `ghostty_shot.rs:9-12`.

## Acceptance
- [ ] Red-first test named; green after.
- [ ] Re-shot PNG ≤200 KB; caption and grid agree; user judges by eye, colour unverified.
- [ ] Stale lines fixed, each listed.
- [ ] Five gates workspace-wide; Touches: line.

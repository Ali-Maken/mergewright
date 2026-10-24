# 22 — The Ghostty capture test skips the tall row the real frame applies

Status: done · Build with: Sonnet · Blocked by: none
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
- [x] Red-first test `ghostty_shot::the_shot_shows_the_lines_the_real_frame_places` (heavy tier, `cargo test --release --features heavy-tests --test it ghostty_shot`): red at HEAD (PNG showed Row(189) where the real frame places the cursor row's second line), green after.
- [x] Re-shot PNG 1476 B; caption and grid agree (12 lines, pane lines 34..46, cursor line 44, second line 45; `ghostty_shot::fix_scroll_ghostty_png` compares the bytes; grid retaken by hand-run of step 5's pty call). User judges by eye, colour unverified.
- [x] Stale lines fixed: `47-scroll-acceptance.sh` header comment + `PANE_TOP=2` (was +1) + GRID_COLS; `scroll-lag-pty.py` docstring (:34) and `write_grid` (GRID_COLS, no-divider case); `ghostty_shot.rs` header (:9-12).
- [x] Five gates workspace-wide (counts in report); Touches: line below.

## NOTES
- No compaction. No scout beyond the named report (read, premise held: red-first test failed at HEAD).
- Shared path: `event_loop::placements` (new, pub; `Graphics::drawn` now calls it). `Room` made pub.
- Cursor 44 vs 45: not a frame difference. Pane starts on screen row 2 (title bar, blank line), old script added 1; grid cursor on screen 46 is pane line 44, the PNG's. Line 45 is the second line (`┃`).
- Split `tests/it/ghostty_shot/pixels.rs` to stay under 400 lines (no over_400 row raised).
- Merge note for ticket 20: this ticket edits `tests/it/ghostty_shot.rs` (composed, Shot, header, new test; stack/on_dark/crop moved to `ghostty_shot/pixels.rs`) and `MAP.md` (one event_loop line).
- Touches: `Graphics::drawn` (placements extracted, behaviour same: nextest incl. graphics tests green); `pieces_shot.rs` imports `ghostty_shot::{answered, on_dark}` (re-exported, compiles under heavy-tests clippy); captures: fix-scroll-ghostty.png, -release-unicode.txt, -contact.md; script 47 step 5; MAP.md. Not checked: running whole 47 script (perf run).

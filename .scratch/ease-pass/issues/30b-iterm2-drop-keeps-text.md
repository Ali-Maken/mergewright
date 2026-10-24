# 30b — iTerm2: the drop frame's text survives

Status: done
Blocked by: none (30 merged)
Build with: `builder-opus` (model sonnet, per the 2026-10-20 rule). Why: terminal protocol write order (ADR table: terminal protocols → builder-opus).

Scope is ticket 30's box 11, unchanged — read `.scratch/ease-pass/issues/30-gfx-failed-falls-back.md` (box at its line 98, its NOTES, and its stop condition). The red test body is in `.scratch/ease-pass/issues/30-iterm2-drop-test.rs.txt`.

Defect, with lines (builder 30): on iTerm2 the frame that drops to Unicode writes the graph glyphs, then `bands.hide` (`event_loop/graphics.rs:180`) writes `ECH` over the cells the last frame's bands covered (`mergewright-term/src/iterm2.rs:114-120`), so the column is erased after it is drawn (`event_loop/running/frame.rs:106`). Same path on `F3` from pixels to Unicode.

## Acceptance (tick each with its test, inline)
- [x] Ticket 30 box 11 test, red first on main then green. <- `event_loop::tests::gfx_failed::an_iterm2_drop_keeps_the_text_column`
- [x] `F3` pixel → Unicode on iTerm2 keeps the column. <- `gfx_failed::f3_off_iterm2_pixels_keeps_the_text_column` (red on old order: `F3's glyphs survive the band erase: 0`)
- [x] Kitty path unchanged: all `gfx_failed::*` pass, full nextest 6844/6844. <- the family
- [x] Touches listed and checked (see NOTES).

## NOTES

- Red seen on main: `an_iterm2_drop_keeps_the_text_column` panicked `the drop frame's glyphs survive its erase: 0`.
- Cause fix, not symptom: the band erase (`bands.hide`) was written after the text. Now `Graphics::erase_bands` runs in `draw_frame` before `terminal.draw`; `frame`'s None branch no longer hides bands; `bands.frame` still hides (now a no-op, covered already drained). No re-draw after erase. Byte order change: SYNC_BEGIN, erase (iTerm2 only, when bands were drawn), text, images; DESIGN doc 04 line 163 updated.
- Touches: only `bands.hide` callers are `iterm2.rs::frame` (internal, unchanged) and graphics.rs None branch (removed). Preview open/close over bands, tall row: the same erase-first order applies; covered by full nextest + walks (463, failed=0). Kitty untouched (`erase_bands` is a no-op with no bands). No docs/shots bytes changed.
- Cost: pixel frames on iTerm2 now emit the erase before the text instead of inside bands.frame after it; same bytes, moved.
- No compaction. No scout.

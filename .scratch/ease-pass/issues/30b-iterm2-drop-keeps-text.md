# 30b — iTerm2: the drop frame's text survives

Status: ready
Blocked by: none (30 merged)
Build with: `builder-opus` (model sonnet, per the 2026-10-20 rule). Why: terminal protocol write order (ADR table: terminal protocols → builder-opus).

Scope is ticket 30's box 11, unchanged — read `.scratch/ease-pass/issues/30-gfx-failed-falls-back.md` (box at its line 98, its NOTES, and its stop condition). The red test body is in `.scratch/ease-pass/issues/30-iterm2-drop-test.rs.txt`.

Defect, with lines (builder 30): on iTerm2 the frame that drops to Unicode writes the graph glyphs, then `bands.hide` (`event_loop/graphics.rs:180`) writes `ECH` over the cells the last frame's bands covered (`mergewright-term/src/iterm2.rs:114-120`), so the column is erased after it is drawn (`event_loop/running/frame.rs:106`). Same path on `F3` from pixels to Unicode.

## Acceptance (tick each with its test, inline)
- [ ] Ticket 30 box 11 test, in the tree, red first on main then green. <- name it here
- [ ] `F3` pixel → Unicode on iTerm2 keeps the column (same test family). <- name it here
- [ ] Kitty path unchanged: `event_loop::tests::gfx_failed::*` all pass. <- the family
- [ ] Touches listed (other `bands.hide` callers, preview open/close, tall row) and each checked.

## NOTES

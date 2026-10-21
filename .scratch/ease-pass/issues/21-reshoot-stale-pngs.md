# 21 — Two PNG captures no longer match what the app draws

Status: done · Build with: Sonnet · Blocked by: none
Source: 16 NOTES `16-fix-heavy-tier.md:31-32`; scout `.scratch/ease-pass/scouts/19b-held-bytes-and-pngs.md`.

## Need
`ghostty_shot::fix_scroll_ghostty_png` (`ghostty_shot.rs:77`: committed 10128 B, drawn 1720 B) and
`pieces_shot::emphasis_pieces_png` (`pieces_shot.rs:62`: 34406 vs 29756 B). Committed in ea4b9b36 (m8-05); output changed
since, likely ease-pass-07 (f2269602) / 13 (bcc58b40, 9450a77a) — unconfirmed.

## Scope
1. Decode both drawn PNGs (dimensions, distinct colours, non-background pixel count) and the committed ones. The 1720 B
   one may be blank or broken: if it is, stop — that is a bug, report the commit that broke it (bisect the test), don't re-shoot.
2. If both are real drawings: name the commit that changed each (bisect), confirm it was an intended visual change (ticket),
   re-shoot, update each "what right looks like" line and the contact sheet. The user judges by eye (rule 6); colour unverified (rule 5).
3. ≤200 KB per file.

## Acceptance
- [x] Decode figures for all four images written in NOTES. (decoded with a throwaway zlib/PNG script; no test)
- [x] Changing commit named per PNG (bisect result). (bisect = runs of the two shot tests at merge commits)
- [x] Re-shot (or stopped as bug), both tests green, named. (`ghostty_shot::fix_scroll_ghostty_png`, `pieces_shot::emphasis_pieces_png`: 2 passed after the re-shot)
- [x] Touches: line (contact sheet, captions, other shots from the same commits).

## NOTES (ease-21)
Decode (RGBA PNG, bg #1E1E2E):
| image | bytes | px | colours | non-bg px |
|---|---|---|---|---|
| ghostty committed (ea4b9b36) | 10128 | 850x407 | 203 | 34458 of 345950 (10.0%) |
| ghostty drawn/new | 1720 | 34x444 | 58 | 3812 of 15096 (25.3%) |
| pieces committed (ea4b9b36) | 34406 | 1190x1192 | 260 | 39720 of 1418480 (2.8%) |
| pieces drawn/new | 29756 | 408x1192 | 109 | 19426 of 486336 (4.0%) |

Both new images are real drawings (not blank): narrower because the strips are as wide as the lanes in view (2 lanes and 24 lanes' worth, not the 50 / 70 cap); the ghostty crop is 12 rows (was 11). Looked at the pieces PNG: lit lane, dots, dashed lanes drawn. Colour unverified (rule 5); the user judges by eye (rule 6).

Changing commit (shot tests run at first-parent merges, the heavy-tier build patched per checkout to compile; sanity: both pass at ea4b9b36, 74f5d13a (05), a00dc75b, e31a2d2e):
- First change, both PNGs: d6052f73 (merge ease-pass 06 fold app chain, 06 cont/06d/06e/06f): ghostty 6026 B, pieces 20031 B drawn. Inner commit not narrowed.
- Later steps (drawn bytes, ghostty/pieces): 9394aebb 06r 9841/21649; by 4a90c7e0 (06h) 1410/24016, unchanged through 953dc667/f62e5a61 (07) and c1cb3182; c1cb3182..f83d3241 (13: bcc58b40 roles, 9450a77a tall row) 1720/29756 = today. The 850->34 px width step is in the 06 chain (06b/06u lanes on screen, not bisected finer); intended per ticket 13 ("graph_columns sets the strips' width") and 06b.
- The scout's guess (07/13 only) was incomplete: the first change is 06.

Touches: `docs/shots/m7/fix-scroll-contact.md` (caption sizes and rows edited; checked by reading). TESTING.md:3188/3227 captions say "same rows as the Unicode grid" and name no sizes, left. **Open, not fixed (outside scope):** `docs/shots/m7/fix-scroll-release-unicode.txt` and the contact text still say screen rows 38-48 / history 178-188 for the Unicode grid, while the PNG is now rows 178-189: the Unicode grid is not re-taken here (its test is a pty run, `scripts/47-scroll-acceptance.sh`) and may be stale too, unverified. M8 contact sheets (`m8-contact-sheet-*`) name neither PNG (grep). ADR-0053, docs/log/m7.md, perf files cite the PNGs historically, left. Not compacted. No scout used (the existing scout report was read).

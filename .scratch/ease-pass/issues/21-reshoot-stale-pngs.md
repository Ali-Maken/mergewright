# 21 — Two PNG captures no longer match what the app draws

Status: ready · Build with: Sonnet · Blocked by: none
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
- [ ] Decode figures for all four images written in NOTES.
- [ ] Changing commit named per PNG (bisect result).
- [ ] Re-shot (or stopped as bug), both tests green, named.
- [ ] Touches: line (contact sheet, captions, other shots from the same commits).

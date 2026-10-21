# 20 — Held-key pixel frames write 1.8–4.6 MB each

Status: ready · Build with: debugger (Opus high) · Blocked by: none
Source: 16 NOTES `16-fix-heavy-tier.md:30`; scout `.scratch/ease-pass/scouts/19b-held-bytes-and-pngs.md`.

## Need
Holding `j` at the pixel tier (the user's daily view in Ghostty is Unicode, but pixel is shipped) writes 1.8–4.6 MB per
held frame (debug) against the test's 512 KiB (`held_key.rs:331`, 844c1e5b; not a design number). The test was never
genuinely green (`m8-settle.txt:7-9`: frames_while_held=0). Scout could not find the cause line.

## Scope
Find what a held pixel frame writes (image data re-sent vs placed by id, full redraw, role strokes from ease-pass-13 —
the scout's unconfirmed guess). Name the cause line. If the fix is at cause and needs no decision, fix it. If it is
"budget vs code" (the bytes are what the design asks for), stop and report with numbers both ways.

## Acceptance
- [ ] Cause named with file:line and a measurement (bytes per held frame, split by what they are).  <- test/script
- [ ] Either fixed at cause with the three held_key pixel tests green (name them), or a decision written for the conductor.
- [ ] Measurement from a script → `docs/perf/20-held-pixel-bytes.txt`.
- [ ] Touches: line (other screens drawing strips, captures, kitty/iTerm2 paths).

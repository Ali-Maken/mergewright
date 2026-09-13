# m1-the-graph/34 — Bug: commit tooltip shows only the subject

Status: ready
Build with: **Sonnet** (medium) — plain widget bug (17 was Sonnet)
Blocked by: none (run after 32, 33)
Leaves runnable: yes — tooltip shows subject, full body and parents.

Found by the user in Ghostty (2026-09-13, screenshot), debug build around 1508cdd, ASCII tier, window ~200×37.

## Repro
Hover commit `8219724` in this repo. Its message is a long subject + blank line + `Co-Authored-By: …` trailer; one parent `fc4afcd`.
Seen: the box shows only the subject, wrapped over 4 lines, then ~3 **empty** rows. No body, no parent line.
Expected (ticket 17 / KNOWN_GAPS row): subject, full body, parent shas. ("Contained in" stays an M2 gap.)

Suspects, unverified: box height computed from the subject only and the rest clipped; wrapped-line count vs. content row mismatch; trailer/body dropped before render. **Find the cause.** 17's pty capture claims the body showed — check why that capture didn't catch this (short subject? no trailer?).

## Read first
- Ticket 17, "Scope" and "Notes"
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`. Scouts for source.

## Scope
In:
- Root cause and fix. Long wrapped subject + body + trailers + parents all visible, or visibly truncated with an ellipsis if the box can't fit (never silently blank).
- Label parents so they read as parents (e.g. `parent fc4afcd`, `parents a1b2c3d e4f5a6b` for merges).

Out:
- "Contained in" (M2). Tooltip styling beyond the fix.

## Acceptance
- [ ] A test that fails before the fix, using a message shaped like `8219724` (subject wider than the box, blank line, trailer).
- [ ] Merge commit: both parents shown (test).
- [ ] Box taller than the space below the row still shows everything or an explicit truncation marker (test).
- [ ] pty capture `docs/shots/34-*` hovering `8219724`. **By eye (user)** unticked.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.

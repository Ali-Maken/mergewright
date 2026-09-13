# m1-the-graph/34 — Tooltip: show the whole commit, add author and date, pin on Enter

Status: ready
Build with: **Sonnet** (medium) — plain widget bug + small input change (17 was Sonnet)
Blocked by: none (run after 33)
Leaves runnable: yes — tooltip shows the whole commit; `Enter` pins it so text can be copied.

Found by the user in Ghostty (2026-09-13, screenshots), debug build around 1508cdd, ASCII tier, window ~200×37.

## Bug repro
Hover commit `8219724` in this repo. Its message is a long subject + blank line + `Co-Authored-By: …` trailer; one parent `fc4afcd`.
Seen: the box shows only the subject, wrapped over 4 lines, then ~3 **empty** rows. No body, no parent line.
Expected (ticket 17 / KNOWN_GAPS row): subject, full body, parent shas.

Suspects, unverified: box height computed from the subject only and the rest clipped; wrapped-line count vs. content row mismatch; trailer/body dropped before render. **Find the cause.** 17's pty capture claims the body showed — check why that capture didn't catch this (short subject? no trailer?).

## User decisions (2026-09-13)
The user wants to copy text (author, date, hash) out of the tooltip, but hover replaces it as soon as the pointer moves.
1. **Pin on Enter: yes.** A tooltip opened with `Enter` is pinned: pointer movement and hover do not replace or close it. `Esc` closes it (keeps its place first in 17's `Esc` order); `Enter` again closes it too. A hover tooltip (not pinned) behaves as today.
2. **Author and date in the tooltip: now** (not waiting for M2's detail pane). This adds to PROMPT.md §1.3's tooltip contents (full message, parents, containing branches) → write a short ADR recording both decisions.

## Read first
- Ticket 17, "Scope" and "Notes" (single `AppState.tooltip` slot shared by hover and `Enter`; `Esc` order)
- Ticket 10, notes on `.mailmap` (author names go through it)
- PROMPT.md line 134 (tooltip contents), line 216 (Shift+drag stays the terminal's selection)
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`. Scouts for source.

## Scope
In:
- Root cause of the missing body/parents, and the fix. Long wrapped subject + body + trailers + parents all visible, or visibly truncated with a marker if the box can't fit (never silently blank).
- Tooltip content, top to bottom: full hash (40 chars, selectable), author `Name <email>` (mailmap applied), date absolute with offset **and** relative (e.g. `2026-09-13 14:02 +0500 · 4h ago`), committer only if different from author, `parent fc4afcd` / `parents a1b2c3d e4f5a6b`, blank line, subject, body. Labels sanitised text like the rest.
- Pinned state: split 17's single slot into hover vs pinned (or equivalent) so hover can't overwrite a pinned tooltip. Decide what `j`/`k` and scrolling do to a pinned tooltip (stay on its commit, or close) and state the choice in the ticket.
- Pinned tooltip visibly different from hover (e.g. title `pinned · esc close`), per the status-field rule: state shown by name.
- Footer hint for `Enter` reads as pin (coordinate with 33's hint bar).

Out:
- "Contained in" (M2). Clipboard / copy action (`.` context actions, later). Tooltip styling beyond the above.

## Acceptance
- [ ] A test that fails before the fix, using a message shaped like `8219724` (subject wider than the box, blank line, trailer).
- [ ] Merge commit: both parents shown (test).
- [ ] Box taller than the space available still shows everything or an explicit truncation marker (test).
- [ ] Author (mailmap-applied), absolute + relative date and full hash present (test).
- [ ] Pinned: hover over other rows leaves it unchanged; `Esc` and `Enter` close it; hover tooltip still follows the pointer (tests).
- [ ] ADR written.
- [ ] pty capture `docs/shots/34-*`: hover `8219724`; `Enter` pin then pointer moved to another row, box unchanged.
- [ ] **By eye (user):** in Ghostty, `Enter` pins, then Shift+drag selects and copies the hash or author from the box. Unticked until the user tries it.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; stop at a clean seam with a handoff note.

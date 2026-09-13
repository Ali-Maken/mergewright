# m1-the-graph/48 — Relative dates round like git; one gallery stop spans minutes to years

Status: ready
Build with: **Sonnet** (medium) — verify against real git output, then match it; one fixture and one gallery stop
Blocked by: 46
Leaves runnable: yes — the date column reads the same unit and number `git log --date=relative` would, and the gallery has one stop where dates vary.

Why (user, 2026-09-14, after 43): "a, but narrowly: add one fixture whose commits span minutes to years, as a single gallery stop. Don't re-date all twenty repos and regenerate every snapshot for a column I can judge on one screen.
Before c: verify the rounding. You said Graft matches git log, but git rounds relative dates to nearest while you round down — 46 hours is "2 days ago" in git and "1d" in Graft. Check it against real git output. If they differ, that's a correctness fix (match git), not a format change, and c is moot."

Ticket 43 found the date column is committer time, floor-divided into m/h/d/mo/y. The user's point: a commit from yesterday evening reading "1d" when git says "2 days ago" is a small wrongness that erodes trust in every other number.

## Read first
- Ticket 43's verdict and evidence (`43-every-row-shows-1d.md`), captures `docs/shots/43-*.txt`
- crates/graft-ui/MAP.md (the date column), crates/graft-fixtures/MAP.md (fixture set), TESTING.md gallery section
- PROMPT.md / DESIGN.md: only the lines that define the date column's format

## Scope
In:
1. **Verify first, with real git output.** For a set of commit ages at and around every unit boundary (e.g. 89 s, 90 s, 89 min, 90 min, 35 h, 36 h, 46 h, 13 d, 14 d, 8 w, a year and a bit), compare `git log --date=relative` (real git binary, fixed `now` via a controlled clock or computed ages) with Graft's column. Record the table in this ticket.
2. **If they differ:** match git's rounding and unit thresholds (correctness fix, not a format change), keeping Graft's compact units (`m`/`h`/`d`/`w`/`mo`/`y` as the column already uses — no new units unless git's thresholds need one, and then say so). A failing test first; tests pin every boundary in the table. State cause vs symptom and the cost to the user.
3. **If they already match:** say so with the table; change nothing.
4. **One new fixture** whose commits span minutes to years (deterministic: fixed commit times relative to a fixed clock the gallery uses, so its snapshot doesn't drift), added as **one** gallery stop with a banner (`shows:` / `differs:`) and a TESTING.md entry saying concretely what right looks like.

Out:
- Re-dating the other fixtures or regenerating their snapshots.
- Changing the date format beyond matching git (option c is moot if rounding is fixed; if you think a format change is still needed, report it — don't do it).

## Acceptance
- [ ] Table of git vs Graft at every boundary, before and after, in this ticket.
- [ ] Date column agrees with `git log --date=relative` on every row of the table (units abbreviated).
- [ ] One new gallery stop shows dates from minutes to years; other fixtures' snapshots unchanged.
- [ ] Capture of the new stop straight to `docs/shots/48-*`; the user judges it by eye.
- [ ] Gates clean.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; if a second compaction is coming, stop at a clean seam with a handoff note.

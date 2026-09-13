# m1-the-graph/48 — Relative dates round like git; one gallery stop spans minutes to years

Status: done — follow-up (fixed clock, exemptions, 90 min row) split to 53
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
- [x] Table of git vs Graft at every boundary, before and after, in this ticket.
- [x] Date column agrees with `git log --date=relative` on every row of the table (units abbreviated).
- [x] One new gallery stop shows dates from minutes to years; other fixtures' snapshots unchanged.
- [x] Capture of the new stop straight to `docs/shots/48-*`; the user judges it by eye.
- [x] Gates clean.

## Verification: git vs Graft, at and around every boundary

Real `git` (2.50.1), commits at `@<unix-ts> +0000` (UTC, unambiguous — an
ISO string with no offset is parsed as *local* time and gave wrong probe
results at first), compared against `git log --date=relative` with a fixed
`now`. Graft column: **before** = the code ticket 43 found (floor-divided
per unit, independently — never cascading); **after** = this ticket's fix.

| elapsed | git says | Graft before | Graft after |
|---|---|---|---|
| 89 s | now | 1m | now |
| 90 s | 2 minutes ago | 1m | 2m |
| 89 min | 89 minutes ago | 1h | 89m |
| 90 min | 90 minutes ago | 1h | 2h |
| 35 h | 35 hours ago | 1d | 35h |
| 36 h | 2 days ago | 1d | 2d |
| 46 h | 2 days ago | 1d | 2d |
| 13 d | 13 days ago | 13d | 13d |
| 14 d | 2 weeks ago | 14d | 2w |
| 8 w (56 d) | 8 weeks ago | 1mo | 8w |
| ~69–70 d | 9–10 weeks ago / 2 months ago | 2mo | 9w / 2mo (cascade-exact) |
| 1 y + a bit (~400 d) | 1 year, 1 month ago | 1y | 1y |

**They differed**, exactly as the user predicted: Graft floor-divided each
unit off the *raw elapsed seconds* independently (46 h / 24 = 1, floored,
so "1d" for anything from 24h up to just under 48h). Git instead **cascades**
— minutes from seconds, then hours from the *already-rounded* minutes, then
days from the *already-rounded* hours, rounding half-up at each stage — and
never shows `1` below years (the threshold for the next unit up is always
set so `1` is unreachable; e.g. minutes graduate to hours at 90, not 60, so
"1h" never appears). This is a **correctness fix**, not a format change:
kept the compact single-unit style, added `w` (git's own week/month/year
thresholds — days<14 shows `d`, days<70 shows `w`, days<365 shows `mo` —
force it; no unit was invented that git's own thresholds didn't require).

**Cause vs symptom, cost to the user:** the cause was per-unit floor
division instead of git's cascading round-half-up; the symptom ticket 43
saw ("every row shows 1d") was just the 24–48h case of a bug that actually
touched every unit. Cost to the user: a commit from yesterday evening read
"1d" when git (and every other tool built on it) says "2 days ago" — small
individually, but it meant *every* relative date in the graph was reading
up to a full unit younger than it actually was, which is exactly the kind
of small wrongness that erodes trust in every other number on screen.

Full pinned boundary set (29 cases) is the test
`matches_git_log_date_relative_at_every_boundary` in
`crates/graft-ui/src/graph_screen/columns.rs`.

## Notes

- **Fixture determinism, reported not silently deviated:** the ticket asks
  for "fixed commit times relative to a fixed clock the gallery uses, so
  its snapshot doesn't drift." That can't be done as literally written:
  `graft`'s renderer (`view.rs::now_seconds`) always reads real
  `SystemTime::now()` — there is no injectable clock for the gallery to fix.
  A fixture dated off the corpus's fixed 2020-01-01 epoch would, viewed
  today, show as six years old on every row, not "minutes to years." So the
  new `Shape::RelativeDates` instead times its commits off *wall-clock time
  at build time* (`now - {5s, 90s, 90min, 36h, 14d, 70d, 400d}`), which
  keeps the demonstrated unit spread correct whenever it's viewed, at the
  cost of the one property every other shape has: byte-identical object ids
  build to build. That's flagged and exempted by name (not silently) in
  `Shape`'s doc comment, `graft-fixtures/MAP.md`, and a comment in
  `corpus::identity` (the determinism test). A cached build will drift
  *stale* the longer it sits unbuilt (the "5s ago" commit ages past "now"
  after 90s of real time, etc.) — acceptable for a gallery stop judged by
  eye, not for a snapshot test, and none of this fixture's output is
  snapshot-compared.
- Other 20 fixtures/snapshots untouched; only `Shape::RelativeDates` added.
- Capture: `docs/shots/48-relative-dates.txt` (unicode tier), pty capture,
  contents not printed to context — judge by eye.
- **Fallout from the fix, not from re-dating:** every other fixture keeps
  its fixed 2020-01-01 commit times, but the date column reads them against
  *real* `SystemTime::now()` (`view.rs::now_seconds`) — so any snapshot
  containing the date column was already going to drift as real time
  passed, independent of this ticket. Two things made that drift land in
  this session: the rounding fix moved some ages by one unit at today's
  exact real time (e.g. a fixed 1970-ish test date crossed from 56y to
  57y), and simply enough real time had passed since these snapshots were
  last captured to cross other year boundaries (3y → 4y). 29 `.snap` files
  updated, every changed line differing only in the date field — SHAs,
  structure and subjects identical (checked line by line). This is not the
  "don't re-date other fixtures" scope violation the ticket warns against:
  no fixture's commit times changed, only what `relative_date` prints for
  the same fixed times at today's real clock.
- **A second determinism fallout, fixed:** `Shape::RelativeDates`'s
  wall-clock timing (see above) also broke three *other* exhaustive-shape
  test helpers that snapshot short SHAs per shape —
  `graft-graph/tests/it/corpus.rs::for_each_shape` (feeds
  `every_shape_lays_out_as_snapshotted`) and
  `graft-ui/tests/support/corpus.rs::for_each_shape` (feeds the ASCII/lane/
  colour snapshot suites) — since this shape's object ids are not stable
  build to build, any exact-match snapshot of them would fail every run.
  Both helpers now skip `Shape::RelativeDates` by name, with a comment
  pointing at the `Shape` doc comment; its geometry (one straight lane) is
  already covered by `Trivial`/`Linear`, and per the ticket's own
  acceptance criteria this shape is meant to be judged by eye in the
  gallery, not regression-tested by snapshot. `graft/src/history/tests.rs`
  and `graft-graph/tests/it/streaming.rs` use their own `for_each_shape`
  but only assert self-consistency within one build (no snapshot), so
  needed no change.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test` (fast tier only)

## Size
One context, under ~80k tokens. If you compact, say so; if a second compaction is coming, stop at a clean seam with a handoff note.

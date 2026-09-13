# m1-the-graph/53 — Dates on a fixed clock: snapshots stop drifting, the exemptions go, the 90-minute row is settled

Status: done
Build with: **Sonnet** (medium) — one injected value threaded through the date path, a fixture re-dated off a constant, snapshots re-recorded once, one boundary checked against real git. Escalates to Opus on a second failure.
Needs: none. Touches `update::Harness` like `commit-graph-offer` does (one new field each): sequence this one first.
Blocked by: 48

Why (conductor review of ticket 48, 2026-09-14): (a) 29 snapshots embed the date column rendered against real `SystemTime::now()`, so they drift; (b) `Shape::RelativeDates` is dated off the wall clock and exempted by name from two exhaustive snapshot helpers and the identity check — a workaround where 48 asked for a fixed clock; (c) 48's table says git prints "90 minutes ago" at 90 min while Graft prints "2h": one of them is wrong.

## Read first
- Ticket 48: "Verification" table, "Notes" (all four bullets — they name every file this ticket touches)
- `crates/graft-ui/MAP.md` (`graph_screen::columns::relative_date`, `GraphEntry::when`), `crates/graft/MAP.md` (`update::Harness`, `Banner`, `view::graph_view`, `examples/gallery`), `crates/graft-fixtures/MAP.md` (`Shape`, the `RelativeDates` sentence)
- TESTING.md "Graph gallery" → "The stops" (the `relative-dates` entry only)
- git's own rule, so the probe has an expectation: `date.c` `show_date_relative`: `< 90 s` seconds · `(s+30)/60 < 90` minutes · `(m+30)/60 < 36` hours · `(h+12)/24 < 14` days · `< 70` weeks `(d+3)/7` · `< 365` months `(d+15)/30` · years; `tv_sec > now` → "in the future".

## Decisions (planner-fable)
1. **One clock, injected.** `SystemTime::now()` is called in exactly one place on the date path: `Clock::Wall`. Everything that formats a date takes `now_seconds: i64`. The harness (what a wrapper around `run` asks for) carries the clock, so the gallery and tests fix it and the binary uses the wall.
2. **The corpus has one viewing instant, `CORPUS_NOW`**, a constant in `graft-fixtures`, ≥ every corpus commit's committer time (`SkewedDates` has one a day ahead of its children — count it). `Shape::RelativeDates` is dated off `CORPUS_NOW − {5 s, 90 s, 90 min, 36 h, 14 d, 70 d, 400 d}`, so it builds byte-identical and, viewed at `CORPUS_NOW`, reads exactly `now · 2m · 2h · 2d · 2w · 2mo · 1y`. The gallery views corpus stops at `Fixed(CORPUS_NOW)` and real-repo stops (`graft-self`, `git-git`, `synthetic-100k`) at `Wall`.
3. **The 90-minute row:** by git's cascade 5400 s → 90 min → not `< 90` → `(90+30)/60 = 2` → "2 hours ago"; git can never print "90 minutes ago" (the last minute value is 89, at ≤ 5339 s). Expectation: the table's git column is wrong and Graft's `2h` is right. The probe settles it; the code changes only if real git disagrees.
4. **Future dates** (`then > now`, clock skew): git says "in the future"; Graft prints `future`, fitted like any other value. Pinned by a test; never `now`, never an age. (Minor; the user may prefer another word.)

## Scope
In:
- `graft-ui::graph_screen::columns::relative_date(then_seconds, now_seconds)` — pure, this shape (keep it if it already has it); `future` arm added; the 29-case boundary test gains `5339 → 89m`, `5340 → 2h`, `5400 → 2h`, and one future case.
- **Probe, real git (≥ 2.38), before the code:** commits at `@<now−5400> +0000`, `@<now−5340>`, `@<now−5339>`, `@<now+60>`; `git log --date=relative`. Write the four lines into this ticket. Correct ticket 48's table row in place with a one-line note ("corrected in <this ticket>"); if git prints "90 minutes ago" at 5400, the cascade in `relative_date` is wrong instead — fix it, test first.
- `graft::session::Clock` (`Wall` / `Fixed`), `Harness.clock`, `view::graph_view` takes `now_seconds`; `view.rs::now_seconds` deleted. `rg 'SystemTime::now' crates/` → one hit on the date path; any other hit (crash reports, job timing) listed in the report as not-date.
- `graft_fixtures::CORPUS_NOW`; `Shape::RelativeDates` re-dated off it; exemptions removed from `graft-graph/tests/it/corpus.rs::for_each_shape`, `graft-ui/tests/support/corpus.rs::for_each_shape`, `corpus::identity`; the `Shape` doc comment and MAP.md sentence rewritten to say the opposite (byte-identical, dated off `CORPUS_NOW`).
- Every snapshot suite that renders a date passes `CORPUS_NOW`; the 29 `.snap` files re-recorded **once**; `RelativeDates` gains its snapshots in both exhaustive suites (it is no longer skipped).
- Gallery: `tests/it/gallery.rs` `Stop::clock()`; the `relative-dates` banner's `shows:` line names the seven values; TESTING.md's entry says the column reads `now, 2m, 2h, 2d, 2w, 2mo, 1y` top to bottom, any day, any machine.

Out: re-dating any other fixture's commits; the date format beyond `future`; the ASCII/pixel tiers (text is shared).

## Signatures
```rust
// graft-fixtures (lib.rs)
pub const CORPUS_NOW: i64;   // unix seconds, UTC: the instant every corpus snapshot and gallery stop is viewed at; ≥ every corpus commit's committer time (doc says which commit sets the floor)

// graft-ui::graph_screen::columns
pub fn relative_date(then_seconds: i64, now_seconds: i64) -> String;   // "now" < 90 s; git's cascade; "future" when then > now

// graft::session
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Clock { #[default] Wall, Fixed(i64) }
impl Clock { pub fn now_seconds(self) -> i64; }   // the one SystemTime::now() on the date path

// graft::update
pub struct Harness { pub banner: Option<Banner>, pub hand_back: Vec<KeyCode>, pub clock: Clock }   // Default: Wall

// graft::view
pub fn graph_view(/* existing */, now_seconds: i64) -> GraphView;   // callers pass state.harness.clock.now_seconds()

// graft (tests/it/gallery.rs, the gallery's pure half)
impl Stop { pub fn clock(&self) -> Clock; }   // Fixed(CORPUS_NOW) for corpus shapes, Wall for real repositories
```
Callers: `view::view` (passes the clock's seconds), the gallery (`Harness { clock: stop.clock(), .. }`), every `.snap`-producing test (passes `CORPUS_NOW`), `Shape::RelativeDates`'s builder.

## Acceptance
- [x] Probe lines (four) in this ticket; 48's table row corrected; the boundary test covers 5339/5340/5400 and a future case and agrees with the probe.

  **Probe (real git 2.50.1, macOS, `/tmp/probe53`, commits at `@$((NOW-offset)) +0000`, `NOW` = the actual wall clock at probe time, `git log --date=relative --format='%ad %s'`):**
  ```
  offset=5400  -> "2 hours ago"
  offset=5340  -> "89 minutes ago"
  offset=5339  -> "89 minutes ago"
  offset=-60   -> "in the future"
  ```
  Confirms decision 3: git's minute value is `m = (s+30)/60` (integer division). At `s=5339` and `s=5340`, `m = 89` (`5369/60` and `5370/60` both truncate to 89), and `89 < 90` keeps it in the minute stage: "89 minutes ago". Only at `s=5400` does `m = (5430)/60 = 90`, which is `≥ 90` and steps to hours: `h = (90+30)/60 = 2` → "2 hours ago". So `m=90` — and the string "90 minutes ago" — never actually prints; the cascade jumps straight from 89 to the next stage. Ticket 48's table row was wrong; corrected in place there (see its Verification table) with a note pointing here. Graft's existing cascade (`relative_date`) already matched real git at 5339/5340 (`89m`) and 5400 (`2h`) — **no code change to the cascade itself**, only the future arm and the `(then, now)` parameter order were added/changed.

- [x] `cargo test` twice, a minute apart — n/a as literally stated (nothing left reads the wall clock on the date path); concretely: the date column's snapshots contain no value computed from the wall clock. Diff of every re-recorded `.snap` confirmed to touch only the date field:
  - `crates/graft/src/snapshots/graft__view__tests__renderer-{Auto,Unicode,Ascii,Pixel}.snap`: `57y` → `63y` only (the `history_of` test helper's near-epoch commits, now read against `Clock::Fixed(2_000_000_000)` instead of a drifting `SystemTime::now()`).
  - `crates/graft-graph/tests/it/snapshots/it__corpus__relative-dates.snap` (new — shape no longer skipped).
  - `crates/graft-ui/tests/snapshots/graph_column__relative-dates.snap` (new — shape no longer skipped).
- [x] `Shape::RelativeDates`: `corpus::identity` passes with it included (`cargo test -p graft-fixtures` green, no skip block remains); both `for_each_shape` helpers (`graft-graph`, `graft-ui`) iterate `Shape::ALL` with no skip.
- [x] `rg 'SystemTime::now' crates/ -t rust`: two hits — `crates/graft/src/session.rs` (`Clock::Wall`'s `now_seconds`, the one date-path read) and `crates/graft/src/crash.rs:253` (crash-report timestamp, not-date, out of scope).
- [x] Gallery capture: `docs/shots/53-relative-dates-fixed-clock.txt` (Unicode tier, 80×24, `Fixed(CORPUS_NOW)`); `when` column reads `now 2m 2h 2d 2w 2mo 1y` top to bottom. `docs/shots/48-relative-dates.txt` untouched. User judges by eye.
- [x] MAP.md for `graft-fixtures`, `graft-ui`, `graft`; TESTING.md entry.
- [x] Report: cause vs symptom — **cause** (not symptom): a wall-clock read (`SystemTime::now()`) sat directly on the snapshot-rendering path (`view.rs::now_seconds`, and `RelativeDates`' own builder), instead of every other fixture's fixed build-time clock. **Cost to the user:** snapshots that silently fail on a calendar boundary days or months after they were recorded — a CI run failing for no code reason — and one shape permanently exempted from the two checks (`for_each_shape`, `corpus::identity`) that catch every other shape's regressions, so bugs specific to `RelativeDates` had no test coverage at all.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`

## Size
One context, under ~80k tokens. Seam: (a) probe + `relative_date` + `Clock`/`Harness`/view; (b) fixture, exemptions, snapshots, gallery, capture. Stop at the seam with a handoff note rather than compacting.

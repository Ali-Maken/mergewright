# Review: M8 group "palette, landing and stop fixes"

Fixed hash `c893abaa` (main). Read-only, one round. Each merge diffed against its first parent:
12e `e23dc4ee`, 12d `df67c81a`, 12f `f23703e3` + `bb86918f` (post-hoc ticks), 13l A `badaf456`,
13m `990f809e`, 13l B `ae3142f2`.

Already filed, not re-filed: `M8/hangup-first-test-blind`, `M8/12f-find-rs-407`, `M8/launch-open-land-alike-flaky`.

## What was run, and what was not (rule 1)

- `cargo fmt --check`, workspace: **clean** (2.2 s).
- **Not run:** `cargo clippy` (plain and `--features heavy-tests`), `cargo nextest run`, `cargo test --doc`, the heavy
  tier and the release perf run. The conductor said builders were running and ruled out heavy and perf. At review
  time `ps` showed 3 cargo processes from other helpers, and the two-cargo-helper cap (memory, 2026-10-02) rules out a
  third full build. No test result in this review comes from my own run. Every claim that a test is red or green
  comes from the builder's NOTES, and I say so where it matters.
- Not measured: fast-suite time, the 5 slowest tests, peak memory. `target/` is **80 GB**. That is worth a
  `cargo clean`, but measuring it is not this group's job.
- Skill: `code-review`, done in one context. I have no sub-agent tool, so both axes are below, kept apart.

## Tick spot-check

| Ticket | Checked | Held | How |
|---|---|---|---|
| 12e | 7 (D1 ×5, D2, captures) | 7 | Read every named test (`update/palette/tests.rs:102,227,246,351,402`; `tests/it/palette_in_progress.rs:139`). Ran `git diff` for the banner-dir and `docs/shots` claims. |
| 12d | 7 (all) | 7 | Read `outside_repo.rs` (new tests and 08's body, only the doc changed) and `banner/tests/arrival_landing.rs`. Checked the `repo()` diff and `docs/shots` in the merge stat. `git grep 'lands on .HEAD'` gives MAP :103 and :273. |
| 12f | 7 (all) | **6** | `git grep graft_core::matcher` at base and at D4. `git diff` base..D3 for tests and shots. A line-set check that D3 is a move. The MAP greps. `wc -l` at `593f763f`: **the D3 tick fails** (Critical 1). |
| 13l | 5 (A1, A2, B1, B2 no-unsafe, C) | 5 | `wc -l` of the split files. `Drop` is 13 lines (`listener.rs:119-131`). `remote/tests.rs` is not in the merge. `git grep unsafe crates/graft`. `Cargo.lock` adds one line, no `[[package]]`. The design note cites `m8-look-polish` 9×. |
| 13l, not checked | 3 (B2 real hang-up, B2 seen red, B2 `ps`) | — | These need the heavy tier or a build, which were ruled out. Both "seen red" and `ps` changed the box's own criterion when they were ticked: red on the second test, not the first, and `ps` reading 6 with an explanation. Both tick texts say so, the conductor merged them, and the first already has a gap row. Not false ticks. |
| 13m | 0 | — | The ticket has **no Acceptance section**. Its regression test is named only in NOTES (Minor 7). |

**Total: 26 checked, 25 held, 1 false (Critical).**

## Critical

### C1. The 12f D3 tick reports `wc -l` numbers the D3 commit does not have (false tick)
- **Where:** `.scratch/m8-polish-and-ship/issues/12f-palette-split-and-matcher.md` D3 tick ("`wc -l` in NOTES —
  find.rs 407, one over") and NOTES ("find 407, input 264, listing 394, mod 377, run 82").
- **What is wrong:** at the D3 commit `593f763f`, `git show 593f763f:crates/graft/src/update/palette/<f>.rs | wc -l`
  gives **find 429, listing 406, mod 387, run 86, input 265**. No counting method I tried (all lines, non-blank,
  non-comment) gives the NOTES figures. The box's condition was "each production file ≤ ~400 (one over → NOTES names
  it and its seam)". In fact **two** files are over: `listing.rs` is unnamed, and `find.rs` is 29 over, not 7.
  These ticks were filled in after the conductor's check (`bb86918f`), and NOTES already admits an earlier
  unverified claim. This is the same failure again, one line lower.
- **Why it matters:** a tick is the evidence. Numbers the commit doesn't have are a false tick (CLAUDE.md: Critical,
  whatever its size). The gap row `M8/12f-find-rs-407` (`KNOWN_GAPS.md:1252`) carries the wrong number into 21. Its
  seam ("move `landed`, 34 lines") still brings `find.rs` under 400 (429 − 34 = 395), but by luck, not by the
  arithmetic it was filed with.
- **Fix:** correct the D3 tick and NOTES to the real `wc -l`, and name `listing.rs` and its seam. Re-word gap row
  :1252 to 429 lines (the conductor's file). Add a `listing.rs` row (Major 2).

## Major

### M1. ARCHITECTURE.md says seven loop-carried commands and that `not_a_job` is the full list; `not_a_job` has ten
- **Where:** `ARCHITECTURE.md:160-165` ("**Seven commands are the exception** … The full list is the `not_a_job`
  arms") and `:194-197` ("with `Retheme` they are the **only** commands carried out by the frame loop"). Against
  `crates/graft/src/jobs/runner.rs:1455` `CopyToClipboard`, `:1457` `SaveLayout`, `:1458` `FirstLaunchDone`,
  `:1460` `Retheme`, and `:1463-1469` the six hand-overs: ten `not_a_job` commands.
- **What is wrong:** when D4 wrote it (`67970ffe`) there were already nine (`CopyToClipboard` and `SaveLayout`
  predate it: `git log -S` gives `5ae38ac3`, `b2a528b8`). 18d added the tenth. The doc names the true list and then
  miscounts it. The "seven" came from the 12e spec (D4: "Six → seven"), and the tick checked the spec's word, not
  the code. 12e's own ruling said "state the reason the code gives … do not guess".
- **Why it matters:** PROMPT 6.1: ARCHITECTURE.md must match the code. A builder adding a loop-carried command reads
  "only these seven" and either misses the other three or believes they are jobs.
- **Fix:** drop the count. Say "the commands the loop carries out itself (the `not_a_job` arms in
  `jobs/runner.rs`): the six that hand the terminal over, plus `Retheme`, `CopyToClipboard`, `SaveLayout`,
  `FirstLaunchDone`". At :194-197, say the six *hand over*, without "only". A test that counts `not_a_job` arms
  against the doc is optional.

### M2. `update/palette/listing.rs` is over the size rule with no row (PROMPT 6.2)
- **Where:** `crates/graft/src/update/palette/listing.rs`: 406 lines at the split, **424** at `c893abaa`.
  `palette/mod.rs` is at 391 and still growing.
- **What is wrong:** the split's purpose was files of about 400 lines or fewer. Neither the tick (C1) nor any
  `KNOWN_GAPS.md` row names `listing.rs`. The umbrella row `M8/files-over-400-unrowed` (:1242) is an estimate by eye
  and predates this file.
- **Fix:** add a row. The obvious seam is the `IN PROGRESS`/`HERE`/`EVERYWHERE` assembly (`listing.rs:177-216`)
  apart from spelling and filter.

### M3. Doc comments left saying the old behaviour (PROMPT 6.2, knock-on not checked)
- `crates/graft/src/update/palette/mod.rs:7-10`: "`⏎` runs a row the way its key would — the default chord,
  straight to [`super::dispatch_key`]". Since 12e D1, an `IN PROGRESS` row goes to `banner::banner_press` instead
  (`palette/run.rs:32-40`). The module doc now describes the route that was the review's Major 1 bug.
- `crates/graft/src/event_loop/startup.rs:248-249`: `Arrival::on_head` is still documented as "Opened from the empty
  state". Since 12d, every repository arrival on a tty sets it (`places.rs:436` `on_head: true`), launch included.
  It is false only off a tty.
- **Why it matters:** these are the first lines a reader meets in each file, and both now state the behaviour the
  ticket removed.
- **Fix:** one sentence each: "an `IN PROGRESS` row runs as the banner's key (`banner_press`), any other through
  `dispatch_key`" and "every arrival on a terminal — launch, open, clone, init; off a tty (`run`) none".

## Minor

1. **13m's regression test can't tell "re-finds the commit" from "keeps the row number".**
   `crates/graft/src/history/tests.rs:1122` uses `Shape::Linear`, where the two orders number every row alike
   (the comment at :1170 says so). `promoted`'s fallback (`history/scroll/apply.rs:146-149`, cursor kept by number)
   also gives `cursor == 2`. The test does go red for the stale-anchor bug: per NOTES, the old code found the stale
   commit and gave 0. But it doesn't pin the new rule. Fix: add a case on a shape whose topological order differs
   from date order, asserting the commit, not the row.
2. **The design-note Lands row 12d asked for was not added.** 12d NOTES :109 asks the conductor for
   `Launch inside a repo | graph, cursor on HEAD`. `docs/design-notes/m8-polish-and-ship.md:187-194` has Open/init
   and Clone only. The `:116` journey start covers it in spirit; the Lands table doesn't.
3. **KNOWN_GAPS rows cite pre-split paths** (textual): `:1164` `update/palette.rs:589` → now
   `update/palette/listing.rs:86`; `:1170` `update/palette.rs:1258–1261` → `palette/run.rs:50-52`; `:1191`
   `http.rs:42,383` → `remote/http/mod.rs:46`, `listener.rs:99`; `:1200` `http.rs:383`, `handle :732-737` →
   `listener.rs:99`, `cgi.rs:32`. Row :1191 says "the builder updates them", but builders don't read
   `KNOWN_GAPS.md` (CLAUDE.md), so it falls to the conductor.
4. **`palette/mod.rs:47` has a bare `#[allow(clippy::wildcard_imports)]`.** Its four siblings give the reason
   inline (e.g. `palette/run.rs:3`). More broadly, every child does `use super::*`, and `mod.rs` re-exports every
   child (`:44-48`), so each file sees every other file's items unnamed. Where a function comes from can't be read
   off the file. There is a precedent (`update/place/history/open.rs:3`) and the ticket allowed it, so this is a
   judgement call. Explicit `use super::{…}` lists would make the five-file seam readable.
5. **`ArrivalWatch(Option<(bool, Option<Oid>)>)`** (`update/banner/stop.rs:305`) is a nested anonymous tuple. The
   `bool` is `on_working_tree`, but only `cursor()` (:313-320) says so. A two-field struct
   (`GraphCursor { on_working_tree, commit }`) would name it. It also sits in `banner/stop.rs`, though arrival
   landings aren't the banner's. The landing machinery lives there, so this is placement by convenience.
6. **Duplicated `next_id` plumbing.** `palette/run.rs:33-40` copies `state.local_jobs`, builds a closure and writes
   the copy back, the same dance as `update/keys.rs:136-138` and `keys/commit.rs:20-21`. A small
   `with_next_id(state, |state, next_id| …)` helper would remove the third copy. Possible Duplicated Code smell.
7. **13m is "Status: done" with no Acceptance boxes.** The gate is that every tick names its test, but there are no
   ticks to check. The debugger's NOTES are thorough: the test is named, seen red ×2 and then green, and a
   widened-gap probe was run 6/6 → 0/12. A one-box Acceptance naming
   `history::tests::a_move_between_the_two_answers_of_a_promotion_is_kept` would let the gate see it.

## Questions (behaviour I could not prove with a line; rule 7)

- **Q1 (13m residual edge).** `promoted` looks for the cursor's commit only in the answered window's rows
  (`history/scroll/apply.rs:138-143`). The worker lays that window out around the **ask-time** anchor
  (`history/scroll/query.rs:264-268` → `history/views/promotion.rs:56`). A cursor moved farther than one window from
  that anchor, without a request (over rows on hand), drops into the row-number fallback (`apply.rs:146-149`). After
  a reorder, that is a different commit with no sign of it. The 13m NOTES name this edge. On his 17,694-commit repo
  the gap between the two answers is longer than the 1 ms seen in the test. Is it reachable by holding `j`?
  Settling it needs a probe like 13m's widened gap, with more than a window of moves.
- **Q2 (12e).** `Continue` run from `?` with the graph focused goes through `banner_press` → `continue_pressed(…,
  ReturnTo::Banner, …)` (`update/banner/focus_keys.rs:165`). `Esc` from the finish summary then sets
  `banner_focus = true` (`update/finish/keys.rs:297-305`), so he returns to the banner, not the graph he started
  from. The graph's own `c` does the same (`update/keys/commit.rs:20-24`), so this may be intended (consistency).
  Is it?

## 13m: cause or symptom?

**Cause.** The defect was stored state going stale: `Promotion::Swapping { anchor }` was refreshed by `ask` and by
provisional windows, but not by moves that asked for nothing. `promoted` then snapped the cursor back to it (removed
code: `apply.rs` `reaim_promotion`, `query.rs` `*anchor = selected`). The fix deletes the stored copy and derives the
anchor when it is used (`apply.rs:114-120`), so nothing is left to go stale. It adds no timeout, retry or sleep, and
the test is unchanged. The NOTES rule out 13k's hypothesis with evidence: both natural failures were at setup, at
`pty.rs:217`, before any stop. The fix closes the stale-anchor path entirely. What remains is the separate
window-coverage edge in Q1, which the old code also had. Not verified by me: the "red ×2" and the widened-gap runs.
I read the code; I didn't run it.

## Knock-on checks

- 12e: `run` callers. `banner_key` behaviour: `banner_press` is called before the modal match
  (`focus_keys.rs:109-111`), and the moved arms are disjoint from Tab/Esc/⏎/`q`. Recent rows keep a Banner `ctx`:
  `everything` puts `in_progress` first and dedupes by id (`listing.rs:246-250`), and `recent_rows` matches by id
  (`mod.rs:206-211`), so a recent Abort still runs as the banner's. Held.
- 12d: `gives_way` is set only in `lands_on_head_at_open`. `finished_lands_on` and `relaunch_lands_on_head` set it
  `false` (`stop.rs:195-200`, `:261-266`). The harness prelude still holds keys while a landing is owed
  (`update/prelude.rs:109`), and the prelude is empty in the shipped binary (`prelude.rs:17`). Stale docs: Major 3.
- 12f: `git grep graft_core::matcher` is empty. graft-ui has no `matcher` hit, so the ui → ops edge is not
  introduced. `docs/shots`, `crates/graft/tests` and `update/palette/tests.rs` have empty diffs base..D3. D3's
  line-set check shows only visibility, path and `use` changes.
- 13l A: `Drop`'s steps keep the original order line for line (compared with `badaf456^1:…/http.rs:401-470`).
  `remote/tests.rs` is untouched.
- 13l B: `rustix` is a `[dev-dependencies]` line (`crates/graft/Cargo.toml:95`). `Cargo.lock` adds one dependency
  line and no package. The `it` root has `#![forbid(unsafe_code)]` (`tests/it/main.rs:12`). ADR-0015 and the
  workspace comment are untouched.
- 13m: the `Promotion::Swapping` readers all match `{ .. }`: `update/history.rs:93`, `view/rows.rs:412`,
  `update/history/tests.rs:50`, `history/promotion_tests.rs:200`. `git grep reaim_promotion` is empty.

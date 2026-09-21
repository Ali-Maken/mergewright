# every-capture-is-reproducible: a fixed clock in every file that writes to docs/shots

Status: done
Blocked by: none — 15 merged
Build with: **Opus.** The rule is simple; applying it without changing what the
captures *show* is not, and the M5 walk is about to depend on it.
Crates: `graft` (tests + examples), `graft-graph` (`tests/it/raster.rs`), and
`graft-fixtures` if the fixed-clock helper needs extending. No other builder is
running once 15 has merged — confirm with the conductor before starting.
Scout: **yes** — `Explore`, sonnet, ≤1 page. See §1.

## Why now, and why it is not a nice-to-have

Determinism is a principle we have applied in **two files and skipped in five**.
`Clock::Fixed` is in `tests/it/preview.rs`, `tests/it/context_menu.rs` and
`examples/gallery.rs` (the last for `Shape` fixtures only — real repositories
correctly use the wall clock, and that must stay).

It is **not** in `tests/it/conflict_interim.rs`, which produced this round's
banner captures. That file's own capture header admits it
(`conflict_interim.rs:1226`: *"the last pass because this fixture is built at
wall-clock"*), and its fixture builder `about_to_conflict_over`
(`conflict_interim.rs:45-46`) copies `Shape::Trivial` with no fixed clock.
Three rows of those captures read `now` off the wall clock, and ticket 21
recorded that the object ids — and sometimes the lane order of `main` and
`feature` — differ on every regeneration.

**Groups 10 and 11 are the M5 walk, and the walk builds the contact sheet.**
The milestone's most important evidence is about to be produced by the machinery
that does not have this rule. A capture that changes when the code has not is
weak evidence, and the user reviews the sheet by scrolling once.

## 1. The sweep — do this first, and report it as a table

Every file that **writes** to `docs/shots/`. The conductor's grep found five
test-side writers and three examples; **treat that list as a lead, not a
finding** (CLAUDE.md rule 7 — it is a grep, not a reading of each file):

| File | Fixed clock today? |
|---|---|
| `crates/graft/tests/it/preview.rs` | yes |
| `crates/graft/tests/it/context_menu.rs` | yes |
| `crates/graft/examples/gallery.rs` | yes for `Shape`, wall for real repos — **correct, leave it** |
| `crates/graft/tests/it/conflict_interim.rs` | **no** |
| `crates/graft/tests/it/captures.rs` | **no** (the `frame`/`write_shot` helpers ticket 21 extracted) |
| `crates/graft/tests/it/lane_cap.rs` | **no** (ticket 21's new capture) |
| `crates/graft-graph/tests/it/raster.rs` | **no** — check whether it has any time in it at all |
| `crates/graft/examples/m3_walk.rs` | **no** |
| `crates/graft/examples/m4_walk/` | **no** |

The scout's one page: for each writer, **what time enters the capture** — a
committer date, a relative age (`now`, `7y`), an object id derived from a
timestamp, or nothing at all — and which helper it would reach for.
A file with no time in its output needs no change; **say so rather than
changing it**.

## 2. The fix

`graft_fixtures::git::run_at` already stamps fixed dates — it is what made the
context-menu captures byte-identical across three runs (verified by the 12/13
review). Prefer extending what exists over inventing a second mechanism.

**The bar: the same code and the same command produce a byte-identical file.**
Prove it by regenerating each capture **three times** and diffing, the way the
12/13 review proved it for the context menu. A capture that still differs is a
finding, named, not a shrug.

**Do not change what a capture shows.** The dates and ids may change once, as a
consequence of pinning them; the frames, the wording and the layout must not.
Any capture whose *content* changes beyond dates and ids is a regression —
stop and report it rather than committing it.

The examples (`m3_walk`, `m4_walk`) are the M3 and M4 walks. Their captures are
listed in `KNOWN_GAPS.md` as needing regeneration anyway (stale 7-glyph mask);
**this ticket pins their clock, it does not regenerate them** — that is the
acceptance group's, and doing both at once makes it impossible to tell which
change caused a difference.

## 3. Also here, and small

**`crates/graft-ui/src/graph_screen/footer.rs:1-3`.** The module doc lists "the
commit-graph offer" among the alternatives `footer_line` chooses between. It is
not one: `footer_line` (footer.rs:44-100) matches on problem / search / notice /
compare / reveal / hints, and the offer is drawn by the separate
`commit_graph_line` (footer.rs:22) on its own row. The doc's second clause says
so correctly — "and the commit-graph offer's own line above it" — so the
sentence contradicts itself within one line.

**This misread cost the user a wrong defect.** Reading that doc, he concluded
the offer renders through the `problem` branch in `Token::Warning`
(footer.rs:73) and filed it as a colour defect; the offer in fact draws in
`Token::Info` with `Token::Accent` key letters (footer.rs:22-36), which is
already right. Fix the sentence so it cannot be read that way again. Doc only —
**no behaviour change here**; if you find one needed, report it, don't take it.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The sweep table above is confirmed file by file, with what time enters
      each capture — evidence: the **confirmed table** below, from a scout
      (`Explore`, sonnet) that read each file, plus my own
      `grep -rn 'docs/shots' crates`. The ticket's table was wrong in two
      places and short by five writers; both corrections are recorded there.
- [x] Every writer that puts time in its output uses a fixed clock — test:
      `conflict_interim::capture_banner_for_docs_shots`,
      `lane_cap::capture_lane_cap_for_docs_shots`,
      `hint_bar::capture_hint_bar_for_docs_shots`,
      `preview::capture_preview_for_docs_shots`,
      `preview::capture_scrolled_preview_for_docs_shots`. **Three writers are
      deliberately not pinned** — see "Not taken, and why" below; they are
      reported, not silently skipped.
- [x] Each affected capture regenerates byte-identically three runs in a row —
      test: those five, run three times with
      `cargo nextest run -E '…' --run-ignored all`, each run's 22 files copied
      aside and `diff -rq`'d. r1≡r2 and r1≡r3, all 22 files. A fourth run
      after a rebuild (the mutation revert) also matched r1 byte for byte.
- [x] No capture's content changed beyond dates and ids — evidence:
      `git diff docs/shots` on the 22. Ages and short oids change; frames,
      wording, captions and row counts do not. **One consequence worth
      naming:** `m5-lane-cap-160x24.txt`'s *row order* changed. That is the
      defect being cured, not a regression — 28 objects previously shared one
      wall-clock second, so the graph's date tie broke on the commit id
      (DESIGN.md §3.2) and ordered them arbitrarily. The capture still shows
      what it exists to show: the `▾` fold marker appears exactly once before
      and after, and the header still reads `31 commits  +1 lanes`.
- [x] `gallery.rs` still uses the wall clock for real repositories — test:
      `gallery::every_entry_uses_the_clock_its_source_needs` (the `Source::…`
      match at `crates/graft/tests/it/gallery.rs:50-54` and its assertion at
      `:1004-1008`). Neither `tests/it/gallery.rs` nor `examples/gallery.rs`
      is touched by this ticket — `git diff --stat` on both is empty.
- [x] The `footer.rs` module doc no longer reads as if the offer were a
      `footer_line` alternative — test: read against
      `footer_line`'s own doc (`footer.rs:38-42`, which lists problem /
      search / notice / compare / reveal / hints and no offer) and
      `commit_graph_line` (`footer.rs:22`). Doc-only; `cargo doc` has no
      behavioural test to name, and none is claimed.

## The confirmed sweep table

Two rows of the ticket's table were wrong and five writers were missing.

| File | Writes shots? | Time in output | Was pinned? | Now |
|---|---|---|---|---|
| `tests/it/preview.rs` | yes, 3 captures | age per row | **no — ticket said yes** | 2 of 3 pinned |
| `tests/it/context_menu.rs` | yes | age per row | yes (`:504`) | unchanged |
| `examples/gallery.rs` | coverage matrix only | **none** | n/a | unchanged |
| `tests/it/conflict_interim.rs` | yes | age + tie-broken lane order | no | pinned |
| `tests/it/captures.rs` | helper only | **none of its own** | n/a | gained `frame_at` |
| `tests/it/lane_cap.rs` | yes | age + tie-broken row order | no | pinned |
| `graft-graph tests/it/raster.rs` | yes, PNGs | **none — geometry only** | n/a | unchanged |
| `examples/m3_walk.rs` | coverage matrix only | (walk captures are pty-side) | no | **not taken** |
| `examples/m4_walk.rs` | not a writer | (walk captures are pty-side) | no | **not taken** |
| `tests/it/hint_bar.rs` **(missed)** | yes, 14 files | age per row | no | pinned |
| `tests/it/remote_by_key.rs` **(missed)** | yes | age of an app-made commit | no | **not taken** |
| `tests/it/coverage.rs` **(missed)** | yes | **none** | n/a | unchanged |
| `src/view/tests.rs` **(missed)** | yes, 19 files | near-epoch, scripted | mixed | unchanged |
| `graft-ui` × 7 **(missed)** | yes, 8 files | **none — no time source at all** | n/a | unchanged |

Verified rather than assumed: `grep -n 'SystemTime\|now()\|Utc::now\|run_isolated'`
over all seven `graft-ui` writers returns nothing, and `write_shots` plus the
seven `graft-ui` `capture_for_docs_shots` tests run twice produce
byte-identical output. **A file with no time in its output needs no change,
and none was made to one.**

## Not taken, and why (all three need a behaviour change)

`Clock` is not the only place the wall clock is read. `session.rs:44` claims
it is "the one place `SystemTime::now()` is read on the date path", but
**`crates/graft/src/jobs/operate.rs:270-275` and
`crates/graft/src/jobs/undo.rs:297-302` both call `SystemTime::now()`
directly**, bypassing `Clock` entirely, to stamp `at_unix_ms` on a journal
entry and to date the commits the app itself makes.

So any capture whose repository is written *by the app during the capture*
cannot be pinned from the test side. Pinning only the render clock would date
those objects after the pinned instant, and a future commit is not drawn as
an age at all — `relative_date` returns the literal word `future`
(`crates/graft-ui/src/graph_screen/columns.rs:61-63`). That would change what
the capture shows, which §2 forbids. The three:

1. **`preview::capture_time_machine_for_docs_shots`** — its journal entries
   are written live by `jobs/operate.rs:270`.
2. **`remote_by_key::capture_undo_after_push_for_docs_shots`** — its commit is
   made through the commit editor (`pushed_one_commit`, `:460-469`).
3. **`examples/m3_walk.rs` / `m4_walk.rs`** — the walks include `commit`,
   `amend`, `commit-on-unborn-head`, `stash-save` and `stash-pop`
   (`tests/it/m3_walk.rs:908, 1013, 1070, 1245, 1303`), all of which create
   objects live.

I caught this in my own work first: my initial pin of the scrolled-preview
capture used `FORK_NOW`, which is 61 commits too early for `left_behind`, and
would have rendered 61 rows as `future`. `LEFT_BEHIND_NOW`
(`preview.rs`) exists because of that.

**The fix is one seam** — give the job layer the session's `Clock` instead of
reading `SystemTime` — but that is a behaviour change in `graft`'s job path,
not a test change, so it is reported here rather than taken (§3's rule,
applied to §2).

## Findings

- **Nothing in the suite defends capture determinism.** With
  `conflict_interim`'s fixture clock unpinned, the full workspace suite was
  **2226/2226 green** — 0 red. Every capture writer is `#[ignore]d`, so the
  only thing that catches an unpinned clock is regenerating three times and
  diffing by hand. A future builder can unpin one and every gate stays green.
- **`captures::frame` was a copy of `preview::frame_at` with the clock
  parameter dropped.** That is the cause of this whole ticket: ticket 21
  extracted the helper and lost the one argument that mattered. There was
  also a *third* copy in `hint_bar.rs:87-104`. Both now delegate to the one
  in `captures.rs`, so the next extraction cannot lose it again.
- **17 captures under `docs/shots/` are stale, unrelated to the clock.**
  Regenerating the `view::tests` `capture_m2_*` / `m3_*` / `m4_*` family and
  `graft-ui`'s `m5-ui-banner.txt` shifts the age column one cell and adds a
  space after the short oid — the 7-glyph-mask staleness already on the
  books. I regenerated them while sweeping and **put them back** (by
  `git show HEAD:<path> > <path>`, not `git checkout`), so this ticket's diff
  is only the 22 clock captures. They are the acceptance group's to redo.

**Mutation-check the clock ones.** Unpin one fixture's clock and show the
three-run diff going red. If it stays green, the determinism is coming from
somewhere else and you have not found it.

## How to run a mutation check (new, from 23 onward)

Mutation checks have been costing an hour a ticket by running the whole
workspace suite once per mutation — and in one case **twice**, the first run
piped to `/dev/null`. Do it this way instead:

- **Per mutation, run only the tests that tick names.** A filtered
  `cargo nextest run -E 'test(<name>)'` finishes in seconds. Narrowing cannot
  produce a false green: if the named test catches the fault, it catches it
  under a filter exactly as it does in a full run.
- **Once, on the mutation that restores the actual reported bug, run the full
  workspace suite** and report the blast radius — "n red, and only these n".
  That claim has been load-bearing twice this milestone (ticket 18 proved no
  other test pinned the old behaviour; ticket 22 discovered a second guard
  because an expected red did *not* appear). Keep it where it earns its place.
- **Never run the same suite twice to parse it twice.** Capture the output
  once and read it.

The rebuild after each edit is the floor and this does not remove it.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g23`, deleted
before hand-back. Baseline **2226/2226** — check it yourself.

**Never `git checkout` or `git restore` across your worktree** — a builder
destroyed five uncommitted files that way this milestone. Revert a mutation by
editing the file back.

**Captures stay under 200 KB each.** Commit, never push.

## Report

Cause or symptom per fix. The sweep table. Which captures changed and by how
much. Anything unverified — which is not broken.

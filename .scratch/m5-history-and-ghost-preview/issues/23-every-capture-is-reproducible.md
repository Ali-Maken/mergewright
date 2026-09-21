# every-capture-is-reproducible: a fixed clock in every file that writes to docs/shots

Status: ready
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

- [ ] The sweep table above is confirmed file by file, with what time enters
      each capture — test/evidence:
- [ ] Every writer that puts time in its output uses a fixed clock — test:
- [ ] Each affected capture regenerates byte-identically three runs in a row —
      test:
- [ ] No capture's content changed beyond dates and ids — evidence:
- [ ] `gallery.rs` still uses the wall clock for real repositories — test:
- [ ] The `footer.rs` module doc no longer reads as if the offer were a
      `footer_line` alternative — test:

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

# G-palette 2c — a row's ref badges cost what fits on screen, not what is on the commit

Status: partial (built 2026-10-07: every box but the ADR-0021 release run, which fails at load ~12 with and without
this change — NOTES "ADR-0021 run"; closes 12's open perf box and KNOWN_GAPS `M8/12-palette-perf` once that run is
quiet and green)
Build with: **Opus** — diagnosis from numbers, fix at the cause, count-based guards; correctness is judged from the
guards and the perf file, so not Fable. Brownfield (ADR-0035): **scout first**; the conductor confirms the model after.
Blocked by: 12 merged (its `scripts/perf-palette.sh`, `docs/perf/m8-palette.txt`). **Not concurrently with 05**: both
touch the `view` row builders; if 05 is unbuilt, this ticket goes first and 05 inherits its guard form.
Needs: none new. Calls 12's perf script; `graft_fixtures::synthetic::Scale::with_refs` (the heavy tier's 50k-ref
fixture, refs spread over history).

## Ruling (planner)
- **Facts** (12 NOTES; `docs/perf/m8-palette.txt` on 12's branch): loop p95 60.7 ms, FAIL. `update_us` p95 5.4 ms
  (palette in budget). `draw_us` p50 30.6 / p95 39.7 ms; `write_us` p95 24.6 ms. Same fixture, no palette, held `j`:
  draw p50 33.4 ms. 40k branches + 10k tags on 3 commits → ~16 000 badges on one row.
- **The fixture stays, and the budget is met on it.** PROMPT §6: "repos with 50k+ refs", "input to frame: under
  16 ms", with no rule on how refs spread. Many refs on one commit is a real shape (tag farms, mirrors whose remote
  branches sit on one commit). A 120-column row draws at most ~40 badges; work past the edge is waste. Swapping the
  fixture for a spread one would hide the evidence (ADR-0021 rejected hiding evidence).
- **Fix is in the graph's per-row badge work, not the palette.** Skipping the graph redraw under the palette is a
  symptom fix: held `j` with no palette fails the same budget. Cause rule: **per-row badge work (build, sanitise,
  order, span, draw) is bounded by the row's width, independent of refs on the commit**; any ordering that needs every
  ref is done once per refs read, not per frame.
- **Same cells drawn.** No `+N` overflow mark: DESIGN has no badge-overflow rule (grep `docs/design` for badge
  overflow: none), and adding one is a design change. Badges clipped at the edge draw as today (`…`).
- **Who owns it:** ticket 05 owns per-frame cost growing with refs *as his trace shows it* and is held for his run.
  This is a measured case of that class, so it is filed here and does not wait.
- **`switch_refs` ~2 s on 50k refs** (BRANCHES `reading…`; the `C` picker shares it): **not a budget breach**. It is
  a job; the loop stays live and `reading…` is the honest state. Measured split only in this ticket (box below); a
  fix, if the split shows one, is a separate ticket.
- **Launch cursor on the newest row, not HEAD: a gap, not built here.** Intended: HEAD — design note
  "Where he lands" (`docs/design-notes/m8-polish-and-ship.md:160`, "Open / init succeeds → cursor on HEAD") and the
  Journeys' start state (:100). `crates/graft/MAP.md:272` documents `lands_on_head_at_open` as armed only for an
  open **from the empty state**, which matches what 12 observed for a CLI launch. Unconfirmed in source (rule 7):
  the conductor files it as its own ticket in `graft` (`opening`), first box "the line that skips the landing on
  launch", test-first. Not here: different crate and files. Unnoticed till now because HEAD is usually the newest row.

## Read first
- This file; 12's NOTES ("Perf box — open")
- `crates/graft-ui/MAP.md`: `badge_spans` (227), `graph_screen::badges` (230–236), `GraphEntry` (206)
- `crates/graft/MAP.md`: `refs` (104–119: `RefBadges::on`, drawing order), `view` (rows, entries), `trace` (126–129)
- `docs/adr/0021-time-budgets-fail-in-release-only.md` (Decision, CPU caveat)
- `.scratch/m8-polish-and-ship/issues/05-trace-followup-per-turn-cost.md` Acceptance (the guard form taken here)

## Scout questions (≤1 page)
Per frame, per drawn row, which code walks every ref on the commit rather than the ones that fit (file:line):
`RefBadges::on` (sorts or filters per call?), the `view` builder that fills `GraphEntry::badges` (sanitises or clones
each?), `badge_spans`, `render::draw_badges`, `hit::badge_index_at`, radar markers per ref. Who else reads
`GraphEntry::badges` in full (preview `badges_for_preview`, context menu, drag)?

## Scope
In: find the per-row costs that grow with refs on the commit, by count, and fix each at the cause: bound the work by
width, move any all-refs ordering to the refs read. `badge_spans` stays the one packing (hit-test and draw share it);
a badge past the edge it never returns is never hit. A guard per fix. Perf re-run on two fixtures. `switch_refs` split.
Out: the launch cursor (gap above); `switch_refs` speed; a `+N` badge mark; palette code (`update/palette.rs` — grep
shows no change needed, or stop and report); 05's other candidates (strip search-hit set).

## Signatures
None public new. If a signature in `graft-ui` changes (e.g. `badge_spans` takes a width bound, or `GraphEntry::badges`
holds only what can fit plus a count), MAP.md graft-ui and graft record it; every caller listed under Touches is
updated in the same commit.

## Acceptance (each box names its test or file at tick time)
- [x] NOTES, before any change: the costs found, each with file:line and its count at 10 vs 16 000 refs on one row.
      — NOTES "Costs found", written before the code change.
- [x] Guard per fix, count-based (calls or allocations, not time): per-frame badge work on a 120-column row is equal
      at 10 and 16 000 refs on the commit (e.g. `row_badge_work_is_bounded_by_width`).
      — `graft::view::tests::many_refs::row_badge_work_is_bounded_by_width`: badges built (= `badge_view` calls) at
      120 columns: 10 refs → 10, 1 000 → 61, 16 000 → 61. Literally "equal at 10 and 16 000" cannot hold (10 refs
      build 10); bounded and equal at every pile over the width is what is asserted. Seen failing first on its
      assertion (`left: 1000, right: 61`).
- [x] Same cells: `badge_spans_match_the_drawn_cells` still green unchanged; a new case draws a row with 16 000 refs at
      80/120/200 columns and reads every cell back equal to the first ~40 badges drawn alone
      (`many_refs_row_draws_like_its_first_badges`).
      — `graft_ui::graph_screen::tests::many_refs_row_draws_like_its_first_badges` (whole `Buffer`s equal, symbols
      and styles; both icon sets; one-cell and `team/N` names; with and without `only in stash` pushed last).
      Seen failing on its assertion with a too-small stub bound (`columns / 40`: "the first 2 draw different
      cells"); `badge_spans_match_the_drawn_cells` unchanged and green.
- [x] Hit-test: a click on the last drawn badge of a 16 000-ref row hits it; a click past it hits none
      (`many_refs_row_hit_test_matches_draw`).
      — `graft::view::tests::many_refs::many_refs_row_hit_test_matches_draw`. Regression guard: it passed before the
      fix too (not fail-first; the cut must not change it).
- [x] Drawing order unchanged on a mixed row (HEAD, current, local, remote, tag, stash) (`badge_order_unchanged`).
      — `graft::view::tests::many_refs::badge_order_unchanged`: detached `[HEAD]` first, else `[◆ main]`; then
      local, remote, tag, stash; with 16 000 tags added the row equals the first 61 of `refs.on` mapped. HEAD and
      a current branch cannot both be on one row (a current branch means HEAD is not detached), so two rows.
      Its cut half failed first on its assertion.
- [x] Perf, release, from the script, summary lines only: `scripts/perf-palette.sh` gains a fixture argument; runs on
      the piled fixture (12's) and on `Scale::with_refs` → `docs/perf/m8-palette.txt` (12's file, both sections).
      **`check loop_p95_under_16ms ok` on both.** Held-`j` on the piled fixture (`scroll-lag-pty.py`) → same file:
      `draw_us` p95 under 16 ms.
      — `scripts/perf-palette.sh both` → `docs/perf/m8-palette.txt`: piled `check loop_p95_under_16ms ok` (p95
      12 929 µs), with-refs `check loop_p95_under_16ms ok` (p95 11 336 µs), held j `check
      held_j_draw_p95_under_16ms ok p95=1158`. Load average 8–12 during the run (other builders); see NOTES.
- [x] If loop p95 is still over after `draw_us` is in budget and `write_us` dominates: stop, quote the lines, say
      whether the pty reader (harness) or Graft's write path blocks (measure, don't guess); no fix in this ticket.
      — Not triggered: loop p95 is under on both fixtures (`docs/perf/m8-palette.txt`). `write_us` is now the
      largest part (p95 10.7 ms piled), recorded in NOTES, not diagnosed.
- [x] `switch_refs` on the piled fixture split by phase (git process, parse, sort; script → `docs/perf/m8-switch-refs.txt`),
      quoted in NOTES with file:line per phase. No code change.
      — `scripts/m8-switch-refs.sh` (example `crates/graft-git/examples/switch_refs_cost.rs`) →
      `docs/perf/m8-switch-refs.txt`; NOTES "switch_refs split". No product code changed.
- [ ] `heavy::budgets_hold` and `frames::` release run still pass (ADR-0021 command); quote the summary lines.
      — **Not ticked.** Fails at load ~12 with and without this change (NOTES "ADR-0021 run").
- [x] KNOWN_GAPS `M8/12-palette-perf`: fate in NOTES for the conductor (closed / narrowed to `write_us` / `switch_refs`).
      — NOTES "Gap fate".

## Touches (each checked in the report)
`graft-ui` `graph_screen` (`badge_spans`, `render::draw_badges`, `badges`), `graft` `view` row builder, `refs`
(`RefBadges::on`, `badges` — order fixed at read), `view::hit::badge_index_at` (shares the packing), preview
`badges_for_preview` (borrows live badges: grep that it still sees every ref it moves, including ones off-edge),
context menu / drag on a badge (grep `badge_index_at` callers), radar `ref_findings` (per commit, unchanged — grep),
pixel tier rows (same entries), captures with badges (`docs/shots/*` re-run by their tests; same cells expected, any
diff is a defect here), 12's perf script, `many_refs.rs` debug guard (still passes), MAP.md graft + graft-ui. Keymap,
journal/undo: none (no key, no operation).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` —
workspace-wide. Perf runs from the script, never in the builder's context.

## Size
One context, ~50k tokens. More than two causes → fix the larger two, file the rest for the conductor.

## NOTES (builder)

Scout used: `.scratch/m8-polish-and-ship/scouts/12c-palette-loop-budget.md`.

### Costs found, before any change (per drawn row, per frame; N = refs on the commit)
Counts read from the code, not timed. 10 refs vs 16 000 refs on one row:
- `crates/graft/src/view/entries.rs:324-329` (`row_entry`): one `badge_view` per label (`entries.rs:374`: a
  `sanitize_path_bytes`, a `BranchIdentity::of_name`, one `Badge` pushed). **10 vs 16 000** calls, a `Vec` of N.
- `crates/graft-ui/src/graph_screen/render.rs:556-570` (`badges_width`): one `Badge::text` `String` per badge,
  collected into a `Vec<String>`, before the half-column budget is applied. Called three times per drawn row
  (`draw_badges` :475, `badge_spans` :530, `subject_truncated` :591 via the footer reveal) → **30 vs 48 000**
  allocations.
- `render.rs:533` (`badge_spans`): `Vec::with_capacity(N)`; its loop already stops at the edge. **1 alloc of 10 vs
  16 000 slots.**
- `crates/graft/src/view/hit.rs:42-56` (`badge_index_at`): rebuilds the whole `graph_view` → the `row_entry` cost
  above for every row, once per mouse event (not per frame).
- Not per frame, not fixed here: `update/radar.rs:440-455` `force_push_candidates` walks `refs.on(id)` per resident
  row on the update side (`update_us` p95 5.4 ms, in budget); `view/preview.rs:236-237` maps every overlay label for
  a preview row (ref-move preview only; `entry` there has no `AppState`, so no width to bound by — filed as a gap
  below).
- `RefBadges::on` (`refs/mod.rs:41`): a hash lookup returning a slice; the order is fixed once in `refs::badges`
  (`refs/mod.rs:269-327`). No all-refs ordering per frame.

One cause behind the first three: the row builder hands `graft-ui` every label on the commit, so every downstream
step is O(N). Fix at the builder: take at most `badges_that_fit(screen columns)` labels.

### The fix (cause)
`view/entries.rs` `row_entry`: `.take(badges_that_fit(state.size.columns))` on the badge iterator (compare base
included), in `refs::badges`' order. `graft_ui::graph_screen::badges_that_fit(columns) = columns / 2 + 1`
(`render.rs`, beside `badge_spans`). Why the cells cannot change: badges get under `columns / 2` cells; every
badge's text is at least 2 cells and each after the first adds a separator, so badge number `columns / 2` starts at
or past the edge and `badge_spans` stops there; the first `columns / 2 + 1` already want the whole half column, so
the room is unchanged. `state.size.columns` bounds the graph pane (`frame::screen_area` reads the same field).

**Scout's contradiction (`mark_stash_only`, `rows.rs:141-157`, pushes `only in stash` last): no stop needed.** On a
cut row the stash badge lands at index `columns / 2 + 1`, past the edge, exactly as it was behind 16 000 refs —
`many_refs_row_draws_like_its_first_badges` checks it with whole buffers. On a row that is not cut nothing changes.

### Perf (release, `scripts/perf-palette.sh both`, summary lines; machine loaded by other builders)
Before = the same build with only the `.take(..)` line commented out, run minutes earlier (not committed).

| run | before | after |
|---|---|---|
| piled, palette: loop_us p50 / p95 | 19 282 / 32 524 µs (FAIL) | 4 127 / 12 929 µs (ok) |
| piled, palette: draw_us p50 / p95 | 15 175 / 19 853 | 668 / 1 186 |
| piled, palette: write_us p95 | 13 265 | 10 710 |
| piled, held j: draw_us p50 / p95 | 14 374 / 17 797 (FAIL) | 450 / 1 158 (ok) |
| with-refs, palette: loop_us p50 / p95 | 7 206 / 20 056 (FAIL) | 3 364 / 11 336 (ok) |
| with-refs, palette: draw_us p95 | 1 955 | 887 |
| with-refs, palette: write_us p95 | 18 749 | 10 230 |

Load average at start of each section: before 14.5 / 13.1 / 16.2; after 11.7 / 10.0 / 8.4. **with-refs' before/after
gap is write_us, not draw**: its refs are spread (no piled row), so this fix barely touches it; the write drop
tracks the load falling, not the change. Its `ok` is one run on a loaded machine. `write_us` is now the largest
term of the loop on both fixtures; not diagnosed here (the ticket's stop box did not trigger).

### switch_refs split (`docs/perf/m8-switch-refs.txt`, release, piled fixture, 10 runs, load ~11)
- git `for-each-ref` process (`crates/graft-git/src/switch_refs.rs:59-66`): p50 929.4 ms, p95 1 515.0 ms,
  3.8 MB of output, 40 001 records.
- `@{-1}` processes (`switch_refs.rs:76-97`): p50 11.0 ms.
- parse (`parse_listing`, `switch_refs.rs:127-166`): ~47 ms (derived: total − git − previous; it is private).
- sort: **none in the read** — `switch_refs` keeps git's ref order; the palette's ranking runs per keystroke in the
  names job (`jobs/palette.rs`), not here.
- total `switch_refs`: p50 987.8 ms. ~94 % is git's own process. 12 measured ~2 s at the UI; the rest was not
  split here. Any fix (e.g. fewer `--format` fields, `%(subject)` costs an object read per ref) is a separate
  ticket, as ruled.

### ADR-0021 run — box NOT ticked
`cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold --test-threads=1`,
twice with the fix, once without, load average 7.5–14 (other builders' cargo):
- with the fix, run 1 and 2: `test result: FAILED. 12 passed; 9 failed`. Failing: `frames::fifty_thousand_refs_…`
  ×5 (unicode, pixel, scrolling_the_diff, detail_pane_open, map_view_open), `frames::journal_find::j_with_a_broad_find…`,
  `frames::scrolling_the_100k_fixture_…` ×2, `heavy::budgets_hold_at_pixel_on_monorepo_with_j_held_40mb`.
  Example: `p99 18.61ms is over the 16ms input-to-frame budget at Unicode with FiftyThousand refs`.
- without the fix (only the `.take` line out): `FAILED. 14 passed; 7 failed` — the same set less `map_view_open`
  and the 40mb monorepo test, with load falling 12.8 → 7.5 during that run.
- Tests that draw no piled row (100k fixture, J find, monorepo) fail too, so this reads as machine load, but **I
  could not confirm it**: the run needs a quiet machine (ADR-0021's CPU caveat). Next step: rerun the command
  above with no other cargo running, quote `test result`.
- All 7 `heavy::budgets_hold_*` first-paint tests but the 40mb pixel one passed in every run.

### Gap fate (KNOWN_GAPS `M8/12-palette-perf`)
Narrow, don't close yet: the draw cost is fixed (cause) and both palette fixtures pass the loop check once; close
it when the ADR-0021 run is green on a quiet machine. Watch `write_us` (largest loop term now, p95 ~10 ms on both).
`switch_refs` ~1 s is git's process (see split) — a job, not a budget breach, per the ruling.

### Found, not fixed (for the conductor)
- `view/preview.rs:236-237`: a Ghost Preview row maps every overlay label through `badge_view` — the same O(refs)
  per row, during a preview only. Its `entry` has no `AppState` (no width); passing one is a signature change in
  `view::preview`. Unmeasured.
- `update/radar.rs:440-455` `force_push_candidates`: O(refs) per resident row on the update side; `update_us` p95
  is 2.6 ms here, in budget.

### Touches (how each was checked)
- `graft-ui` `badge_spans` / `draw_badges` / `badges_width`: unchanged code; bounded by the list they are given.
  `badge_spans_match_the_drawn_cells` green; new `many_refs_row_draws_like_its_first_badges`.
- `graft` `view::entries::row_entry`: the one change. `view::hit::badge_index_at`: same `graph_view`, so same cut
  list; doc comment updated; `many_refs_row_hit_test_matches_draw`. `update::geometry::badge_at` indexes
  `refs.on(at)` with the hit index — a prefix, so indices still match (same test, plus the existing
  `a_drag_lands_on_the_badge_the_frame_drew_under_the_pointer`, green).
- `view::rows::cursor_row_context` (`rows.rs:774-780`) takes the first `Branch` in the cut list: HEAD and branches
  come first, at most index 2 (base, HEAD), and 41+ survive at 80 columns. Covered by the full nextest run only,
  no dedicated test.
- Preview `badges_for_preview`: reads `RefBadges`, which is unchanged, so every moved ref is still seen (grep:
  only `view/preview.rs:236` maps it, untouched).
- Radar `ref_findings`/`force_push_candidates`: read `RefBadges`, untouched (grep `refs.on` in `update/radar.rs`).
- Pixel tier: `view::strips` reads no badges (grep `badges` in `view/strips*`: none).
- Captures with badges: their tests ran in the full nextest (5 820 passed); no shot diff.
- `many_refs.rs` heavy guard: not rerun (heavy tier, debug); `heavy::budgets_hold_on_the_100k_fixture_with_50k_refs`
  passed in every release run above.
- Scripts: `scripts/perf-palette.sh` (fixture argument, held j, load in each header), new `scripts/m8-switch-refs.sh`.
  `crates/graft-fixtures/examples/synthetic.rs` gains `--refs N` (outside graft/graft-ui; 12b holds neither file).
  New example `crates/graft-git/examples/switch_refs_cost.rs` (timing only).
- MAP.md graft-ui (`badges_that_fit`) and graft (`view::entries`). Keymap, journal/undo: none.

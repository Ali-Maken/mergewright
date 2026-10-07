# G-palette 2c — a row's ref badges cost what fits on screen, not what is on the commit

Status: draft (spec-only pass 2026-10-07; closes 12's open perf box and KNOWN_GAPS `M8/12-palette-perf`)
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
- [ ] NOTES, before any change: the costs found, each with file:line and its count at 10 vs 16 000 refs on one row.
- [ ] Guard per fix, count-based (calls or allocations, not time): per-frame badge work on a 120-column row is equal
      at 10 and 16 000 refs on the commit (e.g. `row_badge_work_is_bounded_by_width`).
- [ ] Same cells: `badge_spans_match_the_drawn_cells` still green unchanged; a new case draws a row with 16 000 refs at
      80/120/200 columns and reads every cell back equal to the first ~40 badges drawn alone
      (`many_refs_row_draws_like_its_first_badges`).
- [ ] Hit-test: a click on the last drawn badge of a 16 000-ref row hits it; a click past it hits none
      (`many_refs_row_hit_test_matches_draw`).
- [ ] Drawing order unchanged on a mixed row (HEAD, current, local, remote, tag, stash) (`badge_order_unchanged`).
- [ ] Perf, release, from the script, summary lines only: `scripts/perf-palette.sh` gains a fixture argument; runs on
      the piled fixture (12's) and on `Scale::with_refs` → `docs/perf/m8-palette.txt` (12's file, both sections).
      **`check loop_p95_under_16ms ok` on both.** Held-`j` on the piled fixture (`scroll-lag-pty.py`) → same file:
      `draw_us` p95 under 16 ms.
- [ ] If loop p95 is still over after `draw_us` is in budget and `write_us` dominates: stop, quote the lines, say
      whether the pty reader (harness) or Graft's write path blocks (measure, don't guess); no fix in this ticket.
- [ ] `switch_refs` on the piled fixture split by phase (git process, parse, sort; script → `docs/perf/m8-switch-refs.txt`),
      quoted in NOTES with file:line per phase. No code change.
- [ ] `heavy::budgets_hold` and `frames::` release run still pass (ADR-0021 command); quote the summary lines.
- [ ] KNOWN_GAPS `M8/12-palette-perf`: fate in NOTES for the conductor (closed / narrowed to `write_us` / `switch_refs`).

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

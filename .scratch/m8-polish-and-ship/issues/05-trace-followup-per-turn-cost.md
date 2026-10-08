# G-trace follow-up — remove the per-frame cost that grows with history or refs

Status: **deferred by the user 2026-10-08 ("lets finish the product, and then deal with this") — runs after the rest of M8 is built, before the milestone stop.** His trace run exists (2026-10-08, docs/perf/m8-trace-2026-10-08-{ghostty,zed}.txt; was held until it did) (`docs/perf/m8-trace-<date>.txt` from his machine, not the
builder's). No run → this ticket does not start (rule 1: green fixtures did not match his screen, :84).
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: diagnosis from numbers, a fix at the cause.
Blocked by: 02, 03
Needs: `settle-work-at-rest` merged (Part B of that ticket is this one); `pace-fence-and-sync` (`fence_us` filled).

## Read first
- `docs/design-notes/m8-polish-and-ship.md`: Need (scroll row), Journeys J1–J3, "Where he lands" first row
- His summary `docs/perf/m8-trace-<date>.txt` and the builder's `docs/perf/m8-trace-monorepo.txt` — summary lines only
- `crates/graft/MAP.md`: `event_loop` (`pump`, `joins_frame`), `view` (rows, strips), `update` (cursor motion)
- `.scratch/m8-polish-and-ship/scouts/frame-loop.md`

## Scope
In:
- Read his summary: which field dominates a frame (`update_us`, `draw_us`, `bytes`, `write_us`, `fence_us`) and how
  the tail after release compares with the fixture's. State the finding with its numbers in NOTES before any change.
- Find, by test, any per-frame or per-turn work whose cost grows with history length or ref count. Known candidate
  (unverified since M2): the overview strip's search-hit set is rebuilt every frame it is shown (`view/strip.rs`).
  Fix each found at the cause: computed once per change, not per frame.
- A guard test per fix: frame work counted (calls or allocations, not time) at 1k and 100k commits / 10 and 50k refs
  is equal within the window.
Out:
- Anything his numbers don't point at. If his run shows the paced loop within budget (tail ≤ 100 ms, p99 frame
  ≤ 16 ms), close with the summary in NOTES and no code. Wheel step change (his scene 1 verdict, separate).

## Signatures
None public. Internal names are the builder's; MAP.md records them.

## Acceptance
- [ ] NOTES: his numbers quoted (file:line of the summary), the dominant field named, the cause named with file:line.
- [ ] Each fix: a count-based guard test at two history sizes and two ref counts (test named at tick).
- [ ] `scripts/m8-trace-summary.sh` re-run on MONOREPO after the fix → `docs/perf/m8-trace-followup.txt`, before/after
      lines side by side. No timing assert in the fast tier.
- [ ] TESTING.md "How he runs the trace": one line asking him to run it again after this lands.

## Touches
- `view` per-frame builders (strip, rows, header tail), `update` motion path; the held_key and frames tests; MAP.md
  graft; KNOWN_GAPS rows 414, 1095 (fate in NOTES for the conductor).

## Scout questions
- Per frame, which builders iterate over all refs, all laid-out rows or all search hits rather than the visible
  window (file:line)? Does any run when nothing it reads changed?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~50k tokens. More than two causes found → fix the larger, file the rest for the conductor.

# The lane cap is a budget; pins are priorities, not vetoes

Status: **Accepted** (user, 2026-09-13; planner-fable, spec-only pass after
ticket 39).
Changes docs/design/03-graph-layout-algorithm.md §3.5's "never fold … a lane on the ancestry path of the
currently selected commit". ADR-0022 said the fold ranking was unchanged; this
is why it is a new ADR, not an addendum.

## Context

Ticket 39's captures (`docs/shots/39-git-git-80x24-after-jj.txt`,
`39-many-lanes.txt`): after `j j` on git/git the selection sits on a `seen`
merge, its ancestry is every topic merged into `seen` below, ticket 16 pins
every lit column, so 20 columns are kept under a cap of 8. The graph column
is cut at half the screen, the rail with it, while the header says
`+2 lanes`. On the many-lanes fixture the top merge's thread is every topic
lane and nothing folds.

The cause is not the thread emphasis and not the fold vocabulary: it is that
a "pin" was read as a veto, and a set of vetoes has no upper bound. DESIGN
§3.5's own reason for the rule was narrower than its wording — "so selecting
a commit never causes *its own lane* to disappear behind a `+N`". On a merge
train the ancestry path is the whole graph.

Ticket 14 already stated the consequence of unbounded vetoes: "if only
unfoldables remain the window draws wider than the limit". That is the rail
cut.

## Decision

1. **The cap is a budget.** `Fold::plan` draws at most `max_lanes` columns,
   rail included. Columns are admitted in priority order until the budget is
   spent; every column past it folds. The header's `+N lanes` is exact: N =
   columns open somewhere in the window and not drawn.
2. **Priority, highest first:**
   1. lane 0 (always; the cap is ≥ 2, so lane 0 and the rail always fit);
   2. any column holding a node in the window (the one class that may
      exceed the budget — a row without its node is not a graph; see the
      residual below);
   3. the selected commit's own column — DESIGN §3.5's stated intent;
   4. reserved integration lanes;
   5. the selection's thread columns (`Ancestry::columns`);
   6. everything else.
   Within a class, least-recently-touched folds first (ticket 14's rank,
   unchanged). A column open in no row is dropped, not counted (unchanged).
3. **The screen clamps the cap to what fits.** `max_lanes_for` returns
   `min(configured or breakpoint value, lanes the graph column can hold at
   this width)`. A configured `graph.max_lanes` larger than the screen is a
   number the screen cannot honour; clamping it is a clarification of
   ADR-0022's "holds at every width", not a reversal.
4. **A thread column that folds is not lost.** Its edges draw as stubs on
   the kept nodes in the branch's identity colour (ADR-0022), which is the
   thread's colour; the thread is still there to be read, just not opened
   out.

## Residual (accepted for M1, logged)

Class 2 can still exceed the budget: a 22-row window can hold up to 22
distinct node columns. It exceeds *what fits on screen* (20 columns at 80
columns, 30 at 120) only when nearly every row in the window is a commit on
a different open lane — possible in date order before promotion, unlikely in
topo order, which groups a topic's commits. If it is ever seen, the fix
inside this decision is: a node column past the budget folds too, and its
node draws **on the rail** on its row (`●`/`◉` in the rail column, branch
colour; `⋮` elsewhere), its edges to kept columns as the existing stubs
(`▾` on the rail node, `▴` on the kept node where the lane joins), and a
kept lane that opens or closes on that row without a node draws `╷` / `╵`
at its column. No crossings, exact count. Not built until the residual is
observed; KNOWN_GAPS names the trigger.

`+` (expanded) draws every lane and can exceed half the screen on git/git
(54 lanes). Pre-existing, the user's explicit action; separate KNOWN_GAPS
row, not planned for M1.

## Consequences

- `Fold::plan(max_lanes, keep: &Keep, rows)`; `Keep { selected, reserved,
  preferred }` replaces the flat `pinned` set. `Compaction.pinned` becomes
  `Compaction.keep`. `Ancestry::columns` feeds `preferred`.
- Property: `drawn_columns().len() <= max_lanes` whenever node columns plus
  lane 0 fit in the budget; `kept + folded = open columns in the window`.
- TESTING.md's git/git row: `N ≥ 10` at 80×24, not `N ≥ 40` (the design
  pass's number assumed the first window was the `+54` window the user
  scrolled to; N is per window) and not `N > 0` (too loose to catch this
  regression). After `j j`: still `N ≥ 10`, rail in the last graph column.

## What would reopen this

The user's eye on git/git `seen` at 120 columns after 39, 40 and this: if a
folded thread reads as "my selection's history vanished", promote class 5
above class 4, or above class 2 with the rail-node drawing built.

## User notes on approval (2026-09-13)

- "The veto-versus-budget diagnosis is right and the residual is honestly logged."
- The header's `+N lanes` must stay exact after this change (ticket 47 acceptance).
- After 40 and 47 land: capture git/git `seen` at 120 columns and at 80×24 after `j j`; the user judges both by eye before either ticket closes.
- 39's bar of at least 10 folded lanes at 80×24 stands; "above 0" was meaningless and "at least 40" came from the wrong model.

## Amended (ease-pass 06b, 06u; 2026-10-18)
The residual is built: a node column past the budget folds and its node draws on the rail, so the budget holds for every class (`drawn_columns() <= max_lanes`, property-tested). The `╷`/`╵` end marks are not built. Before ranking, a column with no commit in view is dropped unless it connects two commits in view; such connectors rank after every node column and, out of budget, drop rather than fold. `+N lanes` is replaced by `lanes drawn: N of M`. Current rule: DESIGN §3.5.

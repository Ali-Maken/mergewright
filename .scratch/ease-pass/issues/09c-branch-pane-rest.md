# branch-pane-rest — 09's four unbuilt checks

Status: draft (planner spec-only pass 2026-10-19; split from 09 at 59b290d1, 23/27 ticked, split approved by the user)
Blocked by: none (09 merged at 59b290d1).
Beside 09b: **yes**, disjoint files (lists below). Beside 09p: **yes**. 09p holds ops, git, the cost script and `docs/perf/`. This ticket holds only the app's test tree and one capture.
Order: build with 09p, the two helpers together, before 09b.
Build with: Sonnet (builder-sonnet), down from 09's Opus. The Opus reasons were the cross-crate wiring, the cache and the settle tests, and those are built and merged. What is left is four tests and one capture against built code, with each rule written out below. If a test shows the product is wrong, the builder stops and files it with the line (rule 7). A Sonnet builder can do that.
Needs: from 09, all merged: `update::branch_pane::{focused, candidates, settled, owed, pane_holds}`, `view::branch_pane::{pane_view, pane_slot}`, ui `branch_pane::{BranchPaneView, PaneLine, pane_width}`, ui `graph_screen::layout::beside_pane` / `screen_rows(..).pane`, `Look.route`, `Look.reads`, `Product::BranchStatusRead`, `captures::at_rest` / `settled_frame`; fixtures in `tests/it/branch_pane.rs` (`with_sibling`, `drawn`, `cursor_on`, `read`, `pane`).

## Read first
- `.scratch/ease-pass/issues/09-branch-pane.md`: Decided (M5, M6, rulings R and W), Scope "States", Acceptance (the four open boxes), and NOTES. Read continuation 2 "Unbuilt: seams for a split" and the first NOTES "Determinism risk" and "Exact next steps" 2.
- `docs/design-notes/ease-pass.md` New UI (`:268-272`, pane states).
- `crates/mergewright/MAP.md`: `update::branch_pane`, `view::branch_pane`, `update::geometry`, `jobs::branch_status`. `crates/mergewright-ui/MAP.md`: `branch_pane`, `graph_screen` (`ScreenRows.pane`).
- `.scratch/ease-pass/issues/14-ease-acceptance.md` "A second small repo, `team`", for the topology used in check 2.

## Ruled here
- **A failed read: keep it as built. All four lines read `can't tell: <reason>`.**
  - One `branch_status` call answers all four lines (09 NOTES "Decisions taken"), so all four fail together.
  - If only one line said `can't tell`, the other three would read `reading…` with no read running. That is a status that lies.
  - The note's "in that line only" (`ease-pass.md:272`) assumed one read per line, and the design never had that. The test asserts four lines. NOTES carries a **Proposed edit** to `:272`: "a failed read → `can't tell: <reason>` on every line it answers". The builder does not edit the note.
- **Check 2 is retargeted from wallet-release to the integration base.** In 09's fixture wallet-release has no lane beside sara's rows (09 NOTES, settle 2 and "Untested so far"):
  - Under R1, an `Other` base needs `base_only == 0`. Its commits then lie on sara's first-parent chain, so it draws no lane of its own.
  - A moved `Other` is off the route (Q15 a).
  - Only a moved **integration** base can be both on the route and drawn as its own lane. The check uses one. This changes the fixture, not the rule.

## Scope
In: tests and one capture only.
1. **`reading…` / `can't tell:` state test.**
   - Before the answer lands, on sara's row, the pane shows the heading and four lines, each `reading…`.
   - With a forced failure (the job answers `Err`), all four lines read `can't tell: <reason>`, with the same reason on each. The heading stays. No line reads `!`, `✓` or `·`.
   - Drive this through the harness by withholding or replacing the `BranchStatusRead` answer. No sleep, retry or raised timeout.
2. **The route reaches the colours.**
   - Add a fixture beside `with_sibling`, `with_moved_integration`:
     - `develop` off `main`;
     - `wallet-release` off `develop`;
     - `sara/fix-toc` cut from wallet-release's tip, with 2 commits;
     - then 1 commit on `develop`, dated after sara's newest commit, so develop's lane passes sara's rows.
   - With the cursor on a sara row, after the read:
     - (state) `look.route` names `main`, `develop`, `wallet-release`, `sara/fix-toc`;
     - (glyph) develop's lane cell on that row is `│`.
   - With the cursor on a `main` row, the same cell is **not** `│`. This proves the colour came from sara's route, through `frame_route`'s focus gate (M5).
   - If develop's lane cell is `│` both ways, or never, stop and file it with the line. Do not change the assertion to fit.
3. **Geometry mirror at 90/110/120, detail pane open and closed.**
   - At each size, `update::branch_pane::pane_holds` equals ui `screen_rows(..).pane`, and the graph columns that `view/frame/layout.rs` `graph_columns` hands out equal the ui rows' width.
   - At 90: no pane, in both states.
   - With the detail pane open: no branch pane, at every width (09 Decided).
   - Put it beside `update/tests/tab_strip_geometry.rs:221` (09 NOTES). Extend that test in place, or add a sibling file. Say which in NOTES.
4. **Capture** `docs/shots/ease-pass/main-pane-reading-120.txt`.
   - Cursor on sara at 120, answer withheld (the same mechanism as 1). All four pane lines read `reading…`.
   - Taken by `tests/it/branch_pane/shots.rs` beside the other three captures. Two runs must give the same file.
   - Colour unverified (rule 5).
- If `#![allow(dead_code)]` is still at the top of `tests/it/branch_pane.rs`, remove it once these tests use the fixtures.

Out: product code. If a check fails because the product is wrong, stop and report with file and line; don't fix it here. Also out: the cost (09p), `C` and `not merged` (09b), and the note's edit (Proposed only).

## Files (for disjointness)
- `crates/mergewright/tests/it/branch_pane.rs` (fixtures)
- `crates/mergewright/tests/it/branch_pane/reads.rs`
- `crates/mergewright/tests/it/branch_pane/shots.rs`
- `crates/mergewright/src/update/tests/tab_strip_geometry.rs`, or a new `update/tests/pane_geometry.rs` plus its one `mod` line in `update/tests/mod.rs` (or wherever `tab_strip_geometry` is declared)
- `tests/support/*` and `captures.rs`: only if withholding an answer needs a test-only hook. Name the file in NOTES. Never product code.
- `docs/shots/ease-pass/main-pane-reading-120.txt`
- `tests/it/over_400.txt`: only if a file crosses its row. Split the file instead of raising the row.

Not touched: any MAP.md (no new public item), `src/` outside `update/tests/`, 09b's picker and entries files.

## Signatures
None new in product code. Test helper, if added:
```rust
// tests/it/branch_pane.rs
fn with_moved_integration() -> Fixture;   // shape as Scope 2; `Fixture` = the type `with_sibling` returns
```

## Touches
- `captures::at_rest` callers: unaffected if the withholding hook is opt-in. Show that with a grep or a green full run.
- The tour IN_FLIGHT list: it reads `│ reading…`. Check it still matches.
- 14's `team` walk, which uses the same topology shape (no edit).
- `ease-pass.md:272`: Proposed edit, in NOTES only.

## Acceptance
Each tick names its test inline.
- [ ] Before the read lands, each of the four lines reads `reading…`. A forced failure shows `can't tell: <reason>` on all four lines, with one reason, the heading kept and no mark.
- [ ] After sara's read, develop's lane draws `│` on a sara row, and `look.route` names develop. With the cursor on a main row, the same cell is not `│` (state + glyph).
- [ ] The geometry mirror is green at 90/110/120 with the detail pane open and closed.
- [ ] `docs/shots/ease-pass/main-pane-reading-120.txt`: four `reading…` lines under sara's heading, identical over 2 runs, ≤200 KB. Colour unverified.
- [ ] NOTES: the Proposed edit to `ease-pass.md:272` and where the mirror test lives.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~30k.

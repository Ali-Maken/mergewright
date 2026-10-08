# Stop-now flake recurs: `j j` undone by the topological swap

Status: done
Build with: debugger (Opus).
**Crate set: graft only.** Files: `crates/graft/src/history/{state.rs,scroll/apply.rs,scroll/query.rs,tests.rs}`,
`crates/graft/MAP.md`.
Source: KNOWN_GAPS M8/stop-now-signal-flaky. `quit_waits::signal::stop_now_after_the_real_offer_restores_then_says_where_git_stopped`
failed again in 13l-A's loaded run, with 13k's fix in the base. 13k never reproduced the natural failure.

## NOTES (debugger, 2026-10-08)

No compaction. No scout: one test, then the code a trace pointed at.

### The failure, kept this time

- **Loop:** the `it` test binary, this one test, 4 lanes in a row, beside 18d's build (load 18–34).
  Script: `loop.sh` in the session scratchpad. Not committed.
- **Natural rate:** 2 failures in 216 runs (1 in 24, then 1 in 100). Both took 20.6–20.8 s, the same as the
  original 20.6 s.
- **Assertion, both times:** `pty.rs:217`, `waited 20s for main's row`. The screen shows the graph with `▸` still
  on row 0 (`finish the feature`).
  - So the test failed **in setup**, at `signal.rs:93`, before `m`, the merge or any stop. 20.6 s is
    `PATIENCE` (20 s) plus the startup.
  - The stop code 13k changed never ran in the failing runs. 13k's link is ruled out: the flake was never in
    `graft-git` `run/stop.rs`. 13k's fix stays; the bug it fixed was real, just not this one.

### Cause: `crates/graft/src/history/scroll/apply.rs` (as at HEAD 75fb28b4)

- A temporary probe (`[DEBUG-13m]`, removed) logged each event `update` took, with `History::cursor` before and
  after.
- Failing run, in order:
  1. the first windows (`W1`, `W2`) draw the graph, and the test sends `jj`;
  2. `TopologicalOrder` arrives, and `promote` asks request 3 with the order;
  3. request 3 is answered **in the order on screen** first (m7/12 Part C). `apply` (:70, `reaim_promotion`)
     saves the cursor's commit as the swap's anchor: row 0;
  4. `j`, `j`: cursor 0 → 1 → 2. Moves over rows on hand ask for nothing, so the saved anchor is not updated;
  5. 1 ms later request 3 is answered again, **promoted**. `promoted` (:127–:157) puts the cursor back on the
     saved anchor: **cursor 2 → 0**.
- The test then waits 20 s for `▸` on `main moves again`, and fails.
- The saved `Promotion::Swapping { anchor }` was refreshed only by `ask` and by provisional windows. A `j`/`k`
  or a click (`select_at`) that asked for no rows left it stale.
- **What the user would see:** in a repository without a commit-graph file, keys pressed in the moment between
  the two answers are undone, and the cursor jumps back.

### Fix (cause, not symptom)

- `promoted` re-finds the commit the cursor is on **when the promoted window lands**. That is `selected()`, read
  from the rows on hand in the old order, at its current offset on screen.
- The stored `anchor` in `Promotion::Swapping` and `reaim_promotion` are removed, so nothing can go stale.
- `ask` still sends the selection at ask time in `Promote`. The walk lays out the promoted window around that
  selection, as before.
- No timeout, sleep or retry was added. The test is unchanged.
- **Cost to the user:** none. Moves made during the swap are kept.
  - Edge case: the commit he is on may not be in the promoted window. Today that happens only if he moved
    without a request after the last ask. The cursor then keeps its row number, which is the same fallback as a
    request with no anchor.
  - Before the fix, it snapped to the stale commit.

### Tests

- **Regression:** `graft history::tests::a_move_between_the_two_answers_of_a_promotion_is_kept`, on a real
  `Linear` fixture. Steps: promote, serve the provisional answer, `Down(1)` twice with no request, then the
  promoted answer.
  - **Red** against HEAD's history code, run twice (before and after the helper split): `left: 0, right: 2`.
  - **Green** with the fix.
  - `served` was split into `answered` and `swapped_in` so the gap can be driven. `served` behaves as before.
- **Original scenario, gap widened:** a temporary 400 ms sleep before the promoted answer (`views/promotion.rs`,
  removed), with the probe on.
  - Unfixed: in every run where the keys fell in the gap (6 of 8 runs), the promoted window moved the cursor
    2 → 0.
  - Fixed: the keys fell in the gap in 12 of 12 runs, and every one stayed 2 → 2.
  - With that sleep the snap comes after `m`, so the test itself passes. The probe's cursor line is the signal.
- **Natural loop after the fix:** not run long enough to mean anything. At a 1% base rate, a clean 100 proves
  little. The gap-widened loop is the evidence.

### Hypotheses ruled out

- **13k's: a slow `ps` used up git's grace in the stop.** Ruled out because both failures came before the stop.
- **Keys lost to the input flush or probe at startup.** Ruled out because the probe shows both `j` applied
  (cursor 0 → 2).

**Touches.**
- `History::promoted`: every promotion swap (`update::history::walk_progress`). Checked by `history::tests::*`
  (promotion end to end, `g`/search during a swap, page-down past the rows on hand) and `history::promotion_tests`.
- `History::ask`: every request made while swapping sends the same anchor as before (the selection at ask time).
  Checked by the same tests.
- `Promotion::Swapping` lost its field. The other readers match `{ .. }`: `update/history.rs:93`,
  `view/rows.rs:358`, `update/history/tests.rs:50`. Checked with grep and by compiling.
- `crates/graft/MAP.md` `Promotion` row updated.
- Every pty test that moves the cursor right after the first frame shares this race, for example
  `quit_waits::signal::a_signal_restores…` through `merging`. The same fix covers them.
- No keymap, capture, journal or other crate changes.

**Gates.** All workspace-wide.
- `cargo fmt --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo nextest run --test-threads 4`: 5933 passed, 11 skipped (796 s, run beside 18d).
- `cargo test --doc`: ok.

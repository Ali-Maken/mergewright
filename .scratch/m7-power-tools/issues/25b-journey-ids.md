# m7-journey-ids — give plain push `P y` a journey-counts id and walk it; drop the pull-rebase-push half; fix m6 note :130

Status: done
Blocked by: 25 (merged, b42f8454)
Crates: graft (tests only: `tests/it/m5_walk/`), docs (`docs/design-notes/m6-conflicts.md`: the fence row, :130, :528
only; `TESTING.md` only if it lists fence rows). No production source.
Source: ticket 25 NOTES item 12 and its unticked box; `docs/reviews/m6-look-finish-wave2.md:179-188` (m3);
`docs/design-notes/m6-usability-fixes.md:59,62` (the approved counts).
Build with: **Sonnet**. One fence row, one scenario in an existing harness, two text edits.
Scout: none. Every place is named.
Needs: none (`parse_counts`, `within_the_note`, `journey_counts_match_the_note` exist).

## Ruling (planner, 2026-09-26)
- **(a) Plain push: yes, an id.** `P y` = **2** was approved as P15 (`m6-usability-fixes.md:59`) but sits only in a
  table, so no walk fails if it grows. Row goes in the **M6 fence** (`m6-conflicts.md:490-527`): the count is M6's
  decision, and the M6 stash/switch rows already live there. Best tool: lazygit `P` **1**, with no box. Rejected:
  a box before a write git can't undo (`m4-remotes.md:125`); `y` is the one extra key.
- **(b) Pull-rebase-push: drop, already covered.** Same five keys `p r r P y`, same count 5, already gated as
  `rejected-push-dirty-to-pushed` (`m6-conflicts.md:521`). That route sets aside too, so it walks more than the clean
  one, never less. A clean-tree id would add a second walk for the same count. The review's fix (wave2 m3) asked for
  `rejected-push-to-pushed`; ticket 74 delivered it as the dirty variant. **Unverified by me:** the builder-reported
  scenario at `every_scenario.rs:139`. The first acceptance box below confirms it before the drop stands.
- **(c) :130: yes, append, don't rewrite.** :130 is a record of what was proposed. Rewriting it would falsify the
  record, so mark it superseded, the way item 11 did for :374.

## Scope
In:
1. Add to the M6 fence, after `rejected-push-dirty-to-pushed`:
   `push                          2                     # P y · m6-usability-fixes.md:59 (P15, approved) · lazygit P 1 (no box; rejected, m4-remotes.md:125) · walked in m5_walk (25b)`
2. One m5_walk scenario, journey id `push`. Start: graph, cursor on the current branch row, clean tree, one commit
   ahead of `origin` (fixture `one_to_push`, `Remote::Local`). Keys: `P y`, both routed as journey inputs.
   **Landing**, from truth and the frame:
   - the bare's `refs/heads/<branch>` equals the local tip (git's truth);
   - the push box's title `Push <branch> → origin/<branch>` is gone from the frame;
   - no sticky failure line is drawn.
   Add it to the file where the remote scenarios live (`remotes.rs` per MAP), with its test in `every_scenario.rs`.
3. `m6-conflicts.md:130`: append after `conflicts open`: ` (title superseded by Amendment 2: see the mockup at :153;
   format from crates/graft/src/view/resolver.rs:183)`. Find it by its text; the line may have drifted.
4. `m6-conflicts.md:528`: the sentence "the only fence tagged `journey-counts`" has been stale since M7 added two fences.
   Reword it to what `journeys.rs` does now: read the MAP entry first, then the function. Say whether it reads every
   fence, and whether ids must be unique across them.

Out: a clean-tree `p r r P y` id or walk (ruling b). The force push (`P → ⏎`, 6; P17 is still open). m4_walk's
`push_plain_by_y`: leave it; it checks the push, not the count. No capture.

## Read first
- `crates/graft/MAP.md` :22-24 (m5_walk, M6 additions, M6 journeys). Read it before any test source.
- `docs/design-notes/m6-conflicts.md` :490-529 (fence), :127-131.
- `docs/design-notes/m6-usability-fixes.md` :56-78 (Journeys, where he lands).
- Ticket 25 NOTES item 12.

## Acceptance (name the verifying test or the diff line at tick time)
- [x] The drop stands (`stash_journeys.rs:603`, run by `every_scenario::journey_rejected_push_dirty_to_pushed`, `every_scenario.rs:139`; green in full nextest): `rejected-push-dirty-to-pushed` is walked. Name the scenario's file:line and the test that runs
      it (builder said `every_scenario.rs:139`). If it's absent, **stop**: don't drop (b), and report it.
- [x] Row `push 2` (`m6-conflicts.md:522`) in the M6 fence: the diff line.
- [x] `every_scenario::push_journey (green; scenario `remotes.rs::push`)_is_walk_gated` green. It routes `P y` (2 inputs ≤ 2) and asserts the three landing
      facts above.
- [x] Red first (seen: `FAIL push: push: 2 inputs sent, the note's journey-counts allows 1: P y`; before the row: `no row for it`): in a scratch copy, lower `push` to 1, run it, and confirm `within_the_note` fails naming 2 vs 1. Say
      so in NOTES; don't commit the scratch.
- [x] `journey_counts_match (green; `every_counted_journey` now adds `remotes::counted_journeys`)_the_note` green with the new id (ids both ways).
- [x] :130 and :528 (diff hunks @130, @522, @529 only): the diff lines. `git diff --stat -- docs/design-notes/m6-conflicts.md` shows only the row, :130
      and :528.
- [x] No capture (empty) changed: `git diff --stat -- 'docs/shots/**'` empty.

## Touches (check each, report how)
- `journey_counts_match_the_note` / `within_the_note`: a new id with no scenario, or the reverse, fails. Check: run both.
- The M7 fences (`m7-power-tools.md:811`, `:1242`): an id that collides fails the parse. Check: `grep -n '^push ' ` in
  both.
- `every_scenario.rs`'s captures/matrix check and `coverage.rs::history_matrix` (`M5_STEMS`): a new scenario may need
  a matrix row. Add one if a check demands it. **If a check demands a capture, stop:** report `blocked: capture
  required`; add none.
- `TESTING.md` :2339-2363: if it lists fence ids, add `push`; else nothing. Check: `grep -n rejected-push-dirty TESTING.md`.
- Keymap: none, since no key changes.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~20k.

## NOTES (25b)
- No compaction. No scout (ticket: every place named).
- TESTING.md lists fence rows (:2452): `push` row added after it.
- `journey_counts_match_the_note` needed the scenario in its set: `every_counted_journey` extends with `remotes::counted_journeys()` (`#[cfg(test)]`, else dead code in the example).
- :528 now says: every `journey-counts` fence in m6-conflicts.md is read, ids unique across them; M7 fences not read by this walk (`journeys.rs:21`).

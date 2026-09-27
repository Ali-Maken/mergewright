# 20j blame-shallow-capture — 20f's last box: blame in a shallow clone, captured; no commit-graph offer there

Status: draft
Blocked by: **32** (Graft can't open a shallow clone until it lands). Run after **33** too: 33's harness deadline turns
any leftover loop into a failure instead of a 9-minute spin.
Crates: graft (`repository.rs` commit-graph fact, `tests/it/blame_place.rs`, `tests/it/m7_walk/blame.rs`,
`docs/shots/m7/`, MAP.md).
Build with: **Sonnet**. One capture on a decided ruling (20's "Ruling (planner): shallow boundary") and one fact rule,
decided below; plus two test-only folds from review 18e-20f.
Scout: none. 20f's NOTES name the setup; 32's NOTES name git's commit-graph rule.
Needs: `graft_git::shallow::ShallowList::read(&RepoHandle) -> Result<ShallowList, ShallowError>`,
`ShallowList::is_empty` (32).
Split by: planner, 2026-09-27, from 20f's "Left". Folds added by the planner, 2026-09-27, from review
`docs/reviews/m7-18e-20f.md` Minors 6 and 7 (gap rows M7/18e20f-m6, -m7).

## Ruling: no commit-graph offer in a shallow clone
After 32, neither git (`commit_graph_compatible`, pinned in 32's NOTES) nor Graft reads a commit-graph in a shallow
repository, so writing one can't help him; offering it is a dead end, and the offer takes `B` (graft MAP.md
`blamable`: "no commit-graph offer"). In a shallow repository the fact is `CommitGraphFact::Shallow` and the offer is
`NotNeeded` (no line, `B`/`X` not taken). The fact is read beside the existing one (`repository.rs:111-112`, where
`Absent` is turned into the offer); an unreadable shallow file is the existing `Unreadable(msg)` fact, shown as a
problem, no offer.
If 32's NOTES say git **does** use a commit-graph in a shallow repo: stop, report, don't build this part.

## Scope
- The fact rule above, with tests.
- The capture: `docs/shots/m7/blame-shallow-187x50.txt`, drawn by `view::view` via `frame_at` (20f's mechanism).
  Setup (20f): `git clone --depth 2 file://<session fixture>`; **no** `commit-graph write` (the rule above removes the
  offer); `⏎` on `main`, `j` to `src/auth/session.rs`, `⏎`, `]`, `j` to line 22, `B`, `k`×4 to line 18. Line numbers
  from git's own blame on the clone (20d: every line but 3 and 26 is the boundary's); if 18 isn't a boundary line
  there, pick one that is and say which.
- Tick 20's **Captures** shallow box, naming the test.
- The two folds below (Minors 6 and 7).

Out: any drawing change (20c/20e drew the shallow row; a difference from the Right line is reported, not fixed here).

## Folded in (review m7-18e-20f; graft tests only; planner, 2026-09-27)
- **Minor 6, walk counts are hand-written labels** (`m7_walk/blame.rs:235-242`, `:290`, `:329`, `:362`): `route` is
  filled by `push`es beside `app.key` calls, so a key sent without a push is never counted. Make the route drive the
  keys, as `m5_walk/steps.rs` does: one helper sends each counted key **and** records it; no walk body sends a key
  after setup any other way. Reuse `m5_walk`'s driver if it takes the m7 harness as is; else a local `press` helper of
  the same shape. Name it in NOTES. Counts unchanged: `blame_line_to_commit` 2, `commit_back_to_blame` 1 (and
  `…_with_a_base_pinned` 1), `blame_who_at_line` 1, `blame_line_before` 2 (m7 note `:1249-1252`).
- **Minor 7, caption drift** in `tests/it/blame_place.rs` (these become the contact sheet's "Right" lines):
  - `:115` "the 150-character line" → **137** characters (`graft-fixtures/src/shapes/trace.rs:51`).
  - `:80-81` "The spec's 'the trailer whole' assumed the mock-up's shorter line" → the spec's arithmetic was wrong: the
    mock-up's line 22 is the same text (m7 note `:129`), the design sheds the sha first (`:137`), 134 + ` · 2f48ca3`
    (10) = 144 > 140. Match ticket 20's corrected Captures bullets (2026-09-27).
  - Line numbers are at `b79f6351`; re-find by text.

## Signatures
```rust
// graft::repository — CommitGraphFact gains
/// A shallow clone: git and Graft read no commit-graph here, so none is offered.
Shallow,
```

## Acceptance (name the verifying test at tick time)
- [x] **Red first:** `a_shallow_clone_offers_no_commit_graph` — open the depth-2 clone; `commit_graph_line()` is
      `None` and `B` answers blame, not the offer. Red before the rule. (Red: `a shallow clone must not offer a
      commit-graph` left `Some(Offered)` right `None`; green after.)
- [x] `a_full_clone_without_a_graph_still_offers_one` — unchanged behaviour, green.
- [~] `blame_shallow_capture` (all asserted **except** the graph-pane clause — see NOTES) writes `docs/shots/m7/blame-shallow-187x50.txt` (≤ 200 KB). **Right** (20's ruling,
      words exact): row 18 reads `▸ ^0a2762 shallow clone · can't say who`; its trailer `◂ can't say who · this clone
      starts at 0a27627`; the boundary block's other rows carry brackets and no name; `Priya` appears nowhere on the
      screen; the graph pane's last row is the boundary commit, no lane below it. The test asserts each; the sha is
      the clone's boundary id (the fixture's `0a27627`, or say what it is).
- [x] 20f's walks and captures still green; `commit_graph_line() == None` asserts unchanged (full nextest run,
      `blame_place::capture_*` and `m7_walk::blame::*` green; `assert_no_offer` untouched).
- [x] Minor 6: the five walks in `m7_walk/blame.rs` send every counted key through `Route::press`. Red by mutation:
      `blame-line-before took 3 inputs ["B", "^", "MUT"], the note allows 2` (applied, reverted). Grep in NOTES.
- [x] Minor 7: both captions corrected (`blame_place.rs` `capture_blame_140x40`, `capture_blame_long_line_187x50`
      docs); `git status docs/shots/` shows only the new untracked shallow capture.
- [x] MAP.md: `CommitGraphFact::Shallow`, the offer rule (graft MAP.md `CommitGraphFact` line).
- [x] Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest (4235 passed, 3 skipped), `test --doc`.

Status: done (graph-pane clause not assertable, below).

## NOTES (builder 20j, 2026-09-27)
- Scout: none (ticket says none; 20f/32 NOTES name setup).
- Rule: `repository.rs` `commit_graph_fact` reads `ShallowList` first; non-empty → `Shallow`; read error →
  `Unreadable(msg)`. `update/commit_graph.rs` maps `Shallow` → `NotNeeded`.
- **Graph-pane clause not asserted:** at 187×50 blame fills the screen; no graph pane is drawn in the capture, so
  "the graph pane's last row is the boundary commit" can't be seen here. Reported, not fixed (drawing out of scope).
- Boundary sha: `0a27627` (asserted, `main~1` of the clone). Line 18 is a boundary line (asserted).
- Block heads at lines 1, 4, 27 also read `^0a2762 shallow clone · can't say who` (the ruling's "block's first row"),
  and line 3/26 are `74f9a9e Graf…` — as 20d's NOTES said.
- Route helper: local `Route { id, keys }` with `press`/`within_count`; `m5_walk`'s driver is a scenario runner
  (`Step::Press`) not taking this harness. Grep `app.key` in `m7_walk/blame.rs` after the edit: lines 98–146 are
  `open_on_line_22`/`file_row` setup, 163 `blame()` (setup-only helper), 253 is `Route::press`, 328 is setup in
  `back_to_blame` (`⏎` before the counted `Esc`), 372 setup `k` in `blame_who_at_line`. No uncounted key after setup.
- Touches: `CommitGraphFact` matches grepped — `repository.rs`, `update/commit_graph.rs` (arm added); tests/journal/
  radar/view literals construct other variants only; `commit_graph_line` matches `CommitGraphOffer`, unaffected;
  `repos.rs:237` reads `commit_graph::status` directly (32 makes it `Absent` in a shallow repo), unaffected. `B`/`X`:
  `a_shallow_clone_offers_no_commit_graph`. Captures: only the new file. TESTING.md: `### M7 blame in a shallow clone`.
- Colour unverified (rule 5): heatmap on the boundary rows.

## Touches (check each; NOTES says how)
- **`CommitGraphFact` / `CommitGraphOffer` matches** in graft (`repository.rs`, the offer's `update`, `commit_graph_line`,
  `repos.rs:237`'s presence check): grep; each gains the arm or is shown unaffected.
- **`B` and `X`** in a shallow clone: now answer their non-offer meaning (`docs/design/02-keymap.md`); test above.
- **Captures:** only the new one; no existing capture is shallow (grep). Minor 7 edits comments, not pictures.
- **Walk harness** (Minor 6): `m5_walk` walks green if its driver is reused; `m7_walk::noted_keys` unchanged.
- **Gap row M7/boundary** ("blame in a shallow clone credits the boundary's author … not seen on screen"): this capture
  is its by-eye evidence; the conductor closes it at the M7 stop if the user agrees. Also closes M7/18e20f-m6, -m7.
- **TESTING.md:** the capture's row (the acceptance ticket assembles the sheet).
- Colour unverified (rule 5): the heatmap's oldest bucket on the boundary rows.

## Size
~42k. One context.

## Rules
Worktree only, own `CARGO_TARGET_DIR`. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

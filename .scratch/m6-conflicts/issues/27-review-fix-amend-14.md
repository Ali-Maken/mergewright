# review-fix-amend-14: three over-long files, clever over plain in the pane code, the `two_boxes_over_content` flake

Status: done
Blocked by: 25-stable-file-order-and-done-file-open.md (building; same crates, ADR-0036). Runs before 15.
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-14.md` Major 1, 2, 3.
Build with: **Opus** (low) — splits, plain-code rewrites, one test wait.
Needs: none.
Crates: graft, graft-ui.

## Read first
- `docs/reviews/m6-amend-14.md`: Major 1, 2, 3 — each names file and line.
- `crates/graft/MAP.md` (`view::resolver`), `crates/graft-ui/MAP.md` (`conflict_resolver::panes`).
- `ARCHITECTURE.md`; `codebase-design` for the splits. PROMPT.md 6.2 (readability).

## Scope
In:
1. **Major 1 — sizes.** Split, pure moves, to ~400 lines where a seam exists: graft `view/resolver.rs` (727); graft-ui `conflict_resolver/tests/three_panes.rs` (467); `conflict_resolver/draw.rs` (402). MAP.md updated. No seam → say so with the count.
2. **Major 2 — plain over clever.** `panes/draw.rs:88` tuple parameter → named struct or separate params; `panes.rs:264-289` magic pane indices 0/1/2 → named (enum or consts); `panes.rs:225-236` a `let-else` that can never fire → remove or make the impossibility a type. Behaviour unchanged; existing tests and goldens pass unchanged.
3. **Major 3 — flake.** `tests/.../two_boxes_over_content.rs:102-121` waits for the frame without waiting for the git version check; the commit-graph footer and row count depend on it (`update/state.rs:355`, `:378`). Fix the cause: the wait also requires `state.system_git.is_some()`. No sleep, no retry (rule 2). Search for other captures with the same wait shape and list them in NOTES (fix them if the fix is the same one line).

Out: review-14 Minors (→ KNOWN_GAPS); Minor 5 (`three_panes_capture` vs 15's advance) is ticket 15's.

## Acceptance (each tick names its test, at tick time)
- [x] Major 1: splits done, line counts in NOTES, MAP.md updated — gates green; `view::resolver::tests::*`, `view::resolver::whole_file::tests::sizes_read_the_way_a_person_reads_them`, `conflict_resolver::tests::three_panes::*` pass unchanged; files in NOTES
- [x] Major 2: three rewrites, goldens unchanged — `it::resolver::three_panes::three_panes_capture` (check_shot), `conflict_resolver::tests::three_panes::*`, `it::resolver::whole_file::*`; `docs/shots` untouched after the full run
- [x] Major 3: the wait includes `system_git`; other same-shape waits listed — test: `it::two_boxes_over_content::capture_two_boxes_over_real_content_for_docs_shots` (passes; flake itself not reproduced, see NOTES)
- [x] Gates workspace-wide: fmt, clippy `-D warnings`, nextest 2830 passed / 1 skipped, doctests — this worktree, `CARGO_TARGET_DIR=/private/tmp/claude-501/graft-target-27`

## NOTES

No scout: the review names every line; re-read against `c191a91`.

**Major 1 — splits, pure moves.**
- graft `view/resolver.rs` 801 → 323; new `view/resolver/whole_file.rs` 182 (whole-file and done cards, `absent_on`, `size_words` + its test); `view/resolver/tests.rs` 311 (test-only; old `mod tests` flattened, plus `drawn_for_tests`, re-exported at its old path).
- graft-ui `conflict_resolver/draw.rs` 402 → 224; new `draw/block.rs` 193 (`draw_header`/`labels`/`pair`/`side`/`end`, now `pub(super)`).
- graft-ui `tests/three_panes.rs` 467 → 358; new `tests/three_panes/fixtures.rs` 119 (`CORPUS`, `text_of`, `doc_of`, `Sweep`, `SWEEPS`, `swept`).
- `panes.rs` 289 → 356 (Major 2's types); under 400.
- MAP.md: graft (`view::resolver` files row), graft-ui (draw/block.rs, fixtures, `Pane`/`PaneWidths`).
- Review's "add a gap row for view/resolver.rs": moot, file now 323. KNOWN_GAPS untouched (builder rule) — conductor to update the M6/14 flake row.

**Major 2.**
- `draw_cell` takes `at: &CellAt { block, pane, number_width }`; the loop zips panes with cells, no index.
- `Pane` moved to `panes.rs`; `pane_number_widths` returns `PaneWidths { yours, result, theirs }` with `of(Pane)`; no `0/1/2`.
- `whole_file_rows` takes a `WholeFileRows` struct built from the variant's fields at the call site; the unreachable `let … else` is gone.

**Major 3 — cause fix, verified by reading only.** `ready()` now requires `state.system_git.is_some()`. Flake not reproduced before or after; fix closes the ordering gap the review names (`update/state.rs` `commit_graph_line`'s `?`). Test-only; no user cost.
Same-shape waits (predicate lacks `system_git`, shot shows the commit-graph footer), fixed with the same line:
- `tests/it/context_menu.rs` `settled` (context-menu, name-prompt, drag captures).
- `tests/it/hint_bar.rs` `opened` (`m5-hints-*`).
- `tests/it/conflict_interim.rs` `opened` (via `stopped_on_the_conflict`; `m5-ui-banner-conflict-*`).
Not changed, unverified whether exposed: `lane_cap::capture_lane_cap_for_docs_shots`, `remote_by_key::capture_undo_after_push_for_docs_shots`, the `m4_walk`/`m5_walk` runners.

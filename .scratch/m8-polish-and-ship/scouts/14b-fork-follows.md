# Scout — 14b fork-follows-engine (+14c preview words)

All paths under `crates/`. Code at HEAD 75e6ae29.

## 14b questions
1. **`r` does NOT go through `RewriteParams`.** `apply/engine.rs:156-185`: `Operation::Rebase(params)` builds rows itself via `rebase_rows(repo, params.onto_tip, params.branch.oid)` (:162), marks `params.drops`, line = `params.line` (`RebaseParams`, `operation/history.rs:89`). Only `Operation::Rewrite` reads `params.todo` (:186-198). Both then meet at `git_todo(&rows, &messages, …, &followers)` (:208; `rewrite/todo.rs:186`, text at :273-294). Preview of `r`: `preview/kinds/rebase.rs:14`, own `todo_steps` (:122), left_behind at :81-82. → per ticket, **`r` splits off** (needs fork data on `RebaseParams` or a todo-level hook in engine.rs).
2. `LeftBehind` computed in `rewrite/dependents.rs:143-168` (`for-each-ref --contains oldest_replaced` minus line, cap `LEFT_BEHIND_CAP=8` `preview/effects.rs:165`, one `merge-base(local, old_top)` per named branch :175). Wrapper `preview/rewrite.rs:257-279`; called `preview/rewrite.rs:78` (Rewrite) and `kinds/rebase.rs:81` (`r`). Holds: on/merge-base, cap, line exclusion. No forecast, upstream, worktree, merge checks.
3. Tests asserting left_behind on a clean-replay fork (all files distinct, `commit()` `graft-ops/tests/it/rewrite_preview.rs:44`): `:973` feature-b mid-chain; `:986` backup at tip + feature-b (backup has 0 own commits — spec silent on `commits==0`); `:1017` rebase fork (this is `r`); `:1032` cap, 9 `park-k` at a2 (0 own commits). `:1003` empty, unaffected. graft: fixture `graft/tests/it/preview.rs:665` (`main`, `redesign` 60 commits, `r` topic→old-release) used at :778,:805,:903,:934 — redesign > `FORECAST_CAP=50` (effects.rs:12) so likely stays; could not confirm.
4. `RewriteProgress` (`rewrite.rs:292`) built only in `rewrite/progress.rs:39-88`. Readers: `graft/src/update/banner.rs:226-279` (step, branch chain :233, `pending_here`), `banner/stop.rs:31`. **Counts update-ref**: `progress.rs:60-63` (branch index = done `UpdateRef`s + 1) and `followers()` segments per UpdateRef `:113-121`; `pending_here` `rewrite.rs:316`. `rows()` `:305-336` skips unknown words, so `label`/`reset` already ignored.
5. `plan.json`: write `saved::save` `rewrite/saved.rs:88` (called `apply/engine.rs:74`); read `saved::read` :107 (via `engine.rs:350 plan_of`), `saved::all` :119 (progress.rs:57). serde, `SavedPlan` :40; unknown field → ignored (no `deny_unknown_fields`); missing field → error unless `#[serde(default)]` (precedent `index_base` :51, test :188).
6. `Operation::tips` `operation.rs:1446` → `rewrite::tips` `operation/rewrite.rs:40` (head + line). Feeds `check` via `operation/rewrite.rs:55-60` (`tip_holds` → Moved), `facts_scope` `operation.rs:1472`, `touches` `operation.rs:1801` (refs_before/after). `r`: `history::tips_of_rebase` :454, `refs_of_rebase` :445; `touches` Rebase records HEAD ref only (`operation.rs:~1790`).
7. `RewriteParams {` literals: graft-ops `rewrite/plan.rs:121,207,272,319`, `rewrite/absorb.rs:154`, `operation/tests.rs:1367,1647`, tests/it `put_back_index, in_the_way/replay, rewrite_oracle, absorb_staged`; graft **non-test** `update/rewrite_editor.rs:253` (uses `..self.opened`, compiles), tests `rewrite_editor/tests.rs:186,884`, `amend_into/tests.rs:177`, `jobs/rewrite/tests.rs:30`. `PreviewPlan {` full literals: `preview.rs:192 blank()`; graft tests `picker/tests.rs:384`, `stash/tests.rs:161`, `preview/tests.rs:100,136`, `preview/slice/tests.rs:47`, `amend_into/tests.rs:212`, `view/tests.rs:3805` (others use `..blank()`/`..a_plan()`).

## 14c questions
1. Readers of left_behind outside graft-ops: `graft/src/update/preview/line_rows.rs:11,134-174`, `preview.rs:26`, `plan_lines.rs:34`, `view/rewrite_editor/right.rs:216`; tests `preview/tests.rs:1809-1841` + literal sites above.
2. Shared builder `update/preview/plan_lines.rs:18 plan_lines` → `line_rows::*`; editor calls `left_behind_rungs` directly (right.rs:216). Per-kind callers not traced.
3. Dependent row text `FOLLOWS` `line_rows.rs:19`; glyph/theme token not checked — could not confirm.
4. Chain line drawn in **graft-ui** `graph_screen/banner.rs:104` (`{branch}, branch {index} of {of}`) from `progress.branch` via `graft/src/update/banner.rs:233`.
5. Not grepped (TESTING.md, shots) — could not confirm.

## File/crate sets
- 14b: graft-ops `rewrite/{dependents,todo,saved,progress,plan,absorb}.rs`, `rewrite.rs`, `preview/{effects,rewrite,as_run,chain}.rs`, `preview/kinds/rebase.rs` (if `r` kept), `operation/rewrite.rs`, `operation/history.rs` (`r`), `operation.rs`, `apply/engine.rs`, graft-ops tests; graft test literals (7 PreviewPlan, 4 RewriteParams).
- 14c: graft `update/preview/{line_rows,plan_lines}.rs`, `preview.rs`, `view/rewrite_editor/right.rs`, `update/banner.rs`, tests; **graft-ui `graph_screen/banner.rs`**; graft-ops effects.rs.

## Contradictions / overlaps
- 14b: `r` not via RewriteParams (engine.rs:162) → split.
- 14b "app draws as today": graft fixture `preview.rs:665` and m5 sheet `left behind: main, redesign` change if redesign follows (cap may prevent; unconfirmed).
- 14b `apply/engine.rs` "todo hand-off only": tips/journal for `r` live in `operation/history.rs`, `operation.rs`.
- 14c says graft only; chain line is graft-ui `graph_screen/banner.rs` — near 13b's graft-ui `banner_list`; same file? not checked.
- 06 (header/bar view code) may touch graft-ui banner; could not confirm. 04: no overlap found.
- 12b ordering: merged status could not confirm.

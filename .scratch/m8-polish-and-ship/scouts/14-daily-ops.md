# Scout 14 — daily ops fixes (read at 01c7abba)

## Scout questions
- **Dependents** — `graft-ops/src/rewrite/dependents.rs:75-121` `dependents()`: local branches `--contains` head tip (`:226-236`), skip self/default/same-oid (`:87-91`), keep those whose first-parent chain reaches head tip with no merge (`chain_above`, `:183`). Sorted by chain length; two not stacked → `Dependents::Fork` (`:111-118`) → `line_of` refuses `ForkedDependents` (`:37-39`). **A mid-chain fork is not a dependent because it does not contain HEAD's tip** — `--contains branch.oid` excludes it; it surfaces only via `left_behind` (`:128-143`, "mid-chain fork meets it at the fork").
- **fold_rebases** — `graft-ops/src/journal_list/steps.rs:114`; runs of `HEAD` records matching `is_rebase` (`:96-98`): prefix `rebase (` or `rebase -i (`. `rebase: fast-forward` matches neither → not folded (still open: what J shows; no test read).

## Scope items, still present?
1. Yes. Refusal is `plan.rs:228-231` in `amend_into` (ticket says `:178-185` — **stale line**; 178-185 is `open`). Reword exists only as `TodoAction::Reword` (`rewrite/todo.rs:206-235`).
2. Yes (follows from dependents logic above).
3. Yes. `graft/src/update/preview/history.rs:85-112` has no stash check (`source_at` then `build`); ticket's `:92-106` approx right.
4. Words live in graft-ops `apply/set_aside/{put_back,answer,returned}.rs`, `apply/error/words.rs`, **and graft `update/clash.rs:417`**, `update/switch.rs:239`, `update/state.rs:315,1114` — **outside the ticket's graft file set (history.rs only)**. Which composes the landing line: could not confirm.
5. Yes. `graft-git/src/history/divergence.rs:107-114` counts at pop; re-queue at `:161-170` changes flags only.
6. Open question; see above.

## File set
graft-ops (rewrite/, journal_list/, apply/set_aside/), graft-git (history/divergence.rs), graft `update/preview/history.rs` (+ possibly `update/clash.rs` for 4). Ticket 02 holds graft-term + graft `event_loop/`: no overlap seen.

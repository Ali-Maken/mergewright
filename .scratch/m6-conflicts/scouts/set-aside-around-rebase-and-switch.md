# Scout — 73 set aside around rebase and switch (main @ d572438f)

**Q1 graft-git stash wrappers.** `crates/graft-git/src/stash.rs` has only `list` (:91) and `commits` (:147). No `create`, `store`, `apply <commit>` or `drop` wrapper in graft-git. Stash writes are built as git args in graft-ops: `operation/stash.rs:92-95` (drop via `plan_at`), `operation.rs:1012,1014` (`plan_at("apply"/"drop", index, ..)`), and undo's executor `restore/steps.rs:376` (`stash store`) and `:400` (`stash drop`), which scan the reflog first (`undo.rs:70-85`).

**Q2 apply shape.** `apply` `graft-ops/src/apply.rs:198-239`: stop check :199, `read_facts` :202, `op.check` :204, `prepare_restart`/fold/`guard_repush` :208-212, touches/plan :214-215, `Opening{.. put_back: put_back_for(op)}` :217-224, `begin` (snapshot + Pending) :225, `run_plan` :237, `closed` :238. One linear function; no per-kind hook between begin and run_plan or after `closed`. `put_back_for` (`apply/record.rs:379-386`) returns Some only for StashPop/StashApply. `closed` (:243) checks `stopped_on_conflict` first (:258) → returns `Conflicted`, entry stays Pending; then `clashed_put_back` (:397-425) runs only if the entry has `put_back` and git failed, reading unmerged paths. `stopped_on_conflict` (:499-514) keys only on `op.starts_in_progress()` and `state::in_progress` — it does not look at `put_back`, so a stopped rebase with S aside would return Conflicted like today.
Rebase/checkout checks: `operation/history.rs:460-464` and `:491-501` both end in `clean_tree` (:432-437 → `DirtyTree{count}`; count from `dirty_tracked` :423, staged|unstaged, untracked excluded).

**Q3 resume.** `apply/resume.rs:185-243`; still-live after failure → `still_pending` (:290) returns early. All finishes go to `close_resume` (:318-380): status Aborted on abort/failure, Committed on continue/skip, `finish`, then Ok/Git error. The entry is the journal's Pending entry (`resume_target` :258, `live_entry` :86), which carries `put_back` from its Pending line (`journal/lines.rs:136,163`). Nothing in resume reads or acts on `put_back` today.

**Q4 recover.** `apply/recover.rs:30-80` calls `journal.mark_interrupted` (:73); `journal/file.rs:177-215` does `closed = held.clone()` so `put_back` is carried; recover itself never runs a put-back. Live entry skipped (:37-47).

**Q5 inverse.** `undo.rs:236` appends `stash_step(entry, root, forward)` (:342-392), matched on **root.kind**, not on `touches` including `refs/stash`: Push → Drop/Store of `stash_moved_to`; Pop/Drop/BranchFromStash → Store pinned, redo Drop only if undo's own `refs/stash` moved to it (07 F2, :366-381); every other kind → None (:391). Rebase/Checkout get no stash step. Nearest stash-ref test: `undo/tests.rs:195 the_stash_ref_is_never_moved_by_update_ref`; kind table `undo/tests.rs:56`. Could not confirm a test named "refs/stash moved".

**Q6 Stop.** Trait `graft-git/src/run.rs:84`. Injection precedent: `graft-ops/tests/it/undo_cancel.rs` — `StopAtPoll::new(at)` :109-124 (fires on the Nth poll), `StopOnce` with `Trigger` :129-174. Also `remote_ops.rs:764 StopAfterProgress`.

**Q7 preview.** `preview::plan` `graft-ops/src/preview.rs:326`, runs `op.check` at :338, dispatches Rebase :341 (`preview/kinds.rs:100`) and Checkout :345 (`kinds.rs:351`); struct `preview.rs:168`, final build :656, `blank()` :367. A separate `dirty_tracked` → Vec<PathBuf> at `preview.rs:484` (reset).

**Q8 DirtyTree asserts.** graft-ops pure: `operation/history/tests.rs:568 a_dirty_tracked_tree_refuses_rebase_and_switch_but_not_merge` (:582), :783-784, :819 (explain words). graft-ops it (`tests/it/history_ops.rs`): :1495 (rebase apply refused, inside `stale_preview_is_refused` :1466 or a later fn — could not confirm fn boundary), :2342 (in `branch_from_stash_is_refused_before_git_can_half_do_it` :2304), `a_rebase_preview_refuses_a_dirty_tree` :2490/:2499, `a_checkout_preview_refuses_a_dirty_tree` :2507/:2518, `every_preview_refuses_exactly_what_check_refuses` :2596. graft: `update/preview/tests.rs:1076 r_on_a_dirty_tree_opens_no_preview_and_says_why`, also :1165, :1176; `update/operate.rs:2109` (test), handling :1025, :1055.

**Q9 is_history.** `operation.rs:791-806`: a fixed kind list (Merge, Rebase, CherryPick, Revert, Reset, Checkout, Branch/Tag create/delete, BranchFromStash) — not a refusal set.

## Mismatches
- Ticket assumes graft-git may have stash wrappers: none exist beyond `list`/`commits` (stash.rs:91,147); all stash writes are graft-ops arg builders (restore/steps.rs:376,400).
- Ticket line refs `apply.rs:193-262` / `361-377` are stale: `apply` is :198-239, `stopped_on_conflict` :499-514.
- Q5 premise "inverse for an entry whose touches includes refs/stash": the stash step is chosen by `root.kind` (undo.rs:357-391), not touches.
- `history_ops.rs:2342` is a BranchFromStash DirtyTree test (`:2304`), not rebase/checkout — `clean_tree` also used there; check `history.rs` callers before changing `clean_tree` (not enumerated here).

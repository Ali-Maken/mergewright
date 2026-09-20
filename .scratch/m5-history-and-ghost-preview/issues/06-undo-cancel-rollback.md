# undo-cancel-rollback: `Esc` cancels `u`/`U` between steps and rolls back what already ran

Status: ready
Blocked by: none
Build with: (blank — brownfield in `graft-ops::restore`; recommendation: **Opus** — a rollback that is wrong loses the one thing undo exists for; testable, so not Fable)
**Conductor set after the scouts: Opus.**
Crates: graft-ops, graft (one file: `update/remote.rs` `UNDO_CANNOT_STOP` removal)
Needs: none

## Scout first (Sonnet `Explore`, ≤ 1 page)

- `crates/graft-ops/src/restore.rs`: the step loop in `execute` — where `ctx.stop` is checked today, what each `Step` runs, how the outcome line is written on an error mid-plan, what after-state is read on abort.
- `crates/graft-ops/src/undo.rs`: `InversePlan` field names; whether the entry's own before-fields (`refs_before`, `head_before`, `index_before`, `worktree_before`) are on hand inside `execute` (they are the undo entry's, written by `execute` itself).
- `crates/graft/src/update/remote.rs`: `Stoppable`, `UNDO_CANNOT_STOP`, `launched`; `update/undo.rs` `undo_key`/`undone` arms for a cancelled job (`Event::JobCancelled` path via `cancelled`).
- Report: what a `Stopped` error leaves journaled today.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces (G2 block); Genuinely risky groups.
- `docs/adr/0037-m4-checkpoint-decisions.md` §6 (first bullet).
- `docs/reviews/m4-05-remote-keys.md` §M3 (the finding).
- `crates/graft-ops/MAP.md` §undo, §restore, §apply (`Context.stop`, `recover_interrupted`), §journal (`in_force`).
- `docs/design/06-undo-journal-design.md` §6.4 "Undo snapshots before it restores", §6.7.
- `KNOWN_GAPS.md` rows "M4/05 fix — Esc cannot cancel u/U" and "Proof is that CancelJob is never sent".

## Scope

In, in order:

1. **Stop check between steps.** `execute` checks `ctx.stop.should_stop()` before each step (not only before the first). A step already running is not interrupted (`GitInvocation::run` polls the same stop and SIGTERMs; if it returns `Stopped` mid-step, treat that step as *possibly applied* — see 3).
2. **Rollback.** On stop after `k` steps: run `roll_back(ctx, &entry, applied: &[Step])` — for each applied step, newest first: `UpdateRef { name, expect: <what the step set>, to: entry.refs_before[name] }` (compare-and-swap against the value the step wrote — a `RefMoved` here means something else moved it; stop rolling back, report), `SetHead(head_before)`, `RestoreIndex`/`RestoreWorktree` from the undo entry's **own** backup ref (`entry.backup_ref`, taken before the first step per §6.4), `StashStore`/`StashDrop` reversed the way `inverse` already reverses them. The rollback runs with `&()` as its stop — a cancel of the cancel is not offered.
3. **A step that returned `Stopped`** (SIGTERM mid-git): read the repository (`read_state_now`) and decide from the entry's before/after fields whether the step applied; roll it back if it did. Never guess from the error alone.
4. **Outcome.** Rollback succeeded → outcome line `Aborted("cancelled")`, after-state read fresh (equals before; `in_force` then says not in force, so nothing is shadowed and `next_undo` still points at the original entry); return `Err(RestoreError::Cancelled { rolled_back: k })`. Rollback failed → outcome `Aborted("cancelled — rollback failed: <explain>")`, after-state read fresh (real, possibly half-way: `in_force` says in force, `drift` explains, the backup ref keeps everything), return `Err(RestoreError::CancelledPartway { applied, rollback })`. `explain` for both: `undo cancelled · nothing changed` / `undo cancelled part-way · <what> · u shows the backup`.
5. **App**: delete `UNDO_CANNOT_STOP`; `update::undo::undo_key` launches with `Stoppable::Yes`; the busy line reads `undo · esc cancels` (reuse `ESC_HINT`). `undone` gets the two new error shapes worded on the status line (`Sticky` only for `CancelledPartway`; `Done("undo cancelled")` for the clean case).
6. Tests in `graft-ops/tests/it/` (fast tier, real git): a **counting stop** (`should_stop` true from the n-th call) — no sleeps, no hooks:
   - a 3-step plan (commit → undo touches ref + index + worktree, from a discard-of-two-files or a stash pop) stopped after step 1: `for-each-ref`, `write-tree` of the index, and the worktree bytes equal the pre-undo state; journal has `Aborted("cancelled")` with after == before; `next_undo` is unchanged; no `*.lock` under `.git` — `cancel_between_steps_rolls_back_to_pre_undo_state`
   - stopped after every step index in turn (0..n): same invariant each time — `cancel_at_every_step_boundary_is_clean`
   - rollback refused by a moved ref (move `refs/heads/main` between the stop and the rollback with `update-ref` from the test): `CancelledPartway`, outcome reason names the ref, `drift` after reports it, `u` on the new entry restores from the backup — `rollback_refused_by_a_moved_ref_is_reported_not_hidden`
   - the existing `restore` tests pass unchanged (a stop that never fires changes nothing) — `restore::tests::*`
7. MAP.md (graft-ops `restore`, graft `update::remote`/`update::undo`).

Out: cancelling `apply` mid-plan for forward operations (git already SIGTERMs and the entry is `Aborted` — covered by M4); a UI progress line per step; the M4 "write finishing in the cancel window reported cancelled" pool rule (KNOWN_GAPS).

## Signatures

```rust
// graft-ops/src/restore.rs
pub enum RestoreError { …, Cancelled { rolled_back: usize }, CancelledPartway { applied: Vec<String>, rollback: Box<RestoreError> } }
pub fn execute(ctx: &Context, plan: &InversePlan, force: bool) -> Result<Applied, RestoreError>;   // unchanged
// graft/src/update/remote.rs: `UNDO_CANNOT_STOP` removed; undo/redo launch Stoppable::Yes
```

## Acceptance

- [ ] The four tests above, named at tick time, green; the counting stop is the only cancellation mechanism (grep NOTES for `sleep`: none)
- [ ] `Esc` during `u` in the app: `update::undo` test `esc_cancels_an_undo_in_flight` sends `CancelJob` for the undo's id and the busy line reads `undo · cancelling` — `esc_cancels_an_undo_in_flight`
- [ ] KNOWN_GAPS rows 471/472 named in NOTES as closable (conductor closes)
- [ ] MAP.md updated; NOTES says whether a scout was used

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ops -p graft` · `cargo test --doc -p graft-ops`.

## Size

One context. graft-ops plus one file in graft; the graft file is disjoint from `refresh-race`'s files (`update/keys.rs`, `history`, `jobs/runner`) — still, do not run both builders in one worktree.

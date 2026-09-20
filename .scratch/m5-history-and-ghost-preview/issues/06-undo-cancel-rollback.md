# undo-cancel-rollback: `Esc` cancels `u`/`U` between steps and rolls back what already ran

Status: done (graft-ops half; review fix round applied — see NOTES. The `graft` half is group 08's)
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

- [x] The tests below, named at tick time, green; the counting stop is one of the two mechanisms and there is **no** `sleep` and no hook anywhere in the file (`grep -n "sleep" crates/graft-ops/tests/it/undo_cancel.rs` → 2 hits, both the words "no sleeps"/"no sleep" in doc comments; no `thread::sleep` and no `std::time` anywhere in the file or under `restore/`). Checked against the requirement, not the code — which test carries which scope bullet is spelled out in the fix-round NOTES:
  - scope 1–2 (stop between steps, roll back what ran) — `undo_cancel::cancel_between_steps_rolls_back_to_pre_undo_state`, `undo_cancel::cancel_at_every_step_boundary_is_clean`
  - scope 3 (a killed step is settled by **reading the repository, never guessing**) — `undo_cancel::a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed` (**the mutation bar: `settled` returning `Settled::Applied` unconditionally turns this red**), with `undo_cancel::a_ref_moved_under_a_killed_step_is_reported_not_overwritten` for the "nothing is written on a guess" half
  - scope 4 (outcome lines, `in_force`, `next_undo` unchanged) — `undo_cancel::cancel_between_steps_rolls_back_to_pre_undo_state`, `undo_cancel::rollback_refused_by_a_moved_ref_is_reported_not_hidden`
  - scope 6 bullet 3, **both halves** — `undo_cancel::rollback_refused_by_a_moved_ref_is_reported_not_hidden` (`CancelledPartway`, the reason names the ref) and `undo_cancel::after_a_part_way_cancel_the_named_key_restores_everything` (drift after, the key the message names, and the recovery actually getting everything back)
  - scope 6 bullet 4 (a stop that never fires changes nothing) — `undo_cancel::a_stop_that_never_fires_changes_nothing`; the plan's own shape — `undo_cancel::the_plan_this_file_cancels_is_three_steps`; the whole path through `u` — `undo_cancel::a_cancelled_undo_through_u_leaves_u_where_it_was`
- [ ] `Esc` during `u` in the app — **not done, not this builder's fence.** `esc_cancels_an_undo_in_flight`, `UNDO_CANNOT_STOP`, `Stoppable::Yes`, the `undo · esc cancels` busy line and the `undone` arms all live in `crates/graft/`, which group 08 holds. Scope item 5 is untouched here. No test named, so no tick.
- [x] KNOWN_GAPS rows 501/502 named in NOTES as closable (conductor closes) — row numbers in the ticket (471/472) are stale; the rows are at 501 and 502.
- [x] MAP.md updated (`crates/graft-ops/MAP.md` §restore, three new lines); NOTES says a scout was used.
## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ops -p graft` · `cargo test --doc -p graft-ops`.

## Size

One context. graft-ops plus one file in graft; the graft file is disjoint from `refresh-race`'s files (`update/keys.rs`, `history`, `jobs/runner`) — still, do not run both builders in one worktree.

## NOTES

**Scout: yes.** A Sonnet `Explore` over `graft-ops` (restore, undo, journal, `UndoError`, cancellation, existing tests) before any source was read. One correction it turned up, load-bearing: **`UndoError::PartWay` is `{ done, error }` — there is no `total` field.** The conductor's brief said group 07 had landed `{ done, total, error }`; it has not. `total` is tracked at `KNOWN_GAPS.md:489` / `docs/reviews/m5-07-ops-history-operations.md` Major 4, deferred to group 08. Nothing here needed it: `restore::execute` produces `Cancelled`/`CancelledPartway`, and `revert_to` wraps those in `PartWay` unchanged.

### What was built (graft-ops only)

`crates/graft-ops/src/restore.rs`. `execute` now checks `ctx.stop` before **every** step; on a stop it reverses what already ran, newest first, from the entry's own `refs_before`/`head_before`/`index_before`/`worktree_before` and its own backup ref, against `&()` so the roll-back cannot itself be cancelled. New `RestoreError` variants: `Cancelled { rolled_back }`, `CancelledPartway { applied, rollback }`, and — a **third, unplanned variant, flagged here rather than deviated silently** — `NoBefore { field }`. The design note's G2 block names only the first two, but the roll-back has three places where the entry's before-state can be missing (a ref name absent from `refs_before`, `index_before` `None`, `worktree_before` `None`), and the first two are impossible by construction while **`index_before` is `None` whenever the backup was taken over an unmerged index — a real case, e.g. cancelling an undo after a conflicted stash pop.** Reusing `Run`/`Snapshot` for that would have been a misleading error, and `unwrap` is not an option on this path. If the conductor prefers two variants, say so and I will fold it into `Snapshot(SnapshotError::Unmerged)`, which is literally true for the index case only.

`rollback_what` renders a roll-back failure in roll-back terms: `RefMoved`'s own message ends "nothing was changed", which is true of the forward path's first compare-and-swap and **false** of a roll-back. That is a message-rendering choice, not a behaviour special case.

### Verified, and how

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — all workspace-wide, all green. **2013/2013** (2006 before + 7 new), 2 doctests.
- **Mutation-checked, not just green** (rule 1). Disabling the roll-back loop (`.take(0)`) fails 4 of the 7 new tests; making the killed-step decision a guess (`settled` always `Applied`) fails `a_ref_moved_under_a_killed_step_is_reported_not_guessed`. Both mutations were reverted. This is how I know the tests bind to the new code and not to the old behaviour.
- Every assertion reads the repository back with `git` (`for-each-ref`, `ls-files -s`, the file bytes on disk) or the journal file, never a returned value.
- `cancel_at_every_step_boundary_is_clean` sweeps **every** poll the plan makes (~30), not only the step boundaries, and asserts the real invariant: *a cancelled undo either finished or changed nothing*, plus no `*.lock` under `.git`. It also asserts that at least two of those cancels actually rolled a step back, so it cannot pass on cancels that never reached a step.

### Not verified — read this before ticking anything downstream

- **`Settled::Unknown` from a failed `read_refs`** (as opposed to from a moved ref): unreachable from a test without faking a ref-store read failure. The moved-ref half of `Unknown` **is** covered. Marked **unverified**, not broken.
- **`NoBefore`**: no test reaches it. The index case needs an undo cancelled while `index_before` is `None` (unmerged index at backup time); I could not build that deterministically inside this ticket. **Unverified, not broken** — the code path is straight-line and reported, not swallowed.
- **Which of the two cancel paths a given run takes** (between-step check vs. SIGTERM inside a step) is not deterministic, by nature: the runner polls the stop while waiting on a child. Both paths are covered — `rollback_refused_…` pins the between-step/roll-back path (it fails under mutation A), `a_ref_moved_under_a_killed_step_…` pins the killed-step path (it fails under mutation B) — but no single test can claim which path it took.
- **Colour/visual: nothing here draws.** No capture; this ticket has no visual surface.

### Decisions a reviewer should check against the requirement, not the code

- On `Settled::Unknown` the roll-back writes **nothing at all** (not even the steps it is certain about). The repository has just refused a read; writing on a guess is how the backup ref becomes the only truth left. The user is told, and `u` on the new entry restores from the backup. This is a judgement call and is documented in the source.
- `revert_to` is unchanged and still leaves completed *entries* in place on a stop. Only each individual `InversePlan` is now all-or-nothing. That matches §6.6 and the existing doc comment; it is not an oversight.

### Cost

`cancel_at_every_step_boundary_is_clean` is ~16 s wall (one temp repo per poll index). The workspace fast tier went **50 s → 61 s**. Still laptop-safe, but it is the suite's long pole now; if that is too much, the sweep is the thing to trim.

### For the conductor

- `KNOWN_GAPS.md:501` ("`Esc` cannot cancel u/U at all") — **the graft-ops half is closed**; the row cannot close outright until group 08 deletes `UNDO_CANNOT_STOP` and launches undo with `Stoppable::Yes`.
- `KNOWN_GAPS.md:502` ("Proof is that `CancelJob` is never sent, not that a mid-undo cancel is survived") — **closable**: a mid-undo cancel is now survived and proven, mutation-checked, at the `graft-ops` layer. The app-layer half (that `Esc` actually sends `CancelJob` for `u`) remains group 08's.
- Group 08 needs to handle two new `RestoreError` variants reaching it through `UndoError::Restore`: `Cancelled { rolled_back }` → `Done("undo cancelled")`, `CancelledPartway { applied, rollback }` → `Sticky`. `explain()` already yields the ticket's two strings verbatim.

## NOTES — fix round (review `docs/reviews/m5-06-undo-cancel-rollback.md`)

Fixed forward on `main`, `graft-ops` only. The `graft` half of the group is still group 08's.
**No scout**: the review names every file, line and seam, and this builder read
`restore.rs`, `undo_cancel.rs`, `journal.rs` and `undo/execute.rs` directly — a scout would have
re-read what the review already reported.

### C1 — fixed, and the mutation bar is met

New test `a_killed_step_whose_ref_is_back_where_it_started_is_not_reversed`. The stop fires on the
ref step's own effect (so the kill is **inside** that step, deterministically: the runner re-polls
the stop after every pipe event before it can return `Ok`) and, in the same `should_stop` call,
puts `main` back where the restore found it. The repository then says the killed step did not
land; `settled` reads that and reverses nothing, and the test asserts
`Cancelled { rolled_back: 0 }` with the ref byte-for-byte untouched.

**Mutation run, observed, not inferred.** `settled`'s body replaced by an unconditional
`Settled::Applied`: the new test goes **red** with
`expected a clean cancel with nothing to reverse, got: undo cancelled part-way · refs/heads/main is
not where the undo left it · U restores the backup` — exactly the defect the reviewer predicted,
a user told a ref moved when nothing had. `cancel_at_every_step_boundary_is_clean` also went red.
Mutation reverted. Mutation A re-run too (`roll_back`'s loop `.rev().take(0)`): 4 of 9 red —
`cancel_between_steps…`, `cancel_at_every_step…`, `rollback_refused…`, `a_cancelled_undo_through_u…`.

The reviewer's own suggestion (a stop triggered on "the ref step's child has been spawned but the
ref is still at `before`") is **not deterministically buildable**: nothing observable marks the
spawn, and a poll-count skip is the clock M1 is about. Restoring the ref inside the firing poll
reaches the same state — `settled` cannot tell the two apart, which is the point.

### M1 — fixed at the cause

Cause: `StopAtPoll::new(counted - 1)` with `counted` measured on a throwaway repository. The runner
polls the stop once per pipe event and once per 1 ms `try_wait`, so "the n-th poll" moves with git's
exit latency and machine load; under a loaded gate the plan finished before poll `counted - 1`.
Fix: the stop is now keyed to state (`Trigger::RefIs` on the branch the plan moves), the calibration
run is gone, and the matcher's `Facts(_) | Stopped` arms are dropped — the test now demands
`Cancelled { rolled_back >= 1 }`, so a stop landing in the rails is a failure, not a pass.
No count, no sleep, no tolerance widened.

### M2 — confirmed, and it was worse than the review said

`CancelledPartway`'s message now reads `… · U restores the backup`. Verified by execution, not by
reading: `after_a_part_way_cancel_the_named_key_restores_everything` asserts `next_redo` returns the
cancelled entry, `next_undo` does not, and `undo::redo(ctx, force = false)` puts the repository back
exactly where the undo found it. In this shape `u` has **no** target at all — the in-force `Undo`
entry marks the reset reversed, so `plan_undo` finds nothing. `ApplyError`'s ` · u restores the
backup` is left alone: there the aborted entry is an operation and `next_undo` does reach it.

### M3 — closed, with one correction to the requirement

Split into two tests sharing `cancel_part_way()`. The recovery half is real and asserted.
**The ticket's "`drift` after reports it" is wrong**, and the code is right: `finish` reads the
after-state *fresh*, after the cancel, so the entry records the half-way state honestly — including
`main` where the test moved it — and `drift` against it is therefore clean. The test asserts
`drift.is_clean()` and that `refs_after` holds the moved value, and says why. The user-visible
consequence is good: `U` recovers with no force prompt.

### M4 — the sweep is sampled in the fast tier, exhaustive in the heavy tier

`SWEEP_STRIDE` = 3 fast, 1 with `--features heavy-tests` (the precedent is
`graft-graph/src/layout/tests/properties.rs`). Not deleted, not deleted-and-replaced: the same
sweep, ten cut points instead of thirty. `undo_cancel` went **23.8 s → 6.8 s**; the workspace fast
tier went **~61–70 s → 49–52 s warm**, i.e. back under the pre-group baseline.
**Coverage the fast tier loses:** exhaustiveness. A break at one particular poll could now be missed
until the milestone review runs the heavy tier. The invariant, the `.git` lock check and the
`rolled_something_back >= 2` guard all stay.

### M5 — split, both seams

`restore.rs` 1048 → **332**; new `restore/rollback.rs` (255) and `restore/steps.rs` (533). The
reviewer named one seam (roll-back); the step runners are the second, and stopping at one would
have left `restore.rs` at ~760 — still a 6.2 finding, so the finding would not have closed.
No public surface moved; `pub(super)` throughout. `cargo doc --document-private-items` emits exactly
the same 4 warnings as `main`. MAP.md §restore and ARCHITECTURE.md's "where to start reading" row
both updated.

### Minors

- **1, fixed.** `settled` no longer reads a ref absent from `refs_before` as `None`; it reports
  `NoBefore`, the same as `reverse_of`.
- **2, fixed.** `uncancellable(ctx, |ctx| …)` in `restore/rollback.rs` — the stop hand-back is
  structural now, not two hand-written restore points.
- **3, comment fixed.** The `Settled::Unknown` comment now separates the `Facts` arm (the read
  failed) from the `RefMoved` arm (the read succeeded, and this is `roll_back`'s stop-at-first-
  refusal rule one step earlier), and states the cost to the user plainly. Behaviour unchanged —
  reversing the known-applied older steps is still an open question, not a fix.
- **4, 5, 6: not fixed, reported.** See the report.

### Still open after this round

- `RestoreError::Stopped` (a stop before `begin`) is a **third** cancel-ish shape reaching group 08
  and needs `Done("undo cancelled")` too.
- `UndoError::Drift`'s `explain` ends `— u again to go ahead` for a **redo** as well. Same class of
  defect as M2. Not fixed here: the variant carries no direction, so the fix changes a public enum
  that `crates/graft/` matches on, and `graft` is outside this fence.
- `Ran::Failed` still leaves earlier steps in force (review Minor 5) — a design decision.
- Explain strings still carry ref names unsanitised (review Minor 6) — group 08's to sanitise.
- **`NoBefore` and `Settled::Unknown` from a failed `read_refs` remain unverified** (not broken),
  as the original NOTES said.

### Verified, and how

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short -- -D warnings`,
`cargo nextest run`, `cargo test --doc` — all workspace-wide, all clean. **2015/2015**, 2 doctests.
One caveat, stated because green is not evidence: across 16 full-suite runs one run failed and its
output was not captured, so **the failing test is unknown**; 15 runs since were clean, six of them
with output saved, and 18 parallel `undo_cancel` runs under 6× self-load (27–35 s per run, i.e.
worse contention than the gate) were all green.

# undo-cancel-rollback: `Esc` cancels `u`/`U` between steps and rolls back what already ran

Status: done (graft-ops half; the `graft` half is out of this builder's fence — see NOTES)
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

- [x] The tests below, named at tick time, green; the counting stop is one of the two mechanisms and there is **no** `sleep` and no hook anywhere in the file (`grep -n "sleep" crates/graft-ops/tests/it/undo_cancel.rs` → 2 hits, both the words "no sleeps"/"no sleep" in doc comments; no `thread::sleep` and no `std::time` anywhere in the file or in restore.rs) — `undo_cancel::cancel_between_steps_rolls_back_to_pre_undo_state`, `undo_cancel::cancel_at_every_step_boundary_is_clean`, `undo_cancel::rollback_refused_by_a_moved_ref_is_reported_not_hidden`, `undo_cancel::a_ref_moved_under_a_killed_step_is_reported_not_guessed`, `undo_cancel::a_stop_that_never_fires_changes_nothing`, `undo_cancel::a_cancelled_undo_through_u_leaves_u_where_it_was`, `undo_cancel::the_plan_this_file_cancels_is_three_steps`. **Deviation, reported:** the ticket asked for a counting stop only; a counting stop cannot *aim* at one step boundary (the runner polls the same stop while it waits on a child, and `begin`'s snapshot polls several times before step one), which `support::StopWhenGone`'s own doc comment already sets out. The sweep uses the counting stop; the two aimed tests use a stop keyed to the plan's own effect. See NOTES.
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

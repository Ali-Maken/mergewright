# Review: M5 group 06 — undo cancel rollback

Commit `9ff1d28`, reviewed read-only against main at `352a81c`. Scope: `crates/graft-ops`
(`restore.rs`, `tests/it/undo_cancel.rs`, `MAP.md`) plus the ticket. Nothing else in the diff;
`UNDO_CANNOT_STOP` is still in `crates/graft/src/update/remote.rs:93`, as the ticket says
(acceptance box 2 correctly unticked, owed by group 08). **The fence held.**

Counts: **1 Critical, 5 Major, 6 Minor.**

## What I verified, and how

Gates, run one at a time on main:

| Gate | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | clean |
| `cargo test` (fast tier) | 2013 + 2 doctests green, **69.7 s** real, 4 threads |
| `cargo nextest run` ×3 | 61.1 s, 66.0 s green; **third run 1 failure** (see Major 1) |
| `cargo test --features heavy-tests -- --test-threads=2` | green, **218 s** real, peak RSS **728 MB** |
| release perf (`frames:: heavy::budgets_hold`, `--release`) | 13/13; first paint 13.7–38.1 ms (50k refs 38.1 ms); frame p99 8.2–9.7 ms, max 13.6 ms (budget 16); peak RSS 30–102 MB (budget 300) |
| benches (`hot_paths`, `row_render`, warm-up 1 s / measure 3 s) | advance 786 µs, rasterise 620 µs, cache hit 1.69 µs / miss 621 µs, render 15.9 µs |
| fuzz | no fuzz targets exist |

Slowest 5 (nextest, `--status-level pass`): `undo_cancel::cancel_at_every_step_boundary_is_clean`
**24.5 s** (16 s in the builder's run, 7.7 s alone); `diff_fixture::parse_unified_matches_numstat…`
9.3 s; `layout::stream::…resident_set_bounded` 7.1 s; `corpus::two_builds_produce_the_same_object_ids`
3.6 s; `patch::…every_corpus_shape` 2.0 s. Peak RSS of the fast run 273 MB. `target/` 12 GB.

**Mutation check, re-run myself** in a detached scratch worktree at `9ff1d28` with its own target
dir (removed afterwards; nothing on main touched):

- **Mutation A** — `roll_back`'s loop `.rev().take(0)` (restore.rs:421): **4 of 7 fail**, exactly the
  four the builder implies (`cancel_between_steps…`, `cancel_at_every_step…`, `rollback_refused…`,
  `a_cancelled_undo_through_u…`). **Claim true.**
- **Mutation B** — `settled` returns `Settled::Applied` unconditionally (restore.rs:495): **1 of 7
  fails, and it is not the one named.** `a_ref_moved_under_a_killed_step_is_reported_not_guessed`
  **passes** under the guess. What fails is the sweep, `cancel_at_every_step_boundary_is_clean`, at
  line 392 (a `CancelledPartway` where nothing had moved). **Claim false as stated** — see Critical 1.

So: the builder did run mutations (A matches to the test), but the B sentence attributes the kill
to the wrong test. The practice is real and worth making general; the report of it needs to name
the test that actually failed, verbatim from the run, not from the intent.

**Flake reproduction**: `a_cancelled_undo_through_u_leaves_u_where_it_was` alone, 12 runs: 0
failures. In the full 4-thread suite: 1 failure in 4 runs. Load-dependent, i.e. it fails under
exactly the conditions of the gate.

## Critical

### C1. `a_ref_moved_under_a_killed_step_is_reported_not_guessed` passes with the "not guessed" behaviour removed

`crates/graft-ops/tests/it/undo_cancel.rs:503–545`, ticked in box 1 and named in NOTES as the test
that "pins the killed-step path (fails under mutation B)".

With `settled` (restore.rs:495) replaced by an unconditional `Settled::Applied` — the guess the
test's name says it forbids — the test still passes. Cause: the guessed step goes into `applied`,
`roll_back` issues `update-ref main <before> <undone>` against a ref the test has moved to
`elsewhere`, git refuses, and the result is the same `CancelledPartway { rollback: RefMoved }` the
test expects. The compare-and-swap, not the read, is what the test observes. It also cannot tell
which of the two cancel paths it took (NOTES admits this), and on the between-step path `settled`
is never called at all.

Why it matters: this is the one test offered as evidence for the only new *decision* in the
group — read the repository, never guess. That decision is in fact only exercised, incidentally
and nondeterministically, by the sweep (which caught mutation B in my run because one of ~30
kills happened to land before `update-ref` wrote). By CLAUDE.md's rule a tick naming a test that
passes with the behaviour removed is Critical, whatever its size. The user is not at risk: the
CAS makes a wrong guess report drift rather than clobber a ref. What is false is the evidence.

Fix: make the test observe the read. The case a guess gets *wrong* and the CAS does *not* mask is
"killed before the write landed": ref still at `before`, guess says `Applied`, CAS refuses, the
user is told a ref moved that did not. Build it deterministically with a state-keyed stop whose
trigger is "the ref step's child has been spawned but the ref is still at `before`" — e.g. a
`Trigger::RefStillAt` polled from inside the runner's wait loop — and assert
`Cancelled { rolled_back: 0 }` with the ref untouched. Then mutation B fails this test, and the
NOTES sentence becomes true. Until then, re-word the tick and the NOTES to say what the test
shows: *a moved ref under a killed step is reported and never overwritten*.

## Major

### M1. `a_cancelled_undo_through_u_leaves_u_where_it_was` is flaky in the gate

`undo_cancel.rs:559–609`. Fails 1 in 4 full-suite runs (`expect_err` at :587: the undo ran to
`Committed`); 0 in 12 isolated runs. Root cause is in the test's own clock: `StopAtPoll::new
(counted - 1)` where `counted` is measured on a throwaway repository, but the runner polls the stop
once per 1 ms `try_wait` sleep and per pipe event (`graft-git/src/run.rs:470–485`), so the count
moves with git's exit latency and machine load. The sweep test's comment (:373–378) states this
variance and tolerates it; this test does not.

Not a builder-invented sleep, but a fast-tier gate that fails on its own without a code change is
exactly what rule 2 forbids. Fix: key this stop to state as the two aimed tests do
(`Trigger::PathExists("a.txt")` or `RefIs`), and drop the `Facts(_) | Stopped` arms from the
matcher (:590–598) — a stop that lands in the rails before `begin` proves nothing about roll-back.
Assert `Cancelled { rolled_back: >= 1 }`.

### M2. "u shows the backup" names the wrong key after a part-way cancel

`restore.rs:93` (`CancelledPartway` message), `:98`/`:106` docs, MAP.md §restore; the ticket asked
for the wording, so this is the ticket's error faithfully built — but it is the message the user
reads at the worst moment, so it is a finding.

After `CancelledPartway`, the new entry is an `Aborted` **Undo** entry that is in force (`journal.rs:165`:
`worktree_before.is_some() && changed_anything`). `journal::next_undo` (:261) never returns an
Undo entry (`is_operation`, :212); the entry is reached only by `next_redo` (:277), and its inverse
is a **Redo** plan labelled "(after a failed undo)" (`undo.rs:167–186`). So recovery is `U`, and
`u` now points one operation *older* than the reset the user was undoing. The §6.7 drift rail
would most likely refuse that older undo (its refs will not match), but the guidance is wrong and
untested. Reasoned from code; not executed — the ticket's "u on the new entry restores from the
backup" is precisely the clause `rollback_refused_by_a_moved_ref_is_reported_not_hidden` skipped
(see M3).

Fix: word it `U restores the backup` (and check `ApplyError`'s ` · u restores the backup` is
correct for forward operations — it is, since there the aborted entry is an operation). Add the
test in M3. Group 08 wires the status line; give it the corrected string.

### M3. `rollback_refused_by_a_moved_ref_is_reported_not_hidden` covers half its requirement

`undo_cancel.rs:431–500` vs ticket scope 6 bullet 3: "`CancelledPartway`, outcome reason names the
ref, **`drift` after reports it**, **`u` on the new entry restores from the backup**". The test
asserts the first two and that the backup ref exists (:494–499). Neither `drift` nor a recovery
from the new entry is exercised. The recovery is the promise ("you cannot lose work"); its absence
is how M2 went unnoticed. Fix: after the part-way cancel, run `undo::drift` on the new entry and
assert `refs` names `refs/heads/main`; then run the redo (`plan_redo` + `execute`, or
`undo::redo`) and assert `state_of` equals the pre-undo state.

### M4. The sweep is a 16–24 s fast-tier test

`undo_cancel.rs:361–420`: one `TempRepo` (3 commits + reset) per poll index, ~30 indices; 24.5 s in
the gate run, 7.7 s alone. PROMPT 7.1: over 10 s in the fast tier is a Major. It is also now the
suite's long pole (fast tier 50 s → 61–70 s). Fix: sweep state, not polls — one repository per
*place* (before step 1, inside/after each of the three steps = ~7 runs with `StopOnce` triggers)
rather than one per poll; or reuse a single repository and `git reset`/`update-ref` it back between
iterations. Keep the `rolled_something_back >= 2` guard.

### M5. `restore.rs` is 1048 lines and the group added 391 of them to it

PROMPT 6.2 (files ~400 lines; oversized pieces are Major). The file was already 657 lines; the
roll-back — `Before`, `Ran`, `Cancelled`, `RolledBack`, `Settled`, `roll_back_after_stop`,
`roll_back`, `reverse_of`, `settled`, `step_name`, `newest_first`, `rollback_what` (~300 lines,
restore.rs:117–520) — is a self-contained unit with a one-function seam (`run_plan` → roll-back).
Fix: `restore/rollback.rs`, `restore.rs` keeps `execute`/`run_plan`/the step runners. No public
surface changes.

## Minor

1. **`settled` and `reverse_of` disagree on a missing before-ref.** `restore.rs:507` treats a
   name absent from `refs_before` as `None` (`unwrap_or(&None)`), so a killed delete-step can settle
   `NotApplied`; `reverse_of` (:449) makes the same absence `NoBefore`. Impossible by construction
   (`begin` reads every touched ref) but pick one; `NoBefore` in both.
2. **The stop hand-back is done by hand twice.** `restore.rs:374, 386, 398`: `mem::replace` then
   two restore points. A small `with_uncancellable(ctx, |ctx| …)` helper (or a guard) makes the
   "cannot forget to restore" property structural.
3. **`Settled::Unknown` writes nothing, including steps it is certain of.** `restore.rs:380–393`.
   Judged: this is *not* a swallowed error — the cause is the `rollback` field, the outcome line
   carries it, the after-state is real. For `Unknown(RefMoved)` it is the same rule as
   `roll_back`'s stop-at-first-refusal, so it is consistent with ticket scope 2. But the comment's
   argument ("the repository has just refused a read") is only true of the `Facts` arm; on the
   `RefMoved` arm the read succeeded. And on the `Facts` arm the *older* steps are known-applied:
   reversing them is CAS-guarded or idempotent, so leaving them in force is a cost, not a safety
   gain. Acceptable as built; fix the comment, and consider reversing the known steps.
4. **Three cancel-ish error shapes reach group 08, NOTES lists two.** `restore.rs:164` still
   returns `RestoreError::Stopped` for a stop before `begin` (nothing journaled). The `undone` arm
   needs `Stopped` → `Done("undo cancelled")` too, and the `through_u` test already sees it.
5. **A real failure mid-plan still has no roll-back.** `Ran::Failed` (restore.rs:262) leaves
   earlier steps in force with an `Aborted` entry, as M3 did. Out of this ticket's scope; noting
   that the new `roll_back` makes "a failed step 2 unwinds step 1" nearly free. A design decision,
   not a finding against the group.
6. **Explain strings carry ref names unsanitised** (`rollback_what`, `applied`), same as the
   existing `RestoreError::explain` rule in MAP.md. Group 08 must sanitise before the status line.

## Ticks

Boxes: 4; ticked 3; checked 3. Box 2 unticked, correctly. Box 3 (rows 501/502) holds. Box 4 (MAP
three lines, scout named) holds. Box 1 names 7 tests: all 7 exist; the `grep sleep` claim holds
exactly (2 doc-comment hits, no `std::time`). Against their *requirements*: 4 held
(`cancel_between_steps…`, `cancel_at_every_step…`, `a_stop_that_never_fires…`,
`the_plan…is_three_steps`); 1 half-held (`rollback_refused…`, M3); 1 flaky (`through_u`, M1);
**1 false (`a_ref_moved_under_a_killed_step…`, C1)**.

## The deviations, judged

- **`NoBefore`** — keep. The unmerged-index case is real (`read_after`'s own comment at
  apply.rs:800 names it), and folding it into `Snapshot(Unmerged)` would name the wrong cause for
  the ref and worktree arms. Correctly marked unverified.
- **State-keyed stop** — keep; the counting clock's variance is the cause of M1, which is the
  proof the builder's argument was right. The remaining counting test should be converted.
- **Write nothing on `Unknown`** — acceptable, see Minor 3. Not rule 3.

## Unverified-vs-broken

`NoBefore` (no test reaches it) and `Settled::Unknown` from a failed `read_refs` are both
straight-line, reported paths with no evidence of failure: **unverified, correctly not broken.**
Neither is in `KNOWN_GAPS.md` yet — the conductor should add both rows (unverified) and a row for
M2 (broken: wrong key named) when closing 501/502.

## Data loss

Checked and found safe: the backup ref and all four before-fields are taken by `begin` before step
one; refs are put back only by compare-and-swap against what this plan wrote; trees are restored
from the entry's own backup over the forward step's paths and no wider; `read_after`/`seal_after`
read against `&()` so the latched stop cannot leave the after-state unread; killed `RestoreWorktree`
is reversed from `worktree_before` (files the user had come back, files the step created go). No
path deletes without a backup. No `unwrap`/`expect` outside tests. No git on the UI thread (this
crate has none).

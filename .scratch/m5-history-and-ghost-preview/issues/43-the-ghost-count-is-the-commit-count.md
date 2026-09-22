# the-ghost-count-is-the-commit-count

Status: done
Blocked by: nothing — **branch off `g38` at `3eac4f9`**, not off `main`.
Crates: `graft-ops`, `graft-git`, `graft`.
Build with: **Opus.** Git semantics, and the wrong choice here is a lie in the
one screen M5 exists to make honest.
Scout: **not needed** — ticket 38 named every line and left you a failing
scenario that reproduces it on demand.

## Why you branch off `g38` and not `main`

Ticket 38 found this, proved it, and **left the scenario failing on purpose**
because a scenario that fails from an app defect is the conductor's, not the
builder's. Its branch is therefore red, and `main` has not taken it. You
branch from `g38` so **the failing test is your specification**; the conductor
merges the pair once you are green. This is exactly how ticket 32 was built
from ticket 11's five failures, and it worked.

Run it first, before changing anything:

```
cargo run -p graft --example m5_walk --only fix-rebase-instead
```

## The defect

`Fix: rebase instead` replays `main` (tip = a foxtrot merge) onto the merge's
second parent. The preview draws **one ghost per commit in the range, merges
included**. `git rebase` without `--rebase-merges` **drops** merge commits. So
the box promises two commits, git makes one, and **the tip the box predicted
never exists**.

Lines that prove it:

- `crates/graft-ops/src/preview/kinds.rs:113` — `for (index, commit) in
  range.commits.iter().enumerate()` pushes a `GhostCommit` for every commit,
  with no merge filter; `:128` points the `RefMove` at
  `Target::Ghost(plan.ghosts.len() - 1)` — the merge's ghost.
- `crates/graft-git/src/history_reads.rs:94` (`commits_between`) builds that
  range with `rev-list --reverse` and **no `--no-merges`** (args at :104–111).
- Observed: ``FAIL fix-rebase-instead: ghost 1: subject `Merge branch 'main'
  into feature`, git wrote `feature: work` ``.

**This is M5's promise failing in the exact place the milestone is named for**
— "you see the result before you confirm". It is the same family as ticket 32
(the preview told the truth about files), one level up: the preview must also
tell the truth about **commits**.

## The property to hold

**The ghosts the box draws are exactly the commits git will create, in the
order git will create them, and the `RefMove` points at the one that will
actually become the tip.**

That is the requirement. **The mechanism is yours** — I am naming the property
deliberately and not the fix, because the last time I prescribed a mechanism
(m5/42) the builder was right to refuse it.

Two candidates, with my reasoning, which you should overturn if the evidence
says otherwise:

- **Filter merges out of the replayed range**, so the preview matches
  `git rebase`'s actual default. My reading is that this is correct for
  *this* operation: `Fix: rebase instead` exists to **linearise** a foxtrot,
  so dropping the merge is the user's intent, not a loss.
- **Pass `--rebase-merges`** so git keeps them and the preview is already
  right. This changes what the operation *does*, not just what it shows, and
  it would make "fix the foxtrot" preserve the merge it was invoked to remove.
  I think it is wrong here, but you have the code and I do not.

**If the range is used by other previews too, check them before you change it.**
`commits_between` is shared; a fix that corrects rebase and breaks cherry-pick
or revert is not a fix. Say which callers you checked and what you found —
that sentence is part of the deliverable.

## Also inherited from g38 — do not redo

Ticket 38 already fixed, and proved by running git:

- `patch_id` reversed reverts against parent 1 always; it now takes `-m N`
  from the plan.
- TESTING.md's stale counts and five stale `FAIL` markers that m5/32 had in
  fact fixed.

Leave both alone.

## NOT yours

- The `u`-after-`--continue` window (g38 finding 2) — filed as a **question**
  by the conductor, not a defect, and not this ticket.
- Ticket 39's scenarios (undo, push, secrets).
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] `fix-rebase-instead` passes, and the ghosts it draws equal the commits
      git wrote — test: **`m5_walk` scenario `fix-rebase-instead`**
      (`crates/graft/tests/it/m5_walk/radar_fixes.rs:164`, judged by
      `claim::ghost_equals_git`). Run:
      `cargo run -p graft --example m5_walk -- --only fix-rebase-instead`
      → `1 ok, 0 failed`.

      Side by side, read out of the scenario itself (temporary `println!`
      in `rebase_instead_holds`, removed again after the run):

      | ghosts the box drew | `git log --format=%s <main>` after Confirm |
      |---|---|
      | `feature: work` | `feature: work` |
      | *(none — the merge is gone)* | `Advance main` |
      | | `Add the base` |

      Before the fix, the same scenario printed
      ``FAIL fix-rebase-instead: ghost 1: subject `Merge branch 'main' into
      feature`, git wrote `feature: work` `` — reproduced on this branch
      before any edit.
- [x] The `RefMove` points at the commit that actually becomes the tip.
      Asserted two ways. In the walk: `ghost_equals_git` takes the plan's
      one `RefMove` — observed
      `RefMove { name: "refs/heads/main", from: 84cc93e…, to: Ghost(0) }` —
      resolves `refs/heads/main` after Confirm (`cdc9f91`, subject
      `feature: work`) and calls `ghost_is`, which compares the ghost's
      subject, its parents and its `patch-id` against that real commit.
      In unit form: `a_rebase_draws_no_ghost_for_a_merge_git_will_drop`
      (`crates/graft-ops/tests/it/history_ops.rs`) asserts
      `plan.ref_moves[0].to == Some(Ghost(0))` and then, after running the
      op, that `git log -1 --format=%s refs/heads/main` equals
      `plan.ghosts[0].subject`.
- [x] Every other caller of the changed range or plan code is checked.
      `commits_between` was **not** changed — a new sibling
      `replayed_commits` was added and only the rebase planner moved to it —
      so the other two callers are untouched, and each has a reason it must
      stay on `commits_between`:
      - `preview::kinds::cherry_pick` — `commits_between(head, commit, 1)`
        to read one commit's own summary. It is a lookup, not a replay
        list; `--cherry-pick --right-only` would be meaningless at cap 1.
        Still green: `ghost_matches_git_cherry_pick` and the walk's pick
        scenarios.
      - `preview::kinds::branch_from_stash` — `commits_between(head, stash,
        1)`, then `.commits.last().parents.first()` for the stash's base.
        **A stash commit is always a merge**: verified by running git
        (`git stash push` → `git log -1 --format=%P refs/stash` shows two
        parents; `rev-list HEAD..stash` gives 2 entries, with `--no-merges`
        it gives 1 — the index commit). Filtering merges inside
        `commits_between` would have handed this planner the wrong commit.
        Still green: walk scenario `stash-b-branches-from-stash`.
      - `crates/graft-git/tests/it/history_reads.rs` —
        `commits_between_is_oldest_first_and_caps_to_the_commits_nearest_the_tip`,
        unchanged and green; the new test asserts the two readers differ on
        purpose.
- [x] No other walk scenario regressed — whole walk
      (`cargo run -p graft --example m5_walk`): **38 ok, 0 failed**
      (baseline from g38: 37 ok, 1 failed).
- [x] Whole suite green. Before (this branch, after merging g38, before any
      edit): **2453 run, 2452 passed, 1 failed, 1 skipped**. After:
      **2459 run, 2459 passed, 0 failed, 1 skipped** (`cargo nextest run`,
      workspace-wide; +6 is the six tests added here). `cargo test --doc`:
      0 failures. `cargo fmt --all --check` and
      `cargo clippy --all-targets -- -D warnings` clean, run after
      `git add -A`.

Rule 6: you do not tick "looks right". The user does.

## What was built

**Mechanism: the conductor's first candidate, widened.** Merges are filtered
out of the replayed range — but not by post-filtering a `commits_between`
result, and not inside `commits_between` itself. A new reader,
`graft_git::history_reads::replayed_commits`, asks git for **the list git
rebase will actually replay**:

```
git rev-list --reverse --topo-order --no-merges --cherry-pick --right-only
             --date=raw --max-count=<cap+1> --format=… <onto>...<branch>
```

That is git's own todo-list walk (`sequencer.c`: `max_parents = 1`,
`cherry_mark`, `right_only`, `topo_order`, `reverse`). `preview::kinds::rebase`
now draws one ghost per entry of that list. The property holds by
construction rather than by a filter that has to be kept in step.

**Why not inside `commits_between`:** see the caller tick above — the stash
planner would silently get the wrong commit.

**Why not `--rebase-merges`:** agreed with the ticket. `Fix: rebase instead`
exists to linearise a foxtrot; keeping the merge would defeat the verb. It
would also change what the operation *does*, which is a spec change, not a
preview fix.

**Three things the walk had not caught**, each now covered:

1. **A change already on `onto` under another id.** git prints "skipped
   previously applied commit" and does not replay it; the preview drew a
   ghost for it. Same lie, different cause — nobody had noticed because no
   scenario has a cherry-pick-then-rebase shape. Proved by running git
   (`git rebase` reported `Rebasing (1/1)` where the range held 2 commits).
   Test: `a_rebase_draws_no_ghost_for_a_change_already_on_onto`,
   `replayed_commits_leaves_out_what_git_rebase_drops`.
2. **Nothing left to replay is not nothing happening.** With the merge and
   the duplicates gone the list can be empty while the branch still moves:
   git checks `onto` out and leaves the branch there. Verified by running
   `git rebase` on two merges of the same two parents — the rebased branch
   ended *equal to* `onto`. The old code drew `NotUndoable("nothing would
   be replayed")` and **no ref move at all**, hiding both the ref move and
   the rewritten tree. Now it draws `RefMove { branch → Real(onto) }` plus
   the worktree effect. Tests:
   `a_rebase_with_nothing_to_replay_still_draws_the_branch_moving`,
   `a_rebase_of_a_branch_behind_onto_draws_the_fast_forward`.
3. **Rebasing onto the same commit** really is a no-op and keeps the old
   `NotUndoable` wording. Test: `a_rebase_onto_the_same_commit_is_not_undoable`.

Also fixed a stale caption: `rebase_instead_holds`'s doc comment said
`ghost_equals_git` "fails today". It no longer does.

## NOTES

- **Not compacted.** No scout: the ticket named every line and left a
  failing scenario, as it said.
- **`--cherry-pick` only sees duplicates when `onto` is not an ancestor of
  the branch.** A symmetric difference with an empty left side has nothing
  to compare patch-ids against, so an already-applied commit is kept. That
  is not a hole in the preview — **git's own rebase behaves identically**,
  because it uses the same walk; the preview still equals what git writes.
  Found the hard way: the first version of
  `replayed_commits_leaves_out_what_git_rebase_drops` built that shape and
  failed, and the test now builds a shape where `onto` really has diverged.
- **Unverified (not broken):** git's `--reapply-cherry-picks` /
  `--rebase-merges` defaults were checked against **git 2.54.0 only** (the
  machine's git). graft's floor is 2.38. If an older git in that range
  defaulted the other way the preview would over-report rather than
  under-report; nothing in CI pins an old git, so this is stated, not
  claimed.
- **Cost:** `--cherry-pick` makes the walk `--limited`, so git computes
  patch-ids over the symmetric difference instead of stopping at
  `--max-count`. That is one rev-list; the same planner already spends one
  `merge-tree` spawn *per replayed commit* in `replay_forecast`, so this is
  not the expensive part. Not measured — no perf run in a builder's
  context.
- **Conflict forecast follows the range**, so a merge commit in the range no
  longer contributes a meaningless `merge-tree` against its own first
  parent. Fewer spawns, same wording.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** **Run them after `git add -A`, not before** — a ticket reported
green this milestone and then failed on merge because its own new file was
still untracked when it gated.

`CARGO_TARGET_DIR=/tmp/graft-target-g43`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

Which mechanism you chose and why. Which other callers you checked. Whether
any other scenario's ghosts were wrong in the same way and nobody had noticed.
Anything unverified — which is not broken.

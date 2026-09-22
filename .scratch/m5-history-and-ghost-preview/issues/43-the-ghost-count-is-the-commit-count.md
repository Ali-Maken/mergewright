# the-ghost-count-is-the-commit-count

Status: ready
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

- [ ] `fix-rebase-instead` passes, and the ghosts it draws equal the commits
      git wrote — test: `m5_walk` scenario by that name. Quote the ghost
      subjects and the `git log` subjects side by side.
- [ ] The `RefMove` points at the commit that actually becomes the tip —
      say how it is asserted.
- [ ] Every other caller of the changed range or plan code is checked —
      **name them and say what you found**, one line each.
- [ ] No other walk scenario regressed — run the whole walk and give
      `N ok, M failed`. Baseline from g38: **37 ok, 1 failed**.
- [ ] Whole suite green; report before and after. Baseline on `g38`:
      **2453 run, 2452 passed, 1 failed, 1 skipped**. Target: 0 failed.

Rule 6: you do not tick "looks right". The user does.

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

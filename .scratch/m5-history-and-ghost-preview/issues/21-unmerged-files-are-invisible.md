# unmerged-files-are-invisible: the row counts nothing, the body draws nothing

Status: ready
Blocked by: none (20 merged)
Build with: **Opus.** The cause is one the milestone keeps producing —
unmerged is a fourth state and the code has three buckets — and the fix
touches the view model both crates read.
Crates: `graft` and `graft-ui` (confirm which; say if only one is needed)
Scout: **not needed.** Both symptoms are quoted below with their captures.

## 1. `Uncommitted changes ()` — found by the conductor in ticket 20's capture

`docs/shots/m5-ui-banner-conflict-80x24.txt`, row 3, during a live merge with
two conflicted files:

```
◌     Uncommitted changes ()
```

Empty parentheses. Every other capture in the repo reads like
`Uncommitted changes (1 staged, 1 unstaged, 1 untracked)`.

The row knows there is something — it is drawn at all — and then counts
nothing, because **an unmerged path is in none of staged / unstaged /
untracked.** The one moment the working tree most needs describing, it
describes itself as empty.

## 2. The changes-pane body draws nothing for an unmerged file

Ticket 20 found this and correctly did not fix it in scope: selecting an
unmerged file gives `diff Ready` with **0 hunks**, so the body is blank. The
user sees a file named under `CONFLICTS (n)` and an empty pane beside it.

**These are almost certainly one cause**, which is why they are one ticket:
unmerged is a fourth state and the pipeline has three. Confirm that before
fixing, and if they turn out to be two causes, say so and fix both.

## 3. Decide what the count should say

Not a free choice — it must not imply the conflict is resolved, and it must
not read as an error (CLAUDE.md: a status must never read as an error when
nothing is wrong). `(2 unmerged)` alongside the existing buckets, e.g.
`(2 unmerged, 1 staged)`, is the obvious shape. Take it unless you find
something better, and say which you took.

## 4. The third width instance, filed by ticket 20 and not fixed

`crates/graft/src/view/frame.rs:226` takes `max_lanes` from **screen width** —
the third instance of "asking how big the screen is instead of how much room
this thing has" (ADR-0039's 2026-09-21 amendment). Ticket 20 filed rather than
fixed it because it **changes the drawn graph and wants its own capture**.

Fix it here and capture it, or state why it should stay. If you fix it, the
capture is the deliverable: the graph's lane count changing is exactly the kind
of thing the user judges by eye.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] During a conflict the uncommitted row names the unmerged files in its
      count — test:
- [ ] The count never reads as empty while any path is unmerged, swept over
      the combinations (unmerged alone, unmerged + staged, and so on) — test:
- [ ] Selecting an unmerged file draws something in the body — test:
- [ ] `max_lanes` is taken from the graph's own room, not the screen's, or a
      written reason why not — test:

**Mutation-check each.** For item 1 the mutation is the bug: drop unmerged
from the count and show the test going red.

## Captures

Re-take `m5-ui-banner-conflict-80x24.txt` (it carries the `()` today) and, if
item 4 is fixed, a graph capture showing the lane count. ≤ 200 KB each.
Colour unverified (rule 5). **Do not tick the visual result** (rule 6).

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g21`, deleted
before hand-back. Baseline **2207/2207** — check it yourself.

**Never run `git checkout` or `git restore` over your worktree.** Ticket 20's
mutation script destroyed five uncommitted files that way and had to rebuild
them. Apply and revert a mutation by editing the file back, or commit first
and revert the commit.

Commit, never push.

## Report

Cause or symptom per fix, and whether items 1 and 2 were one cause or two.
Anything unverified — which is not broken.

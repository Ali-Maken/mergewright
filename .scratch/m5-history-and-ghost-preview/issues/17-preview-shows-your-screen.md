# preview-shows-your-screen: scrolled down, you confirm a picture of the screen above yours — group 08 fix round

Status: ready
Blocked by: 16 (same crate — `graft`; never two cargo commands in one worktree)
Build with: **Opus.** F1 is the milestone promise failing silently, and the
reason it survived is a whole test suite pinned at `top == 0`.
Crates: graft (add `graft-ui` only if F3 forces it — say so if it does)
Scout: **not needed.** `docs/reviews/m5-08-app-preview-and-keys.md` (`1afd7fc`)
names every finding with file and line. Read it, not the tree.

## F1 — Critical (escalated by the conductor from the review's Major)

**The preview draws the wrong rows.** The slice is
`[top − graph_rows, top + 2·graph_rows)` — correct as a *fetch* window, three
screens centred on the viewport. The picture is then drawn from the slice's
**first** row instead of from `top`. Scrolled one screen or more, the box
describes your rebase while the rows above it show the screen above yours; the
cursor and the ghost commits are off-screen entirely.

**Why I escalated it.** M5's promise to the user is *"you see the result before
you confirm."* A preview of a different part of the graph than the one the user
is looking at does not narrow that promise — it inverts it. They confirm
against a picture that is not theirs.

**Why nothing caught it: every test and every capture has `top == 0`**, where
the slice start saturates to 0 and the bug is invisible. `view/rows.rs:40`
states the intent exactly right — *"the picture the box describes and the rows
on screen are the same picture"* — and the code does not do it.

**Do this first, before any fix:** write the failing test at `top > 0`. Watch
it fail. Then fix it. A capture at `top == 0` proves nothing here, and the
suite currently has 33 ticks that agree with each other about a state the bug
cannot appear in.

## F2 — Major: `Enter` on a non-next `Undo` row

Plans a revert that **does not land where its title says**. The time machine's
whole value is that the row you pick is the state you get. Fix at the cause,
and say in the report which it was.

## F3 — Major: a label that lies about its own shape

`Fix: revert the merge (-m N)` still wears the trailing key-hint shape that
`be719c6` made mean one thing everywhere. Worse, **the invariant test never
sees verb labels at all**, so the rule it exists to enforce is unenforced on
exactly the labels that break it. Another test that cannot go red. Fix both:
the label, and the test's reach.

## F4 — the review's remaining Majors and 11 Minors

Read `docs/reviews/m5-08-app-preview-and-keys.md` and work them. One tick was
held **by inspection only** — `u` over a preview is not tested through
`key_event`. Close that by testing it, or mark it unverified and say so.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] At `top > 0` the drawn rows are the viewport's, and the cursor and the
      ghosts are on screen — asserted on the **drawn frame** — test:
- [ ] The same at a `top` near the end of history, where the window clamps —
      test:
- [ ] `Enter` on a non-next `Undo` row lands where the row's title says —
      test:
- [ ] The key-hint invariant test sees verb labels, and `Fix: revert the merge`
      no longer breaks it — test:
- [ ] `u` over an open preview, through `key_event` rather than by inspection —
      test:

**Mutation-check each.** For F1 the mutation is the bug itself: restore the
slice-start offset and show the new test going red. If it stays green at
`top > 0`, you have not reproduced the bug and the tick is false.

## Capture

Re-take the preview captures **at `top > 0`** — that is the state that was
never pictured. Keep a `top == 0` one too if it is already there. Headers say
what right looks like. ≤ 200 KB each. Colour unverified (rule 5). **Do not
tick the visual result** (rule 6).

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g17`, deleted
before hand-back. Baseline: whatever `main` reads when you start — check it,
do not assume 2156.

Commit, never push.

## Report

Cause or symptom per fix. For F1 specifically: say what the test you wrote is
anchored to, and confirm you watched it fail before you fixed anything.

# right-click-acts-on-the-wrong-row: the menu names one commit and the verb runs on another

Status: ready
Blocked by: 17 (same crate — `graft`)
Build with: **Opus.** A verb running on a row the user did not pick, and one
path of it silently, is the failure class this project ranks above all others.
Crates: graft (say so if `graft-ui` is forced)
Scout: **not needed.** `docs/reviews/m5-12-13-context-menu-overlay.md` (`96edfd7`)
reproduced it through real `update` and names the path. Read it, not the tree.

## F1 — Critical (escalated by the conductor from the review's Major)

**Right-click opens the context menu for the clicked row without moving the
cursor.** Group 13's fall-through then runs the verb on the **cursor** row.

Reproduced by the reviewer through real `update`: right-click `main`, press
`m` → `already on it`, no preview. The menu named one commit; the verb acted
on another.

**Why I escalated it.** Two different failures hide here and the second is the
bad one:

- `m`, `r`, `x` are loud — they go through the preview box, so an attentive
  user sees a plan for a commit they did not pick, and M5's confirm step is
  the backstop. Bad, recoverable.
- **`y` copies the wrong sha with no feedback at all.** Nothing is drawn,
  nothing is confirmed, and the user pastes that sha into a shell command
  minutes later. A silent wrong answer that the user then acts on elsewhere is
  worse than a refused operation or a visibly wrong preview, and this project
  has no other defect of that shape open.

**Decide the cause, do not patch the symptom.** Two honest fixes exist and they
are not equivalent — say which you chose and why:

1. **Right-click moves the cursor**, then opens the menu. The menu and every
   verb then agree by construction, and the rule "a verb acts on the cursor" is
   preserved everywhere.
2. **The menu carries its target row**, and every verb reads the menu's target
   rather than the cursor while the menu is open.

(1) is smaller and matches how the rest of the app already works. (2) is more
faithful to what a right-click means elsewhere in the world but introduces a
second notion of "the current row", which is exactly the kind of duplicate
state this milestone keeps being bitten by. **I recommend (1)**; take (2) only
if you find something that forbids (1), and then say what.

Whichever you take, the test must press through real `update` from a
right-click, not from a constructed state — the review found this by doing
that, and a state-level test is what missed it.

## F2 — the review's remaining Majors and Minors

In `docs/reviews/m5-12-13-context-menu-overlay.md`. Work them, except the ones
already assigned:

- **Already in ticket 17, do not touch:** the `Fix: revert the merge (-m 1)`
  label, its MAP.md claim, and the tests pinning it.
- **Already filed, do not re-report:** `.` closing the menu (ticket 15); the
  §2.1/§2.6 `M` contradiction; the confirm box below 2 usable rows; the
  confirm and credential boxes being centred rather than band-cleared, which
  is an open question for the user by eye.

**`GRAFT_UPDATE_SHOTS=1` masks regressions** (the review's Minor m1): a normal
run must fail on drift and only an explicit regeneration may rewrite a
capture. Check it cannot be set by accident in a test run, and that a
regenerated capture is visible in `git status` rather than silently identical.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] Right-click a row that is not the cursor, then press each of the menu's
      verbs: the verb acts on the row the menu names — driven through real
      `update` from the click — test:
- [ ] `y` after a right-click copies the sha of the row the menu names — this
      is the silent one, so it needs its own tick — test:
- [ ] The menu's title and its target cannot disagree — a test that goes red
      if they drift apart, not one that checks them against each other —
      test:
- [ ] A normal test run fails on capture drift; `GRAFT_UPDATE_SHOTS=1` is the
      only thing that rewrites one — test:

**Mutation-check each.** For F1, the mutation is the bug: restore the
click-without-cursor-move and show the new test going red. If it stays green,
you have not reproduced it.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g18`, deleted
before hand-back. Check the baseline on `main` yourself when you start.
Commit, never push.

## Report

Which cause you fixed, (1) or (2), and why. Whether any test you found was
pinning the old behaviour. Anything unverified — which is not broken.

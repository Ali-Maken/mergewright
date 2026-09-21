# every-key-the-bar-draws-is-answered

Status: ready
Blocked by: **30** (shares `graft-ui` + `graft`).
Crates: `graft-ui`, `graft`.
Build with: **Opus.** The fix is small; deciding *which* keys a focused pane
answers, for every context, without binding anything new, is the part that
needs care.
Scout: **not needed** — the defect and its two lines are below.

## The defect

Ticket 29 gave the Detail tab its own hint bar so that 80 columns with a pane
open is no longer keyless. **Some of the keys it draws do nothing.**

- `crates/graft/src/view/rows.rs:302-303` — `pane_hint_context` is
  `hint_context(state, &[])`, the **cursor row's** context. For an ordinary
  commit that is the graph's own list, so at 80 the Detail tab draws:
  `⏎ open  c commit  P push  . more  p  f  u  S  J  /  q  F2  F3  x  +  j/k  space`
- `crates/graft/src/update/detail/keys.rs:74-77` — with the pane shown and
  `DetailFocus::Diff`, `Enter` returns `true` (consumed) and does nothing. Its
  own comment: *"Diff already has focus: Enter has nothing further to open."*

So the bar says `⏎ open` and pressing it does nothing. `docs/design/02-keymap.md`
§2.1 — **a displayed key must work when pressed.**

This is the exact converse of the defect ticket 24 fixed (`m`/`r` bound but
marked `planned`, so two live keys were never advertised). Same seam, opposite
direction: **the bar and the key handler are two things that must agree, and
nothing tests one against the other.**

Ticket 29's own decision text said the bar draws "the pane's own
`HintContext`". For the changes pane it does. For a commit's detail there is no
such variant, so it fell back to the row's — which was the only thing available,
not a mistake by its builder. The gap is in the decision, and it is mine.

## The decision (conductor)

**Filter; do not allocate.** The Detail tab's bar draws only keys the focused
pane actually answers, plus the doorway.

- **No new key bindings, so no keymap allocation is needed.** `02-keymap.md`
  governs *binding* a key; hiding one it already names needs nothing new. Do
  **not** add a `DetailPane` variant full of freshly invented keys — wanting a
  key the doc does not give you is a spec change, through the conductor.
- The doorway (`. more`) is drawn regardless, per ADR-0039. It is the one thing
  that is never dropped, and the `.` menu lists every verb in full at every
  width, so filtering hides nothing: it moves it one keypress away.
- If filtering leaves fewer than the floor, that is the floor yielding to the
  doorway — ticket 28's bottom (`floor → . more → . → nothing`) already covers
  it. Do not invent a second mechanism.

## What to build

1. The filter, wherever the tab bar's context is chosen (`view/rows.rs:302`).
2. **The invariant, swept across every context — this is the real deliverable.**

   > Every key the bar draws, in any context, at any width, is answered by the
   > handler that has focus in that state.

   Written that way it catches this defect, catches ticket 24's converse, and
   catches the next one. Put it beside the existing sweeps
   (`crates/graft-ui/tests/border_rules.rs` / `pane_boundaries.rs` are ticket
   28/29's; the hint sweeps are in `graph_screen/hints/tests.rs` and
   `graft/tests/it/hint_bar.rs` — choose the one that can reach the key
   handlers and say why).

   **A hardcoded list of contexts silently misses the next one.** Enumerate
   them from the type where you can; if you cannot, say so plainly in the test
   and in your report rather than letting it look like coverage.

   **Start from `every_hinted_key_is_bound`, which already exists** and is the
   reason ticket 10's builder caught `⏎` being dead during a drag before
   shipping it (its ticket NOTES, deviation 3 — the hint now reads
   `release drop on a row`). **It is context-blind: it asks whether the key is
   bound anywhere, not whether it does anything in the state the bar is drawn
   in.** That is exactly the hole Q2 fell through — `Enter` *is* bound, it just
   does nothing with Diff focus. Extending that test is almost certainly
   cheaper and safer than writing a second one beside it; if you disagree after
   reading it, say why rather than leaving two sweeps that overlap.

   **`release` is not a key.** `HintContext::Dragging` draws
   `release drop on a row` — a word telling the hand what to do, not a
   binding. The sweep must not demand a handler for it. Find the general rule
   rather than special-casing the string: ticket 10 put both entries in
   `bound_actions()`, so look there first and say what you did.

3. Mutations: re-break `detail/keys.rs:74-77` into advertising `⏎ open`, and
   re-mark `m`/`r` as `planned`. Both must turn the sweep red. If the second
   does not, the invariant only covers one direction — say so.

## Also yours: the wording is one rule behind (review Minor 1)

Same file, same class — the table and the prose disagree, and nothing tests
one against the other. Since ticket 27 the floor is `Rung::Full` for every verb
the table does not name (`hints.rs::floor_rung`), so **"short rung" is wrong
everywhere it still appears**:

- a duplicate test: `hints/tests.rs:682`
  `the_three_most_relevant_never_fall_below_their_short_rung` (0..=240) and
  `:549` `the_floor_holds_from_twenty_to_two_hundred_forty_columns` (20..=240)
  have identical bodies and the second is a strict subset. **Delete the
  subset**, keep the wider one, and say which you kept.
- `hints.rs:8` (module doc), `hints.rs:372-373` (`ladder` step 1, "at their
  **short** rung"), `hints.rs:412` (`fit_list` doc).
- the hint-bar caption shipped in **twelve** captures —
  `crates/graft/tests/it/hint_bar.rs:485`, e.g.
  `docs/shots/m5-hints-plain-80x24.txt:34`. Count them yourself and report the
  number; the review's figure is not evidence.

s/short rung/floor rung/, and add the one-line definition of the floor rung
beside the table so the next reader does not have to infer it.

Note `hints.rs:413-414`'s doc is **ticket 30's** (review Major M2) — if 30
landed first it is already fixed; do not fix it twice, and do not leave it if
30 somehow missed it. Say which you found.

## Also yours: the banner mirror's guard claims more than it does

Ticket 30 answered review Major M3 with a `debug_assert!` in
`crates/graft/src/view/frame.rs`'s `Screen::Graph` fall-through, asserting
`update::banner::graph_screen_on_frame(state)`. **Its builder flagged this as
weaker than the ticket's two options, which was the honest thing to do. It is
also weaker than its own comment says**, and the conductor checked it:

The comment claims "a fifth overlay added to the guards above and forgotten in
that function would flip this `debug_assert!` in every dev build and test."
It would not. `crates/graft/src/update/banner.rs:101-107` names exactly
`commit_editor`, `map`, `journal`, `stash`:

- fifth overlay **shown** → the match takes the new arm, the fall-through is
  never reached, **the assert is never evaluated** — while
  `graph_screen_on_frame` returns `true` because it has never heard of it, and
  the banner draws on a frame that is not the graph. **That is the exact bug
  the guard is advertised as catching, and it sails through.**
- fifth overlay **not shown** → fall-through reached, function returns `true`,
  assert passes.

What it *does* catch is the converse: `graph_screen_on_frame` gaining a
condition the match lacks. That is worth keeping — **do not delete it.** Fix
the comment so it describes the direction it actually covers, and close the
other direction.

**This is the same fault as Q2 and as everything else in this milestone: two
things that must agree, with the thing that checks them only able to see one
side.** It belongs in this ticket because this ticket is where that class gets
an invariant rather than a fourth point fix.

**Conductor's decision, so it is not re-derived:** close it by making the
overlay set a **single enumeration both sides read**, so a fifth overlay
cannot be added to one and forgotten in the other — that is ticket 10's
`badge_spans` answer applied to the same shape, and it is the answer this
milestone has already proven works. If you conclude one enumeration genuinely
cannot serve both, **stop and report** rather than leaving a second guard
beside the first. Note while you are there: `frame.rs`'s match guards do
**not** test `commit_editor`, while `graph_screen_on_frame` does — say whether
that asymmetry is reachable, and if it is, that is a defect, not a tidy-up.

## What NOT to do

- Do not bind a new key anywhere.
- Do not make `Enter` on a focused Diff pane *do* something to justify the
  hint. The hint is the thing that is wrong.
- Do not touch `KNOWN_GAPS.md` — the conductor's file. Report rows instead.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] The Detail tab at 80 with Diff focus draws no key the pane does not
      answer — test: name it. Must fail before the change; quote the bar before
      and after.
- [ ] **Every context, every width**: no key drawn is unanswered — test: name
      it, say how many contexts it enumerates and how it finds them.
- [ ] Both mutations turn it red — evidence: the two failure messages.
- [ ] `⏎ open` specifically is gone from the Detail tab's bar and still present
      on the graph row where it works — test: name it. The filter must not
      over-reach.
- [ ] No new key is bound — evidence: `git diff` on `update/keys.rs` and
      `docs/design/02-keymap.md` is empty.
- [ ] Captures showing a Detail-tab bar regenerated, captions read against
      them — evidence: quote the bar row from each.

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Measure the baseline yourself and report both numbers.
`cargo test --doc` is vacuous workspace-wide — run it, do not report it as
evidence.

`CARGO_TARGET_DIR=/tmp/graft-target-g31`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

How many contexts the sweep covers and how it enumerates them. Whether the
second mutation (`m`/`r` marked planned) turned it red — if not, the invariant
is one-directional and that must be said. Anything unverified — which is not
broken.

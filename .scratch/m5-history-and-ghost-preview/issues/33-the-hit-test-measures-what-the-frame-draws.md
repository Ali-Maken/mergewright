# the-hit-test-measures-what-the-frame-draws

Status: ready
Blocked by: **32** (shares `graft`, `graft-ui`).
Crates: `graft`, `graft-ui`.
Build with: **Opus.** M1 is a seam bug that three tickets' worth of unifying
work walked past, and the fix has to hold at the seam, not one level below it.
Scout: **not needed** — every line is below, from `docs/reviews/m5-10-30-31.md`.

## M1. The hit-test measures against a `GraphView` the frame never draws

**This is the one that matters, and it is user-visible on the user's own
setup.** Ticket 10's hard constraint was that badge geometry must have one
implementation, not a hand mirror. It delivered that **inside `graft-ui`** —
`badge_spans` is genuinely shared. **The app seam then reopened the same
fault one level up**, by feeding that one implementation two different views:

- `crates/graft/src/view/hit.rs:46` — `let view = graph_view(state, now_seconds);`
  then passes `view.icons` straight to `badge_spans`. That view carries the
  **defaults**: `Icons::default()` (`view/rows.rs:58`) and
  `DEFAULT_MAX_LANES` (`view/rows.rs:253`).
- `crates/graft/src/view/frame.rs:245-248` — builds the same view and then
  **overrides both**: `screen.icons = session.icons;` and
  `screen.compaction.max_lanes = max_lanes_for(graph_columns(state, area), session.max_lanes);`

So the cells the user sees and the cells the drag measures are computed from
different icon sets and different lane caps. **With Nerd icons — which is how
this user runs it — or with more than the default lanes at ≥100 columns, a
drag lands on the wrong badge, or on none.** That is the same family as the
right-click-acts-on-the-wrong-row Critical already found in this milestone.

`badge_spans_match_the_drawn_cells` cannot see it: it checks `graft-ui`
against itself, which was the right test for ticket 10's scope and is not
this. **And `crates/graft/tests/it/name_and_drag.rs:293-296` claims the
integration test would catch it. It does not — fix that comment; a comment
asserting a coverage that does not exist is worse than no comment.**

**Fix at the seam.** One view, built once, used by both the draw and the
hit-test — the same answer ticket 10 gave, applied where the two callers
actually diverge. **Do not** patch `hit.rs` to re-apply the same two
overrides: that is a second hand mirror, it drifts on the third field
somebody adds, and it is exactly the shape this milestone has now paid for
seven times. If you conclude one view genuinely cannot serve both, **stop and
report** rather than mirroring.

**A test that would have caught it** is the deliverable, not just the fix:
drive a drag against a frame drawn with **Nerd icons and >8 lanes**, and
assert the badge the hit-test returns is the badge under the pointer.

## M2. `answered()` reads any difference from idle as answered

`crates/graft/src/update/tests.rs:3487`. It treats *any* difference from the
idle state as "the key was answered", and **nothing asserts that idle emits no
commands** — so a key that changes state incidentally reads as answered.
**Its comment states the inverse of what actually protects it.** Fix the
comment to describe the real guarantee, and add the missing assertion so the
guarantee is real. This under-writes ticket 31's whole sweep, so it is not
cosmetic.

## M3. The enumeration disclosure is rosier than the code

`crates/graft-ui/src/graph_screen/hints.rs:817` — `every_context()`'s list is
literal. The doc at `:810-814` is honest ("writing the variant into the list
is still a human's job"); the **ticket NOTES and the gap row were not**, and
both claimed the compiler forces a new variant in. It does not: `marked_form`
forces the variant into *its* match, and a human still has to add it here.
**The conductor has already corrected the gap row and said whose error it
was.** What is left for you is the code: make the enumeration come from the
type so the disclosure stops being needed — the `badge_spans` answer again.
If that cannot be had in Rust without a macro or a crate the architecture
forbids, **say so plainly and leave the honest disclosure**; an accurate
limitation beats a fake fix.

## M4. The converse direction is narrower than "both directions are covered"

`every_planned_hint_is_still_unbound` (`crates/graft/src/update/tests.rs:3928-3960`)
covers *bound while the table says planned* — which was ticket 24's defect, so
the mutation asked for was answered honestly. It does **not** cover *bound and
in no table at all*: a key bound in a handler and never added to any
`context_list` passes both sweeps. `bound_actions()` (`keys.rs:723`) is itself
hand-written.

**Fix.** One more walk: every `(key, verb)` in `bound_actions()` appears
`Live` in at least one context's `whole_bar`, with a **named** exemption list
for keys deliberately never hinted. Then the summary sentence is true. If the
exemption list is long, that is itself a finding — report it.

## M6. The sweep varies the bar's width but never the state's

`crates/graft/src/update/tests.rs:3569` fixes every fixture at 160×48; the six
widths (`:3305`) only change what `hints_for` is asked to draw. **So the bar
drawn for 40/80/100 columns is pressed into a side-by-side, non-Tab state —
and the Tab layout (<100 columns) is exactly where tickets 29 and 31's defects
lived.** Nothing is known to differ there today, so this is a **coverage**
finding, not a behavioural one — but "at any width" in the invariant is not
what the test does.

**Fix.** Build the state at each swept width (`state.size = Size { columns,
rows }` inside the width loop) so the handlers see the layout the bar was
drawn for. If some widths cannot settle, say which and why.

## M5 — already done, do not redo

Ticket 30's seven acceptance boxes were merged blank. **The conductor has
ticked them from the builder's Report and labelled the note as post-hoc**, and
added the missing check to its own merge procedure. Nothing here for you.

## NOT yours

- The review's **8 Minors and 2 Questions** — the conductor triages them at the
  milestone close. Do not chase them.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.
- The 22 unwalked acceptance scenarios — a separate gap, not this ticket.

## Acceptance (every tick names its verifying test inline, at tick time)

- [ ] A drag measured against a frame drawn with **Nerd icons and more than
      the default lanes** lands on the badge under the pointer — test: name it.
      **Must fail before the fix**; quote the wrong index it returns.
- [ ] The frame and the hit-test use **one** view — evidence: name the single
      construction site and show `hit.rs` no longer re-applies overrides.
- [ ] `name_and_drag.rs:293-296`'s false coverage claim is corrected — quote
      the new wording.
- [ ] `answered()` has the missing assertion and a comment that matches it —
      test: name it.
- [ ] Every `(key, verb)` in `bound_actions()` is `Live` in some context, or is
      on a named exemption list — test: name it, and give the list.
- [ ] The sweep builds state at each width — test: name it, say which widths
      settle and which do not.
- [ ] Whole suite green; report before and after.

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --all --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.**

`CARGO_TARGET_DIR=/tmp/graft-target-g33`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

Whether one view could serve both, or why not. How long `bound_actions()`'s
exemption list turned out to be. Whether M3 could be closed in the type or had
to stay an honest disclosure. Anything unverified — which is not broken.

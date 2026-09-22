# the-hit-test-measures-what-the-frame-draws

Status: done
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

## Also: `cargo run` does not work, and the user hit it

The workspace has four binaries, so plain `cargo run` fails with *"could not
determine which binary to run"*. The user hit this the first time he tried to
open his own app. Add `default-run = "graft"` so `cargo run -- <repo>` works.
One line; check whether it belongs on the workspace or on `crates/graft`, and
update any doc that currently tells a reader to use `--bin`.

## NOT yours

- The review's **8 Minors and 2 Questions** — the conductor triages them at the
  milestone close. Do not chase them.
- `PROGRESS.md`, `KNOWN_GAPS.md`, `docs/log/` — the conductor's. Report rows.
- The 22 unwalked acceptance scenarios — a separate gap, not this ticket.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] A drag measured against a frame drawn with **Nerd icons and more than
      the default lanes** lands on the badge under the pointer — test:
      `graft::view::tests::a_drag_lands_on_the_badge_the_frame_drew_under_the_pointer`
      (160 columns, 14 lanes, `Icons::Nerd`; finds each badge in the rendered
      buffer, then presses every cell it occupies). **Failed before the fix**:
      `"the frame drew alpha at column 30, so a drag there picks alpha up" —
      left: Some(1), right: Some(0)` — the hit-test answered **`bravo`'s
      index, 1**, where the frame had drawn `alpha`. Mutation-checked after
      the fix, both halves: putting `Icons::default()` back in `graph_view`
      fails it on the drawn-glyph assertion; the lane-cap half is what the
      quoted `Some(1)` failure came from.
- [x] The frame and the hit-test use **one** view — single construction site
      `crates/graft/src/view/rows.rs::graph_view` (its doc now says so); the
      two `session` overrides are **gone** from `view/frame.rs` (the file no
      longer mentions `screen.icons`/`screen.compaction`), and `hit.rs:48-55`
      calls `graph_view` and nothing else. Pinned by the test above plus
      `graft::view::tests::the_resize_key_is_offered_only_where_there_is_a_divider_to_drag`.
- [x] `name_and_drag.rs:293-296`'s false coverage claim is corrected — new
      wording at `crates/graft/tests/it/name_and_drag.rs:292-301`: "**What
      this proves, and what it does not.** … It reads no drawn cell, though,
      so a hit-test measuring geometry the frame never drew would still be
      found here — at the wrong column — and this would pass. It used to
      claim otherwise (m5/33, review M1)."
- [x] `answered()` has the missing assertion and a comment that matches it —
      the assertion is inside
      `graft::update::tests::dead_keys_on` (called by
      `every_key_the_bar_draws_is_answered`), which asserts per swept state
      that a key nothing binds does **not** read as an answer; `answered`
      itself now delegates to the new `did_something` and its comment states
      the real guarantee. See the finding below: the review's prescribed
      assertion (*"nothing asserts that idle emits no commands"*) is **not
      true of this tree** and was not forced.
- [x] Every `(key, verb)` in `bound_actions()` is `Live` in some context, or is
      on a named exemption list — test:
      `graft::update::tests::every_bound_action_is_hinted_live_or_is_named_here`
      (it walks the exemption list too, so a stale row fails). List is **6
      rows**, `NEVER_HINTED` at `crates/graft/src/update/tests.rs:4110`:
      `g/G top/bottom`, `PgUp/PgDn page`, `⌃R refresh`, `U redo`,
      `esc dismiss` — all five are keymap §2.2 globals, live in every pane and
      deliberately never on a context bar — and `a amend`, which is **an open
      question, not a decision** (see findings).
- [x] The sweep builds state at each width — test:
      `graft::update::tests::every_key_the_bar_draws_is_answered`, which now
      calls `state_for(context, divider, columns)` inside the width loop and
      sets `state.size` from the swept width. **All six widths settle**
      (40, 80, 100, 120, 160, 200); none is skipped. The only states the
      sweep is blind in are those with a badge **in flight**, at every width
      equally — `update::drag::drag_key` takes `state.drag` before it reads
      the key, so there every press moves the state. That is asserted, not
      assumed: `dead_keys_on` fails if a state reads blind for any other
      reason.
- [x] Whole suite green; report before and after — before: **2345 passed, 37
      skipped**; after: **2349 passed, 37 skipped** (`cargo nextest run`,
      workspace-wide). `cargo fmt --all --check` clean,
      `cargo clippy --all-targets --message-format=short -- -D warnings`
      clean, `cargo test --doc` ok (0 tests — vacuous workspace-wide, not
      offered as evidence).

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

## NOTES (builder, m5/33)

Scout: **not used** — the ticket named every file and line, and they all held
except where noted below.

**This ticket compacted once, near the end.** Everything in the Acceptance
list was built, gated and mutation-checked before it; what came after was the
Nerd-glyph assertion on the M1 test, the four gates re-run and this write-up.
**Nothing is left over** — no split needed.

### One view *can* serve both — and the seam moved, it was not mirrored

The two overrides were session-owned inputs that decide **where a drag must
land**, so they belong on the state the hit-test can see, not on `Session`
(which `update` cannot see at all). Following the existing
`AppState::settled_tier` precedent, `Session::icons` and `Session::max_lanes`
were **removed** and became `AppState::icons` / `AppState::max_lanes`, seeded
in `event_loop/startup.rs::seed_config`. `view::rows::graph_view` is now the
one construction site; `view::frame` overrides nothing.

Two things fell out that the review had not asked for:

- **A third copy of the lane-cap arithmetic** lived in
  `event_loop/graphics.rs::lane_cap(session, state)`. Deleted; both call sites
  now use `view::max_lanes(state)`.
- **The banner was diverging the same way.** `frame` also set
  `screen.banner` after building, so the hit-test measured rows against a view
  with **no banner** — every row off by the banner's height whenever one was
  up. Same bug, second field, and exactly the drift the ticket predicted "on
  the third field somebody adds". `graph_view` now sets it.

`graph_view` derives the screen rect from `state.size` (new
`frame::screen_area`) rather than taking one, so a caller **cannot** hand it a
different width. That is why the fix holds rather than depending on callers
agreeing.

### M2's premise is not true of this tree — finding, not a refusal

The review asked for an assertion that **idle emits no commands**. Probed
first: it does emit one. A key nothing binds, pressed on the baseline state,
returns `[EvilProbe(JobId(…), Oid(0202…))]` — the ambient evil-merge probe,
legitimate and unrelated to the key. Forcing the prescribed assertion would
have meant either a false test or suppressing a real command, so **it was not
forced.** The real defect the review saw is genuine and is fixed: `answered`
did read any difference as an answer, and its comment stated the inverse of
what protects it. What protects it is that the sweep's own baseline
(`nothing_happened`) is asserted **not** to read as an answer, at each swept
state, before any drawn key is pressed — `dead_keys_on`, `crates/graft/src/
update/tests.rs`. **For the conductor:** the ticket's line 53-54 claim is the
one review assertion that did not survive contact with the tree.

### M6 was not only coverage — it found a live dead key

Building the state at each width turned up **40 dead keys, all `<> resize`, at
40 and 80 columns**. Under 100 columns the detail pane is a **Tab** strip:
there is no divider to drag, and the handler correctly does nothing — but the
bar drew the hint anyway. Verified on a real drawn frame before believing the
sweep. Cause, at `crates/graft-ui/src/graph_screen/footer.rs:92`: the field
was named `detail_shown` and the footer read it as "there is a divider".
Renamed to `GraphView::divider_shown` and given a layout-aware value
(`view::rows::divider_shown` = pane shown **and** not tabbed). Cause, not
symptom. Pinned by
`graft::view::tests::the_resize_key_is_offered_only_where_there_is_a_divider_to_drag`
(80×24 must not draw `<>`; 160×24 must).

So M6 upgrades from a coverage finding to a **user-visible one**: at a narrow
terminal the bar advertised a key that did nothing.

### M3 could **not** be closed in the type

`every_context()` enumerates `HintContext`, which is not a plain C-like enum:
variants carry `bool` fields, so "every context" is a cross product, not a
variant list. Rust has no stable reflection over that without a derive macro
(a new proc-macro dependency the architecture does not have) — so the
disclosure stays, rewritten to say **"It cannot come from the type."** and to
name the three exhaustive matches that *do* fail to compile on a new variant.
Backed by a new test that is not a disclosure:
`graft_ui::graph_screen::tests::every_context_reaches_every_shape` maps each
context to one of `COVERAGE_SLOTS = 17` slots through an **exhaustive** match
on variants and their boolean combinations, and fails if any slot is unvisited
— so a new variant or a new field breaks the build in that match, and a
variant left out of `every_context`'s list fails the test. That is as close to
"from the type" as this type allows.

### Open question for the conductor: `a amend`

`bound_actions()` binds `a` / `amend`, keymap §2.3 gives `a` to the graph pane,
and §2.1 says the bar always shows what is active — but **no hint table has
ever listed it**; the bar draws `c commit` alone. Either the bar is missing a
key or the keymap does not intend it hinted. Not a build decision, so it sits
in `NEVER_HINTED` **labelled an open question, not a decision**, keeping the
walk honest rather than blocked. File or close it at the milestone review.

### `default-run` went on the package, and `default-members` was rejected

`default-run = "graft"` is a **package** key; it is now on `crates/graft`.
Verified empirically: plain `cargo run` prints
`Running /tmp/graft-target-g33/debug/graft`. `[workspace] default-members` was
considered and **rejected** — it would silently narrow every un-scoped cargo
command, the workspace-wide gates included, which is the opposite of the rule
that forbids `-p`. `README.md`'s demo line is now `cargo run -- ~/code/some-repo`.

### Unverified (not broken)

- No colour claim is made anywhere here (rule 5); every assertion above is on
  character cells or on values.
- The M1 test drives the hit-test directly rather than through a real terminal
  mouse event; the end-to-end gesture stays covered by
  `crates/graft/tests/it/name_and_drag.rs`, whose comment now says exactly
  which half each of the two covers.

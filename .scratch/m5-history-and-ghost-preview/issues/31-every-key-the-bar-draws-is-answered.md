# every-key-the-bar-draws-is-answered

Status: done
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

- [x] The Detail tab at 80 with Diff focus draws no key the pane does not
      answer — test
      `it::hint_bar::the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer`.
      Fails before the change (checked by reverting the wrap in
      `view::rows::hint_context` to `if true`):
      before ` ⏎ open  c commit  P push  . more  p  f  b  t  u  S  J  /  q  F2  F3  x  +  j/k`,
      after ` c commit  P push  p pull  . more  f  b  t  u  /  q  F2  F3  x  +  j/k  space  1`.
      The after bar is pinned in the test itself.
- [x] **Every context, every width**: no key drawn is unanswered — test
      `update::tests::every_key_the_bar_draws_is_answered`. **29 contexts**,
      from `graph_screen::every_context()` (the single enumeration; see NOTES
      for exactly how much of it the compiler forces), × 2 dividers × 6 widths,
      and — where the graph footer classifies the state at all — the bar
      `view::rows::footer_context` actually asks for as well as the enumerated
      one. Paired with `update::tests::every_swept_state_is_the_context_it_claims`,
      which makes the app's own classifier confirm each fixture is the screen
      it claims.
- [x] Both mutations turn it red — evidence in NOTES. The first turns **this
      sweep** red (12 entries); the second turns `every_planned_hint_is_still_unbound`
      red and leaves this sweep green, so the invariant is one-directional.
- [x] `⏎ open` specifically is gone from the Detail tab's bar and still present
      on the graph row where it works — test
      `it::hint_bar::the_detail_tab_hides_the_keys_a_focused_pane_does_not_answer`,
      whose first assertion is the graph row still drawing `⏎ open` at the same
      width.
- [x] No new key is bound — `git diff` on `crates/graft/src/update/keys.rs` and
      `docs/design/02-keymap.md` is empty (checked with `git diff --stat`, no
      output).
- [x] Captures showing a Detail-tab bar regenerated — new scenario `detail`
      in `it::hint_bar::capture_hint_bar_for_docs_shots`, three widths, bars:
      - 80x24 ` c commit  P push  p pull  . more  f  b  t  u  /  q  F2  F3  x  +  j/k  space  1`
      - 120x40 ` c commit  P push  <> resize  p pull  f fetch  b branch  t tag  . more  u  /  q  F2  F3  x  +  j/k  space  1  F  H  y`
      - 160x48 ` c commit  P push  <> resize  . more  p  f  b  t  u  /  q  F2  F3  x  +  j/k  space  1  F  H`
      No `⏎`, no `S`/`J`/`M`/`O` at any width; `. more` at all three.
      **Not ticked as "looks right" — the user judges** (rule 6).

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

## NOTES (builder)

**Scout:** none. The ticket named both defect lines and the decision; the
sweep then found the rest of the surface by running, which no scout would
have.

**No compaction.** One session.

### The sweep, and how honest its enumeration is

`graph_screen::every_context()` is the one list. What the **compiler** forces:
every variant must be given a `Marked` form and a `PaneFocused` form (two
exhaustive matches, no `_`), and `update::tests::screen_for` must be given a
state for it. What is still **hand-written**: the plain list of one value per
variant and per field combination — Rust cannot enumerate a type's values, and
there is no `strum` in this workspace. So a *new variant* cannot be missed;
a new *field* on an existing variant (a third `BranchRow` shape, say) can be.
Said here rather than left to look like full coverage.

29 contexts × 2 dividers × 6 widths (40, 80, 100, 120, 160, 200).

**"Answered" is measured, not tabulated.** Each fixture is first *settled* —
an unbound key (`F12`) pressed until the state stops moving, because a freshly
built pane asks for what it is missing on the first event it sees — and then
each drawn key is pressed and compared against that same unbound key's
outcome. Note the direction: a key **consumed by the focused handler that then
does nothing** comes back with *fewer* commands than the unbound key, because
it returns early and the ambient work never runs. "Differs from inert" catches
it; "differs from an empty outcome" would not, which is why the first two
attempts at this test were green under the mutation.

`release` needed no special case: `bound_actions()` carries it (m5/10), and
the sweep performs it as a real mouse-up (`Press::Release`) rather than
exempting it.

**Where the sweep is blind, and it says so out loud:** in a drag, *every* key
ends the gesture (`update::drag::drag_key` takes `state.drag` before it reads
the key), so no press can be told from another. The sweep asserts that this
only ever happens with `state.drag.is_some()` — if some other state starts
moving under an unbound key, the test fails rather than quietly reading every
key there as answered.

### The two mutations

1. **`⏎ open` advertised while the pane eats it** (the ticket's defect, as
   shipped: the consuming arm kept, the bar's filter removed) — **red**, 12
   entries:
   `the bar draws 12 key(s) the handler with focus does nothing with:
   PaneFocused { row: PlainRow, opens: false } (Shown), bar of PaneFocused { … }, draws `⏎ open`` (and the other eleven row/divider combinations).
2. **`m`/`r` marked `planned`** — **green**. A `Planned` hint is dropped
   before the width rule, so the bar draws nothing for the sweep to press.
   It turns `update::tests::every_planned_hint_is_still_unbound` red instead
   (`planned m merge is bound`). **The invariant is one-directional:** it
   catches *drawn but dead*, never *live but never drawn*. The two tests
   together cover both directions, and neither covers the other's.

### Findings the sweep made, beyond the ticket's

All fixed here; all the same class (the bar and the handler disagreeing).

- **`view/rows.rs` `hint_context`: the changes pane's bar was drawn with the
  graph focused.** `AppState::on_changes_pane` has no focus test, while
  `update::detail::keys::changes_pane_write_key` refuses every one of those
  keys with `DetailFocus::Graph => return false`. Reachable with one key:
  `⇥` from the pane back to the graph. `space stage`, `d discard`, `⌃A all`
  and `e edit` were all drawn and all dead. Now the pane's bar needs the
  pane's focus; with the graph focused the row's own bar is drawn. This
  changed one integration expectation
  (`it::conflict_interim::the_doorway_survives_everywhere_the_banner_does_not_own_the_keyboard`:
  after `⇥` the doorway is present but no longer *last*, because the graph
  row's bar has a tail and since m5/28 the doorway leads the tail).
- **The journal and the stash drew `. more`, and `.` opens nothing there.**
  Both are modal overlays, exactly like the focused banner and a drag, which
  ADR-0039's first amendment already covers. Doorway dropped in `whole_bar`.
- **The two overlays drew `<> resize`** with no divider of their own.
- **The working-tree row drew `y sha` and `x compare`**, which need a commit
  under the cursor.
- **Two test-side mirrors of `hints_for` in `hints/tests.rs`.** `whole_list`
  restated the whole-bar rule and `doorway_of` had a `_` arm that claimed a
  doorway for `Dragging` the bar never draws. `whole_list` now reads
  `whole_bar` directly. `doorway_of` cannot: a marked row's `. explain`
  leads the *list* rather than trailing it, so it stays written out arm by
  arm — but exhaustively, no wildcard.

### The banner mirror's guard (review M3)

Closed as the conductor decided: **one enumeration, `AppState::graph_overlay`
→ `Option<GraphOverlay>`** (`CommitEditor` | `Map` | `Journal` | `Stash`).
`view::frame::draw_screen` matches on it — so a fifth overlay is a fifth
variant and the frame stops compiling until it is drawn — and
`update::banner::graph_screen_on_frame` is now
`screen == Graph && state.graph_overlay().is_none()`, with no conditions of
its own to drift. The `debug_assert!` is **kept**, with its comment rewritten:
it no longer claims to catch a forgotten fifth overlay (it never could); it
covers the one direction left, that `graph_screen_on_frame` still *is* that
function rather than a hand-written condition beside it again.

**The `commit_editor` asymmetry the ticket names does not exist.**
`crates/graft/src/view/frame.rs:222` (before this change) read
`Screen::Graph if state.commit_editor.is_some() => draw_commit_editor(…)` —
the match tested all four, the same four `graph_screen_on_frame` tested. Not
a defect, and nothing to fix; the ticket's premise was stale.

### The wording pass

- Deleted `the_floor_holds_from_twenty_to_two_hundred_forty_columns` (20..=240).
  **Kept** the wider `0..=240` one, renamed
  `the_three_most_relevant_never_fall_below_their_floor_rung`.
- `hints.rs` module doc, `ladder` step 1 and the `fit_list` doc now say
  **floor rung**. `hints.rs:413-414`'s `fit_list` doc — the one the ticket
  flags as m5/30's — **was not fixed by 30**; it still read "never below
  their short rung" when I got here, and is fixed now.
- Added the one-line definition beside the table (`short_form`'s doc): a
  hint's floor rung is its short rung where the table names one, its full
  verb where it does not.
- The uses that still mean "the abbreviated form" are left alone
  (`hints.rs:152`, `hints/tests.rs:399/424/431/503`, `hint_bar.rs:483-484`,
  `m5-hints-ladder.txt`).
- **Captions: 15 capture files, not twelve.** Twelve from
  `it::hint_bar.rs` (plain/branch/changes/uncommitted × 3 widths) and three
  from `graph_screen/tests.rs` (marked × 3 widths) — the review's figure
  missed the second writer. All 15 regenerated; the only change in them is
  the caption (no bar drifted), plus the three new `detail` captures.

### For KNOWN_GAPS / the conductor (not edited by me)

- **`space fold` is drawn on the uncommitted-changes row** and folds the
  *history cursor's* merge, not anything on that row. It is answered, so the
  sweep passes it; whether it should be *drawn* there is a design question.
- **`every_context()`'s plain list is hand-written** (above). A new field on
  an existing variant can be missed. Unverified rather than broken: nothing
  is wrong today.
- **Colour is unverified as always** (rule 5) — the captures are character
  grids.

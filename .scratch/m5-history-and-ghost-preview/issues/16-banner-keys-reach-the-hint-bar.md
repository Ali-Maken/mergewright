# banner-visible-and-honest: a modal banner the user cannot see, and a hint bar that will not name its keys

Status: **done** (commit on `m5-g16`; this ticket file edited in the primary
checkout, uncommitted — it does not exist on the worktree's base `c1fa5df`)
Blocked by: 14 (merged)
Build with: **Opus.** Two judgement calls: where a modal banner lives in the
sub-100-column tab layout, and whether `bound_actions` needs to know its
context. Neither is a find-and-replace.
Crates: **`graft-ui` and `graft`** — both, one builder. Flipping either side
alone fails a test; and the capture defects and the hint defects are the same
screen.
Scout: **not needed.** Ticket 14 §"Found on the way, NOT fixed" names both
offending functions and why they agree. `docs/reviews/m5-09-app-conflict-interim.md`
(`c1fa5df`) names C1 and M1–M4 with file and line. Read those two, not the tree.

## What this ticket is

Group 09's review returned 1 Critical and 5 Majors, all on one screen: the
operation banner. Ticket 14 independently found a sixth. They share files, so
they are one group.

## 1. Critical — the banner is not on screen at 80 columns

`docs/shots/m5-ui-banner-conflict-80x24.txt` on `main` **shows no banner**.
Below 100 columns the layout switches to tabs (DESIGN §1.1), the detail pane
takes the whole screen, and the banner goes with it. A conflict stops the
operation, the banner takes focus, it is **modal** — it owns the keyboard —
and the user cannot see it or any of its keys. `c`, `s`, `a` are live and
invisible.

At 160x48 it truncates to `⚠ MERGING — …`: the button slot is reserved at
≥160 and no buttons are drawn. **Only 120x40 delivers.** Group 09 ticked this
acceptance box; the tick was false, which is Critical whatever its size.

**Conductor's ruling, and the reason for it:** *a modal overlay must be
visible wherever it has focus.* If it claims the keyboard it is on screen.
So in tab layout the banner draws **above the tab line**, outside the pane —
it is not a pane's content and must not travel with one. This is ADR-0039's
principle applied to the thing ADR-0039 did not cover: guarantee the small,
always-needed marker rather than choosing between everything and nothing.

The cost is real and I am accepting it deliberately: 1–3 rows of pane content
below 100 columns, **only while an operation is stopped**. Say in your report
exactly how many rows it cost, because the user judges that by eye.

Fix the ≥160 truncation too: either draw the buttons in the slot that is
reserved for them, or do not reserve it.

## 2. The hint bar will not name the banner's keys

The footer under a focused modal banner reads `<> resize  . more` and mentions
neither `c` nor `a`. Two functions are wrong in the **same direction**:

- `graft_ui::graph_screen::hints::banner_list` marks every entry
  `HintState::Planned`; a planned hint is dropped before the width rule runs.
- `graft::update::keys::bound_actions()` omits `c continue`, `s skip`,
  `a abort`.

The two tests written to catch this — `update::tests::every_hinted_key_is_bound`
and `every_planned_hint_is_still_unbound` — **both pass**, because each is
checked against the other.

**Read that twice.** Green tests are not evidence (rule 1). Part of your job is
leaving a test anchored to *what `banner_key` actually binds*, not to the other
table.

## 3. Majors from the review — read it for the detail

- **M1** Tab cycles Diff↔banner; the graph is unreachable while the pane is
  open (`banner.rs:202-210`, `168-173`).
- **M2** `M` and `e` are dead keys when nothing is unmerged, and the hints
  advertise them anyway. Related to item 2: the changes pane hints the same
  letters for different verbs and `bound_actions` cannot tell two contexts
  apart. Decide it properly — a context parameter or a per-context table is a
  signature change **inside your own two crates**, which is allowed. What you
  may not do is make the tables agree by weakening a test. If you cannot bind
  `M`/`e` truthfully in both contexts, leave them `Planned`, say so, and file
  it: unverified is not broken, **but a hint that lies is**.
- **M3** `finishing_clears_the_focus` feeds `Failed` and never asserts focus.
- **M4** `banner_is_modal_when_focused`'s `Ctrl+C` assertion is vacuous — the
  status is already `Sticky`. Both are tests that cannot go red. Fix them so
  they can, and show the mutation that proves it.

`docs/design/02-keymap.md` §2.6a already names every key here. Bind nothing the
doc does not name; you should need no spec change.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The banner is on the **drawn frame** at 80x24, 120x40 and 160x48, with
      its keys — asserted on the frame, not on a view model — test:
      `it::conflict_interim::the_banner_and_its_keys_are_on_the_frame_at_every_width`
      (drives a real repo to a real conflict, draws through `view::view`,
      asserts `MERGING` + `notes.md` + the key line at all three sizes), and
      `it::conflict_interim::below_a_hundred_columns_the_banner_draws_above_the_tab_line`
      (row 0 is the banner, row 1 the tab strip, row 23 the keys).
- [x] At ≥160 the buttons are drawn, or no slot is reserved — test:
      no slot is reserved —
      `graft-ui graph_screen::tests::the_operation_banner_costs_no_row_under_the_header`
      (row 1 is byte-identical with and without a banner, at 80/120/160/200).
      The ≥160 truncation it caused is separately pinned by
      `graft-ui graph_screen::tests::a_long_location_is_cut_before_the_banner_is`
      (new in this pass — see NOTES, mutation C).
- [x] The footer names the banner's keys while it has focus — on the drawn
      frame — test:
      `it::conflict_interim::the_banner_and_its_keys_are_on_the_frame_at_every_width`
      (the line holding `c continue` must also hold `a abort`, `⇥ graph`,
      `M mergetool`, `e edit`, at every one of the three sizes).
- [x] A test that goes red if `banner_list`/`bound_actions` drift from what
      `banner_key` binds, rather than from each other — test:
      `update::banner::tests::every_hint_the_focused_banner_draws_is_answered_by_banner_key`.
      It **presses** every hint the focused banner draws (4 banner states ×
      6 widths × 2 dividers) and asserts the outcome per verb; an unknown
      key or an unknown verb panics rather than passes. The only thing it
      compares the bar with is the dispatcher. Backed by
      `update::banner::tests::the_banner_always_teaches_continue_and_abort`.
- [x] Tab reaches the graph with the pane open (M1) — test:
      `update::banner::tests::tab_out_of_the_banner_reaches_the_graph`
      (+ `backtab_out_of_the_banner_reaches_the_diff_pane`,
      `escape_out_of_the_banner_leaves_the_pane_focus_alone`).
- [x] `M`/`e` are hinted only where they are live (M2) — test:
      `graft-ui graph_screen::hints::tests::the_banner_draws_its_own_keys_and_no_doorway`
      (exact bar text, conflicted and not),
      `view::rows::tests::a_banner_with_no_conflict_says_so_in_its_context`,
      and the other half —
      `update::banner::tests::mergetool_and_edit_refuse_in_words_when_nothing_is_conflicted`.
- [x] M3 and M4's tests can now go red — test: M3 →
      `update::banner::tests::finishing_clears_the_focus` (now asserts on
      `resumed` alone, before any re-read can clear the flag for it; the old
      body kept its claim under the honest name
      `a_failed_resume_keeps_a_abort`). M4 →
      `it::conflict_interim::banner_is_modal_when_focused`, whose `Ctrl+C`
      arm is now `after.should_quit()` alone. Both shown red under mutation
      (F and G below).
- [x] The `. more` doorway still draws at every width (ADR-0039) — test:
      `it::conflict_interim::the_doorway_survives_everywhere_the_banner_does_not_own_the_keyboard`
      plus the existing `graft-ui graph_screen::hints::tests::{eighty_columns_shows_three_and_more,
      hundred_twenty_shows_six, hundred_sixty_shows_the_context_then_the_screen}`
      and `it::hint_bar::*`. **Deviation, deliberate and argued in
      `hints_for`'s doc comment:** under a *focused* banner there is no
      `. more` and no `<> resize`, because the banner is modal and swallows
      both — the doorway's promise is that what the width rule dropped is
      still reachable, and the banner's whole list draws at 80 columns, so
      nothing is dropped. The conductor should confirm this reading of
      ADR-0039.

**Mutation-check each.** Two that matter most: revert `banner_list` to all
`Planned`, and revert the banner to drawing inside the pane. If either turns
no test red, you have rebuilt the original bug and the ticket is not done.

## Re-capture

All three `docs/shots/m5-ui-banner-conflict-*.txt`. Their headers carry the
hint-bar defect as known — **delete that paragraph once it is true**, as
ticket 14 did with its own. ≤ 200 KB each. Colour unverified (rule 5).
**Do not tick the visual result** — the user judges by eye (rule 6), and at
80 columns this capture is the whole point of the ticket.

## Gates

Workspace-wide, never `-p`. `CARGO_TARGET_DIR=/tmp/graft-target-g16`, deleted
before hand-back. Baseline **2156/2156** on `main`. Commit, never push.

## Report

Cause or symptom per fix. How many rows the banner costs below 100 columns.
What your new test is anchored to — the existing pair was anchored to each
other, and that is exactly why this shipped.

## NOTES (takeover builder, 2026-09-21)

Taken over from a builder that stalled with the work **uncommitted** in
`.worktrees/g16` (16 files, +1126/-283), its last action "now update the
capture header text". Nothing was taken on trust; what follows is what I ran.

**Gates, run by me, workspace-wide, in the worktree:** `cargo fmt --check`
clean; `cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run`
**2171/2171** (34 skipped); `cargo test --doc` clean. Baseline on `main` was
2156 — 15 new tests, one of them mine.

**Mutation table** (each applied alone, whole suite run, then reverted):

| # | mutation | red |
|---|---|---|
| A | `banner_list` back to all `Planned` (**required**) | **7** — the anchor, `the_banner_always_teaches_continue_and_abort`, `every_planned_hint_is_still_unbound`, `the_banner_draws_its_own_keys_and_no_doorway`, `no_context_hides_every_key`, and both frame tests |
| B | banner back inside the pane (`draw_tab_banner` not called) (**required**) | **3** — `the_banner_and_its_keys_are_on_the_frame_at_every_width`, `below_a_hundred_columns_the_banner_draws_above_the_tab_line`, `the_tab_layout_banner_costs_one_row_drawn_and_two_focused` |
| C | header reserves no room for the banner (the ≥160 truncation) | **0 → 1** — green as inherited; see below |
| D | `Tab` out of the banner leaves `detail.focus` alone (M1) | 2 |
| E | `M`/`e` hinted with nothing conflicted (M2) | 2 |
| F | `Done` arm stops clearing `banner_focus` (M3) | 1 — `finishing_clears_the_focus` |
| G | banner swallows `Ctrl+C` (M4) | 2 — `banner_is_modal_when_focused`, `the_focused_banner_claims_every_key_but_control_c` |
| H | `. more` trailer removed from `hints_for` (ADR-0039) | 20 |

**The one hole I found and closed.** Mutation C turned **nothing** red as
inherited: the fix for the ≥160 `⚠ MERGING — …` truncation — taking the
banner's width out of the header *before* the location's — had no test. The
integration test's scratch path happens to fit at 160, and the capture
shortens the path before drawing, so both missed it. Added
`graft-ui graph_screen::tests::a_long_location_is_cut_before_the_banner_is`:
a 55-character path, banner drawn whole at 80/100/120/160, the `…` on the
path at the widths where something must give. Red under mutation C.

**Rows the banner costs below 100 columns:** one while an operation is live,
two while its banner has the keyboard, none otherwise — pinned as numbers by
`it::conflict_interim::the_tab_layout_banner_costs_one_row_drawn_and_two_focused`.

**Captures.** Re-generated all four by hand from the current tree
(`capture_banner_for_docs_shots`, `graph_screen::tests::capture_for_docs_shots`)
rather than trusting the inherited files: they came back byte-identical except
for the scratch repo's commit hashes, so the inherited captures did match the
code. The known-defect paragraph is gone from all three
`m5-ui-banner-conflict-*.txt`, and it is gone because it is now false — the
80x24 capture shows `⚠ MERGING — conflict notes.md (+1)` on row 0, the
`Graph [Detail]` strip under it, and
` c continue  a abort  M mergetool  e edit  ⇥ graph` on the bottom row. Sizes
2.6–3.6 KB, well under the 200 KB cap.

**Not verified.** Colour (rule 5 — TestBackend grids carry no attributes;
the `REVERSED` modifier on the focused banner is asserted in
`the_banner_replaces_the_header_slot_and_reverses_when_focused`, the *colour*
is not). Whether the banner row and the two lost pane rows read well by eye
at 80 columns (rule 6 — the user's call; the captures are there for it).
No scout: the ticket says none is needed and the work was already written.

# the-tab-layout-draws-no-keys-at-all

Status: done
Blocked by: **28** (shares `graft-ui` + `graft`).
Crates: `graft-ui`, `graft`.
Build with: **Opus.** It changes every pane's usable height and it moves the
row offsets the mouse hit-test depends on.
Scout: **not needed** — the defect, its test and its cause are all named below.

## Why this exists, and why it is not an M6 row

`crates/graft/tests/it/hint_bar.rs:265-274` holds a test that is
`#[ignore]`d. Its own doc comment states the defect:

> at 80 columns, with the changes pane open, **every** key is hidden. That is
> the one outcome ADR-0037 §4 rules out, and 80 columns is an everyday width
> in an editor's terminal.

It was deferred by the m5-03 fix round because the fix "means giving the tab
layout a footer of its own, which changes the pane's height for every pane and
is a design decision, not this fix round's."

**That design decision is made below.** Three reasons it lands in M5:

1. **It is the one outcome the design forbids.** ADR-0037 §4 and ADR-0039 both
   rest on the bar never being all hidden — ADR-0039's whole architecture is
   that discoverability depends on the doorway being *reliable*. A width where
   the bar is absent entirely is not degradation to a doorway; it is the
   premise failing.
2. **It is this user's everyday width.** He works in editor terminals. 80
   columns with a pane open is his normal case, not an edge.
3. **Ticket 10 depends on the geometry.** Its mouse hit-test reads
   `rows_above_graph` and `graph_row_at`. Reserving a footer row moves those
   offsets. Building 10 first would build it against geometry about to change
   — so 29 runs first.

## The decision (conductor; build to it, do not reopen)

**The Tab detail layout reserves one row for its own hint bar, exactly as the
graph screen does.**

- `DetailLayout::Tab` is the arm below 100 columns
  (`crates/graft-ui/src/detail_pane/layout.rs`, `RIGHT_BREAKPOINT = 160`,
  `BELOW_BREAKPOINT = 100`). In that arm the detail pane owns the screen, so it
  cannot borrow the graph's footer — it needs one.
- **The bar draws the pane's own `HintContext`** — `ChangesPane`, `Journal`,
  `Stash` and so on — not the graph's. The pane that owns the screen owns the
  keys.
- **Cost, stated plainly:** one row off every pane in Tab layout. At 24 rows
  that is about 4%. Losing 4% of a pane beats showing zero keys, and the
  alternative — keys reachable only by widening the terminal — is the failure
  ADR-0039 exists to prevent.
- The floor and the ladder from ticket 28 apply to this bar like any other. It
  is not a special case; it is the same bar in a place that did not have one.

If you find a reason this cannot be one reserved row, **stop and report**. Do
not invent a second mechanism.

## What to build

1. The reserved footer row in the Tab arm, and the bar drawn into it with the
   pane's own context.
2. **Un-`#[ignore]` `the_tab_layout_still_hides_every_key`** and make it pass.
   Rename it to say what it now guarantees rather than what used to be wrong —
   a test named after a bug is a stale caption in test form, which is a class
   this milestone has been fighting.
3. **Every consumer of the changed geometry updated together.**
   `crates/graft/src/update/geometry.rs` mirrors the layout by hand
   (`geometry.rs:1-5`), so `rows_above_graph` and `graph_row_at`
   (`geometry.rs:263-283`) must move with it. A mouse click landing one row off
   below 100 columns is the M5 review's Major 1 returning — see
   `docs/log/m5.md` for ticket 25's fix of exactly that shape.
4. Regenerate any capture that draws a pane below 100 columns, and read each
   regenerated file against its caption before committing.

---

# PART 2 — a box leaves a gutter, the same as a pane does

Small, adjacent, same crates. Ticket 28 landed the column-snap rule and it
works: `docs/shots/m5-ui-confirm-over-graph-80x24.txt` now keeps whole shas in
their column, and the credential prompt clears its band. **One fault remains,
on the edge the rule did not speak to.**

At 80 the confirm box now reads:

```
●   ╭ Push main → origin/main ──────────────────────────────────────────╮60af0eb
●   │ 1 commit · can't be undone from here — origin keeps what you send │aceb8c5
```

The border **abuts the sha with no blank column**. `╮60af0eb` reads as one
token. That is the same fault as `8443─────` in a title — a rule running
straight into text — moved from the horizontal edge to the vertical one, and we
fixed the horizontal case in the same ticket that created this.

It is also the rule ticket 19 already established for panes:
`crates/graft-ui/tests/pane_boundaries.rs` asserts **adjacent panes leave ≥ 1
column**, and its mutation message reads *"the graph ends at column 93 and the
detail pane starts at column 93."* A box beside content is the same problem
with a border drawn on it.

## The decision (conductor)

> **A non-clearing box leaves at least one blank column between its border and
> any content beside it.** The snap boundary is the next column's *gutter*, not
> the first cell of its text.

Read this as a clarification of ADR-0040, not a new rule: a column's trailing
gutter belongs to that column, so bleeding into the next column's leading
gutter was always splitting something. **Amend ADR-0040 in place** — it was
written today; do not append an amendment to a document that new.

If honouring the gutter leaves no snap position that fits the content, the box
**clears**, per ADR-0040's existing final clause. Do not shrink the content.

## What to build (Part 2)

1. The gutter in the snap computation, at the layout site — same place ticket
   28 put the snap, not inside the widget.
2. ADR-0040 amended in place.
3. Extend the existing sweep rather than writing a second one: the
   no-split assertion in `crates/graft-ui/tests/border_rules.rs` (new in 28)
   or `pane_boundaries.rs`, whichever ticket 28 put it in, gains the gutter
   check across the same widths.
4. Recapture the four `m5-ui-{confirm,credential}-over-graph-*` files. They are
   deterministic (ticket 26) — **keep that**, regenerate twice and diff.

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] The bar is present in Tab layout at 80 columns with the changes pane
      open — test: `graft::it::hint_bar::the_tab_layout_draws_the_panes_own_keys`
      (the un-`#[ignore]`d, renamed test). **Fails on the current code:**
      mutated `update::detail::tab_hint_rows` to `return 0` (pre-fix
      behaviour) → FAILED, together with `every_tab_context_draws_a_bar`.
- [x] Present for **every** pane context that can own the Tab layout — test:
      `graft::it::hint_bar::every_tab_context_draws_a_bar`. Covers the Detail
      tab on the tip (`BranchRow { current: true }`), on an ordinary commit
      (`PlainRow`), the changes pane (`ChangesPane`), and the **Graph** tab,
      where the graph screen carries its own footer and the pane reserves
      nothing. `Journal`/`Stash` are unreachable today (those overlays draw
      their own footer) and `Banner` is modal — a focused banner already
      draws its keys on the bottom row, so `tab_hint_rows` returns 0 there.
- [x] The mouse still selects the row the user clicked, below 100 columns —
      test: `graft::update::tests::a_click_below_one_hundred_columns_still_lands_on_the_row_it_named`
      (80x12, pane open, Graph tab). **Mutation:** `+ 1` added to
      `rows_above_graph` → FAILED at "the row the pointer was on".
      **Deviation, see NOTES:** the ticket asked for the mutation "leave
      `rows_above_graph` unchanged"; that mutation cannot exist, because
      `rows_above_graph` needed no change — the reserved row is the Detail
      tab's, and on the Detail tab the graph is not on screen at all.
      The mirror that *did* move is `geometry::detail_split`; its mutation is
      below.
- [x] The graph's own layout at ≥100 columns is unchanged — test:
      `graft::update::tests::the_geometry_mirror_matches_the_rects_the_frame_hands_out`
      (sweeps widths through both breakpoints against the drawing path's own
      rects) plus the whole `graft-ui::pane_boundaries` sweep; no snapshot or
      capture at 120/160 moved (`git diff` over `docs/shots/` and
      `crates/graft/src/view/snapshots/` is empty for those sizes).
      **Mutation on the mirror:** dropped `hint_rows` from
      `geometry::detail_split`'s Tab arm →
      `the_geometry_mirror_matches_the_rects_the_frame_hands_out` FAILED at
      "pane rect at 20x3, focus=Files, layout=Tab { shown: true }".
- [x] Ticket 28's floor and ladder govern this bar too — test:
      `graft_ui::graph_screen::hints::tests::the_floor_holds_from_twenty_to_two_hundred_forty_columns`
      (and the rest of that file's `all_contexts()` sweeps). `draw_tab_hints`
      calls the same `hints_for(context, Divider::Hidden, width)` +
      `hints_text` as every other bar, and `all_contexts()` contains
      `ChangesPane { .. }`, `BranchRow { .. }`, `PlainRow`, `Marked`,
      `Journal` and `Stash` — every context the Tab bar can draw. The Tab
      arm always passes `Divider::Hidden`, which is the arm the sweep runs.
- [x] Captures below 100 columns regenerated, each caption read against its
      own file — evidence, one row per file:
      - `docs/shots/m5-hints-changes-80x24.txt`, last row:
        `` space stage  d discard  c commit  ⌃A all  e edit  ⇥ graph  . more``
        — its caption now reads "the hint bar is the bottom row of whatever
        owns the screen … the pane's own reserved row", and the stale
        `MISSING_BAR` note is gone (it is emitted only when no bar is found).
      - `docs/shots/m3-changes-pane-80x24.txt`, last row: the same bar. Its
        caption already says "The footer offers `space stage` and
        `d discard`" — at 80x24 that was false before this ticket and is now
        true. The pane lost one file row to the bar (`M routes.rs`), which is
        the stated cost; the caption already warns that at 80x24 the list may
        sit below the fold.
      The other eleven `m5-hints-*` files changed in the shared caption only
      (body byte-identical, checked with `diff` per file).

### Part 2

- [x] **No box border touches content beside it**, swept over the same widths
      and heights the no-split assertion uses — test:
      `graft_ui::pane_boundaries::no_box_drawn_over_the_graph_touches_the_content_beside_it`
      (asserts against the **drawn** buffer, not the geometry: it renders the
      graph and reads the cell either side of the border on every box row
      that falls inside the graph body). **Mutation reproducing `╮60af0eb`:**
      dropped the `- 1` gutter step in `snap_to_columns` → FAILED,
      `77x3: the box ends at column 70 and "0" sits against its right border
      on row 1`.
      The no-split assertion itself gained the gutter:
      `no_box_drawn_over_the_graph_splits_a_row_column` now requires the right
      edge to be `frame.right()` or one cell short of a boundary.
- [x] Where the gutter cannot be honoured, the box clears rather than
      shrinking its content — test:
      `graft_ui::pane_boundaries::where_the_gutter_cannot_be_honoured_the_box_clears`.
      It pins the rule on numbers (a boundary set with no frame edge: the
      snap answers `None` rather than a narrower rect) and re-measures the
      sweep. **No width in 20..=240, at any of the five heights, forces the
      confirm box to clear** — the gutter did not change ticket 28's
      measurement, because no gutter is owed at the frame's own edge and the
      frame's edges are in the boundary set.
- [x] ADR-0040 amended **in place**, no appended amendment — evidence, the
      decision now reads:
      "**A non-clearing box covers whole columns only. It never splits one,
      and it leaves at least one blank column between its border and any
      content beside it. If no snap position exists that does both and still
      fits its content, it clears.**"
      The gutter is clause 3 (the old clause 3 became 4); the date line says
      amended the same day and why no amendment section was appended.
- [x] The four over-graph captures regenerated and still byte-identical
      across two runs — evidence: `diff -q` over all four after two
      consecutive runs printed nothing (all four reported `identical`). The
      row beside the box, `docs/shots/m5-ui-confirm-over-graph-80x24.txt`:
      `` ●   ╭ Push main → origin/main ─────────────────────────────────────────╮ 60af0eb``
      — a blank column between `╮` and the SHA, which is the fault this part
      was opened for. The two credential captures did not move (that prompt
      clears its band at every width, so no gutter applies).

Rule 6: you do not tick "looks right". The user does.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Measure the baseline yourself and report both numbers.

Note: `cargo test --doc` is currently vacuous workspace-wide — two `no_run`
doctests, both in `graft-fixtures`. Run it, but do not report it as evidence of
anything.

`CARGO_TARGET_DIR=/tmp/graft-target-g29`, deleted before hand-back. One cargo
command at a time. **Never bare `git stash`.** **Never `git checkout` or
`git restore` across your worktree.** Commit, never push.

## Report

What the row cost each pane. Which captures moved. Whether the geometry mirror
needed changes in more places than `rows_above_graph`. Anything unverified —
which is not broken.

---

## NOTES (builder, m5-history-and-ghost-preview/29)

**Scout:** none. The ticket said not needed and named the defect, its test
and its cause; I read `detail_pane/layout.rs`, `update/geometry.rs`,
`update/banner.rs` and `view/frame.rs` directly.

**Baseline:** 2260/2260 passed, 39 skipped, measured in this worktree before
any edit. **After:** 2265/2265, 38 skipped — five new tests, one of which is
the un-`#[ignore]`d one moving out of the skipped column. `cargo fmt
--check`, `cargo clippy --all-targets -- -D warnings` clean, workspace-wide.
`cargo test --doc` ran and is vacuous, as the ticket says; it is not evidence
of anything.

**What the row cost each pane:** one row, only while the **Detail** tab is
up below 100 columns. Nothing on the Graph tab (the graph screen's own footer
is already there — a second bar would spend the row twice) and nothing while
the operation banner has focus (`tab_banner_rows` is 2 then and the banner
already draws its keys on the bottom row; a bar under a modal would hint keys
the modal swallows). At 24 rows that is ~4% of one pane. Visible in
`docs/shots/m3-changes-pane-80x24.txt`: the file list lost `M routes.rs`.

**Did the geometry mirror need more than `rows_above_graph`?** It needed
*less*, and somewhere else. `rows_above_graph` and `graph_row_at` are
unchanged and could not have changed: the reserved row belongs to the Detail
tab, and on the Detail tab `graph_row_at` already returns `None` (the graph
is not drawn). The mirror that moved is `geometry::detail_split`'s `Tab` arm,
which now takes a `hint_rows` argument beside `banner_rows` and spends both
before carving the strip — restructured so it mirrors
`view::frame::tab_content_area` step for step rather than approximating it
(it now also agrees at `h == 2, banner_rows == 2`, where the old arithmetic
disagreed on `y` with a zero-height rect). All four callers in `geometry.rs`
pass `detail::tab_hint_rows(state)`. Verified by mutation, above.

**One deviation from the ticket, deliberate and reported rather than
silently taken:** the ticket's third acceptance box asked for the mutation
"leave `rows_above_graph` unchanged and show it goes red". That mutation
cannot be built, for the reason above. I built the nearest test that carries
the same guarantee — a real click at 80x12 on the Graph tab with the pane
open — and mutated `rows_above_graph` by `+1` to show it is sensitive to
exactly that offset.

**Finding, not fixed here — stale captures on `main`.** Regenerating every
capture test in the workspace rewrote **27** files, of which only two were
mine. The other 25 are drift from earlier tickets that never regenerated
their pictures: ticket 27's short-form table
(`docs/shots/m2-duplicate-commit.txt` still shows the pre-`hint-bar-redesign`
fixed list), ticket 28's border spacing and column snap
(`docs/shots/m4-credential-prompt-in-app.txt` still shows
`╭ Credentials — 127.0.0.1:8443─────╮`, the exact string ADR-0040 says is
fixed; `docs/shots/m3-confirm-discard.txt`, `docs/shots/m3-undo-drift.txt`,
`docs/shots/m5-ui-undo-after-push-80x24.txt` still show unsnapped boxes), and
the preview wording (`no conflicts expected (forecast)`). I restored all 25
to `HEAD` so this ticket's diff shows only this ticket's work. **They should
be regenerated in one ticket of their own** — several are on the M2/M3/M4
contact sheets, and `m5-ui-undo-after-push-80x24.txt` additionally embeds a
temp path and an oid that move every run, so it is not deterministic and
needs the ticket-26 treatment before it can be regenerated at all. It also
shows the `╮325da3f` fault this ticket's Part 2 fixes — a fifth capture the
ticket did not name.

**Unverified, which is not broken:** colour (rule 5 — the captures are
character grids). And the one visual consequence the user must judge by eye
(rule 6): the gutter costs the box a cell, and at 80 columns that is the cell
the confirm box's longest line used as right-hand padding. It now reads
`│ 1 commit · can't be undone from here — origin keeps what you send│` where
it read `… you send │`. Nothing is truncated and nothing rewraps — the snap
is still outward only, so the box is never narrower than the rect it was
handed — but the trade (one padding cell inside the box, to keep a whole SHA
column outside it) is the user's call. Recorded in ADR-0040 under "What the
gutter costs, for the record".

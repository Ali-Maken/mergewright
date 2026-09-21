# a-box-covers-whole-columns-or-it-clears

Status: done
Blocked by: none — 27 is merged (main `9d215a0`, 2250/2250).
Crate set: **`graft-ui` + `graft`.**

**This ticket has two halves and the seam between them is explicit: Part A, the
boxes; Part B, the bar's bottom.** They are folded into one context because
they share a crate set (ADR-0036 forbids running them side by side anyway) and
because their evidence is one picture, not two — narrow-width behaviour of a
box and of the hint bar belong in the same capture. **If you compact, say so
and say which part you were in**; the conductor splits at that seam rather than
pushing the remainder through.
Build with: **Opus.** The rule is written below and is not yours to change, but
computing a snap position against a live column layout, at every width, without
wiring graph knowledge into a generic widget, is a real design problem.
Scout: **needed.** See below.

## Why this exists

The user judged `docs/shots/m5-ui-confirm-over-graph-80x24.txt` and
`docs/shots/m5-ui-credential-over-graph-80x24.txt` by eye and ruled. The reason
is geometry, not taste, and the evidence is in the files:

**The credential prompt at 80** leaves ` b8c5`, `68ff`, `00bd`, `3e93`, `1ece`
hard against its right border, outside the sha column. Those are the **tails**
of `aceb8c5`, `11d68ff`, `91900bd`, `cec3e93`, `0a91ece` — verified against the
same rows in the confirm capture. Four-character hex tokens that read as valid
short identifiers and are not. His words:

> That is the menu bug with a border around it, and it sits beside a masked
> password field.

**The confirm box at 80** keeps whole shas in the sha column, and the rows
beside it are the graph you are about to act on. That is the M5 promise
working.

**But it survives on geometry, not principle.** Its right edge stops left of the
sha column because of how long "Push main → origin/main" happens to be. The
credential box is a few cells wider and lands mid-column. Same surface class,
same border, opposite outcome, decided by a string length. A per-box ruling
would rest on that accident — hence a geometric rule.

## The rule (decided; record it, do not re-derive it)

> **A non-clearing box covers whole columns only. It never splits one. If no
> snap position exists that covers whole columns and still fits its content, it
> clears.**

Three clauses the conductor added and the user has not countered. Build to
them; if you think one is wrong, **stop and report** rather than choosing.

1. **Both boxes move.** The rule is about splitting, not about the right edge.
   The confirm box at 80 leaves `●   Comm` — that is the **subject column,
   split**. It passes on the right and fails on the left. Its left edge moves to
   the subject column's edge, giving `●   │ box │  <sha>`.
2. **Snap outward if it fits, else clear.** When the nearest boundary would grow
   the box past the screen, it clears. **Never shrink the box's content to reach
   a boundary** — truncating the text to satisfy a layout rule trades a legible
   fragment for an illegible one.
3. **The snap is computed at the layout site, not inside the widget.**
   `ConfirmView` and the credential prompt must not learn the graph's column
   geometry — that couples a generic widget to one screen and breaks PROMPT.md
   6.1. Whoever lays out the screen already knows both; the rect arrives
   computed and the widget stays dumb.

### What each box does

- **Credential prompt: clears its band, at every width.** Two reasons, either
  sufficient. It fragments the sha column (above), and nothing behind it serves
  the task — `docs/design/02-keymap.md:324` makes it **modal**, and the footer
  already carries the context (`push main → origin/main · esc cancels`). The
  graph behind it is decoration next to a secret. Clear **the band it occupies**,
  full width — not the screen. The footer stays.
- **Confirm box: does not clear.** It snaps. The rows beside it are the graph
  you are about to act on, and that is the M5 promise, not a leak.

## Also in scope: one cosmetic the user found in the same pass

Both box titles and footers run the rule straight into the text, and the two
boxes disagree with each other:

```
╭ Push main → origin/main ─────────────────────────────────────╮   (space)
╭ Credentials — 127.0.0.1:8443─────────────────────────────────╮   (none)
╰ ⏎ send  ⌃U clear  esc cancel the push────────────────────────╯   (none)
```

**Make them agree, with the space** — the confirm box is the right one. Beyond
consistency: the rule running straight into a port number makes `8443─────`
read as part of the host. Same fault in the footer.

Apply it wherever a bordered surface puts text in its rule, not just these two
— if a third does it, fix it and say so.

## The scout (run this first, ≤1 page)

`Explore`, **haiku or sonnet**, before any code:

- Where the graph's row columns are computed, and what names them (graph
  glyphs, refs+subject, the `GF` column at wide widths, age, sha). Are the
  boundaries available as data, or only implied by formatting code?
- Every bordered overlay that draws over content today, and where each one's
  rect is decided.
- Whether any overlay already clears what it stands on, so the clear path
  exists or has to be built.
- Where text is put into a border's rule.

It reports **code, not intent**. "Nothing uses this" is not "this is
available."

## What to build

1. **Column boundaries as data.** The snap needs the boundaries; if they only
   exist inside formatting code, lift them. Read `ARCHITECTURE.md` before
   adding or moving a module, and use `codebase-design` if you do.
2. **The snap, at the layout site.** Outward if it fits, else the box clears.
3. **The credential prompt clears its band** at every width.
4. **The border-rule spacing fix.**
5. **ADR-0040**, recording the rule and the reason. A **decision, not an
   autopsy** — the accident that exposed it gets one sentence, not a history.
   Record the border/background line with its clause: a border makes the edge
   explicit, so content beside it reads as background — **and that is only true
   while whole columns survive. Fragment a column and the border does not save
   you.** Write both halves or neither. `docs/design-notes/m5-history-and-ghost-preview.md`
   points here (ticket 27 §7); make the pointer land.
6. **Recapture** the four `m5-ui-{confirm,credential}-over-graph-*` files, whose
   writer is `crates/graft/tests/it/two_boxes_over_content.rs`. They are
   deterministic (ticket 26) — **keep that**; regenerate twice and diff.
   `docs/shots/m5-ui-confirm.txt` draws its box on a blank screen and is
   superseded by those four; `KNOWN_GAPS.md` has it as group 11's call, so
   **leave it alone**.

---

# PART B — the bar's bottom, and the doorway moves to the head of the tail

Everything here is **decided**. None of it is yours to reopen. If you think a
clause is wrong, stop and report; do not choose.

## B1. Variant B wins: the doorway leads the bare keys

The user judged `docs/shots/m5-hints-ladder.txt` by eye and chose variant B.
As built the graph row reads:

```
 ⏎ open  c commit  P push  p  f  u  S  J  /  q  F2  F3  x  +  j/k  . more
```

Three labelled hints, a run of naked letters, then **one labelled hint marooned
at the far end**. His reasoning, which the rule has to preserve:

> The break in the pattern is not where the labels stop — it is `. more`, the
> only labelled item after the run. Nothing marks the boundary that matters,
> and one thing marks a boundary that does not.

And the tail is not uniformly naked — `F2`, `j/k` and `space` sit in it, and
`space` reads as a word, so the run looks like labels that lost their keys
rather than keys that never had labels. Variant B fixes both for **no cells and
no glyph**:

```
 ⏎ open  c commit  P push  . more  p  f  u  S  J  /  q  F2  F3  x  +
```

Every labelled item is contiguous, and the last of them announces the menu.
Everything after `. more` reads as *keys the menu will spell out* — which
resolves `space` and `j/k` too, because the region is labelled even though its
members are not. **The tail stops being what is left over and becomes a preview
of the menu.**

### The rule to implement — one sentence, two layouts, no special case

> **The doorway sits at the head of the bare keys — which is the end of the row
> when there are none.**

That single rule produces the graph rows (tail present, `. more` leads it) *and*
the changes-pane and journal rows (no tail, `. more` stays last). Do not write
two cases.

**This deliberately changes a pinned promise.** `ends_with(". more")` is held by
three tests and by several captions. Change them **in one place**, and say in
the ADR why: the ADR's promise is that the doorway is **reliable**, and
reliable is not the same as last. B keeps it never-dropped and never-clipped
and only moves where it sits.

## B2. The bottom of the ladder, stated completely

ADR-0039 mandates a floor and never says what happens when the floor will not
fit. A floor that cannot be met is not a floor. The sequence, using only
mechanisms already decided:

```
floor  →  `. more`  →  `.`  →  nothing
```

The bare-key rung applies to the doorway like anything else: one column still
says a key exists. Below that, empty. **The bar never draws a fragment of
itself at any width.** A clipped `. mo` on the one element meant to be reliable
is worse than an empty bar — and a clip is worse than a squeeze, because
nobody chose where it cut.

It composes with B1: as width shrinks the tail goes, then the labels, and
`. more` is what remains. Its position stops mattering exactly where it starts
being the only thing there.

**Scope, honestly:** this is **design completeness, not a live defect.**
`crates/graft-ui/src/detail_pane/layout.rs:34-36` clamps the pane to
`RIGHT_MIN = 48` columns or `BELOW_MIN = 8` rows, and below 100 wide the pane
is a full-width tab — so the 20-cell overflow the sweep found is not reachable
by a divider drag. Build it because the rule should be total, not because
something is broken. **Also measure whether the floor fits at 48** and say so;
that number is the smallest width anything reachable actually uses.

## B3. Keep the bare-key rung — the option that was considered and rejected

Letting the ladder stop at the short rung and dropping bare keys entirely was
proposed and **rejected**. Record the reason in the ADR, because it is the
argument that keeps the rung:

> The tail's membership is itself information: on another branch's tip it
> carries `m` and `r`, on an ordinary commit it does not. That is context the
> menu only gives you after a keypress. **The menu covers discovery; the bar
> covers recall.**

## B4. `rev` stays

`revert` keeps its short form. It is a prefix of `revert`, the prefix rule
already decided it, and nothing else in the keymap competes for those three
letters. **No change, no special case, and no exception written down** — if it
needed one, the rule would be wrong. Do not touch it and do not mention it in
the ADR.

## B5. ADR-0039 — amend 27's restatement in place

Ticket 27 restated `## The decision` cleanly today and moved the three
amendments below it under a history heading. **Edit that restatement in place.**
Do not append a fourth amendment and do not add a second restatement layer — a
new layer on top of today's clean one is the autopsy growing back.

Into the decision itself: the bottom (B2), the doorway's position (B1) and the
reason the bare-key rung stays (B3).

## Acceptance (every tick names its verifying test inline, at tick time)

- [x] **No box splits a column.** Test:
      `crates/graft-ui/tests/pane_boundaries.rs::no_box_drawn_over_the_graph_splits_a_row_column`
      — widths **20..=240** × heights **3, 8, 24, 40, 48**; at every width it
      takes the boundaries the rows were drawn to, reads the box's rect back
      out of the drawn buffer, and requires both of its edges to be in that
      set (or the band cleared). Mutation: in `snap_to_columns`, replace
      `let left = boundaries…max()?` with `let left = rect.x` (only the right
      edge moves) — fails at `66x3: the confirm box starts at column 1 and
      the row columns begin at [0, 4, 6, 53, 54, 59, 66]`.
- [x] **A non-clearing box covers; it never alters.** Test:
      `crates/graft/src/view/tests.rs::a_box_that_does_not_clear_covers_and_never_alters`
      — the same state drawn with and without the confirm box, every cell
      outside the box's rect compared. Its doc comment says it plainly:
      "**This checks the character grid only.** `TestBackend` records symbols
      and this assertion reads symbols, so a dimming or tinting layer painted
      over the rows beside the box would pass here unseen — captures carry no
      colour attributes (CLAUDE.md rule 5). The claim is exactly 'no cell's
      character changed', and no more."
- [x] **The credential prompt clears its band at every width.** Test:
      `crates/graft/src/view/tests.rs::the_credential_prompt_clears_the_band_it_stands_on_at_every_width`.
      It fails before the change — mutation putting `draw_credential` back to
      `clear_and_tint(frame, rect, rect, theme)`: `40x24: row 9 column 0 still
      holds "●" beside the credential prompt Rect { x: 2, y: 9, width: 36,
      height: 5 }`. The same test asserts the footer row still has content, so
      it is the band that clears and not the screen.
- [x] **The confirm box does not clear, and its left edge is on the subject
      column's boundary.** Test:
      `crates/graft/src/view/tests.rs::the_confirm_box_keeps_the_graph_beside_it_and_starts_on_the_subject_column`
      — graph rows still standing beside the box, and `box.x` equal to the
      subject column's x read out of the picture. The `let left = rect.x`
      mutation fails it: `80x24: the box starts at column 8 and the subject
      column starts at 4 — the box splits it`.
- [x] **No border puts text against its rule.** Tests:
      `crates/graft-ui/tests/border_rules.rs::the_confirm_box_and_the_credential_prompt_keep_their_rules_off_their_text`
      and `…::the_ghost_preview_and_the_name_prompt_keep_their_rules_off_their_text`
      — **four** surfaces (confirm, credential prompt, Ghost Preview box, name
      prompt) at **five** widths each (40/56/72/80/120). Stated over the drawn
      cells, not over a helper: a `─` never neighbours a cell holding text, so
      a fifth surface growing its own `border_row` fails the day it is drawn.
      Only the credential prompt had the fault; the other three already went
      through `frame_box::border_row`.
- [x] The four captures regenerated, byte-identical across two runs —
      `diff -q` printed nothing for
      `m5-ui-{confirm,credential}-over-graph-{80x24,160x48}.txt` between run 1
      and run 2. The rows beside each box:
      - confirm 80x24 — `●   ╭ Push main → origin/main ──…──╮60af0eb`: the
        marker column left, the **whole** sha right (it was
        `●   Comm│ … │ b6a4a47` before, the subject clipped to `Comm`).
      - confirm 160x48 — `●   ╭ Push main → origin/main ──…──╮ GF 2d   c8254c5`.
      - credential 80x24 and 160x48 — nothing beside it by design; the band is
        blank to both frame edges, and the writer's `check()` asserts
        `rows_with_content_beside_the_box(credential_box) == 0`.
- [x] No plaintext password in any regenerated capture; the mask is the fixed
      8 glyphs. Asserted by the writer before it writes, in
      `crates/graft/tests/it/two_boxes_over_content.rs::check` —
      `!body.contains(TYPED)` per box, and `matches(['•','◦']).count() ==
      text_input::SECRET_GLYPHS` (ADR-0038 §2). By hand as well:
      `grep -c hunter2` = **0** in each of the four files.
- [x] ADR-0040 exists (`docs/adr/0040-a-box-covers-whole-columns-or-it-clears.md`)
      and states the rule — "**A non-clearing box covers whole columns only.
      It never splits one. If no snap position exists that covers whole
      columns and still fits its content, it clears.**" — with both halves of
      the border/background line: "A border makes a box's edge explicit, so
      content beside it reads as *background* rather than as part of the box —
      **and that is only true while whole columns survive beside the box.
      Fragment a column and the border does not save you**… Both halves of
      that sentence are the decision; neither stands alone."

### Part B

- [x] **The doorway leads the bare keys where a tail exists, and is last where
      none does.** One test asserts both layouts:
      `crates/graft-ui/src/graph_screen/hints/tests.rs::one_rule_puts_the_doorway_before_a_tail_and_last_without_one`
      — every context whose doorway is `. more`, swept across widths: where the
      drawn list ends in bare keys the doorway is the first of them, where it
      has none the doorway is last. (Contexts whose doorway is `. explain`,
      i.e. `Marked`, are excluded by construction — they have no `. more`.)
- [x] **The three `ends_with(". more")` tests and every affected caption now
      state the new promise.** Tests, all in `crates/graft/tests/it/hint_bar.rs`:
      the three assertions now read
      `bar.starts_with(" ⏎ open  c commit  P push  . more") && !bar.ends_with(". more")`
      (and the `r rebase` and `⏎ changes` variants), and the bar finder changed
      from `ends_with(". more")` to `contains(". more")`. **Three captions
      moved**, covering 13 regenerated capture files: the hint-capture caption
      in `hint_bar.rs` ("always last" → "never dropped"), the ladder caption in
      `graph_screen/tests.rs`, and `docs/design-notes/m5-history-and-ghost-preview.md`'s
      Hint bar line. `grep` finds no "always last" / "is last and never
      dropped" anywhere under `docs/` or `crates/` except MAP.md's new sentence
      saying it is no longer true.
- [x] **The bar never draws a fragment of itself**, swept 1..=240 across every
      context — test:
      `crates/graft-ui/src/graph_screen/hints/tests.rs::the_bar_never_draws_a_fragment_of_itself`
      (at every width the drawn text is a whole floor, exactly `. more`,
      exactly `.`, or empty, and never wider than the room). Mutation: make
      `below_the_floor` return the doorway at its `Full` rung without checking
      the room — fails with `WorkingTreeRow with Shown asked for 7 cells of a
      1-cell bar, so the footer has to cut it: " . more"`, which at 4 cells is
      the ` . mo` the ticket asks for.
- [x] **The floor fits at 48** — test:
      `crates/graft-ui/src/graph_screen/hints/tests.rs::the_floor_fits_at_forty_eight_cells_of_bar`.
      Measured: the tightest context is `ChangesPane { unmerged: false }` at
      **38 cells**, against the narrowest reachable bar of **48**
      (`detail_pane/layout.rs`'s `RIGHT_MIN`; below 100 the pane is a full-width
      tab). Ten cells of room — a completeness gap, not a live defect.
- [x] **ADR-0039's decision carries the bottom, the doorway's position and the
      reason the bare-key rung stays, with no fourth amendment and no second
      restatement.** Headings are unchanged — `## The decision` … `## History`,
      three amendments, no fourth. Inside `## The decision`, edited in place:
      point 1 gains "**Where it sits: at the head of the bare keys — which is
      the end of the row when there are none.**"; the floor bullet is now
      "**The floor, and what is below it**" carrying `floor → \`. more\` → \`.\`
      → nothing` and "**The bar never draws a fragment of itself at any
      width**"; and a new bullet "**The bare-key rung stays.**" ends "**The
      menu covers discovery; the bar covers recall.**" Point 1 also names, in
      the same breath as "never all hidden", the one place that is false
      today — the Tab detail layout below 100 columns draws no bar at all,
      `crates/graft/tests/it/hint_bar.rs:262-288`
      (`the_tab_layout_still_hides_every_key`, `#[ignore]`d) — recorded as a
      violation with its own ticket, not as an allowance, and the rule is not
      softened to fit it (conductor's instruction, mid-ticket).
- [x] `rev` is untouched — `short_form()` extracted from `HEAD` and from the
      working tree and diffed: identical, 14 lines, `"revert" => "rev"` still
      line 10 of it.
- [x] **One capture, showing both halves at narrow width.**
      `docs/shots/m5-hints-ladder.txt` regenerated **as built** by
      `graft-ui/src/graph_screen/tests.rs::capture_the_ladder_for_docs_shots`:
      the variant A / variant B comparison blocks are deleted (the question is
      answered), the header now states the doorway's position and the bottom of
      the ladder, and every rung in the file draws `. more` leading the bare
      keys. The boxes at narrow width are in
      `docs/shots/m5-ui-{confirm,credential}-over-graph-80x24.txt`, each with
      the hint bar in the same frame.

Rule 6: **you do not tick "looks right".** The user does.

## Gates

`cargo fmt --check`, `cargo clippy --all-targets --message-format=short
-- -D warnings`, `cargo nextest run`, `cargo test --doc`. **Workspace-wide,
never `-p`.** Measure the baseline yourself first and report both numbers.

`CARGO_TARGET_DIR=/tmp/graft-target-g28`, deleted before hand-back. One cargo
command at a time.

**Never bare `git stash`.** **Never `git checkout` or `git restore` across your
worktree.** Captures under 200 KB. **Commit, never push.**

## Report

What the scout found and whether the boundaries existed as data. Which widths
forced the confirm box to clear, if any. Whether a third bordered surface had
the rule-against-text fault. Anything unverified — which is not broken.

## NOTES (builder, 2026-09-21)

**Scout: used** (`Explore`, sonnet, Part A's brief). What it found, and the
one thing that decided the shape of the build: **the column boundaries
already existed as data.** `graph_screen::layout::Columns::compute(area,
graph_width)` is what both the rows and the footer are drawn from — marker,
subject, `SUBJECT_GAP`, author chip (dropped below `AUTHOR_BREAKPOINT`),
when, sha, laid out right-to-left from `area.right()` — but it was
`pub(super)`. So nothing had to be re-derived and nothing had to be
recomputed: `GraphScreen::column_boundaries(area)` hands the same numbers
out, which is why the snap and the draw cannot disagree. `Columns` itself
stays private.

**Which widths forced the confirm box to clear: none.** With the frame's own
two edges in the boundary set, an outward snap always exists inside the
frame, so across 20..=240 the clearing branch is never reached. It is the
rule's floor, not dead weight — an overlay anchored somewhere other than the
frame's centre can reach it — and ADR-0040 says exactly that rather than
implying the branch was exercised. **The `None` arm is therefore covered by
unit tests of `snap_to_columns` only, not by a drawn frame.** Unverified in
the drawn path, not broken.

**A third bordered surface with the rule-against-text fault: no.** Four
surfaces draw a label into a border — confirm, credential prompt, Ghost
Preview box, name prompt. Three already went through
`frame_box::border_row`, which has always put a space on both sides. Only
`credential_prompt` kept a private copy (`cells`/`border_row`/`content_row`)
that spaced the left only. The copy is deleted; `border_rules.rs` now states
the property over drawn cells, so a fifth surface growing its own helper
fails the day it is drawn.

**For the user's eye (rule 6 — not ticked here):**
1. The confirm box's right edge now lands exactly on `sha_x`, so at 80 the
   border touches the sha with no blank between them:
   `╭ Push main → origin/main ──…──╮60af0eb`. Whole columns, per the rule,
   but tighter than the old (column-splitting) picture, which had a space.
   At 160 the gap column belongs to the author chip, so it reads
   `╮ GF 2d   c8254c5`.
2. At 160 the box grows to the subject column's **full** width — a very wide
   box for a two-line message, and a consequence of "never shrink to reach a
   boundary" (clause 2). Both are in
   `docs/shots/m5-ui-confirm-over-graph-{80x24,160x48}.txt`.

**Mid-ticket instruction from the conductor**, carried out in ADR-0039's
`## The decision` only, no code touched: "never all hidden" now names the
Tab-detail-layout violation in the same breath
(`crates/graft/tests/it/hint_bar.rs:262-288`). Recorded as a violation with
its own ticket; the rule is not softened.

**No compaction affected the work** — the session was summarised between
Part A's captures and the acceptance write-up, with both parts' code and
tests already complete and green.

**Gates, workspace-wide:** baseline `2250/2250` measured on this worktree
before the first change; final **2260 passed, 39 skipped**, +10 tests.
`cargo fmt --check` clean, `cargo clippy --all-targets -- -D warnings`
clean, `cargo test --doc` 2 passed. `CARGO_TARGET_DIR=/tmp/graft-target-g28`
throughout, deleted at hand-back.

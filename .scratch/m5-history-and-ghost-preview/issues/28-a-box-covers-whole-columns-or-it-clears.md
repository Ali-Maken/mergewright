# a-box-covers-whole-columns-or-it-clears

Status: ready
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

- [ ] **No box splits a column.** A sweep over widths **20..=240** and heights
      **3, 8, 24, 40, 48** — ticket 19's shape, and it belongs in
      `crates/graft-ui/tests/pane_boundaries.rs` beside that sweep. For every
      width where a box is drawn, either every column beside it is whole or the
      box cleared. Name the test and give a mutation that breaks it.
- [ ] **A non-clearing box covers; it never alters.** Render the same state with
      and without the box; every cell **outside** the box's rect is identical.
      Name the test. **Say plainly in the test's own comment that this checks
      the character grid only** — captures carry no colour attributes
      (CLAUDE.md rule 5), so a dimming or tinting layer would pass this unseen.
      Claim exactly what it checks and no more.
- [ ] **The credential prompt clears its band at every width** — test: name it.
      It must fail before the change.
- [ ] **The confirm box does not clear, and its left edge is on the subject
      column's boundary** — test: name it.
- [ ] **No border puts text against its rule** — test: name it, and say how many
      surfaces it covers.
- [ ] The four captures regenerated, still byte-identical across two runs —
      evidence: `diff -q` output, and quote the row beside each box.
- [ ] No plaintext password in any regenerated capture; the mask is the fixed
      8 glyphs (ADR-0038 §2) — test: name it, and `grep -c` by hand as well.
- [ ] ADR-0040 exists, states the rule and both halves of the border/background
      line — evidence: quote the clause.

### Part B

- [ ] **The doorway leads the bare keys where a tail exists, and is last where
      none does** — test: name one test that asserts *both* layouts, since the
      point is that one rule produces them. A row context with a tail and the
      journal or changes pane without.
- [ ] **The three `ends_with(". more")` tests and every affected caption now
      state the new promise** — evidence: name the tests and say how many
      captions moved. A caption still describing the old position is a false
      picture under rule 6.
- [ ] **The bar never draws a fragment of itself**, swept 1..=240: at every
      width the bar is either a whole floor, exactly `. more`, exactly `.`, or
      empty — never a clipped prefix of any of them — test: name it, and give
      a mutation that produces `. mo`.
- [ ] **The floor fits at 48**, the smallest reachable bar width
      (`detail_pane/layout.rs:34-36`) — evidence: report the measured width for
      the tightest context. If it does **not** fit at 48, that is a live
      defect rather than a completeness gap; stop and report it.
- [ ] **ADR-0039's decision carries the bottom, the doorway's position and the
      reason the bare-key rung stays, with no fourth amendment and no second
      restatement** — evidence: the heading structure and the three sentences.
- [ ] `rev` is untouched — evidence: `git diff` on `short_form()` is empty.
- [ ] **One capture, showing both halves at narrow width** — the boxes and the
      bar in the same picture, which is why these were folded. Regenerate
      `m5-hints-ladder.txt` with variant B **as built** (the variants were the
      question; the question is answered, so as-built is B and the A/B
      comparison goes).

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

# a-box-covers-whole-columns-or-it-clears

Status: blocked
Blocked by: **27** (shares `graft-ui` and the capture writers; ADR-0036 needs
disjoint crate sets, so these run in sequence, not together).
Crate set: **`graft-ui` + `graft`.**
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

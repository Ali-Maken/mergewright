# the-tab-layout-draws-no-keys-at-all

Status: ready
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

- [ ] The bar is present in Tab layout at 80 columns with the changes pane
      open — test: the un-ignored, renamed `hint_bar.rs` test. It must fail on
      the current code.
- [ ] Present for **every** pane context that can own the Tab layout, not just
      the changes pane — test: name it and say which contexts it covers.
- [ ] The mouse still selects the row the user clicked, below 100 columns —
      test: name it. This is the regression this change most likely causes.
      Give a mutation: leave `rows_above_graph` unchanged and show it goes red.
- [ ] The graph's own layout at ≥100 columns is unchanged — test: name it.
- [ ] Ticket 28's floor and ladder govern this bar too — test: name the sweep
      and say it covers the Tab context.
- [ ] Captures below 100 columns regenerated, each caption read against its own
      file — evidence: quote one row per file.

### Part 2

- [ ] **No box border touches content beside it**, swept over the same widths
      the no-split assertion uses — test: name it, and give a mutation that
      reproduces `╮60af0eb`. It must fail on the current code.
- [ ] Where the gutter cannot be honoured, the box clears rather than shrinking
      its content — test: name it, and say which width first forces it (ticket
      28 reported that **no** width in 20..=240 forces the confirm box to clear
      today; the gutter may change that, and if it does not, say so).
- [ ] ADR-0040 amended **in place**, no appended amendment — evidence: quote
      the amended clause.
- [ ] The four over-graph captures regenerated and still byte-identical across
      two runs — evidence: `diff -q` output, and quote the row beside the box.

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

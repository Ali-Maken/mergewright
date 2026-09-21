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

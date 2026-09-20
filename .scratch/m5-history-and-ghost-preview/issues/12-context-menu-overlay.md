# 12 — context menu overlay

Status: ready
Blocked by: none (file-fenced against group 09 — see Fence)
Build with: **Opus** — brownfield in a region three people have already misread; the cause fix is small, the evidence fix is the point.

Filed by the **user**, from `docs/shots/m5-ui-context-menu-verbs-80x24.txt`
and `-160x48.txt`, after a builder, a reviewer and the conductor all read the
same capture and missed it. His words are the spec; the numbering is his.

## Scout first

`Explore`, sonnet, ≤1 page: what draws the `.` menu, what region it claims,
does the conflict banner share that path.

## The four defects

1. **The menu does not clear the cells it covers.** Six menu rows carry the
   underlying commits' right-hand columns:

   ```
    Copy SHA (y)            GT now  6d3bd01
    Checkout main           GF 7y   a82960b
   ```

   Only six because the fixture has 8 commits; **every item would bleed in a
   real repo**. Reads as if the verb is bound to that sha. **Fix the cause —
   clear the region before drawing — not by padding labels.**

   Conductor's trace, to save the scout the work: `view/frame.rs` calls
   `frame.buffer_mut().set_style(rect, bg)` and then renders a widget that
   paints only its own text. `set_style` changes cell colour and leaves cell
   symbols untouched. Four sites share the call — `draw_preview` (453),
   `draw_context_menu` (482), `draw_confirm` (502), `draw_credential` (534).
   One is confirmed by eye; **check the other three, do not assume them**
   either way.

2. **`crates/graft/tests/it/preview.rs:664` `capture_context_menu_for_docs_shots`
   asserts nothing.** It renders, formats the caption, `fs::write`, returns.
   No M5 tick can name it as its verifying test. Add a real test asserting
   the menu's region is **opaque**: for each menu row, everything right of
   the label is blank. **That test is what the tick names.**

3. **`Reset feature to here (mixed)` / `(hard)` collide** with the menu's own
   stated rule that a trailing `(x)` is a key. Pick one shape.

   (That rule is the conductor's, ruled two messages before this ticket was
   written, which is what created the collision. Either shape may win; say
   which and why.)

4. **The caption block is byte-identical in both files**, so the 160x48 file
   carries "At 80 columns the menu still fits…". Make the caption depend on
   the width being captured. These lines feed the M5 contact sheet (ADR-0037).

## Two spec questions — do NOT decide these in the build. Report and stop.

a. The caption claims the menu carries "the keys the hint bar drops". **False
   in both captures**: 80x24 drops neither `⏎` nor `P`, 160x48 drops nothing,
   yet the menu lists Fetch/Pull/Push/Undo. Either the menu filters against
   the live hint bar, or the claim goes.

b. The hint bar is unchanged while the menu is open, and the `.` menu is not
   among `02-keymap.md` §2.1's five modal panes. **Whether it claims keys is
   undecided in the design.** §2.1 says the hint bar always shows what's
   active.

**Read `docs/design/02-keymap.md` §2.1 and §2.15 before touching this.**

## Fence

`crates/graft/src/view/frame.rs`, `crates/graft/tests/it/preview.rs`,
`crates/graft-ui/src/context_menu.rs`, `crates/graft/src/update/context_menu*`,
`docs/shots/m5-ui-context-menu-verbs-*`.

Group 09 holds `update/*`, `view/rows.rs`, `view/journal.rs`, `jobs/*`,
`event_loop/*` concurrently. **Do not touch those.** Need one → report
`blocked:` naming the signature and stop.

## Done means

Cause named in the report; new assertion test passing; both captures
regenerated; gates green workspace-wide (`cargo fmt --check`, `cargo clippy
--all-targets -D warnings`, `cargo nextest run`, `cargo test --doc`).
**Colour ships unverified — say so (rule 5). Do not tick the visual result;
the user judges by eye.**

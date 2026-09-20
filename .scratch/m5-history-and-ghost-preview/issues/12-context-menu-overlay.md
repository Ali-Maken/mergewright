# 12 — context menu overlay

Status: done
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

---

## Status: done (uncommitted; conductor commits)

Scout used (`Explore`, sonnet). It corrected a wrong premise of mine mid-build:
`graft_ui::fit::fit` **pads** to its width argument (`fit.rs:67-70`,
`fit::tests::short_text_is_padded_to_the_width`). I had read only the first
half of `fit` and concluded it truncated without padding.

### 1. The cause — not the one in the ticket's trace

The traced hypothesis ("`set_style` changes colour and leaves symbols
untouched, so the menu's rect keeps stale symbols") is **false for this
widget**. `ContextMenu::render` writes `fit(" {label}", area.width)` per row
and `fit` pads, so every cell of the popup's own rect is written.

The real cause: **`context_menu_bounds` gives the popup `menu.width()` —
longest label + 4, i.e. 33 columns — so the graph rows it floats over keep
drawing their date and sha columns to the *right* of it, on the popup's own
lines.** Nothing was stale inside the popup; the defect is entirely outside
it. In a colourless character grid `Copy SHA (y) … now 6d3bd01` reads as one
row.

Direct evidence, from the red test's own output before the fix:

```
 Compare with this (x)           ure                                now  b1c1a2b
```

`ure` is the tail of "finish the feature" starting at column 33 — the first
column past the popup. The popup is opaque up to 33 and dirty beyond.

Fix (`view/frame.rs`): clear the **band** the popup stands on — its rows,
from its left edge to the end of the pane — with ratatui's `Clear`, *then*
tint the popup's own rect and draw it. Two new private helpers,
`primary_pane_area` and `overlay_band`. The popup is not widened and no label
is padded.

### The four `set_style` sites — checked, not assumed

Determined by reading each widget's `render` and, for the menu, by an
executable probe.

| site | own rect opaque? | how |
|---|---|---|
| `draw_preview` | yes | `preview_box.rs:249-251` blanks every row first |
| `draw_context_menu` | yes | `fit` pads every row (probe-confirmed) |
| `draw_confirm` | yes | `confirm.rs:186-188` blanks every row first |
| `draw_credential` | yes | 5 rows, each padded; `credential_area` caps height at `BOX_ROWS` |

**So none of the four leaves stale symbols on its normal path, and the
ticket's premise holds for none of them.** What *is* shared is the narrow-rect
problem: `draw_confirm` and `draw_credential` are centred boxes, so content
survives beside them too — less misleading than a left-anchored list of verbs,
but the same class. Not fixed: out of this ticket's scope.

**Real defect found, not fixed** — the degraded paths:
`PreviewBoxWidget` (`preview_box.rs:237-247`) and `CredentialPrompt`
(`credential_prompt.rs:189-205`) both early-return after writing **one** row
when the rect is too small, leaving every other row of an already-tinted rect
holding whatever was underneath. Reproduce by shrinking the terminal under an
open preview or credential prompt. A `Clear` at those two sites in `frame.rs`
fixes it, inside this ticket's file fence, but it is not this ticket.

### 2. The assertion test

`crates/graft/tests/it/preview.rs`
`the_context_menu_is_opaque_on_every_row_it_draws`. For each menu label, at
80x24 and 160x48, the drawn line must equal exactly `" {label}"` (`frame`
trims trailing blanks, so any surviving cell to the right makes the line
longer).

Mutation, run against the final tree: deleting the `Clear` call in
`draw_context_menu` and running `cargo nextest run --no-fail-fast`
workspace-wide gives **2110 passed, 1 failed**, the one failure being
`graft::it preview::the_context_menu_is_opaque_on_every_row_it_draws`. It is
the only test in the workspace that catches it. The three `overlay_band` unit
tests in `view/frame.rs` do **not** catch it — they pin the helper's geometry,
not that it is called.

`capture_context_menu_for_docs_shots` still asserts nothing; it is a capture
writer. The tick belongs to the test above.

### 3. Label shape — the conductor's ruling kept

Kept `(key)` = "this letter is live right now". Changed the qualifiers that
were wearing it. Found **three**, not the two the ticket named, all in
`update/context_menu/verbs.rs`; fixing only the two would have left the rule
half-applied and the tick false:

- `Reset <cur> to here (mixed)`/`(hard)` → `Reset <cur> to here, mixed`/`, hard`
- `Checkout <sha7> (detached)` → `Checkout <sha7>, detached`
- `Revert <sha7> (-m 1)` → `Revert <sha7>, -m 1`

Comma over dash: ASCII-safe (the menu widget carries no `ascii` flag, so an
em dash would be wrong at the ASCII tier), and it keeps the shared `Reset …`
prefix so the list still scans. Menu width 33 → 32; `docs/design-notes/
m5-history-and-ghost-preview.md:245` still quotes the old parenthesised form.

### 4. Caption

Width-dependent now, via a new `context_menu_caption(label, columns, rows)`
helper (also clears a `too_many_lines` clippy failure the inline version hit).
Per ruling (a), the false "the keys the hint bar drops" claim is deleted and
replaced with what is true: the menu lists every verb valid for the row at
every width and does not filter against the hint bar. Code confirms no
filtering exists — `ContextMenu::for_row` is row actions + history verbs +
`always_items()`, with no width or hint-bar input.

### Defect found under ruling (b) — reported, not fixed

**`update/context_menu.rs:123`**: `_ => next.context_menu = None,` falls
through to `true` at line 125, and `keys.rs:64` does `if context_menu_key(…)
{ return; }`. So **any** key that is not `j`/`k`/`↑`/`↓`/`Enter` closes the
menu *and is swallowed*. Pressing `m` with the menu open does not merge — it
just closes the menu. That directly breaks ruling (b)'s "a key the menu
displays must work when pressed": `Merge main into feature (m)` promises `m`
is live and it is not.

Fix shape (not built): return `true` for `Esc` only, `false` for anything
else, so the key falls through to the rest of `key_event` in the same call.
That is contained in `update/context_menu.rs` — my fence — but the stale
"is modal" comment at `keys.rs:62-63` is **group 09's file**, and the change
is outside this ticket's four points. Left for the conductor to schedule.

### Captures

Both regenerated from the real path, 3167 B and 3758 B (cap 200 KB).
**Colour ships unverified** — TestBackend records a character grid with no
colour attributes (rule 5). **Not ticked as looking right: the user judges by
eye** (rule 6).

**Disclosure:** the captures are **not reproducible run to run**. The
fixture's commits all share a timestamp, so date-order sorting puts `main` at
a different row each run and the menu anchors at a different height. I ran the
capture twice and kept the run showing more of the graph above the menu; both
runs show the fix equally (no bleed in either), and the caption bytes are
identical between them. Pinning `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE` in the
fixture would make the contact sheet stable — needs `app_harness.rs`'s
`git_as_graft`, which is not in this fence.

### Gates

Workspace-wide, in an isolated worktree and target dir: `cargo fmt --check`
clean, `cargo clippy --all-targets -- -D warnings` clean, `cargo nextest run`
**2111/2111** (2107 on main + 1 opacity test + 3 `overlay_band` unit tests),
`cargo test --doc` ok.

MAP.md: `crates/graft/MAP.md` updated for the three label shapes, the
`(key)`-is-reserved rule, and `draw_context_menu`'s band-clear.
`crates/graft-ui/MAP.md` needed no change — `context_menu.rs` was not
modified.

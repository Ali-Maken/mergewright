# M6 rulings 07b — questions after continuation 2 of 07-acceptance-conflicts

Source: `.scratch/m6-conflicts/issues/07-acceptance-conflicts.md` lines 101 and 141-144. Captures read:
`docs/shots/m6/m6-09-start-over-box-{120x40,80x24}.txt`. No source read.

## Q1 — hint row under a live confirm box offers keys the box swallows

**Ruling (planner, 2026-09-23):** **Defect, M6 fix.** Lines as filed: `update/keys.rs:34` (`confirm_key` claims
first), `view/rows.rs:385-404` (`overlay_context`, no confirm case), `view/rows.rs::hint_context` (same). Observed
on `m6-09` at both sizes: `o ours  t theirs … d start over  esc close` under `Start both.txt over?`. Breaks the
rule at `resolver/answers.rs:248-250` (a line never offers a key that does nothing).

- What code must do: while any modal box is live (confirm, name prompt), the bottom row keeps the status ladder
  (problem, else notice) and otherwise shows **only the keys that box's handler accepts**, read from
  `confirm_key` / the name prompt's handler — no key the builder has not seen handled. The credential prompt
  already does this (ADR-0040: `push main → origin/main · esc cancels`); follow its shape.
- One site, not per screen: decide the hint context for a live box before the screen/overlay match, as F10 did
  for drawing (`draw_modal_boxes`).
- Test: an invariant over every `GraphOverlay`, the bare graph and the status screen, 120x40 and 80x24, in the
  style of `view::frame::tests::a_live_confirm_box_is_drawn_over_every_overlay`: under a live confirm, no key
  of the screen behind appears on the bottom row. Red first.
- Keymap: no new key; this changes which existing keys are shown. Crate set `graft` (+ `graft-ui` if
  `HintContext` needs a variant — `docs/design-notes/m6-conflicts.md:92`).
- Recapture `m6-09` both sizes; its contact-sheet caption names the new bottom row.

## Q2 — 80x24 start-over box does not clear the rows it stands on

**Ruling (planner, 2026-09-23):** **Defect, M6 fix. The resolver's box clears its band.** Observed on
`m6-09-start-over-box-80x24.txt` rows 10-13: `▏ THEIRS╭ Start both.txt over?` — the border fused to the label,
clause 3's `╮60af0eb` fault; theirs' line hidden entirely; the closing rule cut to `▏ ──────│ … │────────`. The
cause the filer names: the resolver passes an empty boundary set (`view/frame.rs`, `draw_confirm` →
`snap_to_columns(natural, &[], area)`).

- The resolver's document counts. ADR-0040's reason is geometric — a fragment beside a border reads as content —
  and cut code lines and a cut side label are that fragment. Clause 4's "empty set, correct by construction"
  was written for lists without columns; it does not hold for a document whose rows are code.
- Snapping cannot help: at <100 the document is one column running the full width, so no outward snap fits
  (clause 2 → clear). At ≥100 the box sits over blank rows today (`m6-09-…-120x40` rows 18-26); clearing
  changes nothing there. So: **clear the band, full width, at every width** — the credential prompt's
  treatment, not the confirm's snap. The M5 reason for snapping (the graph is what you act on) does not carry:
  the box's own text says what is lost.
- ADR-0040 "What each box does" gains one line: *the confirm box over the resolver clears its band.* Journal,
  stash, Map: not re-ruled — no capture shows a fuse there.
- Test: 80x24 and 120x40, `d` in the resolver on `both.txt`: every cell in the box's rows outside the box is
  blank. Red first at 80x24. Recapture `m6-09` both sizes.

## Q3 — status flush left in the resolver footer / 80-col Detail tab; graph footer indents one space

**Ruling (planner, 2026-09-23):** **Defect (textual), M6 fix.** Stands on the capture: the hint row on the same
bottom line starts at column 1 (` o ours …`, `m6-09`), the graph footer's status too
(`graft-ui/src/graph_screen/footer.rs:74,81`); the resolver's status starts at 0. What code must do: the shared
`draw_status_or_hints` (F9) indents a status one space, so status and hints start in the same column on every
bottom row. Test: 80x24 Detail tab and resolver footer, a Sticky — first cell blank, text from column 1. Recapture
the frames that show a status on those rows (`m6-06`, `m6-07`, `m6-08` at 80x24, and any resolver status frame).

## Q4 — `zdiff3.txt` carries its base but draws none until `B`

**Ruling (planner, 2026-09-23):** **Wording drift (TESTING.md), not a code defect.** The design says `B` is
show/hide base for **every** open block at every width (`docs/design-notes/m6-conflicts.md:130`, `:170`); one rule
for every file, and the hint bar always offers `B`. "Carries" means the block has its base from its own markers
before any `B` — no `merge-file` load — which the walk proves (`block.base` set). The code is right.

- TESTING.md scenario 6 rewritten: *`zdiff3.txt`'s one block carries its base from its own markers before `B` (no
  load) and, like every block, draws it only on `B`; …* The rest unchanged.
- `OURS · ours` / `THEIRS · theirs` on `zdiff3.txt`: not a defect — the labels are the file's own marker labels,
  which the fixture wrote as `ours`/`theirs`.

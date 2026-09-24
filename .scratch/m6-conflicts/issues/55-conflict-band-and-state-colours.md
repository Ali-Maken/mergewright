# conflict-band-and-state-colours: the band between the panes, state colours on cards and the file list, `•` for an unread file

Status: ready
From: user picks 2026-09-24 items 1 (separator style A, 30 %, Okabe pair) and 11 (batch 10, `•`); field notes :61;
ADR-0044 §1-§2; KNOWN_GAPS M6/21 :771 (the `…` question), M6/13 :656.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: in `graft-ui::conflict_resolver`, where `panes/draw.rs` draws the two separator columns, the pane titles
and the rule words, and where `draw/block.rs` and `files.rs` style their rows; what `Glyphs` holds per tier; **which
tests outside `graft-ui` pin the separator column, a rule row or the file list's `…`** — split into (a) captures under
`docs/shots/` asserted through `graft_fixtures::capture` (regenerated, no source edit) and (b) `graft` source tests that
assert text or cells (these need `graft`). If (b) is non-empty, list them: the conductor then runs this spec after lane
O with `graft` added. Also: can a `graft-ui` test write a capture (`graft-fixtures` a dev-dependency?), or does the
capture come from `.scratch/m6-conflicts/pty-walk-m6.py`.
Needs: `conflict-colour-tokens` merged (`Token::{Ours, Theirs, Open}`, `Theme::tint`).
Crates: graft-ui only (plus `docs/shots/` regenerated). If the scout's (b) list is non-empty: graft-ui + graft, after
lane O.
Lane: wave 1, lane U, second.

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §1, §2 (the band table is the spec).
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24: "Three panes" (:968-1013), "Card and file list"
  (:1015-1025), the mockup (:993-1007), the Touches items for `conflict_resolver` (:1112-1116).
- `crates/graft-ui/MAP.md` lines 12-15 (`DocView`, `BlockState`, `TakeView`, file list, `Glyphs`, panes).
- Source, only after the MAP: `conflict_resolver/panes/draw.rs`, `conflict_resolver/draw/block.rs`,
  `conflict_resolver/files.rs`.

## Scope
**In:**
- **Three panes (≥ 180 columns):** ADR-0044 §2's table exactly — per block state, the separator glyph on the rule row
  and below it, its colour, and the tint of the yours / result / theirs rows. Tints fill each pane's full width on those
  rows, separator cell included. Plain rows keep `│` in `border`.
  - `Took(Both)`: the first `ours.len()` result rows `ours-tint`, the rest `theirs-tint`, git's order.
  - The cursor block's separator glyphs BOLD; the other blocks the same colours, not bold. `▏` and the reversed rule row
    as built.
- **Titles:** `yours · <branch>` in `ours` BOLD, `theirs · <branch>` in `theirs` BOLD, `result` in `text` BOLD. By
  position: the left title is always `ours`, whatever role word it carries (rebase: `upstream · …` in `ours`).
- **Rule words:** `── o take` in `ours`, `── t take` in `theirs`; `── taken ✓` in the taken side's hue, `── not taken` in
  `text-muted`; result `── i of n · open` in `open`, `── i of n · took theirs` in `theirs` (took yours in `ours`, took both
  in `text`). Text unchanged.
- **Open placeholder:** `? open · o t b` in `open`, BOLD, on `open-tint`.
- **Card (< 180 columns):** no glyph change. Open block, sides paired: left half `ours-tint`, right half `theirs-tint`;
  stacked (80): each side's rows in its tint. Labels in their hue, BOLD. A taken block's lines in the taken side's tint,
  its header's side name in that hue; the header's `open` in `open`.
- **File list:** `n of m open`, `keep or delete` and `⏎ marks done` in `open`; `done ✓` in `text-muted`.
- **Unread mark (batch 10):** `Glyphs` Unicode `…` → `•` for `FileState::Unread` only; ASCII `~` unchanged; `… N more`
  (the strip's overflow cell) unchanged.
- **Fallbacks:** 16 colours and high contrast — no tints (`Theme::tint` is `None`), hue on titles, rule words and
  separator glyphs only, the side not taken `text-muted`. `NO_COLOR` — no colour; DIM on the side not taken, BOLD on
  `? open`, glyphs as in the table.
- **ASCII tier:** `?`, `|`, `>`, `<`, `:` for `?`, `┃`, `▶`, `◀`, `┊`, gated like the existing `dot()`.

**Out:**
- `Took(Both)` after a hand edit (no result tint): needs `Choices.edited` from `graft`
  (`resolver-words-and-marks`). Until then `Took(Both)` always draws split; say so in NOTES.
- The all-done list head and the finish row (`last-file-opens-the-summary`).
- Any width change: the separator stays one cell (ADR-0043 §2).
- Rule-word text, notice text, hint text.

## The failing test first
`graft-ui/src/conflict_resolver/tests/three_panes/` (the fixtures module the MAP names):
`band_follows_the_block_state`.
- A `DocView` with four blocks: open, took yours, took theirs, took both (ours 1 line, theirs 2), at 187 columns,
  truecolor dark theme. Render to a `Buffer`.
- Assert per block, on its rule row and one body row: the left and right separator cell's symbol, fg and modifier; the
  bg of one yours, one result, one theirs cell; for took-both, result row 0 `ours-tint` and rows 1-2 `theirs-tint`.
- Red today: separators are `│` in `border`, no bg.

## Signatures
```rust
// graft-ui: no new public type. Private drawers in panes/draw.rs take the block state they already read:
//   fn separator(state: &BlockState, side: Side, row: RowKind, cursor: bool, theme: &Theme, ascii: bool) -> (char, Style)
//   fn row_tint(state: &BlockState, pane: Pane, result_row: usize, ours_len: usize, theme: &Theme) -> Option<Color>
// files.rs: Glyphs.unread changes to '•' (Unicode); ASCII unchanged.
```

## Touches (each entry, and how the builder checks it)
- **Captures that draw the three panes, a card or the file list** (the separator glyphs, and `•`):
  `m6-10`…`m6-14` at 187 and 200, `fix-three-panes-*`, `fix-advance-187`, `fix-file-keys-200`,
  `fix-stop-lands-in-resolver-*`, `fix-file-list-*`, `fix-done-*-200` (note :1162-1164).
  - Check: `grep -rln "│ result\|FILES ·\| · …" docs/shots/m6` before and after; regenerate with
    `GRAFT_UPDATE_SHOTS=1` (a red run, then green); list every rewritten file in NOTES with the line that changed.
    A file whose only change is outside the band, the list mark or colour is a finding, not a regeneration.
- **`graft` tests pinning text** (scout list (b)). Empty → nothing to do. Non-empty → this spec is sequenced after lane O
  with `graft` in its crate set.
- **Cursor affordance:** the `▏` gutter and the reversed rule row must stay the strongest cue (ticket 44's `selection`
  3.04:1 > every tint).
  - Check: `cursor_rule_row_keeps_its_reverse_over_a_tint` (buffer: modifier REVERSED on the cursor rule row).
- **Depths:** Check: `band_at_sixteen_colours_has_no_background` and `band_under_no_color_is_glyphs_and_modifiers`
  (no cell in the doc area has a bg other than reset; DIM/BOLD where the ADR says).
- **ASCII:** Check: `band_ascii_glyphs` (`?|><:`, no Unicode box glyph in the ASCII buffer), and the existing ASCII
  glyph gate.
- **Card:** Check: `card_tints_follow_the_block_state` at 120 (paired) and 80 (stacked).
- **File list:** Check: `file_list_state_colours` and `unread_file_is_marked_with_a_bullet` (Unicode `•`, ASCII `~`,
  the overflow cell still `… N more`).
- **Other screens that draw the same state:** the changes pane's `CONFLICTS` rows and the banner — unchanged.
  Check: `git diff --stat` shows no `graph_screen` or `detail_pane` file.
- **Journal, undo, keys:** none (pure draw).
- **Docs:** `crates/graft-ui/MAP.md` lines 12-15 (the band, the tints, `•`, "colour asserted by buffer tests; legibility
  unverified by eye"). KNOWN_GAPS rows M6/21 :771 and the resolver's "colour unverified" rows are the conductor's.

## Acceptance (name the verifying test at tick time)
- [ ] `band_follows_the_block_state`: red first, then green.
- [ ] `cursor_rule_row_keeps_its_reverse_over_a_tint`
- [ ] `band_at_sixteen_colours_has_no_background`
- [ ] `band_under_no_color_is_glyphs_and_modifiers`
- [ ] `band_ascii_glyphs`
- [ ] `card_tints_follow_the_block_state`
- [ ] `file_list_state_colours`
- [ ] `unread_file_is_marked_with_a_bullet`
- [ ] `titles_and_rule_words_take_their_side_hue`
- [ ] Capture (rule 6; **colour unverified**, rule 5): `docs/shots/m6/fix-band-187x50.txt` and `fix-band-120x40.txt`.
  - Scene: `notes.txt`, block 1 took theirs, block 2 open with the cursor on it (the note's mockup, :993-1007).
  - **What right looks like (text only):** at 187, block 1's rows show `┊` at the left separator and `◀` then `┃` at the
    right; block 2's rule row shows `?` at both separators and `┃` below; `? open · o t b` in the result pane; plain rows
    `│`. At 120 the card is unchanged in text. An unread file in the list reads `•`. The colours (sky blue left, purple
    right, amber open) are the user's to judge in Zed, dark, 187 columns.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: one module, many small style assertions, a capture sweep. One context.

## NOTES

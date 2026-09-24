# conflict-band-and-state-colours: the band between the panes, state colours on cards and the file list, `•` for an unread file

Status: ready
From: user picks 2026-09-24 items 1 (separator style A, 30 %, Okabe pair) and 11 (batch 10, `•`); field notes :61;
ADR-0044 §1-§2; KNOWN_GAPS M6/21 :771 (the `…` question), M6/13 :656.
Amended 2026-09-24 (spec-only pass) after scout `.scratch/m6-conflicts/scouts/54-56.md`. The planner re-read
`panes/draw.rs:26-59`, `files.rs:70-85, 230-351` and `graft/tests/it/resolver/three_panes.rs:55-89`.
Build with: builder-opus — many small style paths across three drawers, and a file-list cell split whose text must
stay byte-identical.
Needs: `conflict-colour-tokens` merged (`Token::{Ours, Theirs, Open, OursTint, TheirsTint, OpenTint}`, `Theme::tint`).
Crates: graft-ui only (plus `docs/shots/` regenerated). **No `graft` source edit is expected** (decision 1).
Lane: wave 1, lane U, second.

## Read first
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §1, §2 (the band table is the spec).
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24: "Three panes" (:968-1013), "Card and file list"
  (:1015-1025), the mockup (:993-1007), the Touches items for `conflict_resolver` (:1112-1116).
- `.scratch/m6-conflicts/scouts/54-56.md` "Resolver panes today" (the code as it stands, with lines).
- `crates/graft-ui/MAP.md` lines 12-15 (`DocView`, `BlockState`, `TakeView`, the file list, `Glyphs`, the panes).
- Source, only after the MAP: `conflict_resolver/panes/draw.rs`, `conflict_resolver/draw/block.rs`,
  `conflict_resolver/files.rs`.

## Decisions (made in this pass)
1. **The titles row and plain rows keep `│` in `border`.** ADR-0044 §2 puts the band on a block's rows only. That
   keeps `graft/tests/it/resolver/three_panes.rs:67-83` green unedited: it pins `│ result `, `│ yours · main `,
   `│ theirs · feature` on the titles row, and `│ 8 eight` on a plain row. **Crate conflict, resolved:** 55 stays
   graft-ui only and is not serialised behind lane O. If any `graft` test goes red, the builder stops and reports
   `blocked: needs graft`, naming the test and the line; it does not edit the test. The conductor then files that edit
   as a lane-O follow-up after 58.
2. **Titles change from `text-muted` to hue.** Today all three are `side_style(true)` = `TextMuted`, not bold
   (`panes/draw.rs:47`). New: the left title in `ours` BOLD and the right in `theirs` BOLD, by **position** (a rebase's
   `upstream · …` is still `ours`); `result` in `text` BOLD.
3. **`•` gets its own glyph field.** `Glyphs` has no `unread`: `more` (`…`/`~`) serves the unread state (`files.rs:83`),
   left-truncation (`:348`), column overflow (`files/column.rs:98`) and strip overflow (`files/strip.rs:168`). Add
   `unread: char` (Unicode `•`, ASCII `~`); `FileState::Unread` uses it; `more` and its three other users are
   unchanged.
4. **The file list colours its state words by splitting the style, not the text.** Today a cell has one style
   (`files.rs:248-252`), and the state words sit inside its text (`files.rs:71-84`). `Cell` gains
   `words_at: Option<u16>` (the column in the cell where the state words start; `None` when truncation cut them) and
   `tone: WordsTone`. The draw writes the name part in today's style and the words part in the tone's colour; the
   cursor's BOLD stays on both parts. **The cell text is byte-identical to today**, except `•`.

## Scope
**In:**
- **Three panes (≥ 180 columns):** ADR-0044 §2's table exactly. It gives, per block state, the separator glyph on the
  rule row and below it, its colour, and the tint of the yours / result / theirs rows. Tints fill each pane's full
  width on those rows, the separator cell included. Titles row and plain rows: decision 1.
  - `Took(Both)`: the first `ours.len()` result rows get `ours-tint`, the rest `theirs-tint`, in git's order.
  - The cursor block's separator glyphs are BOLD; other blocks get the same colours, not bold. `▏` and the reversed
    rule row stay as built.
- **Titles:** decision 2.
- **Rule words:** `── o take` in `ours`, `── t take` in `theirs`; `── taken ✓` in the taken side's hue, `── not taken`
  in `text-muted`. Result: `── i of n · open` in `open`, `── i of n · took theirs` in `theirs` (took yours in `ours`,
  took both in `text`). The text is unchanged.
- **Open placeholder:** `? open · o t b` in `open`, BOLD, on `open-tint`.
- **Card (< 180 columns):** no glyph change. An open block with its sides paired: left half `ours-tint`, right half
  `theirs-tint`. Stacked (80 columns): each side's rows in its own tint. Labels in their hue, BOLD. A taken block's
  lines in the taken side's tint, with its header's side name in that hue; the header's `open` in `open`.
- **File list:** `n of m open`, `keep or delete` and `⏎ marks done` in `open`, and so are their compact forms
  (`1/2 open`, `keep?`, `⏎`); `done ✓` / `✓` in `text-muted`; `binary` in the name's style (decision 4).
- **Unread mark (batch 10):** decision 3.
- **Fallbacks:** at 16 colours and in high contrast there are no tints (`Theme::tint` answers `None`). The hue goes on
  titles, rule words and separator glyphs only, and the side not taken is `text-muted`. Under `NO_COLOR` there is no
  colour: DIM on the side not taken, BOLD on `? open`, glyphs as in the table.
- **ASCII tier:** `?`, `|`, `>`, `<`, `:` stand for `?`, `┃`, `▶`, `◀`, `┊`, gated like the existing `dot()`.

**Out:**
- `Took(Both)` after a hand edit (no result tint): it needs `Choices.edited` from `graft`
  (`resolver-words-and-marks`). Until then `Took(Both)` always draws split; say so in NOTES.
- The all-done list head and the finish row (`last-file-opens-the-summary`).
- Any width change: the separator stays one cell (ADR-0043 §2).
- Rule-word text, notice text, hint text (hints are 56's).

## The failing test first
`graft-ui/src/conflict_resolver/tests/three_panes/` (the fixtures module the MAP names): `band_follows_the_block_state`.
- A `DocView` with four blocks: open, took yours, took theirs, took both (ours 1 line, theirs 2), at 187 columns, in
  the truecolor dark theme. Render it to a `Buffer`.
- Assert, per block, on its rule row and on one body row: the left and right separator cells' symbol, fg and modifier;
  the bg of one yours, one result and one theirs cell. For took-both, result row 0 is `ours-tint` and rows 1-2
  `theirs-tint`. The titles row's two separators are `│` in `border`.
- Red today: the separators are `│` in `border`, and no bg is set.

## Signatures
```rust
// graft-ui: no new public type. Private drawers in conflict_resolver/panes/draw.rs take the block state they already read:
fn separator(state: &BlockState, side: Side, row: RowKind, cursor: bool, theme: &Theme, ascii: bool) -> (char, Style);
fn row_tint(state: &BlockState, pane: Pane, result_row: usize, ours_len: usize, theme: &Theme) -> Option<Color>;
// conflict_resolver/files.rs (private)
struct Glyphs { mark: char, gutter: char, check: char, dot: char, more: char, enter: char, unread: char }   // `unread` added
struct Cell { /* x, y, width, text, index, cursor, */ words_at: Option<u16>, tone: WordsTone }
enum WordsTone { Open, Muted, Plain }
```
`Side`, `RowKind` and `Pane` stand for whatever the module already names these; add a private enum only where none
exists.

## Touches (each entry, and how the builder checks it)
- **`graft` tests that pin resolver text** (the scout's list (b)): `tests/it/resolver/three_panes.rs:67-83` and
  `src/view/resolver/tests.rs:216` (`│stashed`, the card at 120).
  - Check: both stay green **unedited** (decision 1). Red → `blocked: needs graft`, per decision 1.
- **Captures that draw the three panes at ≥ 180 columns, or an unread file.** The scout's grep found 38 files with
  `o take`/`· open` (including the `m6-01`, `m6-05` and `m6-09` families). Only those at ≥ 180 columns, or with an
  unread row, should change text.
  - Check: regenerate with `GRAFT_UPDATE_SHOTS=1` (a red run, then green). This runs `graft`'s capture tests from this
    worktree and edits no `graft` source. NOTES lists every rewritten file with the line that changed. A capture at
    < 180 columns that changed anywhere but an unread `•` is a **finding**, not a regeneration.
- **Captures shared with the other lane.** `fix-done-*`, `fix-file-keys-*` and `fix-stop-lands-in-resolver-*` are
  also rewritten by 56 (same lane, later) and 58 (lane O). Whichever merges second regenerates on top of the first;
  capture text is never merged by hand (conductor).
- **The cursor stays the strongest cue.** The `▏` gutter and the reversed rule row must stay stronger than any tint
  (ticket 44's `selection` 3.04:1 > every tint).
  - Check: `cursor_rule_row_keeps_its_reverse_over_a_tint` (buffer: modifier REVERSED on the cursor's rule row).
- **Depths.** Check: `band_at_sixteen_colours_has_no_background`,
  `band_in_high_contrast_has_no_background`, and `band_under_no_color_is_glyphs_and_modifiers`. No cell in the doc
  area has a bg other than reset, and DIM/BOLD sit where the ADR says.
- **ASCII.** Check: `band_ascii_glyphs` (`?|><:`, and no Unicode box glyph in the ASCII buffer), plus the existing
  ASCII glyph gate.
- **Card.** Check: `card_tints_follow_the_block_state` at 120 (paired) and 80 (stacked).
- **File list.** Check: `file_list_state_colours` (full and compact forms);
  `unread_file_is_marked_with_a_bullet` (Unicode `•`, ASCII `~`; the overflow cell still `… N more`; a left-truncated
  path still starts `…`); `file_list_text_is_unchanged_but_the_mark` (every existing file-list text test green
  unedited).
- **Other screens that draw the same state:** the changes pane's `CONFLICTS` rows and the banner are unchanged.
  - Check: `git diff --stat` shows no `graph_screen` or `detail_pane` file.
- **Journal, undo, keys:** none (pure draw).
- **Docs:** `crates/graft-ui/MAP.md` lines 12-15: the band, the tints, `Glyphs.unread`, the words split, and the note
  "colour asserted by buffer tests; legibility unverified by eye". KNOWN_GAPS rows M6/21 :771, and the resolver's
  "colour unverified" rows, are the conductor's.

## Acceptance (name the verifying test at tick time)
- [ ] `band_follows_the_block_state`: red first, then green.
- [ ] `cursor_rule_row_keeps_its_reverse_over_a_tint`
- [ ] `band_at_sixteen_colours_has_no_background`, `band_in_high_contrast_has_no_background`
- [ ] `band_under_no_color_is_glyphs_and_modifiers`
- [ ] `band_ascii_glyphs`
- [ ] `card_tints_follow_the_block_state`
- [ ] `file_list_state_colours`, `file_list_text_is_unchanged_but_the_mark`
- [ ] `unread_file_is_marked_with_a_bullet`
- [ ] `titles_and_rule_words_take_their_side_hue`
- [ ] `graft`'s `three_panes.rs` and `view/resolver/tests.rs` green unedited.
- [ ] Capture (rule 6; **colour unverified**, rule 5): `docs/shots/m6/fix-band-187x50.txt` and `fix-band-120x40.txt`.
  These come from the pty walk (`.scratch/m6-conflicts/pty-walk-m6.py`) against the built binary, with no `graft`
  source edit. If `graft-fixtures` (a graft-ui dev-dependency) can write them from a graft-ui test, that works too;
  NOTES says which was used.
  - Scene: `notes.txt`, block 1 took theirs, block 2 open with the cursor on it (the note's mockup, :993-1007).
  - **What right looks like (text only):** at 187, block 1's rows show `┊` at the left separator, and `◀` then `┃` at
    the right. Block 2's rule row shows `?` at both separators, with `┃` below, and `? open · o t b` in the result
    pane. Plain rows and the titles row show `│`. At 120 the card's text is unchanged. An unread file in the list reads
    `•`. The colours (sky blue left, purple right, amber open) are for the user to judge in Zed, dark theme, at 187
    columns.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium: one module, many small style assertions, and a capture sweep. One context.

## NOTES

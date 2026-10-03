# G-look 5b — cursor row leaves the lanes alone; dark theme B; legible text on the fill

Status: ready (re-specced after scout 07, `.scratch/m8-polish-and-ship/scouts/07-look-cursor-row.md`)
Build with: **Opus** — values and rule decided (ADR-0058, DESIGN §1.0), every decision below is fixed, ratios and
paint are asserted by tests; one new cross-screen helper (`on_selection`) with its behaviour fully specified. No
Fable test met: the shape is small and every result is checkable from tests.
Blocked by: 06
Needs: `look-chrome-header-gutter-bar` (the 2-cell gutter and 1-cell margin, layout.rs:44-46; same files — sequential).

## Read first
- `docs/adr/0058-chrome-follows-the-mock-up-and-the-cursor-row-leaves-the-lanes-alone.md` (whole, 66 lines)
- `docs/design/01-view-mockups.md` §1.0 "Cursor row" bullet (lines 45-48)
- `docs/design-notes/m8-polish-and-ship.md`: "Dark pick: B" paragraph (:73-76), Interfaces `graft-ui::theme` (:218-222)
- `.scratch/m8-polish-and-ship/scouts/07-look-cursor-row.md` (whole — file:line for every site below)
- `docs/shots/m8-proto-4a-dark-a-b.txt` lines 4-24 (B's layout and numbers)
- `prototypes/m8-polish-and-ship/polish.py`: **the dark-B hex table only** (:62)
- `crates/graft-ui/MAP.md`: `theme` (`Token`, `TokenColor`, `Theme::token`, `lane`, `branch`, `color_depth`, `mix`),
  graph rows (`GraphLine`, `RowPaint`, marker slot); `crates/graft/MAP.md` graphics (Pixel strip composite)
- KNOWN_GAPS rows 541, 542 (two lines)

## Decisions (scout contradictions, resolved)
1. **Fill geometry.** `row_emphasis` (render.rs:363-371) fills from `area.x` today, covering the graph column. Change:
   the fill rect starts at the **marker slot's x** and runs to the body's right edge (the 1-cell margin stays ground).
   Gutter and graph column keep the ground at every tier; Pixel cells (left blank, :137-138) therefore show ground.
2. **Cursor glyph.** The cursor's `▏` in the marker slot (render.rs:581,584) **goes** — DESIGN §1.0: "the marker slot
   keeps `▏` for the compare base only". The cursor is marked by `▸` (ASCII `>`) in **gutter cell 0**, `Token::Accent`
   bold; gutter cell 1 stays blank. On the cursor row the marker slot draws whatever the row's other role draws
   (compare base `▏`, else blank).
3. **Drop target.** `RowPaint::DropTarget` (render.rs:583) changes `▸`/`>` → `→`/`=` in the marker slot. It keeps the
   fill (same geometry as decision 1) and bold. It does **not** get the gutter `▸` — that is `Highlighted` only. ASCII
   `>` is then the cursor's alone (scout contradiction 3 gone). m5/35's drag tests that assert `▸` on the drop target
   are updated to `→`; their rule (cursor and drop target never look alike) is kept and now asserted both ways.
4. **Types.** `Rgb` is `graft_core::capabilities::Rgb`; `Theme::token` returns `TokenColor` (mod.rs:237, :315).
   `contrast` and `legible_on` stay pure `Rgb` math (testable, as the note says). Draw sites hold `TokenColor`, so they
   call a new `Theme::on_selection(TokenColor) -> TokenColor` that wraps `legible_on` at the terminal's depth (below).
   Draw sites never unwrap `TokenColor` themselves.
5. **Contrast helper.** None exists. New private module `theme/contrast.rs`, re-exported from `theme`. The existing
   `theme::tests` assertion of the 4.28 shortfall and the comment at chrome.rs:34 are replaced by the B numbers.
6. **Direction of the lift.** Toward white if `contrast(WHITE, bg) >= contrast(BLACK, bg)`, else toward black — white
   for dark, black for light (ADR-0058 §3), and whichever works for high contrast. Step 5% (`mix(target, fg, k·0.05)`)
   up to 20 steps; stop at the first ≥ 4.5. If 20 steps don't clear it, return the target colour (test proves no
   shipped theme reaches that).
7. **Picker.** Scout found no `Selection` paint in the branch picker; its cursor style is unconfirmed. **Out of this
   ticket.** Builder records in NOTES the picker's cursor paint (file:line, which style) — if it is reverse video or a
   fill under branch-coloured text, file it in NOTES as a KNOWN_GAPS candidate for the usability walk. Do not change it.
8. **Other screens' cursor glyph.** Journal, stash, map, files, finish summary, rewrite editor, resolver finish row keep
   their cursor style; only `on_selection` is applied where they draw text on `selection` (scout's list). `▸` gutter
   on those screens is out.
9. **16 colours / `NO_COLOR`.** The fill stays skipped (render.rs:352-355, unchanged). The cursor row keeps `▸` and
   bold; its branch badges draw `Token::Text` bold (ADR-0058 §3) — `on_selection` returns `Text` at these depths.

## Scope
In:
- Dark theme B (theme/chrome.rs:37-50): `selection` `#5C667B`, `text` `#F0F2F5`, `text-muted` `#9AA1AC` (polish.py:62);
  ground `#14161B` unchanged. Light and high contrast unchanged unless a `CONTRAST_RULES` row fails — then stop and
  report the ratio (don't pick a value).
- `contrast`, `legible_on`, `on_selection`, `ContrastRule`, `CONTRAST_RULES` (four floors: text/bg 4.5, text/selection
  4.5, selection/bg 3.0, text-muted/bg 4.5 — raised from 3.0, row 541).
- Decisions 1-3 and 9 in `graph_screen/render.rs` (`row_emphasis`, `draw_marker_slot`, `row_rule`, gutter draw).
- `on_selection` on every fg drawn over `selection` on the graph row (badges, ref names, subject, author :190, age,
  sha — :179-200) and at the seven other painters: journal_screen.rs:309, stash_screen.rs:199,
  detail_pane/files.rs:179, finish_summary/list.rs:143, map_screen/render.rs:187, rewrite_editor/draw.rs:233,
  conflict_resolver/finish_row.rs:58.
Out: `[theme]` overrides and `ContrastMiss` (G-theme); other look rows (G-theme); author-chip identity colours (stay
identity-coloured — only lifted for contrast); picker (decision 7); `▸` on other screens (decision 8); thread emphasis
logic (ADR-0024, unchanged — check it still draws on the cursor row).

## Signatures
```rust
// graft-ui::theme (theme/contrast.rs, re-exported); Rgb = graft_core::capabilities::Rgb
pub fn contrast(fg: Rgb, bg: Rgb) -> f32;                       // WCAG 2.x relative-luminance ratio, (L1+0.05)/(L2+0.05)
impl Theme {
    pub fn legible_on(&self, fg: Rgb, bg: Rgb) -> Rgb;          // decision 6; fg unchanged if already ≥ 4.5
    /// `fg` as drawn on this theme's `selection` fill, at this terminal's depth.
    /// Truecolor: `legible_on(fg, selection)`. 256: lift on the truecolor values, re-reduce each step through the
    /// theme's existing 256 reduction, stop at the first *reduced* pair ≥ 4.5. Palette16 / NO_COLOR /
    /// `TokenColor::Default`: `self.token(Token::Text)` (caller adds bold, decision 9).
    pub fn on_selection(&self, fg: TokenColor) -> TokenColor;
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContrastRule { pub fg: Token, pub bg: Token, pub min: f32 }
pub const CONTRAST_RULES: &[ContrastRule];
```
If `TokenColor`'s 256 variant holds only an index (no source `Rgb`), add a private index→`Rgb` (standard xterm cube +
grey ramp) in `theme/depth`; don't widen `TokenColor`.

## Acceptance
- [ ] `contrast` matches WCAG on 3 pairs: white/black 21.0, `#777777`/white ≈ 4.48, equal colours 1.0 (test).
- [ ] Every theme × every `CONTRAST_RULES` row passes (one test, all kinds); dark B: selection/bg ≥ 3.13, text/selection
      ≥ 5.13, text-muted/bg ≥ 6.94 (the note's numbers − 0.01) (test).
- [ ] `legible_on`: every lane hue and every branch colour on dark-B, light and high-contrast `selection` ≥ 4.5 after
      the call; dominant channel unchanged (test over all twelve lanes).
- [ ] `on_selection` at 256: the reduced result vs reduced `selection` ≥ 4.5 for all twelve lanes (test); at Palette16
      and `NO_COLOR` returns `token(Text)` (test).
- [ ] Paint (Unicode and ASCII tiers): on the cursor row, gutter cells and graph-column cells have bg = ground; cells
      from the marker slot to the body's right edge have bg = `selection`; the margin cell is ground (test).
- [ ] Pixel: graph-column cells on the cursor row have bg = ground; `pixel_scroll` / strips tests unchanged and green
      (strips.rs:70 "cursor move leaves every strip") (test names).
- [ ] Glyphs: `▸` (ASCII `>`) in gutter cell 0 of the cursor row only; cursor row's marker slot has no `▏` unless it
      is the compare base; drop target draws `→`/`=` in the slot and no gutter glyph; a cursor and a drop target on
      screen together never share a glyph (tests; m5/35 drag tests updated, named in NOTES).
- [ ] 16 colours and `NO_COLOR`: no fill; cursor-row badges are `Text` + bold; `▸` present (test).
- [ ] Each of the seven other painters: one test asserting a text cell on `selection` has fg ≥ 4.5 against it in dark
      B (or one table test over all seven, named per screen).
- [ ] J23 (0 steps): capture's cursor row shows `▸` in the gutter and the branch label as text.
- [ ] Captures: `docs/shots/m8-look-cursor-row-dark-120x36.txt` and `…-light-120x36.txt`, cursor on a row with a branch
      badge and three lanes, compared beside `m8-proto-4a-dark-a-b.txt` (gutter `▸`, lanes, columns line up).
      **Colour unverified** (rule 5): the ratios are the tests' claim, legibility is his eye.
- [ ] NOTES: picker cursor style (decision 7); every re-shot snapshot listed; KNOWN_GAPS 541, 542 closing evidence
      (test names) — the conductor moves the rows.

## Touches
- `graph_screen/render.rs` `row_emphasis`, `draw_marker_slot`, `row_rule`, gutter draw; `RowPaint::{Highlighted,
  DropTarget, Ghost}` (ghost rows: check they're unaffected).
- Drag tests (m5/35) asserting `▸` on the drop target — grep `▸` and `DropTarget` in graft-ui and graft tests.
- `graft-ui/tests/selection.rs`; theme tests (theme/tests.rs, 3 `Selection` hits) and `theme/snapshots/`.
- Snapshots that contain a cursor row (scout list): graph_screen__feature-branches-{80x24,120x40,200x60},
  ascii_tier__*, lanes_without_hue__*, badges-icons-{nerd,plain}; plus each of the seven painters' snapshots if their
  cursor-row text colour shifts (colour isn't in text snapshots — check, list in NOTES).
- Pixel compositing (graft/src/view/strips.rs): must not re-rasterise per cursor move — no change expected; verify by
  the strips tests, not by reading.
- Status screen's theme view: scout found no `Selection` — confirm with grep, note it.
- Docs: DESIGN §1.0 already written; grep `docs/design/` §5 visualization for "selection under lanes" and ADR-0003's
  band note — update wording if either says the fill covers lanes; `crates/graft-ui/MAP.md` theme entries
  (`contrast`, `legible_on`, `on_selection`, `ContrastRule`, `CONTRAST_RULES`) and the marker-slot entry.
- Keymap: none (no key claimed).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~70k tokens (seven small painter edits added to the scout's estimate).

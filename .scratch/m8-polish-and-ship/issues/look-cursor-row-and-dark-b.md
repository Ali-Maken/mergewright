# G-look 5b — cursor row leaves the lanes alone; dark theme B; legible text on the fill

Status: draft
Build with: _(blank — brownfield, ADR-0035.)_ Planner expects **Opus**: values and rule decided (ADR-0058); ratios are
asserted by tests.
Needs: `look-chrome-header-gutter-bar` (the gutter cells; same files — sequential).

## Read first
- `docs/adr/0058-chrome-follows-the-mock-up-and-the-cursor-row-leaves-the-lanes-alone.md` (whole)
- `docs/design/01-view-mockups.md` §1.0 "Cursor row" bullet
- `docs/design-notes/m8-polish-and-ship.md`: "Dark pick: B" paragraph, Interfaces `graft-ui::theme`
- `docs/shots/m8-proto-4a-dark-a-b.txt` (B's numbers, lines 21–24)
- `prototypes/m8-polish-and-ship/polish.py`: **the dark-B hex table only** (values, not code structure)
- `crates/graft-ui/MAP.md`: `theme` (tokens, `Theme::token`, `lane`, `branch`, `color_depth`), graph rows
  (`GraphLine`, `RowPaint`, marker slot); `crates/graft/MAP.md` graphics (Pixel strip composite)
- KNOWN_GAPS rows 541, 542 (two lines)

## Scope
In:
- Dark theme B: `text`, `text-muted`, `selection` per the prototype's B; ground `#14161B` unchanged.
- `contrast` (public), `CONTRAST_RULES` with the four floors: text/bg 4.5, text/selection 4.5, selection/bg 3.0,
  text-muted/bg 4.5 (raised from 3.0 — row 541). Tested for dark, light, high contrast.
- Cursor-row fill from the marker slot to the right margin; gutter and graph column keep the ground, every tier (Pixel:
  the strip is drawn over ground; transparent pixels show ground).
- `▸` cursor glyph in the gutter (accent, bold; ASCII `>`).
- `Theme::legible_on` on every foreground drawn over `selection` (branch badges, ref names, author, age, sha, subject);
  at 16 colours / `NO_COLOR` the cursor row's badges draw `text` bold.
- Drop-target slot glyph `→` (ASCII `=`); compare base keeps `▏`.
Out: `[theme]` overrides (G-theme); other look rows (G-theme); author-chip colours (stay identity-coloured).

## Signatures
```rust
// graft-ui::theme
pub fn contrast(fg: Rgb, bg: Rgb) -> f32;                          // WCAG 2.x relative-luminance ratio
impl Theme { pub fn legible_on(&self, fg: Rgb, bg: Rgb) -> Rgb; }  // fg unchanged if ≥ 4.5; else mix 5% steps toward
                                                                   // white (dark/high contrast) or black (light) until ≥ 4.5
pub struct ContrastRule { pub fg: Token, pub bg: Token, pub min: f32 }
pub const CONTRAST_RULES: &[ContrastRule];
```
`Rgb`/`Token` are the theme's existing types (scout confirms names).

## Acceptance
- [ ] `contrast` matches WCAG on 3 known pairs (white/black 21.0, `#777777`/white ≈ 4.48, equal colours 1.0) (test).
- [ ] Every theme × every `CONTRAST_RULES` row passes (one test, all kinds); dark B: selection/bg ≥ 3.14 − 0.01, text/
      selection ≥ 5.14 − 0.01, text-muted/bg ≥ 6.95 − 0.01 (asserts the note's numbers).
- [ ] `legible_on`: every lane hue and every branch colour on dark-B and light `selection` ≥ 4.5 after the call, hue
      family kept (red channel stays dominant for a red, etc.) (test over all twelve lanes).
- [ ] Paint test: on the cursor row, the gutter and graph-column cells' bg is the ground, cells from the marker slot to
      the margin carry `selection` (Unicode and ASCII tiers); Pixel: the strip composite has no selection fill (test).
- [ ] `▸` in the gutter of the cursor row only; `>` in ASCII; drop target draws `→`/`=`, never `▸` (tests).
- [ ] 16 colours and `NO_COLOR`: cursor-row badges are `text` + bold (test).
- [ ] J23 (0 steps): capture's cursor row shows `▸` and the branch label as text.
- [ ] Captures: `docs/shots/m8-look-cursor-row-dark-120x36.txt` and `…-light-120x36.txt`, cursor on a row with a branch
      badge and three lanes, beside `m8-proto-4a-dark-a-b.txt`. **Colour unverified** (rule 5); the ratios are the
      tests' claim, legibility is his eye.
- [ ] KNOWN_GAPS 541, 542: closing evidence (test names) in NOTES; the conductor moves the rows.

## Touches
- Theme tokens consumed by every screen (journal, stash, map, resolver, picker cursor rows also draw `selection` —
  each gets `legible_on` where it draws text on the fill; grep `selection`).
- Thread emphasis (ADR-0024) on the cursor row; ghost rows (`RowPaint::Ghost`); `RowPaint::DropTarget` glyph and the
  drag tests (m5/35's rule); compare base `▏`.
- Pixel strip compositing (graft graphics) — must not re-rasterise per cursor move (pixel_scroll tests stay green).
- Status screen's theme view (no change expected — check). All cursor-row captures and goldens: re-shoot, list in NOTES.
- DESIGN §1.0 (written), §5 visualization if it states selection under lanes (grep), ADR-0003 band note; MAP.md graft-ui.

## Scout questions
- Is there a contrast helper already (grep `luminance`, `contrast`)? Where is `selection` painted for the graph row
  (file:line), and does the Pixel strip ever see the cursor?
- Which screens paint `selection`?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens.

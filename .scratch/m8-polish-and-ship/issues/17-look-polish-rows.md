# G-look polish — states you can see: folded merges, faded threads, ghosts at Pixel, ignored whitespace

Status: ready
Build with: **Sonnet** (conductor 2026-10-07 after scout `scouts/17-look-polish.md`: items 2/3/5/7/8 mechanical; item 6 needs design — the builder stops on it and reports, it goes to the planner)
Blocked by: 07, 16, 11b, 09e (shares rows.rs; order 09e → 17 → 11c, planner 2026-10-07)
Needs: `theme-overrides` (same `theme` files, sequential), `look-cursor-row-and-dark-b`.

## Read first
- `docs/design/01-view-mockups.md` §1.0; `docs/design/03-graph-layout-algorithm.md` §3.5 "fold rail" paragraphs
- `docs/design-notes/m8-polish-and-ship.md` Need row "Readable in his theme"
- `crates/graft-ui/MAP.md`: `graph_column`, `graph_screen` (rows, header tail), `detail_pane` header, `theme`
  (`lane`, `raster_lanes`); `crates/graft/MAP.md`: `view::strips`, `update::preview` (ghost rows), `config` (`Ui`)

## Decisions (planner) — each closes one open row
1. **Node glyphs ignore `ui.icons`** — closed by design: Nerd Font node glyphs are private-use code points many fonts
   draw two cells wide, breaking lane alignment; `● ◉ ◆ ◇ ■` read the same in every font. No code. NOTES says so.
2. **A folded merge looks like a plain commit.** Draw a muted ` +N` after the subject of a merge whose side is folded
   (N = commits inside), the same `+N` vocabulary as the lane rail; `space` on the row unfolds as today.
3. **Faded threads at the Unicode tier are bold/dim only** (terminals ignoring SGR 2 show no fade). Off-thread lanes
   draw their hue mixed 45% toward `bg` at truecolor and 256 colours; 16 colours and `NO_COLOR` keep DIM.
   `Theme::lane_muted(i)`. Pixel tier unchanged (alpha, ADR-0024).
4. **A long path cut off the order notice.** Expected closed by look-chrome (path cut from the left, tail right-
   aligned): verify at 80 columns with a 120-char path and the order notice on; fix if not.
5. **No `[ui]` key starts the overview strip on.** `ui.overview = true | false` (default false), seeded like `[diff]`;
   `O` still toggles per run.
6. **Ghost rows at the Pixel tier draw as ordinary rows.** In a Ghost Preview at the Pixel tier, ghost rows' graph cells
   draw the Unicode tier's ghost glyphs as text (no strip for those rows); real rows keep strips.
7. **No ASCII fallback** for the submodule row `▣` and marks `→ ± ?` (literal in `detail_pane/files.rs`): ASCII
   `[S]`, `->`, `+-`, `?`.
8. **Whitespace-ignored mode shows on no screen** (only the bar's `w whitespace`): while on, the detail pane's, blame's
   and history's title tails read `· whitespace ignored` (muted).

## Scope
In: 2–8. Out: Nerd Font support; pixel-tier image diffs (past v1, ADR-0057).

## Signatures
```rust
// graft-ui::theme
impl Theme { pub fn lane_muted(&self, lane: usize) -> TokenColor; }
// graft::config — Ui gains `pub overview: bool`
```

## Acceptance
- [ ] 2: buffer test, folded merge row ends `+3` muted; capture `docs/shots/m8-polish-fold-120x36.txt`.
- [ ] 3: buffer test reads off-thread lane cells' colour = `lane_muted` at truecolor, DIM at 16 (test).
- [ ] 4: header test at 80 cols, long path, order notice visible (test named).
- [ ] 5: `ui.overview = true` → strip shown at startup (test).
- [ ] 6: Pixel fixture preview: ghost rows carry text glyphs, no strip command for those rows (frame-bytes test);
      capture `docs/shots/m8-polish-ghost-pixel-120x36.txt` (text layer).
- [ ] 7: ASCII tier capture of a submodule row: no non-ASCII byte (test).
- [ ] 8: three title tests with `w` on; capture `docs/shots/m8-polish-whitespace-120x36.txt`.
- [ ] **Colour unverified** (rule 5) for 2, 3, 6.

## Touches
- Row painting and the cursor row's fill (5b) — `+N` on the fill goes through `legible_on`; thread emphasis
  (ADR-0024) and G-settle's rest rule; preview drawing at Pixel and `view::strips`; config `[ui]` and README (G-docs);
  captures showing folded merges or faded threads (re-shoot, list in NOTES). MAP.md graft-ui, graft.

## Scout questions
- How does a preview draw at the Pixel tier today — do ghost rows get strips (file:line)?
- Where is the fade applied at the Unicode tier (file:line)?

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
One context, ~60k tokens. 6 is the largest; if the scout finds ghost rows deeply tied to strips, 6 splits off.

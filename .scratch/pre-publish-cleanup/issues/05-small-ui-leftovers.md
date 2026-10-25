# pre-publish: blame's `⊗` in the failure ink, and a capture of the narrow graph drawn as text

Status: ready
Blocked by: none
Build with: Sonnet (`builder-sonnet`, model sonnet); fails twice → `builder-opus`. Why: one draw call made consistent with a decided rule (ADR-0067), plus one capture. Buffer-style tests judge both.
Scout: **yes**, first. It writes `.scratch/pre-publish-cleanup/scouts/05-blame-ink-narrow-graph.md` (≤1 page). It finds:
- how the other panes give `failure`'s mark its ink (26B's pattern), and how blame's sibling `history` gets a theme, if it does;
- every caller of `draw_blame` / `draw_blame_in_tier`;
- the narrowest terminal size and layout (detail pane `Right`/`Below`, branch pane) where the graph area is ≤3 columns at the pixel tier, or the line that makes that unreachable.

Two small items in one context: neither fills one alone.

## Closes
- `ease/26-blame-mark-uncoloured` (**unverified**): "The blame pane's failure `⊗` draws without `FAILED_INK` (blame has no theme). Source: 26 Part B NOTES. Colour unverified anyway (rule 5)."
- `ease/29-narrow-graph-text` (**unverified**), **test half**: "A graph area ≤3 columns has no gutter (`graph_screen/layout.rs:296-302`), so since 29 D5 it draws Unicode, not pixels (`render.rs:151`). Wider screens unchanged. Not run in a terminal." The terminal half is his (00-index, "Checks only you can do"). The row closes when he's done it.

## Part A: blame's failure mark
- **Rule** (ADR-0067, 26 ruling): every failure is drawn as `⊗ <why>` (ASCII `(x)`). The mark gets `Token::Removed` ink (`FAILED_INK`); the words keep `Warning` ink.
- **Cause:** blame draws the mark with no theme. MAP: `draw_blame(frame, area, view, keys: &Keymap)` takes none.
- **Fix:** blame draws its `Failed` body through the `failure` module, the same way the other panes do, and gets its theme the way its sibling `history` place does. If `history` has none either, both get it in the same change: one way, not two.
- **Test:** on a ratatui buffer, the `⊗` cell's foreground equals `FAILED_INK` resolved for the dark and light themes, and a word cell's equals `Warning`. Under `NO_COLOR` / 16 colours / high contrast, behave as 26's other bodies do (read their test; don't invent). At ASCII the mark is `(x)`.
- The buffer test proves the style the app asks for. The colour a terminal shows stays unverified (rule 5).

## Part B: the narrow graph as text
- An existing test covers the logic: `view::tests::pixel_preview::lead::no_strip_without_a_gutter`.
- **Add the capture** his check will compare against: a pixel-tier session at the narrowest reachable graph area ≤3 columns, saved as `docs/shots/pre-publish/29-narrow-graph-text-<W>x<H>.txt`.
  - **What right looks like:** the graph's 1–3 columns show Unicode graph glyphs (`●`, `│`) on every row with a commit. No blank graph column. No image placement in the emitted bytes (assert no Kitty APC / iTerm2 OSC 1337 for that frame).
  - Colour unverified (rule 5).
- NOTES gives him the exact terminal size and keys that reach it, for his Ghostty check.

## Stop conditions (report, don't work around)
- No layout at a supported terminal size gives a graph area ≤3 columns: report the narrowest reachable and the line that forbids narrower. The row then closes as unreachable; the conductor decides.
- Making blame match needs a theme threaded through a crate boundary the MAP forbids (ui may not depend on app state): report it.

## Scope
In: `mergewright-ui` (`blame`, and `history` only if Part A's rule pulls it in), the `draw_blame` callers in `mergewright`, tests, the new capture, MAP lines.
Out: the `failure` module's rules, other panes, theme tokens, the 29 D5 placement rule.

## Signatures
```rust
// crates/mergewright-ui/src/blame (exact form follows how `history` / the other failure bodies take the theme; scout names it)
pub fn draw_blame(frame: &mut Frame, area: Rect, view: &BlameView<'_>, keys: &Keymap, theme: &Theme);
pub fn draw_blame_in_tier(/* same params */, ascii: bool);
```
If 26's pattern puts the theme in the view (`BlameView { theme, .. }`) instead, follow that, and the signature above doesn't change. The MAP line follows either way.

## Acceptance (tick each with its test, inline)
- [ ] `⊗` in blame carries `FAILED_INK`, words carry `Warning`, dark and light ← new ui test by name
- [ ] Blame's no-colour, high-contrast and ASCII forms match 26's other bodies ← test by name (or the existing invariant, extended)
- [ ] `failure_marks::*` invariants still green and now cover blame, if they enumerate bodies ← test names
- [ ] No blame `.txt` capture changes bytes (characters are unchanged) ← `git diff --stat docs/shots`
- [ ] Narrow graph capture written, no image bytes in that frame ← capture test by name; path in NOTES
- [ ] NOTES gives the exact size and keys for his Ghostty check, and how to make a blame fail for the colour check ← NOTES

## Touches (check each, say how)
- Every `draw_blame*` caller (grep).
- Blame captures (bytes unchanged).
- The `history` place, if pulled in.
- `failure` module tests.
- Theme modes (NO_COLOR, 16-colour, high contrast).
- MAP ui `blame` line.
- 29's tests (`pixel_preview::lead::*`).
- Keymap: none.
- `over_400.txt`: `blame/tests.rs` is a row (677). Split, don't raise.

## Read first
- This ticket, then the scout file.
- ADR-0067; `.scratch/ease-pass/issues/26-*.md` (ruling + Part B NOTES); `.scratch/ease-pass/issues/29-pixel-node-cut-left.md` D5 rule 6 only.
- `crates/mergewright-ui/MAP.md` § `blame`, § `failure`, § `history`; `crates/mergewright/MAP.md` (view, pty captures).

## Gates
Workspace-wide: the four CLAUDE.md gates, heavy clippy, `scripts/walks.sh` (draw change).

## Size
~30k tokens.

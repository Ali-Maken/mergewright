# diff-widget: the real unified diff renderer — gutter, tints, pinned heading, word tint, syntax, truncation

Status: draft
Build with: Sonnet — fills in a function `detail-pane-frame` already gave a signature; judged by the capture.
Needs: `detail-pane-frame` (`diff_body`'s signature, `DetailAreas`, `Theme`), `detail-model-and-jobs` (`HunkView`, `LineView`, `LineKindView`), `syntax-highlighting` (`SyntaxRole`, `Theme::syntax`, `SyntaxSpan`)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — New UI "Diff lines:" bullet (gutter, tints, pinned heading, `␍`, 16-colour rule) — this ticket implements it in full; skip "Side-by-side" (next ticket)
- `PROMPT.md` §5 Diffs
- `DESIGN.md` §2.15 (chrome is rules, not boxes; colour restraint)
- `crates/graft-ui/MAP.md` — `detail_pane` (after `detail-pane-frame` lands), `theme` (`Theme::syntax`, `Theme::token`, `Theme::color_depth`, `TokenColor`), `fit`

## Scope

In:
- Replace `diff_body`'s body in `crates/graft-ui/src/detail_pane.rs` (same file, same signature — no new public item beyond what's listed): gutter `old new` numbers, width from the widest line number in the hunk set (min 3), right-aligned, blank on the side with no number; sign cell separate from text; `added`/`removed` rows get a soft background token, `context` none; where `LineView.words` is non-empty, those byte ranges get a stronger background over the sign+text; `syntax` spans paint the text foreground by `SyntaxRole` via `Theme::syntax`, changed-word background wins over syntax foreground contrast (pick a readable combination, don't fight the theme); `HunkHeading` line pinned: while a hunk's lines are scrolled through, its heading (`text-muted`) stays on the body's top row instead of the real top line, then resumes normal scroll once the next heading would show anyway; a trailing `␍` (U+240D, already in the sanitised text per `detail-model-and-jobs`) draws at the line's end, not counted in the gutter; a `truncated: Some((shown, total))` diff ends with one row `… {total-shown} more lines · L load all` in `text-muted`.
- At `Theme::color_depth() == Palette16` or `NO_COLOR`: no soft backgrounds anywhere in the diff — sign glyph plus bold (added) / dim (removed) only, syntax and word tint both off (the m1 12-colour rule already applied to the graph column).
- Tests: unit tests building a `HunkView` fixture (gutter widths, pinned heading across a scroll offset, truncation row, `␍`, 16-colour fallback drops all backgrounds). Snapshot on `TestBackend`. MAP.md line (docstring change only, same signature).
- Capture: the same fixture as `detail-pane-frame`'s but now with syntax/word tint visible, plus a truncated file and a CRLF line, dumped to `docs/shots/m2-diff-widget.txt`, ≤ 200 KB.

Out: side-by-side (`side-by-side-view`), any key handling, `assemble_diff`'s data assembly (`detail-model-and-jobs`, already gives `words`/`syntax`/`pair` populated).

## Signatures

No new public items: this ticket replaces the body of
`pub fn diff_body(hunks: &[HunkView], top: usize, area: Rect, buf: &mut Buffer, theme: &Theme, focused: bool) -> usize`
from `detail-pane-frame`. Internal helpers it may add are private (`fn gutter_width`, `fn pin_heading`, etc.) — not listed here since callers never see them.

## Acceptance

- [ ] Gutter width matches the widest old/new line number in the hunks (min 3); a line missing one side leaves that gutter half blank.
- [ ] A word-tinted range shows a visibly stronger background than the surrounding added/removed soft bg in the snapshot buffer's styled cells (assert on `Buffer::cell(..).style()`, not just text).
- [ ] Syntax foreground differs from `Plain` for at least one span on a `.rs` fixture with a keyword.
- [ ] Scrolling `top` past a hunk's own heading row keeps that heading pinned on the body's first row until the next heading would be visible anyway (assert the pinned text at two different `top` values within the same hunk).
- [ ] Truncated fixture ends with the `… N more lines · L load all` row; non-truncated does not.
- [ ] A CRLF line (`no_newline_at_eof: false`, sanitised text ending `␍`) shows the glyph after the text, outside the gutter.
- [ ] `Theme::color_depth() == Palette16` (or `NO_COLOR` theme) fixture: no cell in the diff area carries a non-default background; added/removed are bold/dim only.
- [ ] Capture at `docs/shots/m2-diff-widget.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

One crate, one file's function body (~200 lines) + tests (~200). ≤ 22 tool calls. Read `detail-pane-frame`'s MAP.md entry and the plain version's shape before touching it, not the whole file if it has grown past `detail_pane`'s docstring section.

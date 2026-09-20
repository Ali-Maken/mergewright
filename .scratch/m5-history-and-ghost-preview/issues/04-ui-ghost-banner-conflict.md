# ui-ghost-banner-conflict: ghost rows, the preview box, N-choice confirm (+ its missing frame), the operation banner, the CONFLICTS section, the name prompt

Status: ready
Blocked by: none
Build with: (blank — brownfield in `graph_column`, `confirm`, `detail_pane`; recommendation: **Sonnet**, pure widgets with per-width tests; the user judges the captures)
**Conductor set after the scouts: Sonnet.**
Crates: graft-ui
Needs: none (view-model types only; the app groups in pass 2 fill them)

## Scout first (Haiku `Explore`, ≤ 1 page)

- `crates/graft-ui/src/graph_column.rs`: how `RowPaint` selects glyph/colour; the Unicode and ASCII glyph tables; where the node glyph is chosen (`NodeKind::glyph`).
- `crates/graft-ui/src/confirm.rs`: why the box draws no frame (KNOWN_GAPS row 484, m4-07 Minor 2 — `Block` never set? area too small?); how `Choice`/`toggle` are used by `graft` (`update/confirm.rs`, `update/remote.rs` `push_box`).
- `crates/graft-ui/src/detail_pane/files.rs` and `graft-ui/src/detail.rs`: how `WorkingTreeSection` headings are drawn; `FileStatusView::Unmerged` rendering.
- `crates/graft-ui/src/graph_screen.rs` header: where ahead/behind is drawn (the banner replaces that slot).
- `crates/graft-ui/src/credential_prompt.rs`: the one-field modal to copy for the name prompt.

## Read first

- `docs/design-notes/m5-history-and-ghost-preview.md` — Interfaces (G6 block); Proposed changes 1, 3; New UI: Ghost Preview, Operation banner + conflict interim, `u` after a push, Name prompt (the mockups are the acceptance).
- `docs/design/01-view-mockups.md` §1.7, §1.13, §1.14; `docs/design/05-visualization.md` lines 80–86 (ghost vocabulary vs Focus fade).
- `crates/graft-ui/MAP.md` §graph_column, §confirm, §credential_prompt, §detail, §detail_pane, §graph_screen, §text_input.
- CLAUDE.md "The six that pay" 5 and 6 (colour unverifiable; captures judged by the user).

## Scope

In, in order:

1. **`RowPaint::Ghost`** in `graph_column`: edges dashed (`┊` vertical, `┈` horizontal, `╭┈`/`╰┈`-style corners drawn with the dashed set), node `○` (merge ghost `◎`), everything in `Token::TextMuted`; ASCII `:` `.` `o` `O`. Dash parity for 16-colour lanes is unchanged for real rows; a ghost row is dashed regardless. Ghost rows are decided by the caller (`GraphLine.paint`), the widget knows nothing about previews.
2. **`GraphEntry`** text for a ghost: the caller passes badges `(ghost)` and `sha`/`when` as `—`; nothing to do in the widget except never draw `compare_marked`/`marker` on a `Ghost` row (assert).
3. **`ConfirmView` N choices**: `choices: Vec<SanitizedText>`, `cursor: usize`; `new(title, message, command)` keeps two (`Cancel`, `Confirm`) with `Cancel` under the cursor; `with_choices(labels)`; `left`/`right` clamp (no wrap); `selected() -> usize`; `with_labels` kept as a shim over the two-choice case. **Frame fix (row 484)**: the box draws a `Block` with rounded borders and its title, on a cleared area, at every size the M4 push box tests use; add the missing test that asserts the top-left corner glyph and the title in the first row.
4. **`PreviewBox`** widget (`preview_box.rs`): docked to the bottom of the given area (never over the footer row), rows = title + lines + commands + buttons + its own footer (`←→ choose  ⏎ act  esc cancel`); `planning: true` draws `predicting…` in place of lines; below 40×12 draws `preview needs 40x12 · esc closes` on one row. Built on the same frame code as `Confirm`.
5. **`BannerView` + `banner_line`**: the header's ahead/behind slot becomes ` ⚠ REBASING step 3 of 7 — conflict in src/auth/token.rs (+1) ` (≥100 cols) / ` ⚠ REBASING 3/7 — conflict src/auth.rs ` (<100); no step → ` ⚠ MERGING — conflict in … `; no conflict → ` ⚠ REBASING step 3 of 7 `; `focused` reverses the slot (text fact: `[focused]` is not needed — the hint bar shows `c continue`). `skippable` only affects the hint bar. Buttons `[ Continue ] [ Skip ] [ Abort ]` are drawn only at ≥160 columns on the banner's own row under the header (they are mouse targets; keys are the way that always works).
6. **`WorkingTreeSection::Unmerged`** first in the changes pane, heading `CONFLICTS (n)`; each row `both modified` / `deleted by them` / `added by us` etc. from `FileStatusView::Unmerged { how }` (extend the variant with the porcelain `XY` pair rendered as words).
7. **`NamePromptView`** + widget (`name_prompt.rs`): title, `field: FieldView`, `option: Option<(label, on)>` (`(•) check out after creating`), `refusal: Option<SanitizedText>`, footer `⏎ create  ⇥ toggle  esc cancel`. Centred, `min(width-4, 60)` wide, 6 rows; too-small message like the credential prompt.
8. **`HintContext` + `hints_for` + `fit_status_line`** are the `hint-bar-redesign` group's; this group must not touch `graph_screen`'s footer.
9. Tests (ratatui `TestBackend`, per width 80/120/160): ghost row glyphs Unicode and ASCII; confirm frame present with 2 and 3 choices, cursor movement clamped; preview box docking (last row of the area is the box's footer, the screen footer row untouched), planning state, too-small state; banner text at <100/≥100/≥160; CONFLICTS heading first with the words above; name prompt with and without the option and with a refusal. Snapshot-free: assert on cell text.
10. MAP.md.

Out: any `AppState` or key handling; colour verification (unverifiable; say so); pixel-tier ghosts (M8).

## Signatures

```rust
// graft-ui/src/graph_column.rs
pub enum RowPaint { Normal, Dimmed, Highlighted, Ghost }
// graft-ui/src/confirm.rs
pub struct ConfirmView { pub title: SanitizedText, pub message: Vec<SanitizedText>, pub command: SanitizedText, pub choices: Vec<SanitizedText>, pub cursor: usize }
impl ConfirmView { pub fn new(title, message, command) -> Self; pub fn with_choices(self, labels: Vec<SanitizedText>) -> Self;
    pub fn with_labels(self, safe: &str, confirm: &str) -> Self; pub fn left(&mut self); pub fn right(&mut self); pub fn toggle(&mut self); pub fn selected(&self) -> usize; }
// graft-ui/src/preview_box.rs
pub struct PreviewBox { pub title: SanitizedText, pub lines: Vec<SanitizedText>, pub commands: Vec<SanitizedText>, pub choices: Vec<SanitizedText>, pub cursor: usize, pub planning: bool }
pub struct PreviewBoxWidget<'a> { pub view: &'a PreviewBox, pub theme: &'a Theme }   // Widget
pub fn preview_rows(view: &PreviewBox, width: u16) -> u16;   // rows the box takes, for the caller's row budget
pub const PREVIEW_MIN: (u16, u16) = (40, 12);
// graft-ui/src/graph_screen.rs
pub struct BannerView { pub kind: SanitizedText, pub step: Option<(u32, u32)>, pub conflict: Option<SanitizedText>, pub more_conflicts: usize, pub focused: bool, pub skippable: bool }
pub struct GraphView { …, pub banner_op: Option<BannerView> }   // replaces the ahead/behind slot while Some
// graft-ui/src/detail.rs
pub enum WorkingTreeSection { Unmerged, Staged, Unstaged, Untracked }
pub enum FileStatusView { …, Unmerged { how: SanitizedText } }
// graft-ui/src/name_prompt.rs
pub struct NamePromptView { pub title: SanitizedText, pub field: FieldView, pub option: Option<(SanitizedText, bool)>, pub refusal: Option<SanitizedText> }
pub struct NamePrompt<'a> { pub view: &'a NamePromptView, pub theme: &'a Theme }   // Widget
pub fn name_prompt_area(frame: Rect) -> Rect;
```

## Acceptance

- [ ] Every test in piece 9 green, named at tick time; each widget draws nothing outside its `Rect` at 80×24 — `<widget>_stays_inside_its_rect`
- [ ] Confirm box frame present at 80×24 and 160×48 (row 484) — `confirm_draws_its_frame`
- [ ] Captures (widget-level, via the existing pty pipeline once the app groups wire them; this group ends with **ratatui buffer dumps** under `docs/shots/m5-ui-*.txt` from a test that renders each widget with fixture data at 80×24 and 160×48: ghost rows, preview box, banner, CONFLICTS, name prompt, 3-choice confirm) — `ui_widget_dumps`; each ≤ 8 KB; the user judges them by eye
- [ ] MAP.md updated; NOTES says colour is unverified and why, scout used or why not

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ui` · `cargo test --doc -p graft-ui`; workspace build once (`GraphView`/`ConfirmView` field changes touch `graft` — add the minimal field initialisers, name them in NOTES; the `Choice` enum's removal may need `update/confirm.rs` edits: if more than initialisers, commit at the seam and report `blocked: needs graft`).

## Size

One context. graft-ui only, files disjoint from `hint-bar-redesign` (`graph_screen/hints.rs`, footer code) — coordinate: this group owns `graph_screen.rs`'s header slot, that group its footer.

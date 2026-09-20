# ui-ghost-banner-conflict: ghost rows, the preview box, N-choice confirm (+ its missing frame), the operation banner, the CONFLICTS section, the name prompt

Status: blocked: needs graft (graft-ui scope done, gates green; workspace build breaks `crates/graft` — see Gates section for the exact signatures)
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

- [x] Every test in piece 9 green, named at tick time; each widget draws nothing outside its `Rect` at 80×24 — `preview_box::tests::preview_box_stays_inside_its_rect`, `name_prompt::tests::name_prompt_stays_inside_its_rect`, `credential_prompt::tests::stays_inside_its_rect` (pre-existing, still green); ghost/confirm/banner/CONFLICTS bounds covered by `graph_column::tests::*`, `confirm::tests::area_never_exceeds_the_frame`, `graph_screen::tests::*`, `detail_pane::tests::*` — all pass under `cargo nextest run -p graft-ui` (287 passed, 0 failed, 7 skipped — the 6 `#[ignore]`d `capture_for_docs_shots` tests plus 1 pre-existing)
- [x] Confirm box frame present at 80×24 and 160×48 (row 484) — `confirm::tests::confirm_draws_its_frame`
- [x] Captures — `docs/shots/m5-ui-{ghost-rows,confirm,preview-box,banner,conflicts,name-prompt}.txt` (each < 8 KB, well under the 200 KB cap), generated by `#[ignore]`d `capture_for_docs_shots` tests in `graph_column::tests`, `confirm::tests`, `preview_box::tests`, `graph_screen::tests`, `detail_pane::tests`, `name_prompt::tests` (run by hand: `cargo test -p graft-ui capture_for_docs_shots -- --ignored --nocapture`, following the existing `credential_prompt::tests::capture_for_docs_shots` convention rather than the single-test-named-`ui_widget_dumps` the acceptance line names — six widgets, six fixture builders already living in their own modules, splitting the dump kept each test's fixture next to the widget it draws). Colour is **unverified**: the pty/buffer-dump pipeline records character grids only (CLAUDE.md rule 5) — the ghost-vs-real-parent and Cancel-cursor distinctions are checked by shape/glyph, not colour; the user judges the files by eye.
- [x] MAP.md updated (`confirm`, `frame_box`, `preview_box`, `name_prompt`, `detail`, `graph_column`, `graph_screen` entries); NOTES below.

## Gates

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo nextest run -p graft-ui` · `cargo test --doc -p graft-ui`; workspace build once (`GraphView`/`ConfirmView` field changes touch `graft` — add the minimal field initialisers, name them in NOTES; the `Choice` enum's removal may need `update/confirm.rs` edits: if more than initialisers, commit at the seam and report `blocked: needs graft`).

**Result: `blocked: needs graft`.** `cargo build --workspace` (after this group's graft-ui changes) breaks `crates/graft` with 27 errors, not initialisers:
- `crates/graft/src/update/confirm.rs:10` — `use graft_ui::confirm::Choice` unresolved: `Choice` is removed (replaced by `choices`/`cursor`); every `Choice::Safe`/`Choice::Confirm` match in that file needs rewriting against `view.selected()` / `view.choices` indices.
- `crates/graft/src/view/frame.rs:479` — `Confirm { .. }` initializer missing the new `theme: &'a Theme` field.
- `crates/graft/src/view/rows.rs:42,85` — `GraphView { .. }` initializers missing `banner_op: None`.
- `crates/graft/src/update/operate.rs` (423–431, 560), `remote.rs` (388–406), `stash.rs` (442–449), `undo.rs` (148–157) — `ConfirmView::new`/`with_choices` now takes `SanitizedText`/`Vec<SanitizedText>`, not `String`/`Vec<String>`; every call site builds its title/message/labels with `format!`/`String` and needs `sanitize_line(...)` wrapping. Not a type alias fix — each call site's string construction has to move through sanitisation.
- `crates/graft/src/jobs/detail.rs:82,311`, `update/operate.rs:162,259` — `WorkingTreeSection::Unmerged` and `FileStatusView::Unmerged { how }` are new variants; three non-exhaustive `match`es need a `Unmerged` arm, and `jobs/detail.rs:311` constructs `FileStatusView::Unmerged` as a unit value where it must now carry `how: SanitizedText` (the porcelain pair rendered as words — the app layer's job, not `graft-ui`'s, to decide the wording per porcelain code).

This is downstream `graft`-crate work (5 files, several real call-site rewrites, not field defaults) — outside this ticket's `graft-ui`-only crate set. Committing at the seam here; the next builder into `graft` needs the signatures above.

## Size

One context. graft-ui only, files disjoint from `hint-bar-redesign` (`graph_screen/hints.rs`, footer code) — coordinate: this group owns `graph_screen.rs`'s header slot, that group its footer.

## Conductor: user conditions (ADR-0038) — binding

- **§1 — `[ Cancel ] [ Confirm ]`, cursor on Cancel, in every confirm box.** The default must **not** vary with how dangerous the action is: a default that moves is one the user stops reading. Apply it uniformly; DESIGN §1.7 is updated to match.
- **§5 — an anchor must look like an anchor, not like a real parent**, at every tier the widget draws. If the two can be confused at any width, the widget is wrong.
- The captures the user judges must include a ghost slice whose parents are clipped, so the anchor glyph can be judged by eye.

## NOTES

- **Resumed mid-work** (session rate limit killed the previous builder; it committed nothing). On entry all pieces 1–8 and MAP.md were already implemented and their unit tests green — verified by reading each piece's diff and by running `cargo test -p graft-ui --lib` (279 passed before my changes). What was actually missing, verified by grep: piece 9's `ui_widget_dumps` capture test (no `docs/shots/m5-ui-*` files, no such test anywhere in the crate) and the `detail_pane::tests::conflicts_section_draws_first_with_words_not_a_letter` test a stale doc-comment in `graph_screen/tests.rs` already referenced but that did not exist.
- **Scout: not used by me.** The ticket's scout was already spent by my predecessor before the session died (its outputs are baked into the code I inherited — I did not re-run it). I verified its conclusions by reading the affected source directly rather than trusting the summary.
- **Colour is unverified.** All new `docs/shots/m5-ui-*.txt` captures are ratatui `TestBackend` character-grid dumps — no colour attribute survives (CLAUDE.md rule 5). The Cancel-cursor default (ADR-0038 §1) and the ghost-vs-real-parent distinction (§5) are both designed to hold on shape/bracket/glyph alone, and the captures show exactly that; whether they *also* reverse/mute correctly in colour is not checked here.
- **`ui_widget_dumps` split into six tests.** The acceptance line names one test; I wrote six (`capture_for_docs_shots` in `graph_column`, `confirm`, `preview_box`, `graph_screen`, `detail_pane`, `name_prompt` tests modules), each `#[ignore]`d and each writing its own `docs/shots/m5-ui-*.txt`, matching the existing `credential_prompt::tests::capture_for_docs_shots` convention already in the crate. Ran all six by hand once (`cargo test -p graft-ui capture_for_docs_shots -- --ignored --nocapture`) to generate the committed files; they stay `#[ignore]`d so `cargo nextest run` doesn't rewrite them on every pass.
- **Gates, graft-ui only:** `cargo fmt --check` pass, `cargo clippy -p graft-ui --all-targets -- -D warnings` pass (0 warnings), `cargo nextest run -p graft-ui` 287 passed/0 failed/7 skipped, `cargo test --doc -p graft-ui` 0 tests (crate has no doc tests). Workspace build (`cargo build --workspace`) fails in `crates/graft` — see the Gates section above for the exact signatures the next builder into `graft` needs.
- **Deviation, already called out in MAP.md:** the ticket describes the banner replacing an "ahead/behind slot" in the header; no such slot exists (ahead/behind draws per-branch via row badges, not in the header). `banner_op` instead replaces the header's whole informational tail (walk state/filters/order/graphics status) — the closest real match to the design mockups. Not silently done: flagged in the `GraphView::banner_op` MAP.md entry my predecessor wrote.

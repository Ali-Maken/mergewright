# ui-ghost-banner-conflict: ghost rows, the preview box, N-choice confirm (+ its missing frame), the operation banner, the CONFLICTS section, the name prompt

Status: done (seam closed in a follow-on session; **fix round applied** — 1 Critical, 7 Majors, 2 Minors closed; ticks re-checked and three rewritten; gates workspace-wide 1898/1898)
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
// fix round: which lanes on a Ghost row are the preview's. A real lane through a ghost row stays solid.
pub struct GraphLine<'a> { …, pub ghost_lanes: std::collections::BTreeSet<usize> }
impl<'a> GraphLine<'a> { pub fn ghost(self, lanes: BTreeSet<usize>) -> Self; }   // sets paint + ghost_lanes together
// graft-ui/src/confirm.rs
pub struct ConfirmView { pub title: SanitizedText, pub message: Vec<SanitizedText>, pub command: SanitizedText, pub choices: Vec<SanitizedText>, pub cursor: usize }
impl ConfirmView { pub fn new(title, message, command) -> Self; pub fn with_choices(self, labels: Vec<SanitizedText>) -> Self;
    pub fn with_labels(self, safe: &str, confirm: &str) -> Self; pub fn left(&mut self); pub fn right(&mut self); pub fn toggle(&mut self); pub fn selected(&self) -> usize; }
// graft-ui/src/preview_box.rs   (fix round: choices/cursor private, area helper added)
pub struct PreviewBox { pub title: SanitizedText, pub lines: Vec<SanitizedText>, pub commands: Vec<SanitizedText>, pub planning: bool, /* private: choices, cursor */ }
impl PreviewBox { pub fn new(title: SanitizedText) -> Self;            // ["Cancel","Confirm"], cursor 0 — the only constructor
    pub fn with_choices(self, labels: Vec<SanitizedText>) -> Self;     // resets the cursor to 0
    pub fn choices(&self) -> &[SanitizedText]; pub fn selected(&self) -> usize; pub fn left(&mut self); pub fn right(&mut self); }
pub struct PreviewBoxWidget<'a> { pub view: &'a PreviewBox, pub theme: &'a Theme }   // Widget
pub fn preview_rows(view: &PreviewBox, _width: u16) -> u16;  // rows the box takes, for the caller's row budget
pub fn preview_box_area(frame: Rect, view: &PreviewBox) -> Rect;  // docked above the screen footer, or one row for the too-small message
pub const PREVIEW_MIN: (u16, u16) = (40, 12);   // measured against the FRAME, never the docked rect
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

**Ticks re-checked in the fix round (see "Fix round" below). Three were wrong as
worded and are rewritten here; one is now split because part of it is not
tested at all.**

- [x] Every test in piece 9 green, named at tick time — `cargo nextest run
      --workspace` 1898/1898, 28 skipped (the `#[ignore]`d capture
      generators)
- [x] The widgets that have a bounds test draw nothing outside their `Rect`
      — `preview_box::tests::preview_box_stays_inside_its_rect`,
      `name_prompt::tests::name_prompt_stays_inside_its_rect`,
      `credential_prompt::tests::stays_inside_its_rect` (proptest, pre-existing),
      `graph_column::tests::a_ghost_row_draws_nothing_outside_its_rect`
      (written in the fix round: the ghost rows drawn into an offset rect in
      a sentinel-filled buffer)
- [ ] **Not ticked:** the banner and the `CONFLICTS` section have **no**
      bounds test. The original tick named `graph_screen::tests::*` and
      `detail_pane::tests::*` — wildcards, and no such test exists in either
      (review Major 6). `graph_screen` is held by another builder this
      round, and `detail_pane`'s test harness draws into a full-screen
      `Terminal<TestBackend>`, so an offset-rect test needs a new helper.
      Handed back as gap 6 below.
- [x] `confirm_area` never exceeds the frame it is given —
      `confirm::tests::area_never_exceeds_the_frame`. This checks the
      **rect**, not drawn cells; the drawn-cell claim for the confirm box is
      carried by `confirm::tests::confirm_draws_its_frame` and
      `confirm::tests::every_choice_is_a_bracketed_button_and_only_the_cursor_is_marked`,
      which read the rendered buffer
- [x] Confirm box frame present at 80×24 and 160×48 (row 484) — `confirm::tests::confirm_draws_its_frame`
- [x] Captures exist and are regenerated from the current widgets —
      `docs/shots/m5-ui-{ghost-rows,confirm,preview-box,banner,conflicts,name-prompt}.txt`
      (largest 4.8 KB, far under the 200 KB cap), written by the `#[ignore]`d
      `capture_for_docs_shots` tests in `graph_column::tests`,
      `confirm::tests`, `preview_box::tests`, `graph_screen::tests`,
      `detail_pane::tests`, `name_prompt::tests`. The preview-box and
      confirm dumps no longer filter blank rows — that filter is what let a
      box the widget never drew look right (review Critical 1). Colour is
      **unverified**: these are character grids (CLAUDE.md rule 5) — the
      ghost-vs-real-parent and cursor-on-`Cancel` distinctions are carried by
      shape, glyph and the `▸` mark, not by colour
- [ ] **Not ticked (rule 6):** whether the captures look right is the
      user's call, and the user has not seen this round's files yet
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


---

## Follow-on session: the seam, and the masked field's pulse

Crate set: `graft-ui` + `graft`. Worktree `agent-ac6481546dbf30070`, `main`
merged in first (clean; group 04 never touched `text_input.rs`, which main
had rewritten).

### Job 1 — the seam `blocked: needs graft` stopped at

Every call site above fixed to the new shapes; the graft-ui API was not
reshaped to avoid the work.

- `update/confirm.rs` — `Choice` gone. The box acts when
  `view.selected() != 0`: index 0 is always the safe choice and always
  where the cursor starts (ADR-0038 §1), so "not 0" is the acting answer
  whether the box has two choices or five.
- `view/frame.rs` — `Confirm { view, theme }`.
- `view/rows.rs` — `banner_op: None` in both `GraphView` initialisers, with
  a comment saying why (nothing in `app` starts a rebase or merge yet;
  filling it is pass 2's).
- `update/{operate,remote,stash,undo}.rs` — every `ConfirmView::new`
  argument now goes through `sanitize_line`. Test sites follow
  (`view.title.as_str()`, `view.choices[0]/[1]`, `message` mapped to `&str`
  before `join`).
- `jobs/detail.rs` — `diff_spec_for` treats `Unmerged` like `Unstaged`
  (worktree against index: the file with its conflict markers, which is
  what the user needs to see).
- **A conflicted path is refused in words, not ignored.** `Space` on an
  `Unmerged` row and `d` on a hunk of one both answer the key and put a
  reason on the status line (`conflict_refusal`); `patch_direction` now
  returns `Option` and gives `None` for `Unmerged`, because a file with
  conflict markers has no clean side to cut a hunk against. A key that
  silently does nothing reads as broken, and guessing a patch side would
  have been worse.

### Job 2 — a per-keystroke pulse that still hides the length

`SecretField::pulse` was `!self.is_empty()`, so the first character lit the
row and nothing after it changed anything — half of ADR-0038 §2 (a
keystroke must visibly register) failing so the other half (the response
must not encode length) could hold.

Design, as the conductor proposed and as the old doc comment's own "entropy
the field is seeded with" pointed at:

- `SecretField::new(pulse_seed: bool)` is now the **only** constructor —
  `Default` is gone, so there is no way to build a field that starts at a
  fixed bit by accident. The bit flips on every edit (insert, each pasted
  character, backspace, delete, `Ctrl+U`), exactly as it did before the
  group 01 fix round.
- The seed is drawn by the app, once per prompt:
  `graft::jobs::askpass::pulse_seed()` (`getrandom`, falling back to the
  clock's nanosecond bit — see below), carried on
  `Progress::CredentialNeeded { prompt, pulse_seed }` and through
  `credential_needed` → `PromptField::new` → `SecretField::new`. It rides
  the event rather than being drawn inside `update` so `update` stays pure
  and every test pins the seed.
- The resting row is therefore `seed XOR (edits mod 2)`: uniform, and
  independent of what was typed. The same password draws a lit row in half
  its prompts and an unlit row in the other half.

**Stated plainly, in the doc comment and in the captures:** the seed does
not make the leak zero. Somebody watching that one screen from the moment
the prompt opened can count the flips and so count the keystrokes; seeing
it at two moments gives them the parity in between. That is the visible
feedback the user asked for and cannot be removed while keeping it. What
the seed closes is everything after the fact — the resting row, a
screenshot, a capture committed to this repository — where there is no
"when it opened" to count from.

`getrandom` failure: the fallback is the clock's nanosecond bit, not a
constant. Commented at the call site as a genuine ignore — the only thing
that turns on the draw is *which* bystander-unpredictable bit is used, and
refusing to open a credential prompt because the RNG is unavailable would
be a far worse answer than the fallback. A constant fallback would have put
the length leak straight back.

### Acceptance (this session)

- [x] The workspace builds and every gate is workspace-wide, never `-p` —
      `cargo fmt --all --check` (clean), `cargo clippy --all-targets
      --message-format=short -- -D warnings` (no output),
      `cargo nextest run --workspace` **1885 passed / 1885 run**, 28
      skipped (the `#[ignore]`d capture generators), `cargo test --doc
      --workspace` 2 passed / 0 failed
- [x] Staging or discarding a **conflicted** file answers the key and says
      why, instead of doing nothing or guessing a patch side —
      `update::operate::tests::space_on_a_conflicted_file_says_why_instead_of_staging_it`,
      `update::operate::tests::d_in_the_diff_region_of_a_conflicted_file_says_why_instead_of_asking`
- [x] **A keystroke visibly registers** — every one, not just the first
      (ADR-0038 §2, first half; the user: a field that looks dead while
      typing is not acceptable) — `graft_ui::text_input::tests::
      every_keystroke_visibly_changes_the_masked_row`: eight inserts, a
      backspace, a delete and `Ctrl+U`, from **both** seeds, each moving
      exactly one cell between `•` and `◦`
- [x] **That response does not count characters** (second half; the user:
      neither is one whose feedback counts characters) — `graft_ui::
      text_input::tests::the_resting_row_is_decided_by_the_seed_not_the_length`:
      a 3-character secret and a 30-character one rest at the *same two
      rows*, one per seed, so with the seed unknown and uniform a resting
      row tells an observer nothing about the length
- [x] The seed reaches the field from the app, per prompt, and the widget
      stays pure — compile-enforced: `SecretField::new(pulse_seed: bool)`
      is the only constructor and `Default` is gone, so no caller can get a
      fixed start by omission; `graft-ui/Cargo.toml` names no RNG, and the
      one production construction site is
      `update::credential::PromptField::new`, reached only from
      `credential_needed`. Tests pin the seed
      (`view::tests::credential_state_seeded`,
      `update::credential::tests`), which is why the captures are
      reproducible
- [ ] **Corrected in the fix round — this was a false tick.** It claimed a
      user judgement the builder cannot make (rule 6). What is true: the
      captures were regenerated. Whether they look right is the user's, and
      the user has not said. `docs/shots/m4-credential-prompt-in-app.txt` (14 KB) now shows
      **the same 7-character password at both seeds** — the two rows differ
      in the first glyph and in nothing else, which is the condition made
      visible; `docs/shots/m4-credential-prompt-and-hints.txt` caption
      rewritten (it still named the deleted
      `eight_glyphs_and_an_identical_row_for_any_length`). Colour is
      **unverified** — these are `TestBackend` character grids, no colour
      attribute survives (rule 5)
- [x] MAP.md updated in both crates — `graft-ui` (`SecretField::new`,
      `pulse`, no `Default`), `graft` (`Progress::CredentialNeeded`,
      `PromptField::new`, `credential_needed`, `jobs::askpass::pulse_seed`,
      the two refusals in `update::operate`)

### Gaps handed back

1. **`FileStatusView::Unmerged { how }` cannot say the real words yet.**
   The design wants `both modified` / `deleted by them` from git's
   porcelain `XY` pair. `graft_git::diff::FileStatus::Unmerged` is a unit
   variant — `file_list.rs` maps every `U` to it — so `jobs/detail.rs` can
   only pass `"unmerged"`. Carrying `XY` through needs `graft-git`, which
   this session did not hold. The widget side is done and tested
   (`detail_pane::files::tests::an_unmerged_row_shows_how_as_words_not_a_letter_and_counts`).
2. **The `CONFLICTS` section is never populated.** `jobs/detail.rs`'s
   `by_section` is a fixed array of three (Staged, Unstaged, Untracked);
   nothing builds an `Unmerged` group. Pass 2's, as the ticket says.
3. **Two committed captures are stale and I did not regenerate them.**
   `docs/shots/m4-03-credential-masked-*.txt` and
   `m4-04-credential-username-*.txt` still show the pre-M5 variable-length
   mask (28 `•` glyphs for a 28-character password). They come from the M4
   acceptance walk, which needs a local HTTP remote — heavy tier, not a
   builder's. When they are regenerated, note that the first mask glyph is
   the pulse and will differ between runs: that is the seed doing its job,
   not a diff to chase.
4. **The confirm-box section of
   `docs/shots/m4-credential-prompt-and-hints.txt` predates this group's
   frame fix.** That file's body is pasted together by hand from three
   generators; `docs/shots/m5-ui-confirm.txt` shows the framed box
   correctly. Re-pasting was group 04's to do and I left it rather than
   hand-edit a capture body.

---

## Fix round (review `docs/reviews/m5-04-ui-ghost-banner-conflict.md`)

Crate set: `graft-ui` + `graft`. Worktree `agent-ad3132a722c22aeef`, branched
from `main` at `37d2cc6`. **Fenced off and not touched:**
`crates/graft-ui/src/graph_screen/**`, `crates/graft/src/view/rows.rs`,
`crates/graft/src/update/remote.rs` (another builder holds them).

**Scout: not used.** The review names the file and line of every finding, and
the only structural question (which lanes a ghost row owns) was answered by
reading `graph_column/{mod,geometry,style}.rs` directly. A scout would have
re-read what the review already located.

### Critical 1 — the preview box never drew a box. **Cause.**

`PREVIEW_MIN`'s 12 rows was checked against the **docked rect**, whose natural
height is `preview_rows` = 7, so the box always fell through to the too-small
message; handed ≥12 rows it drew a title, a borderless hole and a footer.

- The screen-size decision moves to a new `preview_box::preview_box_area(frame,
  view) -> Rect` (the shape `name_prompt_area` already uses): it docks the box
  at its own height above the screen's footer row, or returns the **one row**
  the message needs when the *frame* is below `PREVIEW_MIN`. The widget's own
  floor is now `MIN_BOX_ROWS` (4) — the least a frame with something legible
  in it can be.
- `draw_body` writes a `│ … │` content row on **every** interior row, blanks
  included, with the buttons on the last one. A taller rect is still a box.
- Tests assert the interior, not the frame edges:
  `preview_box::tests::every_interior_row_is_a_bordered_content_row`,
  `::a_taller_rect_still_draws_a_box_on_every_interior_row`,
  `::preview_box_docks_above_the_footer_row_untouched` (now through
  `preview_box_area`, with a stand-in screen footer),
  `::too_small_screen_shows_one_line_and_nothing_else`,
  `::a_rect_below_the_box_floor_draws_the_message`.
- The capture's blank-row filter is **gone**, in `preview_box` and in
  `confirm`: `docs/shots/m5-ui-preview-box.txt` now prints every row of the
  screen. Cost to the user: the file is longer (4.3 KB).

### Major 1 — `←` wrapped onto the acting choice. **Cause.**

`update/confirm.rs`: `Left => left()`, `Right => right()` (both clamp),
`Tab | BackTab => toggle()`. Tests:
`update::confirm::tests::left_on_cancel_stays_on_cancel` (two- and
three-choice boxes) and `::right_moves_onto_the_acting_choice_and_clamps`.

### Major 2 — the askpass fallback was a constant on macOS. **Cause.**

`subsec_nanos() & 1` → `clock_parity()`, the parity of `as_micros()`. The doc
comment now says what the fallback is and is not worth: it varies between
prompts, it is **not** unpredictable to somebody who knows when the prompt
opened to the microsecond, and it is the floor under a `getrandom` failure
that does not happen on macOS — not the guarantee the seed rests on. Test:
`jobs::askpass::tests::the_clock_fallback_is_not_a_constant` (draws until it
has seen both values; a constant fails it on any platform).

### Major 3 — `d` on a CONFLICTS file row opened a HEAD reset. **Cause.**

`update::operate::request_discard` refuses a `WorkingTreeSection::Unmerged`
row with `conflict_refusal("discard")`, beside the two existing refusals.
Test: `::d_on_a_conflicted_file_row_says_why_instead_of_offering_a_reset_to_head`.

### Major 4 — ghost rows dashed real lanes. **Cause.**

`GraphLine` gains `ghost_lanes: BTreeSet<usize>` — the **real** columns the
preview owns — and `GraphLine::ghost(lanes)` sets it together with `paint`.
On a `Ghost` row only those lanes (plus the anchor's own cell) dash and mute;
every other lane draws exactly as on a real row. This follows DESIGN.md §1.7's
`┊│┊●b┈` and `docs/design/05-visualization.md`'s "real nodes stay solid"
rather than amending them. Empty `ghost_lanes` on a ghost row ghosts only the
anchor — the conservative reading: a real lane is never made to look unreal by
a caller's omission. Group 08 must name the preview's lanes when it builds the
rows. Test: `graph_column::tests::a_real_lane_through_a_ghost_row_stays_solid_and_keeps_its_colour`;
`docs/shots/m5-ui-ghost-rows.txt` now shows both cases side by side.

### Major 5 — `PreviewBox` had no constructor. **Cause.**

`choices` and `cursor` are **private**. `PreviewBox::new(title)` is the only
way in (`["Cancel", "Confirm"]`, cursor 0); `with_choices` resets the cursor;
`choices()` reads them back. Test:
`preview_box::tests::the_only_constructor_opens_on_cancel`.

### Major 6 — tick evidence. **Cause.**

Every tick above rewritten; see the Acceptance sections. One wildcard tick is
now split, with the untested half left unticked and filed as gap 6; the "user
judged" tick is marked as the false tick it was.

### Major 7 — the confirm caption described a cursor that was never drawn. **Cause.**

`frame_box::buttons_row(labels, cursor)` brackets **every** choice — DESIGN.md
§1.7 draws them all bracketed — and marks the cursor's with a leading `▸`,
unselected ones with a space, so the row never shifts. `frame_box::reverse_button`
adds `REVERSED` on top for terminals that have colour; the caption now says
that cannot be judged in a character grid (rule 5) and to judge the `▸` and
the brackets instead. Test:
`confirm::tests::every_choice_is_a_bracketed_button_and_only_the_cursor_is_marked`.

### Also fixed, found while regenerating captures

- **Minor 1, cause:** `frame_box::border_row` now writes a space on **both**
  sides of the label (`╭ main ───╮`), as the mockups draw it and as its own
  doc comment already claimed.
- **Minor 2, cause:** `preview_rows`'s unused parameter is named `_width`; the
  `let _ = width;` inside the body is gone.
- **Not in the review — a drift the stale captures were hiding.**
  `request_discard_at_diff_cursor` built its box with the default
  `Cancel`/`Confirm` labels, so "Discard this hunk of src/a.rs?" sat over a
  button reading `Confirm`. The committed capture showed `Discard`, from
  before the M5 frame fix. Restored with `.with_labels("Cancel", "Discard")`
  (m3/04 review M6: the question and the button name the same act) and
  asserted in `::d_in_the_diff_region_confirms_before_discarding_the_hunk`.

### Fix-round acceptance

- [x] The preview box draws a box at its own height and at any taller rect,
      and the test reads the interior — `preview_box::tests::every_interior_row_is_a_bordered_content_row`,
      `::a_taller_rect_still_draws_a_box_on_every_interior_row`
- [x] The capture shows what the widget draws, unfiltered — the blank-row
      filter is removed from `preview_box::tests::capture_for_docs_shots` and
      `confirm::tests::capture_for_docs_shots`; `docs/shots/m5-ui-preview-box.txt`
      prints all 24/48 rows of each screen
- [x] **`←` on `Cancel` stays on `Cancel`** (ADR-0038 §1: the user's uniform
      default is worth nothing if one key-repeat walks off it) —
      `update::confirm::tests::left_on_cancel_stays_on_cancel`, two- and
      three-choice
- [x] **A conflicted path is refused in words by `d` on the file row too**
      (the group's own principle, applied to the key that was missing it) —
      `update::operate::tests::d_on_a_conflicted_file_row_says_why_instead_of_offering_a_reset_to_head`
- [x] **A real lane is still recognisably real inside a ghost row**
      (ADR-0038 §5's other half: if a ghost can be confused with a real
      parent *or the reverse*, the widget is wrong) —
      `graph_column::tests::a_real_lane_through_a_ghost_row_stays_solid_and_keeps_its_colour`
- [x] **No caller can build a preview box whose acting choice is under the
      cursor** — compile-enforced (`choices`/`cursor` private,
      `PreviewBox::new` the only constructor) and
      `preview_box::tests::the_only_constructor_opens_on_cancel`
- [x] **The fallback pulse seed is not a constant on this machine** —
      `jobs::askpass::tests::the_clock_fallback_is_not_a_constant`
- [x] **Every choice reads as a button and the cursor is visible without
      colour** — `confirm::tests::every_choice_is_a_bracketed_button_and_only_the_cursor_is_marked`
- [x] Gates, workspace-wide: `cargo fmt --all --check` clean,
      `cargo clippy --all-targets --message-format=short -- -D warnings` no
      output, `cargo nextest run --workspace` **1898 passed / 1898 run**, 28
      skipped, `cargo test --doc --workspace` 2 passed / 0 failed
- [x] MAP.md updated in both crates (`frame_box`, `preview_box`, `GraphLine`
      in `graft-ui`; `confirm_key`, `request_discard`,
      `request_discard_at_diff_cursor`, `askpass` in `graft`)
- [ ] **Not ticked (rule 6):** the regenerated captures have not been judged
      by the user.

### Gaps handed back (fix round)

5. **The confirm box overlaps the hint-bar row when the detail pane is open.**
   Visible in the regenerated `docs/shots/m3-confirm-discard.txt`,
   `m3-undo-drift.txt` and `m4-remote-keys.txt`: the box's bottom border lands
   on the `q quit  F2 status …` row. `confirm_area` reserves the last row of
   the rect it is given, and `view::frame.rs` gives it the whole frame — whose
   last row is below the hint bar, not the hint bar itself. **Pre-existing**
   (the old captures show the buttons row on the same line), not caused by
   this round, and the row belongs to the `hint-bar-redesign` group's file.
6. **No bounds test for the banner or the `CONFLICTS` section.** The original
   tick named wildcards. `graph_screen` is fenced this round; `detail_pane`'s
   harness draws into a full-screen `Terminal<TestBackend>`, so an
   offset-rect test needs a new helper. The ghost rows' bounds test was
   written (`graph_column::tests::a_ghost_row_draws_nothing_outside_its_rect`).
7. **Five `docs/shots/m2-*.txt` captures are stale from hint-bar drift.**
   `m2-duplicate-commit`, `m2-evil-merge`, `m2-explain-panel`,
   `m2-force-push`, `m2-overview-strip` regenerate with a different footer
   line (`u undo`/`. explain` in, `1 first-parent`/`F focus` out). Nothing to
   do with this round, and the hint bar is being rebuilt concurrently — I
   reverted those five rather than commit a footer the other builder is about
   to change again.
8. **`ghost_lanes` has no production caller yet.** Group 08 builds the ghost
   rows; if it sets `paint = RowPaint::Ghost` without naming the preview's
   lanes it gets an anchor with solid edges, not a wrong picture of what is
   real. The signature is in `crates/graft-ui/MAP.md`.

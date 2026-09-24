# editor-refusal-on-its-own-row: the commit editor's refusal gets a full-width row above the options, so the whole sentence shows at 80 columns

Status: done
Was: ticket 39 Step 4, blocked by its own stop condition (39 NOTES: at 80×24 the options row reads `[ ] amend    [ ] sign-off    [ ] no-verify    [ ] allow empty    [ Commit ]  …`, the refusal cut completely). Spec-only pass 2026-09-24.
Blocked by: 39 (merged `27c133ce`).
Build with: **Sonnet** — one widget's row layout with the rule, the budget and every stop condition given; the words don't change; no route, no state, no key.
Needs: `graft_ui::commit_editor::{CommitEditorView, CommitEditorScreen, fits}`; `graft::update::commit_editor::{CommitEditor::refusal, live_operation}` — as merged; the only change is inside `CommitEditorScreen`'s draw and `fits`'s row budget.
Crates: graft-ui (the fix); graft (tests, capture fixtures, MAP only — no `src` logic). **Not graft-ops** (ticket 38 holds it).

## Decision: move the words, don't cut them
- Arithmetic from the committed captures (`docs/shots/m3-commit-editor-refused.txt:35`): the options row uses 79 columns up to and including `[ Commit ]  `. At 80 columns nothing fits beside the button; no wording can fix that. Ticket 39's words (60 display columns, InShell merge; 59 InGraft merge) stay exactly as built.
- **The refusal gets its own row, directly above the options row**, indented 2 to line up with `[ ] amend`, full width, in the token it is drawn in today (dimmed; **colour unverified**, rule 5). The options row ends at `[ Commit ]`; nothing after it. The title row's `nothing staged` is unchanged.
- Why (`docs/research/ux-principles.md`): H9 — an error says what, why and the recovery key; the recovery key is what got cut. H1 — status must be on screen at the narrow breakpoint (`docs/design/01-view-mockups.md` §1.3, 80×24). Proximity kept: the reason sits on the row touching the dimmed button's row.
- **Rejected:** shorter words (1 column is left at 80); the footer row (it carries `esc cancel` and the hook-failure `status`; the refusal would evict one or the other); the title row (shares its width with `⎇ <branch>`, so a long branch name cuts it again); wrapping on the options row (two sizes of options row to hit-test).
- **Row budget:** the row is **budgeted always, drawn only when refused**. `fits` needs one more row than today, so it still takes no view. Not refused → the body block keeps that row (non-refused layout identical to today). Refused → the body block ends one row higher.
- Wider than `width − 2` (only below 80 columns): cut with a trailing `…`, as the options row does today. Not wrapped.

## Read first
- This ticket, all of it. Ticket 39: Step 4 and NOTES (`.scratch/m6-conflicts/issues/39-editor-words-name-the-banner.md` :52, :100-107).
- `crates/graft-ui/MAP.md` §`commit_editor` (:304-313). `crates/graft/MAP.md` §`update::commit_editor` (:227-240).
- `docs/research/ux-principles.md` :51, :59, :122.
- Source only after the MAP: `graft-ui/src/commit_editor.rs` (the draw; the refusal at :294-296 per 39's builder; `fits`, `CHROME_ROWS`); `graft/src/view/tests.rs` (`capture_m3_commit_editor_refused`); `graft/tests/it/resolver/shell_begun.rs` (`resolved_shell_merge`, `launched_into`, `the_keys_the_commit_editor_names_reach_the_summary`); `graft/tests/it/resolver/editor_words.rs`.

## Scope
**Step 0 — observe today, before any change.** Over 39's J3 state (resolved shell merge, editor opened from the changes pane), render at 80×24 and at 120×40 on unchanged code. Copy each frame's options row and title row verbatim into NOTES. This answers 39's open **question** (is 120×40 cut too?) by observation; the fix doesn't depend on the answer.

**Step 1 — failing tests first.**
- graft-ui, `commit_editor::tests::the_refusal_has_its_own_row_above_the_options`: a view with `refusal = Some("merge begun in the shell — esc, ⇥ banner, then c finishes it")`, drawn into 80×24. The row directly above the options row equals `"  " + words`, padded; the options row, trimmed, ends with `[ Commit ]`. Red today.
- graft-ui, `commit_editor::tests::without_a_refusal_the_body_keeps_the_row`: `refusal = None`, the same area; the body block has as many line rows as it has today (assert the number; write it in NOTES); the row above the options row is a body row.
- graft-ui, `fits`: the existing boundary test moves up by one row (the smallest `fits` height is one more than today; the height below it does not fit). Name the test and both heights in NOTES.
- graft, `tests/it/resolver/shell_begun.rs::the_shell_merge_words_fit_at_80_by_24` (39 removed it uncommitted; write it again): over J3, 80×24, through the app's own view. One row contains the exact sentence `merge begun in the shell — esc, ⇥ banner, then c finishes it`; no row contains `git commit`; no row ends in `…`.
- graft, `tests/it/resolver/editor_words.rs::the_graft_merge_words_fit_at_80_by_24`: the same over 39's InGraft state; sentence `merge in progress — esc, ⇥ banner, then c continue  a abort`.
- graft, `update::commit_editor::tests::every_refusal_fits_80_columns`: every `refusal` arm (each `InProgress` kind × `Begun`, bisect, nothing staged, empty subject) has display width ≤ 78 (`unicode-width`, or the width the widget already uses). **Stop condition:** any arm wider → `blocked: <arm> is <n> cols at 80 — <words>`. Don't reword; wording is the conductor's.

**Step 2 — the fix, in `graft_ui::commit_editor` only.** The refusal row per the Decision; `fits`'s budget +1 row; the options row stops drawing the reason.
- **Stop condition:** anything in graft computes editor row positions by its own arithmetic (mouse hit-testing, the body's scroll height) instead of asking graft-ui → it must follow the new layout; name each place and line in NOTES. If one can't follow without a signature change → `blocked: <place> <file:line>`.

**Step 3 — pins of the old placement.** `grep -rn "beside\|\[ Commit \]  " crates/graft crates/graft-ui` and every test or capture fixture pinning the reason on the options row: update it. That includes the header words in `capture_m3_commit_editor_refused` (`sits immediately after [ Commit ] on the options row` → `sits on its own row directly above the options row`). Captures regenerated by their own tests only; list every file in NOTES. Don't reassemble `docs/shots/m3-contact-sheet.txt`.

**Out:** the words (39's, unchanged); the title row; the footer and hook `status`; `Field` / `Tab` order; the key route (39's pin `after_esc_the_panes_c_and_a_are_commit_and_amend` stays green, unchanged); every other screen's refusal.

## Signatures
```rust
// graft-ui/src/commit_editor.rs — signatures unchanged; behaviour per the Decision
pub struct CommitEditorView { /* .. */ pub refusal: Option<String>, /* .. */ }   // unchanged
pub fn fits(area: Rect) -> bool;   // unchanged signature; budget = today's + 1 row (CHROME_ROWS + 1)
impl Widget for CommitEditorScreen<'_> { fn render(self, area: Rect, buf: &mut Buffer); }  // row order: title, subject, body block, refusal row, options row, footer
// graft — called, unchanged: update::commit_editor::{CommitEditor::refusal, live_operation}
```

## Acceptance (name the verifying test at tick time)
- [x] Step 0: both frames' options and title rows verbatim in NOTES; 39's 120×40 question answered by observation. — doc, no test
- [x] `the_refusal_has_its_own_row_above_the_options` red before Step 2, green after.
- [x] `without_a_refusal_the_body_keeps_the_row` green before and after (non-refused layout unchanged); body row count in NOTES.
- [x] `fits` boundary test moved by one row; both heights in NOTES.
- [x] **(from 39)** The 80×24 check: `shell_begun.rs::the_shell_merge_words_fit_at_80_by_24` red before, green after; `editor_words.rs::the_graft_merge_words_fit_at_80_by_24` the same.
- [x] `every_refusal_fits_80_columns` green; the widest arm and its width in NOTES.
- [x] 39's tests all still green, unchanged: `after_esc_the_panes_c_and_a_are_commit_and_amend`, `the_keys_the_commit_editor_names_reach_the_summary`, `the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`, `a_merge_in_progress_points_at_c`, `a_graft_begun_operation_names_the_banner`.
- [x] **(from 39)** Capture (rule 6; colour and dimming **unverified**, rule 5): `docs/shots/m6/fix-editor-names-the-banner-120x40.txt`, the editor open over J3. **What right looks like:** the row directly above the options row reads, from column 3, exactly `merge begun in the shell — esc, ⇥ banner, then c finishes it`; the options row ends at `[ Commit ]` with nothing after it; `git commit` and `…` appear nowhere in the editor's rows; the title row is as recorded in Step 0. The user judges by eye.
- [x] Capture: `docs/shots/m6/fix-editor-names-the-banner-80x24.txt`, the same state at 80×24. **What right looks like:** the same four lines, at 80 columns; the sentence ends at column 62, whole.
- [x] MAP.md, graft-ui §`commit_editor`: `CommitEditorView::refusal` "drawn dimmed on its own row above the options row"; `CommitEditorScreen`'s row list gains the refusal row; `fits`'s budget. graft §`update::commit_editor` `Begun` line: "one `⇥` reaches the banner" → "two `⇥` (Files → Diff → banner, ticket 39 NOTES n = 2)". — doc, no test
- [x] NOTES: every file whose pinned placement changed; any graft place from Step 2's stop condition. — doc, no test

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small. One context.

## For the conductor (exact text)
- `TESTING.md:1331`, replace `refused: \`nothing staged\` on the title and beside the button` with `refused: \`nothing staged\` on the title and on its own row directly above the options row (ticket 40)`.
- `TESTING.md:2122`, delete `; at 80 columns they are cut off (ticket 40)` (keep the full stop).
- `docs/design-notes/m6-conflicts.md`, after :483 (39's line): `- The commit editor's refusal has its own row above the options row, full width, budgeted always and drawn only when refused; at 80 columns every refusal fits whole (ticket 40).`

## NOTES (builder, 2026-09-24)
No scout: the ticket named every file and line; one widget's draw, read directly (commit_editor.rs 323 lines). Not compacted.

**Step 0, unchanged code, J3 (resolved shell merge, editor from the changes pane), verbatim:**
- 80×24 title: `COMMIT ── ⎇ main ── 2 files staged`; options: `  [ ] amend    [ ] sign-off    [ ] no-verify    [ ] allow empty    [ Commit ]  …`
- 120×40 title: `COMMIT ── ⎇ main ── 2 files staged`; options: `  [ ] amend    [ ] sign-off    [ ] no-verify    [ ] allow empty    [ Commit ]  merge begun in the shell — esc, ⇥ banner…`
- 39's question answered by observation: **yes, 120×40 was cut too** (the detail pane beside the editor narrows it).

**Ticks → tests:**
- Step 0 — doc (above).
- Own row: `graft-ui commit_editor::tests::the_refusal_has_its_own_row_above_the_options` — observed FAIL before Step 2, PASS after.
- Non-refused unchanged: `commit_editor::tests::without_a_refusal_the_body_keeps_the_row` — PASS before and after. Body block = **18 rows** (label + 17 lines, rows 4–21) at 80×24; options row 22.
- `fits` boundary: `commit_editor::tests::a_tiny_area_still_draws_the_title_and_never_panics` — was 6 no / 7 yes; now **7 no / 8 yes** (`REFUSAL_ROWS` = 1 added to the budget).
- 80×24: `it resolver::shell_begun::the_shell_merge_words_fit_at_80_by_24` and `resolver::editor_words::the_graft_merge_words_fit_at_80_by_24` — both observed FAIL before (row ends `[ Commit ]  …`), PASS after.
- `update::commit_editor::tests::every_refusal_fits_80_columns` — 13 arms, all ≤ 78. Widest: **76 cols**, `cherry-pick begun in the shell — finish it there: git cherry-pick --continue`.
- 39's tests unchanged and green in the full run (3006/3006): `after_esc_the_panes_c_and_a_are_commit_and_amend`, `the_keys_the_commit_editor_names_reach_the_summary`, `the_keys_the_commit_editor_names_reach_the_banner_on_a_graft_begun_merge`, `a_merge_in_progress_points_at_c`, `a_graft_begun_operation_names_the_banner`.
- Captures: `it resolver::shell_begun::capture_fix_editor_names_the_banner` writes `docs/shots/m6/fix-editor-names-the-banner-120x40.txt` and `-80x24.txt` (asserts via `the_words_fit`; pumps to a settled frame first — unsettled, the detail pane's `reading…` was in the 120×40 frame). Colour/dimming **unverified** (rule 5); look judged by the user.
- MAP: graft-ui §commit_editor (`refusal`, row list, `fits` = 8 rows), graft §update::commit_editor `Begun` (two `⇥`) — doc.

**Step 2 stop condition:** no graft code computes editor row positions. `view/frame/boxes.rs:19` only renders `CommitEditorScreen`; no mouse hit-test on the editor (`resolver_mouse` ignores input while the editor is up). Not triggered.

**Pinned placement changed (Step 3):** `crates/graft-ui/src/commit_editor/tests.rs` (`a_refusal_is_drawn_beside_the_button` replaced by the two new tests); `crates/graft/src/view/tests.rs` (`capture_m3_commit_editor_refused` header + comment); `docs/shots/m3-commit-editor-refused.txt` (regenerated by its test). `docs/shots/m3-contact-sheet.txt` not reassembled, per the ticket — it still shows the old placement.

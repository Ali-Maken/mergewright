# G-look 5b — cursor row leaves the lanes alone; dark theme B; legible text on the fill

Status: done (re-specced after scout 07, `.scratch/m8-polish-and-ship/scouts/07-look-cursor-row.md`)
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
- [x] `contrast` matches WCAG on 3 pairs: white/black 21.0, `#777777`/white ≈ 4.48, equal colours 1.0 (test).
      — `theme::contrast::tests::contrast_matches_wcag_on_three_pairs`
- [x] Every theme × every `CONTRAST_RULES` row passes (one test, all kinds); dark B: selection/bg ≥ 3.13, text/selection
      ≥ 5.13, text-muted/bg ≥ 6.94 (the note's numbers − 0.01) (test).
      — `theme::contrast::tests::every_theme_clears_every_contrast_rule`, `…::dark_b_measures_what_the_note_measured`
- [x] `legible_on`: every lane hue and every branch colour on dark-B, light and high-contrast `selection` ≥ 4.5 after
      the call; dominant channel unchanged (test over all twelve lanes).
      — `theme::contrast::tests::legible_on_lifts_every_lane_onto_every_selection` (branch badges take lane hues, so
      the twelve lanes are the branch colours), plus `legible_on_leaves_a_legible_colour_alone`,
      `legible_on_heads_for_black_on_the_light_fill`
- [x] `on_selection` at 256: the reduced result vs reduced `selection` ≥ 4.5 for all twelve lanes (test); at Palette16
      and `NO_COLOR` returns `token(Text)` (test).
      — `theme::contrast::tests::on_selection_at_256_clears_after_reduction_for_every_lane`,
      `on_selection_without_a_fill_is_text`, `on_selection_at_truecolor_is_legible_on`
- [x] Paint (Unicode and ASCII tiers): on the cursor row, gutter cells and graph-column cells have bg = ground; cells
      from the marker slot to the body's right edge have bg = `selection`; the margin cell is ground (test).
      — `graph_screen::tests::cursor_row::the_fill_runs_from_the_marker_slot_to_the_body_edge` (both tiers),
      `tests/selection.rs::the_cursor_row_differs_from_its_neighbours_by_three_named_attributes`
- [x] Pixel: graph-column cells on the cursor row have bg = ground; `pixel_scroll` / strips tests unchanged and green
      (strips.rs:70 "cursor move leaves every strip") (test names).
      — `graph_screen::tests::cursor_row::at_the_pixel_tier_the_graph_cells_keep_the_ground`; strips tests untouched
      and green in the workspace run: `view::strips::tests::the_selection_thread_and_focus_reach_the_strip`,
      `held_j_keeps_the_cursor_lit_under_the_uncommitted_row`, `every_strip_waits_while_the_cursor_row_has_not_arrived`
      (no "cursor move leaves every strip" text exists in the tree — grep empty; these are the cursor/strip tests)
- [x] Glyphs: `▸` (ASCII `>`) in gutter cell 0 of the cursor row only; cursor row's marker slot has no `▏` unless it
      is the compare base; drop target draws `→`/`=` in the slot and no gutter glyph; a cursor and a drop target on
      screen together never share a glyph (tests; m5/35 drag tests updated, named in NOTES).
      — `cursor_row::the_cursor_is_marked_in_the_gutter_not_the_slot`, `…::the_compare_base_keeps_its_rule_on_the_cursor_row`,
      `…::a_drop_target_never_looks_like_the_cursor`; `tests/ascii_tier.rs` (gutter `>`)
- [x] 16 colours and `NO_COLOR`: no fill; cursor-row badges are `Text` + bold; `▸` present (test).
      — `cursor_row::without_a_fill_the_badge_is_text_and_bold`,
      `graph_screen::tests::at_palette16_the_cursor_row_gets_its_mark_instead_of_a_background`,
      `…::with_no_color_the_cursor_row_keeps_its_mark_and_dimmed_rows_stay_faint`
- [x] Each of the seven other painters: one test asserting a text cell on `selection` has fg ≥ 4.5 against it in dark
      B (or one table test over all seven, named per screen).
      — journal `journal_screen::tests::text_on_the_cursor_row_is_legible`; stash `stash_screen::tests::text_on_the_cursor_row_is_legible`;
      files `detail_pane::tests::text_on_the_files_cursor_row_is_legible`; finish summary
      `finish_summary::tests::text_on_the_cursor_row_is_legible`; conflict finish row
      `conflict_resolver::tests::finish_row::finish_row_style`; map `map_screen::tests::text_on_the_cursor_row_is_legible`;
      rewrite editor `rewrite_editor::tests::text_on_the_cursor_row_is_legible`. Graph itself:
      `cursor_row::text_on_the_fill_is_legible`.
- [x] J23 (0 steps): capture's cursor row shows `▸` in the gutter and the branch label as text.
      — `graft::it look_chrome::capture_the_cursor_row_in_dark_and_light` (asserts the row starts `▸ │ │ ●`, no `▏`)
- [x] Captures: `docs/shots/m8-look-cursor-row-dark-120x36.txt` and `…-light-120x36.txt`, cursor on a row with a branch
      badge and three lanes, compared beside `m8-proto-4a-dark-a-b.txt` (gutter `▸`, lanes, columns line up).
      **Colour unverified** (rule 5): the ratios are the tests' claim, legibility is his eye.
      — written by `look_chrome::capture_the_cursor_row_in_dark_and_light`, which measures the real app's buffer:
      worst character on the fill 4.57:1 dark, 4.55:1 light (in each caption). Not judged by eye (rule 6).
- [x] NOTES: picker cursor style (decision 7); every re-shot snapshot listed; KNOWN_GAPS 541, 542 closing evidence
      (test names) — the conductor moves the rows. — see NOTES below.

## NOTES (builder, 2026-10-03)

- **Context compacted once mid-ticket.** Work finished after it; nothing left over.
- Scout used: `.scratch/m8-polish-and-ship/scouts/07-look-cursor-row.md`.
- **Picker (decision 7, not changed):** `picker.rs:238-239` draws its cursor row as `foreground(Accent)` + BOLD text,
  with a `▸ ` prefix (`picker.rs:317`). No fill, no reverse video — so nothing sits on `selection` and it is not a
  legibility gap candidate.
- Status screen: no `Token::Selection` use (grep `Selection` in `status*` empty) — untouched.
- Docs: `docs/design/05*` and ADR-0003 have no "fill under lanes"/band wording (grep) — unchanged. DESIGN §1.0 was
  already written. `crates/graft-ui/MAP.md` updated (theme `contrast`/`ContrastRule`/`CONTRAST_RULES`/`legible_on`/
  `on_selection`, paint `on_selection`/`lift_on_selection`, cursor row, marker slot, `RowPaint::DropTarget`).
- Dark B at **16 colours** reduces differently: `text` → 15 (was 7), `text-muted` → 7 (was 8)
  (`theme/snapshots/…resolved_values.snap`). Light and high-contrast unchanged: every rule passed.
- **Ticket 06 test-reading defect fixed:** `tests/lanes_without_hue.rs`, `tests/selection.rs` (modifier grid) and
  `tests/ascii_tier.rs` read graph cells from x=0, i.e. the two gutter cells, after 06 added the gutter. They now
  offset by `GUTTER`; the `lanes_without_hue__*` and `selection__*` grids now show the full lanes (cause, not symptom:
  the snapshots were reading the wrong cells).
- A drop target is "raised", so its accent `→` is lifted onto the fill like the cursor row's text
  (`view::tests::a_drag_in_progress_is_visible_in_the_buffer_before_the_button_comes_up` now expects
  `on_selection(Accent)`).
- **m5/35 drag tests updated:** `view::tests::a_drag_in_progress_is_visible_in_the_buffer_before_the_button_comes_up`
  (target `→`, cursor gutter `▸`), `name_and_drag::capture_a_drag_in_flight_for_docs_shots` (caption).
  Other tests that found the cursor row by `▏`, now by a leading `▸`: `land_on_result::capture_landed_after_merge`,
  `…::capture_landing_after_undo_and_redo`, `m7_walk::rewrite_stops::edit_stop_lands`,
  `m7_walk::journal::the_three_lost_work_journeys`, `blame_place::capture_blame_then_commit_with_base_120x40`
  (now asserts zero `▏` and one `▸`). Captions: m5 and m7 contact-sheet captions, TESTING.md 07d edit-stop line.
- **KNOWN_GAPS evidence** (conductor moves the rows): 541 → `every_theme_clears_every_contrast_rule`,
  `dark_b_measures_what_the_note_measured`; 542 → `the_fill_runs_from_the_marker_slot_to_the_body_edge`,
  `at_the_pixel_tier_the_graph_cells_keep_the_ground`,
  `tests/selection.rs::the_cursor_row_differs_from_its_neighbours_by_three_named_attributes`.
- **Every re-shot diff checked** by a script that erases `▏ ▸ → > | =` and compares each changed line pair: every
  capture and golden differs only by the mark moving, except the intended ones — theme `resolved_values` (B values),
  `lanes_without_hue__*` / `selection__*` (gutter offset), `m5-ui-dragging-*` (caption words).
- Contact sheets m3, m4, m5 (2 parts), m6 (5), m7 (2) re-assembled with `scripts/contact-sheet.sh`; every part
  ≤ 177 900 B. No m8 sheet yet (acceptance ticket).
- Insta snapshots re-shot: graft_ui__graph_screen__tests__badges-icons-nerd.snap, graft_ui__graph_screen__tests__badges-icons-plain.snap, graft_ui__theme__tests__resolved_values.snap, ascii_tier__ascii-criss-cross-merge-no-color.snap, ascii_tier__ascii-criss-cross-merge-truecolor.snap, ascii_tier__ascii-foxtrot-merge-no-color.snap, ascii_tier__ascii-foxtrot-merge-truecolor.snap, ascii_tier__ascii-octopus-merge-no-color.snap, ascii_tier__ascii-octopus-merge-truecolor.snap, graph_screen__feature-branches-120x40.snap, graph_screen__feature-branches-200x60.snap, graph_screen__feature-branches-80x24.snap, lanes_without_hue__criss-cross-merge-16.snap, lanes_without_hue__criss-cross-merge-256.snap, lanes_without_hue__criss-cross-merge-no-color.snap, lanes_without_hue__foxtrot-merge-16.snap, lanes_without_hue__foxtrot-merge-256.snap, lanes_without_hue__foxtrot-merge-no-color.snap, lanes_without_hue__linear-16.snap, lanes_without_hue__linear-256.snap, lanes_without_hue__linear-no-color.snap, lanes_without_hue__octopus-merge-16.snap, lanes_without_hue__octopus-merge-256.snap, lanes_without_hue__octopus-merge-no-color.snap, selection__criss-cross-16-focus.snap, selection__criss-cross-16-highlight.snap, selection__criss-cross-truecolor-focus.snap, selection__criss-cross-truecolor-highlight.snap, graft__view__tests__renderer-Ascii.snap, graft__view__tests__renderer-Auto.snap, graft__view__tests__renderer-Pixel.snap, graft__view__tests__renderer-Unicode.snap
- docs/shots re-shot (150): m2-duplicate-commit.txt, m2-evil-merge.txt, m2-explain-panel.txt, m2-force-push.txt, m2-overview-strip.txt, m3-changes-pane-120x40.txt, m3-changes-pane-160x48.txt, m3-confirm-delete-untracked.txt, m3-confirm-discard.txt, m3-undo-drift.txt, m4-credential-prompt-in-app.txt, m4-remote-keys.txt, m5-hints-branch-120x40.txt, m5-hints-branch-160x48.txt, m5-hints-branch-80x24.txt, m5-hints-changes-120x40.txt, m5-hints-changes-160x48.txt, m5-hints-detail-120x40.txt, m5-hints-detail-160x48.txt, m5-hints-marked-120x40.txt, m5-hints-marked-160x48.txt, m5-hints-marked-80x24.txt, m5-hints-plain-120x40.txt, m5-hints-plain-160x48.txt, m5-hints-plain-80x24.txt, m5-hints-uncommitted-120x40.txt, m5-hints-uncommitted-160x48.txt, m5-hints-uncommitted-80x24.txt, m5-lane-cap-160x24.txt, m5-ui-banner-conflict-120x40.txt, m5-ui-banner-conflict-160x48.txt, m5-ui-banner-conflict-80x24.txt, m5-ui-banner.txt, m5-ui-confirm-over-graph-160x48.txt, m5-ui-confirm-over-graph-80x24.txt, m5-ui-context-menu-verbs-160x48.txt, m5-ui-context-menu-verbs-80x24.txt, m5-ui-credential-over-graph-160x48.txt, m5-ui-credential-over-graph-80x24.txt, m5-ui-dragging-160x48.txt, m5-ui-dragging-80x24.txt, m5-ui-drop-menu-160x48.txt, m5-ui-drop-menu-80x24.txt, m5-ui-name-prompt-80x24.txt, m5-ui-preview-rebase-160x48.txt, m5-ui-preview-rebase-80x24.txt, m5-ui-preview-scrolled-160x48.txt, m5-ui-preview-scrolled-80x24.txt, m5-ui-preview-time-machine-160x48.txt, m5-ui-preview-time-machine-80x24.txt, m5-ui-undo-after-push-80x24.txt, m6/esc-then-g-120x40.txt, m6/fix-a-mid-merge-answers-in-words-187x50.txt, m6/fix-abort-reads-aborted-120x40.txt, m6/fix-box-keeps-on-y-120x40.txt, m6/fix-box-owns-the-bar-120x40.txt, m6/fix-box-owns-the-bar-80x24.txt, m6/fix-box-takes-y-120x40.txt, m6/fix-box-takes-y-80x24.txt, m6/fix-c-in-pane-finishes-187x50.txt, m6/fix-c-live-frames-187x50.txt, m6/fix-finish-shell-merge-120x40.txt, m6/fix-landing-after-continue-rebase-120x40.txt, m6/fix-landing-after-continue-rebase-187x50.txt, m6/fix-landing-after-esc-120x40.txt, m6/fix-landing-after-esc-187x50.txt, m6/fix-landing-b-launch-120x40.txt, m6/fix-landing-b-launch-187x50.txt, m6/fix-plain-box-after-rebase-187x50.txt, m6/fix-preview-verb-120x40.txt, m6/fix-preview-verb-name-prompt-120x40.txt, m6/fix-radar-bookkeeping-120x40.txt, m6/fix-shell-merge-abort-and-undo-120x40.txt, m6/fix-status-under-a-box-120x40.txt, m6/fix-status-under-a-box-80x24.txt, m6/fix-subject-enter-commits-120x40.txt, m6/fix-subject-enter-commits-80x24.txt, m6/fix-tab-words-120x40.txt, m6/fix-undo-drift-way-back-120x40.txt, m6/fix-undo-drift-way-back-80x24.txt, m6/fix-undo-shell-merge-120x40.txt, m6/fix-undo-shell-merge-80x24.txt, m6/menu-fixed-slots-120x40.txt, m6/set-aside-clash-preview-120x40.txt, m6/set-aside-done-120x40.txt, m6/set-aside-preview-120x40.txt, m6/set-aside-preview-80x24.txt, m6/stash-clash-done-120x40.txt, m6/stash-clash-stop-line-80x24.txt, m6/stash-clash-take-esc-pane-120x40.txt, m7/30b-staging-not-kept-187x50.txt, m7/30b-staging-not-kept-80x24.txt, m7/30c-relaunch-edits-back-120x40.txt, m7/30c-relaunch-edits-back-80x24.txt, m7/absorb-menu-120x40.txt, m7/absorb-preview-120x40.txt, m7/amend-into-landed-120x40.txt, m7/amend-into-left-behind-187x44.txt, m7/amend-into-preview-187x50.txt, m7/blame-enter-partly-staged-120x40.txt, m7/blame-then-commit-120x40.txt, m7/blame-then-commit-with-base-120x40.txt, m7/compare-since-split-187x50.txt, m7/diff-cursor-commit-120x40.txt, m7/diff-pane-b-with-offer-187x44.txt, m7/esc-back-to-blame-from-the-graph-120x40.txt, m7/esc-back-to-history-from-the-graph-120x40.txt, m7/force-box-leaves-120x40.txt, m7/in-the-way-at-80-80x24.txt, m7/landed-after-merge-120x40.txt, m7/menu-slot-two-dim-120x40.txt, m7/new-branch-with-edits-preview-120x40.txt, m7/pane-bar-file-vs-directory-120x40.txt, m7/picker-empty-query-187x50.txt, m7/picker-filtered-100x30.txt, m7/picker-remote-landed-120x40.txt, m7/pull-over-edits-120x40.txt, m7/put-back-edit-landed-187x50.txt, m7/put-back-landed-187x50.txt, m7/put-back-staged-landed-187x50.txt, m7/radar-not-checked-120x40.txt, m7/radar-not-checked-evil-120x40.txt, m7/rebase-line-preview-187x50.txt, m7/rebase-preview-target-top-187x44.txt, m7/rebase-refused-fork-120x40.txt, m7/rebase-squash-merged-preview-120x40.txt, m7/rebase-stop-forecast-80x24.txt, m7/refused-box-single-close-120x40.txt, m7/rewrite-edit-stop-120x40.txt, m7/rewrite-edit-stop-187x50.txt, m7/rewrite-edit-stop-80x24.txt, m7/rewrite-refused-fork-120x40.txt, m7/stash-in-history-120x40.txt, m7/stash-in-history-80x24.txt, m7/submodule-diff-120x40.txt, m7/submodule-row-120x40.txt, m7/submodule-row-187x40.txt, m7/switch-landed-120x40.txt, m7/switch-menu-clean-landed-120x40.txt, m7/take-preview-120x40.txt, m7/take-refused-merge-120x40.txt, m8-banner-bar-q-187x50.txt, m8-banner-bar-q-80x24.txt, m8-landing-undo-120x36.txt, m8-look-chrome-120x36.txt, m8-look-chrome-187x50.txt, m8-look-chrome-80x24.txt, m8-quit-waits-80x24.txt, m8-quit-while-running-80x24.txt, m8/settle-following.txt
- New: `docs/shots/m8-look-cursor-row-dark-120x36.txt`, `docs/shots/m8-look-cursor-row-light-120x36.txt`.
- Gates (workspace): fmt ok; clippy `-D warnings` ok; nextest 5348 passed / 9 skipped; doctests ok.

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

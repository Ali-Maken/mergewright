# finish-summary-is-a-preview: the finish summary is framed as the merge preview; `y` commits, `⏎` shows a file's change; arrows, pages and `⇧⇥` work

Status: done
From: P12 (approved 2026-09-24, picks item 3) and batch 8 (picks item 4); field notes :60, :63; ADR-0044 §3.
Build with: builder-opus. The conductor set this after the scout: two crates, a layout change, new border styling and
many test moves. The shape is decided here, so nothing in it needs Fable.
Scout: `.scratch/m6-conflicts/scouts/59.md` (main @ 63effe89). Line numbers below are the scout's, at that hash.
Needs: wave 1 merged (both lanes). Ticket 52 and `last-file-opens-the-summary` come after this.
Crates: graft-ui, graft.
Lane: wave 2, first.

## Spec-pass reconciliation (2026-09-25, against the scout)
1. **`FINISH_MIN` (60, 14) → (62, 18) is a change**, not a restatement. The design note's first spec (:609, :758) set
   (60, 14). The amendment (:1115-1116) grows it by the frame: 2 columns and 4 rows (2 border rows, 2 preview lines).
2. **Names before the read lands:** `FinishView` gets `names: Option<(from, into)>` in place of `title`. It is `None`
   while reading, or when the names are not known (today's `TITLE_UNREAD`, view/finish.rs:138). It is `Some` exactly
   where view/finish.rs:77 formats `{from} → {into}` today, from the same values. While it is `None`, the top border
   reads `Preview: finish the merge` and the first preview line reads `1 merge commit · nothing is committed until y`.
   No invented names, and no `…` standing in for a name. The other states are unchanged.
3. **`settled` gates no key.** `prelude.rs::settled` (:34-96) waits on `finish.job`/`diff_job` (:91-94). It does not
   hold `⏎` or `y`. "Still reading" comes from `commit` (update/finish :200-249, `Pending` → `still reading · wait`).
   So `y` gets that for free when the `⏎` arm's body moves to `y`. The only `settled` edit is its comment (:88-90),
   which names `⏎` and now names `y`. It gets no behaviour change.
4. **The styling is new code.** `frame_box` is a module of helpers (`border_row` :38, `content_row` :79, `cells` :19),
   and `preview_box` draws its title in `Text` and its footer as one `muted` string (preview_box.rs:262-271, :305-310).
   Nothing today draws a border in `accent` or one key in the border differently. See Signatures.

## Read first
- This ticket's reconciliation above, and the scout file.
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24: "The finish summary is a preview" (:1070-1117), "Where
  each event lands him" → "In the summary" (:929-934), J6 (:904), Touches (:1127-1169).
- `docs/design/02-keymap.md` §2.6b (edited 2026-09-24).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §1 (tokens, the `NO_COLOR` row) and §3.
- `crates/graft-ui/MAP.md` :314-315 (`finish_summary`, `HintContext::Finish`), :32-33 (`frame_box`, `preview_box`).
- `crates/graft/MAP.md` §update::finish (:350-365).

## Scope
**In: graft-ui**
- **Frame:** the summary draws inside a rounded frame, built from `frame_box` helpers, under the header and filling the
  frame.
  - The border is `accent` because the frame has focus.
  - The old title row (finish_summary.rs:179-180, `Finish · merging … · nothing is written until ⏎`) goes.
- **Title** in the top border, from `preview_title`, in `accent` BOLD:
  - Names known: `Preview: finish merging <from> into <into>`.
  - If that does not fit: `Preview: merge <from> → <into>`.
  - If that still does not fit: cut with `…`.
  - Names unknown: `Preview: finish the merge`, cut with `…` if it must be.
- **The two preview lines** come first inside the frame:
  - `1 merge commit on <into> · nothing is committed until y`. While the names are unknown:
    `1 merge commit · nothing is committed until y`.
  - `undo: u takes this back`.
- **Keys in the bottom border.** They come from `HintContext::Finish { focus, expandable }`: `finish_list`
  (lists.rs:360-388), one list drawn in one place. The app's bar stops drawing it.
  - Files: `y commit  ⏎ diff  j/k move  ⇥ message  esc back`. Draw `⏎/space diff` if the bound-action test needs
    `space` named.
  - Subject: `⏎ commit  ⇥ body  ⌃E editor  esc back`.
  - Body: `⌃⏎ commit  ⇥ files  ⌃E editor  esc back`.
  - The first key (`y` in Files) is `accent` BOLD. The other words are `text-muted`, as `preview_box`'s footer is.
  - If `FinishView` does not already carry `focus` and `expandable`, add them, filled where view/finish.rs builds the
    view.
- **Colour** (tokens: ADR-0044 §1 and PROMPT §5's list, nothing new):
  - `yours · <branch>` in `ours`; `theirs · <branch>` in `theirs`.
  - `+n` in `added`; `−n` in `removed`.
  - Section heads in `text-muted` BOLD.
  - `NO_COLOR`: the border is plain `border` glyphs. The title and the `y` key stay BOLD. No hue carries meaning alone.
  - Colour ships **unverified** (rule 5).
- **`FINISH_MIN` = (62, 18)** (a change; see reconciliation 1). Below it, the too-small line (:165-175) reads
  `finish needs 62x18 · esc back`.
- **`layout` and `hit`:** `layout` (layout.rs:148-195) places the frame, the two preview lines, then today's
  list/message/subject/body/notice inside the frame's inner rect. The notice stays on the last inner row.
  `hit` (:206-225) follows from `layout`, so a click lands on the row drawn.
- **Unavailable wording** (finish_summary.rs:291, doc :90): `summary unavailable: <why> · y commits with git's message`.

**In: graft**
- **Files focus** (`files_key` :83-97):
  - `y` commits, with today's `Enter` body (:90) moved.
  - `Enter` and `Space` show/hide the cursor file's diff (`toggle_diff` :175-197).
  - `Down`/`Up` do what `j`/`k` do.
  - `PageDown`/`PageUp` move the cursor by the list's visible height (new helper; it ends with
    `keep_cursor_on_screen` finish.rs:179).
- **`BackTab`:** `finish_key` (:43-47) sends `Shift+Tab`/`BackTab` to `FinishFocus::prev` (new), replacing the "does
  nothing" arm. Its comment :43-44 is rewritten.
- **Subject:** `⏎` still commits; `y` is typed text. **Body:** `⏎` is a newline; `y` is typed text.
- **Words:** `y` while the summary is loading → `still reading · wait`, from `commit`'s `Pending` arm, unchanged.
  The `settled` comment (:88-90) says `y`.
- **The app's bottom row** under the summary (view/frame/conflicts.rs:152-176, `finish_bar` view/finish.rs:219) shows
  `committing…` while committing and nothing otherwise. The keys live in the frame.
  - `summary_area` (view/finish.rs:164) keeps its rect, so the wheel's `max_top` (mouse.rs:33) is unchanged in shape.
  - The frame's two border rows and the preview lines come out of the list's visible height, through `layout`.
- **The view builder** (view/finish.rs:77, :138): it fills `names` instead of the title strings, and `TITLE_UNREAD`
  goes.

**Out:**
- Opening the summary by itself (`last-file-opens-the-summary`).
- `c` from the pane (ticket 52). The confirm box's `y` (ticket 50).
- `FinishSummary` (ops): unchanged. The `settled` logic: unchanged (comment only).
- `preview_box` (the Ghost Preview): unchanged. The new border helper lives in `frame_box`, and `preview_box` is not
  moved onto it.

## The failing test first
graft `update/finish` tests: `y_in_the_file_list_commits_and_enter_toggles_the_diff`.
- Setup: a loaded summary, focus Files, cursor on `README.md`.
- Press `⏎`: `expanded` is `Some(cursor)`, and no command is emitted.
- Press `y`: the same `Command::Resume(ContinueWith|Continue)` that today's `⏎` emits from that state.
- Red today: `⏎` commits.

## Signatures
```rust
// graft-ui/src/finish_summary.rs
pub const FINISH_MIN: (u16, u16) = (62, 18);
pub struct FinishView {
    // existing fields …, minus `title: SanitizedText`
    /// (from, into). `None` while the read has not landed, or when the names are not known.
    pub names: Option<(SanitizedText, SanitizedText)>,
    // pub focus / pub expandable: added only if not already carried (for the footer's HintContext::Finish)
}
/// The top border's words at `width`. With names: the full title, else the short one, else the short one cut with `…`.
/// Without names: `Preview: finish the merge`, cut with `…` if it must be.
#[must_use] pub fn preview_title(names: Option<(&str, &str)>, width: u16) -> String;
// layout / hit: signatures unchanged. FinishLayout drops `title` and gains `frame: Rect` and `preview: Rect`.

// graft-ui/src/frame_box.rs: new, beside `border_row` (:38). It takes border_row's parameters and returns its type,
// but the words are styled spans and the line glyphs take `line`'s style.
pub(crate) fn border_row_spans(left: char, right: char, words: &[(&str, Style)], line: Style, width: u16)
    -> /* border_row's return type */;

// graft/src/update/finish.rs
impl FinishFocus { #[must_use] pub fn prev(self) -> Self; }   // inverse of `next` (:66)
// finish_key: signature unchanged. The BackTab arm calls `prev`. files_key gains y / Enter / Up / Down / PageUp /
// PageDown arms.
```

## Touches (each entry, and how the builder checks it)
- **Unit tests that commit with plain `Enter` from Files:** update/finish/tests.rs:204, 215, 230, 242, 267
  (`enter_commits_with_gits_message_when_unedited`, `enter_while_reading_says_wait` → `y_while_reading_says_wait`, …).
  - Check: each sends `y` and is renamed where the name says `enter`. NOTES lists old → new.
- **Integration tests:**
  - tests/it/finish.rs:118, 306, 334.
  - finish/choices.rs:100, 203, 282, 305.
  - finish/captures.rs:17.
  - resolver/shell_begun.rs `c_enter_finishes_a_shell_begun_merge` :645 → `c_y_finishes_a_shell_begun_merge`.
  - resolver/shell_begun.rs `c_enter_after_u_finishes_the_merge_again` :809 → `c_y_after_u_finishes_the_merge_again`.
  - Check: each sends `y`. Re-run `grep -rn "Enter" crates/graft/tests/it/finish* crates/graft/tests/it/resolver/shell_begun.rs`
    and account for every remaining hit in NOTES (commit, or something else).
- **Walk:**
  - The `c_enter()` helper (conflict_journeys.rs:176-186, used at :746, 765, 777, 1105, 1121, 1140, 1192) sends `y`
    and is renamed `c_y()`.
  - `a-finish`'s inline `Enter` (:699) → `y`.
  - **Not** `walk-back-a-whole-file-choice`'s `Enter` (:1060): that is the confirm box, ticket 50's.
  - Check: `journey_counts_match_the_note`, `every_scenario::journey`. Counts are unchanged.
- **`pty-walk-m6.py`:** the scout found no finish commit in it (grep `finish`/`Enter`/`'c'`).
  - Check: grep it again. If it sends no finish commit, leave it and say so in NOTES. Don't edit on the assumption.
- **Title asserts:** finish_summary/tests.rs:31, 103; update/finish/tests.rs:196; tests/it/finish/captures.rs:37.
  - Check: each asserts the new border title (or `names`), including one unread case (`Preview: finish the merge`).
- **Footer assert:** tests/it/finish/captures.rs:60, `starts_with("⏎ commit")`.
  - Check: it reads the bottom border: `╰ y commit  ⏎ diff`.
- **Hints:** `every_key_the_bar_draws_is_answered`, `every_context` coverage slots 32-35, and
  `every_bound_action_is_hinted_live_or_is_named_here`.
  - `space`, `PgUp`, `PgDn`, `BackTab` and the arrows are either named or on `NEVER_HINTED`'s navigation class. §2.1
    says arrows and pages are never hinted.
  - Check: run them. No edit to `NEVER_HINTED` beyond keymap §2.1's existing class.
- **Mouse:**
  - `hit_round_trips_inside_the_frame` at 80x24, 120x40 and 187x50 (every drawn row's click selects it).
  - The wheel over `summary_area` (mouse.rs:33) still clamps at the list's end.
- **The prelude:** `settled`'s comment (:88-90) only.
  - Check: `git diff` of prelude.rs is comment lines only; `y_while_reading_says_wait` is green.
- **App views:**
  - view/finish.rs: the title at :77 and :138 becomes `names`; `finish_bar` at :219 is `committing…` only.
  - view/frame/conflicts.rs:152-176: the bar row.
  - Check: `summary_draws_inside_the_preview_frame`, and one app-level test that the bottom row carries no keys while
    idle.
- **Strings:**
  - `grep -rn "⏎ commit" crates/graft crates/graft-ui` returns only the Subject bar and the commit editor.
  - `grep -rn "⏎ commits" crates` returns nothing (finish_summary.rs:90, 291 now say `y`).
  - graft-ui MAP.md:320 is updated.
- **Captures:**
  - Regenerate every `docs/shots` file that shows the summary: m6-15, m6-16 and m6-17 at five sizes,
    `fix-finish-summary-*`, `fix-finish-from-the-resolver-*`, `fix-finish-shell-merge-*`, `fix-every-file-done-footer-*`.
  - Also the contact sheet: `m6-contact-sheet-1.txt`, `m6-contact-sheet-2.txt`, and `m6-contact-sheet-captions.tsv`,
    whose captions that say `⏎ commit` for the summary now say `y`.
  - `prototype-journeys.txt` is a prototype record. Leave it, and say so in NOTES.
  - Check: `grep -rln "⏎ commit" docs/shots` afterwards. NOTES lists each remaining hit and why.
  - The walk's own captures are re-keyed by the acceptance ticket.
- **Other screens:** the Ghost Preview's `y` and the commit editor are unchanged (ticket 51 is its own).
  - Check: `git diff --stat` shows no `preview_box` or `commit_editor` file. `frame_box.rs` gains a function and
    changes none.
- **Journal:** nothing new; still one `FinishMerge` / `ContinueWith`. Check: the existing finish journal tests are
  green.
- **Docs:**
  - `crates/graft-ui/MAP.md` :314-315, :320 and :32-33 (the new `border_row_spans`).
  - `crates/graft/MAP.md` §update::finish (the keys for `finish_key` / `files_key`, and `FinishFocus::prev`).

## Acceptance (name the verifying test at tick time)
- [x] `y_in_the_file_list_commits_and_enter_toggles_the_diff` (graft `update::finish::tests`): seen red before the keys changed, then green.
- [x] `y_in_the_subject_is_typed_text`, `enter_in_the_subject_still_commits` (graft `update::finish::tests`)
- [x] `arrows_and_pages_move_the_file_cursor`, `backtab_steps_focus_back` (graft `update::finish::tests`)
- [x] `preview_title_shortens_then_cuts` (graft-ui `finish_summary::tests`): full, short and cut at three widths, plus `None` → `Preview: finish the
  merge`.
- [x] `summary_draws_inside_the_preview_frame` (grid) and `preview_frame_styles` (styles; split for clippy's complexity limit), graft-ui `finish_summary::tests`. Buffer: corners, title, the two preview lines, the footer keys. Style:
  the border cells are `accent`; the footer's `y` is `accent` + BOLD.
- [x] `unread_summary_has_no_names` (graft-ui `finish_summary::tests`; graft `update::finish::tests` checks `names` is `None`): the title is `Preview: finish the merge`, and line 1 is
  `1 merge commit · nothing is committed until y`.
- [x] `hit_round_trips_inside_the_frame` (graft-ui, at 80x24, 120x40, 187x50)
- [x] `y_while_reading_says_wait` (graft), `unavailable_summary_names_y` (graft-ui)
- [x] `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here` (graft `update::tests`, green)
- [x] Capture, by `finish::captures::summary_captures` -> `preview_captures`, which asserts each line below (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-summary-preview-187x50.txt` and `-80x24`.
  - **What right looks like:**
    - The top border reads `╭ Preview: finish merging feature into main ─`.
    - The first two inside lines are `1 merge commit on main · nothing is committed until y` and
      `undo: u takes this back`.
    - The bottom border starts `╰ y commit  ⏎/space diff` (deviation: see NOTES).
    - The app's last row has no keys.
    - At 80: the same order in one column, and nothing is cut but the title, if it must be.
- [x] MAP.md files updated (doc, no test): graft-ui `frame_box`, `finish_summary`, `HintContext::Finish`; graft §update::finish, `view::finish`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium-large: two crates, a layout change, a new border helper, and about 25 test and walk edits. One context; the
capture regeneration is mechanical. If the context runs short, stop after graft-ui is green and hand off (`handoff`).
The graft half is separable at `FinishView.names` / `FINISH_MIN`.

## NOTES
Built 2026-09-25 from scout `.scratch/m6-conflicts/scouts/59.md`. **Compacted once mid-ticket** (after captures, before MAP.md and gates); nothing remains.

- **Deviation:** the footer reads `⏎/space diff`, not `⏎ diff`. `space` stays bound, and `every_bound_action_is_hinted_live_or_is_named_here` needs every bound key named; `("⏎/space","diff")` in `bound.rs`, `presses_for` knows `⏎/space`.
- `border_row_spans` returns `Line<'static>`, not `border_row`'s `String` (it carries styles).
- Added `FinishView.focused` (= no live box, `view::rows::box_context`): under a confirm box the frame is `border` and shows no keys. Found by `the_bottom_row_under_a_live_box_offers_only_its_keys`.
- Wording: section head `· ⏎ shows the change`, more row `· ⏎ hides`; unavailable `summary unavailable: … · y commits with git's message`. Inside width is w-4. `FINISH_MIN` 62x18.
- `finish_bar(state)` lost its width: `committing…` or empty. `finish_hints` is `#[cfg(test)]`.
- Renames: `enter_commits_with_gits_message_when_unedited` -> `y_commits_…`, `enter_while_reading_says_wait` -> `y_while_reading_says_wait`, `enter_to_the_landing` -> `y_to_the_landing`, `c_enter_finishes_a_shell_begun_merge` -> `c_y_…`, `c_enter_after_u_finishes_the_merge_again` -> `c_y_after_u_…`, walk helper `c_enter()` -> `c_y()`.
- Enters left on purpose: subject commits (`finish.rs` 118/306, `shell_begun.rs` 931), confirm boxes (`choices.rs` 203/305, `shell_begun.rs` 339/468/982, walk-back 1060), resolver `⏎` (`captures.rs` 17, `choices.rs` 100). The scout listed captures.rs:17 and choices.rs:100/203/305 as finish commits; they are not.
- `pty-walk-m6.py` sends no finish key itself (grep); its keys come from `conflict_journeys.rs` `captured()` (m6-17 now `c y`; m6-16 still `space`, which still toggles).
- Retaken: `fix-finish-summary-*` (4), m6-15/16/17 at 5 sizes; new `fix-summary-preview-{187x50,80x24}`. Sheet now 3 parts (179 951 / 170 673 / 85 895 B). m6-15/16 at 187/200 are ~10 KB, over the pty script's 8 KB soft note (full-width borders), far under 200 KB.
- `docs/shots/m6/prototype-journeys.txt` still says `⏎ commit`: a prototype record, left alone.
- Colour **unverified** (rule 5): cell-style tests assert tokens only.
- Journey counts unchanged (`⏎` -> `y` is one key for one key); no row added.

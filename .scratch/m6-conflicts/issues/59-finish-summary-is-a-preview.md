# finish-summary-is-a-preview: the finish summary is framed as the merge preview; `y` commits, `⏎` shows a file's change; arrows, pages and `⇧⇥` work

Status: ready
From: P12 (approved 2026-09-24, picks item 3) and batch 8 (picks item 4); field notes :60, :63; ADR-0044 §3.
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: (1) graft-ui `finish_summary`: `FinishScreen`, `FinishLayout`, `layout`, `hit`, `fits`, `FINISH_MIN`,
the title row and the `notice`; how `preview_box` draws its frame and footer through `frame_box`; `HintContext::Finish`
and its lists. (2) graft `update::finish`: `finish_key` per focus (`keys` child), `commit`, `toggle_diff`, the scroll
helpers, `finish_mouse`; where the app draws the bottom row under the summary; `update/prelude.rs::settled` (which keys
it holds while reads land). (3) Every test, walk key and capture that sends or shows `⏎` as the summary's commit:
`grep -rn "⏎ commit" crates`, `grep -rln "⏎ commit" docs/shots`, the walk's `a-finish` / `b-finish` scenarios.
Needs: wave 1 merged (both lanes). Ticket 52 and `last-file-opens-the-summary` come after this.
Crates: graft-ui, graft.
Lane: wave 2, first.

## Read first
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24: "The finish summary is a preview" (:1050-1097), "Where each
  event lands him" → "In the summary" (:929-934), J6 (:904).
- `docs/design/02-keymap.md` §2.6b (edited 2026-09-24).
- `docs/adr/0044-conflict-colours-and-the-confirm-key.md` §3.
- `crates/graft-ui/MAP.md` :314-315 (`finish_summary`, `HintContext::Finish`), :32-33 (`frame_box`, `preview_box`).
- `crates/graft/MAP.md` §update::finish (:350-365).

## Scope
**In — graft-ui:**
- **Frame:** the summary draws inside a rounded `frame_box` (`accent` border, it has focus), under the header, filling
  the frame. The old title row `Finish · merging … · nothing is written until ⏎` goes.
- **Title** in the top border: `Preview: finish merging <from> into <into>`; if it does not fit,
  `Preview: merge <from> → <into>`; then cut with `…`. `accent` BOLD.
- **Two preview lines** first inside: `1 merge commit on <into> · nothing is committed until y` and
  `undo: u takes this back`.
- **Keys in the bottom border**, from `HintContext::Finish { focus, expandable }` (one list, drawn in one place): Files
  `y commit  ⏎ diff  j/k move  ⇥ message  esc back` (`⏎/space diff` if the bound-action test needs `space` named);
  Subject `⏎ commit  ⇥ body  ⌃E editor  esc back`; Body `⌃⏎ commit  ⇥ files  ⌃E editor  esc back`. `y commit` leads,
  key in `accent` BOLD.
- **Colour:** `yours · <branch>` in `ours`, `theirs · <branch>` in `theirs` (the tokens from `conflict-colour-tokens`);
  `+n` `added`, `−n` `removed`; section heads `text-muted` BOLD.
- **`FINISH_MIN`** (62, 18); below it `finish needs 62x18 · esc back`. **`layout` and `hit`** account for the frame (1, 1)
  and the two preview lines, so a click lands on the row drawn.

**In — graft:**
- **Files focus:** `y` commits (today's `⏎` body, moved); `⏎` and `space` show/hide the cursor file's diff; `↓`/`↑` = `j`/`k`;
  `PgDn`/`PgUp` page the list by its visible height; `⇧⇥` (`BackTab`) steps focus back (`FinishFocus::prev`).
- **Subject:** `⏎` commits (unchanged); `y` is typed text. **Body:** `⏎` newline, `y` typed.
- **Words:** `summary unavailable: <why> · y commits with git's message`; `y` while the summary is loading →
  `still reading · wait`. `update/prelude.rs::settled` holds `y` as it held `⏎`.
- **The app's bottom row** under the summary shows the status line only (the keys are in the frame).

**Out:**
- Opening the summary by itself (`last-file-opens-the-summary`).
- `c` from the pane (ticket 52). The confirm box's `y` (ticket 50).
- `FinishSummary` (ops) — unchanged.

## The failing test first
graft `update/finish` tests: `y_in_the_file_list_commits_and_enter_toggles_the_diff`.
- Setup: a loaded summary, focus Files, cursor on `README.md`. Press `⏎` → `expanded` is `Some(cursor)`, no command
  emitted. Press `y` → the same `Command::Resume(ContinueWith|Continue)` today's `⏎` emits from that state.
- Red today: `⏎` commits.

## Signatures
```rust
// graft-ui/src/finish_summary.rs
pub const FINISH_MIN: (u16, u16) = (62, 18);
pub struct FinishView { /* existing …, */ pub from: SanitizedText, pub into: SanitizedText }  // only if not already carried
/// The top border's words at `width`: the full title, the short one, or the short one cut with `…`.
#[must_use] pub fn preview_title(from: &str, into: &str, width: u16) -> String;
// layout/hit: signatures unchanged; FinishLayout gains `frame: Rect` and `preview: Rect`.
// graft/src/update/finish.rs
impl FinishFocus { #[must_use] pub fn prev(self) -> Self; }
// finish_key: Files gains y / Enter / Up / Down / PageUp / PageDown / BackTab arms; signature unchanged.
```

## Touches (each entry, and how the builder checks it)
- **Every test that commits with `⏎` from Files** (e.g. `c_enter_finishes_a_shell_begun_merge`).
  - Check: the scout's grep; each moves to `y` and is renamed where the name says `enter`
    (`c_y_finishes_a_shell_begun_merge`); NOTES lists them.
- **Hints:** `every_key_the_bar_draws_is_answered`, `every_context` coverage slots 32-35,
  `every_bound_action_is_hinted_live_or_is_named_here` (`space`, `PgUp`, `PgDn`, `BackTab`, arrows: named or on
  `NEVER_HINTED`'s navigation class — §2.1 says arrows and pages are never hinted).
  - Check: run them; no edit to `NEVER_HINTED` beyond keymap §2.1's existing class.
- **Mouse:** Check: `hit_round_trips_inside_the_frame` at 80x24, 120x40, 187x50 (every drawn row's click selects it).
- **The prelude:** Check: `y_while_reads_land_is_held` (as `⏎` was).
- **Strings:** Check: `grep -rn "⏎ commit" crates/graft crates/graft-ui` returns only the Subject bar and the commit
  editor.
- **The walk** (`m5_walk`, `pty-walk-m6.py`): `a-finish`, `b-finish`, `b-finish-again-after-undo`, `editor-to-finish`,
  the `a-whole-*` and `b-whole-two-files` rows send `y` for the commit. Counts unchanged here.
  - Check: `journey_counts_match_the_note`; `every_scenario::journey`.
- **Captures:** m6-15, m6-16, m6-17 at five sizes, `fix-finish-summary-*`, `fix-finish-from-the-resolver-*`,
  `fix-finish-shell-merge-*`, `fix-every-file-done-footer-*` (note :1165-1166). Regenerate; NOTES lists each.
  The walk's own captures are re-keyed by the acceptance ticket.
- **Other screens:** the Ghost Preview's `y` unchanged; the commit editor unchanged (ticket 51 is its own).
  Check: `git diff --stat` has no `preview_box` or `commit_editor` file.
- **Journal:** none new; still one `FinishMerge` / `ContinueWith`. Check: the existing finish journal tests green.
- **Docs:** `crates/graft-ui/MAP.md` :314-315; `crates/graft/MAP.md` §update::finish (`finish_key` keys).

## Acceptance (name the verifying test at tick time)
- [ ] `y_in_the_file_list_commits_and_enter_toggles_the_diff`: red first, then green.
- [ ] `y_in_the_subject_is_typed_text`, `enter_in_the_subject_still_commits`
- [ ] `arrows_and_pages_move_the_file_cursor`, `backtab_steps_focus_back`
- [ ] `preview_title_shortens_then_cuts` (full, short, cut at three widths)
- [ ] `summary_draws_inside_the_preview_frame` (buffer: corners, title, the two preview lines, the footer keys)
- [ ] `hit_round_trips_inside_the_frame`
- [ ] `y_while_reads_land_is_held`, `unavailable_summary_names_y`
- [ ] `every_key_the_bar_draws_is_answered`, `every_bound_action_is_hinted_live_or_is_named_here` (green)
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-summary-preview-187x50.txt` and `-80x24`.
  - **What right looks like:** the top border reads `╭ Preview: finish merging feature into main ─`; the first two
    inside lines are `1 merge commit on main · nothing is committed until y` and `undo: u takes this back`; the bottom
    border starts `╰ y commit  ⏎ diff`; at 80 the same order in one column, nothing cut but the title if it must be.
- [ ] MAP.md updated (doc, no test).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium-large (two crates, a layout change, many test renames). One context; the capture regeneration is mechanical.

## NOTES

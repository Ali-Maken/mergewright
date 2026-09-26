# journal-screen — `J` shows the shell's steps and the lost commits as rows; `⏎` puts one back; he lands on it

Status: ready
Blocked by: 22-journal-list, 23-recover-op, 03-land-on-result, 09-amend-into-and-absorb-keys
Blocked by: journal-list, recover-op, 03-land-on-result, 09-amend-into-and-absorb-keys (the row-tag mechanism)
Crates: graft, graft-ui.
Source: `docs/design-notes/m7-power-tools.md`: pass 1b "Journal"; "Pass 2b" F1, F4, F5, J1, "Where he lands" (2b),
keymap diff (2b) §2.11, "Journey counts (2b)", "Where I'd differ (2b)" 9–10; field notes :72, :78; ADR-0044 §1–2,
ADR-0046.
Build with: **Opus**. It is a screen over a decided list and operation, judged by walks, snapshots and captures.
Scout: `scout` first (ADR-0035, brownfield: the `J` browser, its revert preview, the name prompt, and row tags).
`Build with` is set by the conductor after the scout.
Needs:
- `journal_list::{read_outside, find_lost, build, recover_for, JournalList, ListRow, LostScan, Recover}`
  (journal-list);
- `Operation::Recover`, `FilesEffect`, `lost_edits` (recover-op);
- `land_on_result(state, kind, new_head)` (03);
- the row-tag mechanism behind `✎ changed` (09).

## Background (from KNOWN_GAPS; builders don't read it)
Rows this ticket closes:
- **M3 :217:** no status line while `J` is open, so a refusal inside `J` has nowhere to show.
- **M6/68:** an entry aborted by the user reads `failed: aborted`. It should read `aborted by you · nothing changed`.
- **M6 usability (:848):** a row cut to fit runs words together (`the woundone`). Cut at a word boundary, with `…`.
- **M6 usability (:847):** a stash drop has two names on two screens. Use one, `drop the set-aside copy (stash@{0})`,
  in both places.

## Decided (the note; not open)
- **Stages:** `J` opens at once with the `●` rows (stage 1), as today.
  - A job reads the outside rows (stage 2), then another runs the lost scan (stage 3).
  - Each lands by calling `build` again. The cursor stays on the same row across stages (identity, not index), unless
    the preselect below applies.
  - Title suffixes (J1): `· finding lost commits…` / `· looked at the newest 500 steps` / `· couldn't look for lost
    commits: <why>`. Nothing once done.
- **Preselect (F1):** when stage 3 lands, **and no key has been pressed since `J`**, the cursor moves to the newest `✗`
  row whose event is newer than every `●` row. Otherwise it stays.
- **Rows:** pass 1b's mock-up.
  - Columns: glyph `●`/`$`, the source word `Graft`/`outside Graft`, time, words, and `git: "<subject>"` on `$` rows.
  - `✗` rows are indented `└ ✗ lost`, with the sha7, the quoted subject, and `was <b>'s tip · no branch holds it` /
    `· <b> is gone` / `was HEAD`, plus `+N under it`.
  - Lost-edit rows: `└ ✗ lost  your edit to <path> · replaced by this undo · kept in its backup`, with no hash.
  - The legend line: `● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch
    holds`.
  - The muted last line: `a branch deleted outside Graft and never checked out leaves no trace here`.
- **Breakpoints (J1):**

  | Width | Columns |
  |---|---|
  | ≥180 | glyph · source · time · words · `git: "…"` |
  | 120–179 | no `git:` column; the selected `$` row shows it on a line under it |
  | <120 | no source word (the legend carries it) |

  At 24 rows the title, legend and bar stay, and the rows scroll.
- **The command log** of an expanded `●` row: `┆ git …` lines under `commands Graft ran` (was `$ git …`).
- **Keys (§2.11, keymap diff 2b):**

  | Key | Does |
  |---|---|
  | `j`/`k`, `↑`/`↓` | move |
  | `⏎` on `●` | revert-to, unchanged (Ghost Preview, `→ ⏎`, ADR-0044 §2) |
  | `⏎` on `✗` / `$` / lost edit | `recover_for` → `Run(params)` → the Recover Ghost Preview; `y` confirms (ADR-0044 §1). `OfferBranch` → no preview; J's status row shows `why · b makes <name> at <sha7>` |
  | `⏎` on a `$` switch row | the app's switch back (the M7 switch rule) |
  | `b` | on any row: the name prompt at the row's commit, prefilled `<b>-restored` when `recover_for` offered one, else empty |
  | `/`, `n`/`N` | find; next/previous match. With no find active, next/previous `✗` row |
  | `Space` / expand | as today (the command log) |
  | `Esc` | close |

  The bar is per row: `⏎ put main back to it` / `⏎ make spike again` / `⏎ put your edit back` / `⏎ revert to before
  this`.
- **The preview box** is the note's J1 box, exactly: `now` / `after y` / `your files:` / `u undoes it`, and `y put it
  back  esc cancel`.
- **Landings** (via `land_on_result` for `OperationKind::Recover`):
  - **MoveRef:** the graph, cursor on the restored commit, detail pane on it. That row is tagged `↺ put back`, and the
    old tip `○ main was here`. Status `✓ put back · main has "<subject>" again · u undoes`.
  - **Recreate:** cursor on the branch tip, tagged `↺ put back`. Status `✓ spike is back at "<subject>" · u undoes`.
  - **Files:** the graph with the changes pane on the file. Status `✓ your edit to src/a.rs is back · u undoes`.
  - **`b`:** cursor on the new branch's tip. Status `✓ made main-restored at "<subject>" · u undoes`.
  - The tags clear on the next operation, as `✎ changed` does.
  - `J` again: the put-back is a `●` row (`put main back (the shell's reset)`), and its `✗` row is gone.
- **`u`** undoes the put-back (it's a journal entry). `u` never acts on a `$` or `✗` row; nothing changes there.
- **Status row in `J`:** one line under the rows, for refusals and the moved-on words. It is shown only when there is
  something to say (status rule).

## Read first
- The note: pass 1b "Journal"; "Pass 2b" F1, F5, J1, keymap diff (2b) §2.11, "Journey counts (2b)", "Where he lands"
  (2b).
- `docs/design/02-keymap.md` §2.11, §2.3b (`y`); `docs/design/01-view-mockups.md` §1.10.
- `docs/research/ux-principles.md` §3.1, §3.6, checklist 3, 16–17.
- journal-list's and recover-op's NOTES; 03's and 09's NOTES (land_on_result, the row tags).
- `crates/graft/MAP.md`: the journal browser (`update/journal.rs`, `view/journal.rs`), `PreviewSubject`, the name
  prompt. `crates/graft-ui/MAP.md`: `journal_screen` (`RowMark`, `JournalRowView`, `JournalStatus`, `JournalView`).

## Scout brief (≤ 1 page; `.scratch/m7-power-tools/scouts/journal-screen.md`)
1. The `J` open/read/draw path: which job reads the journal, where rows are built, and where the cursor is kept (index
   or id). Lines.
2. How `⏎` on a row opens the revert preview (`PreviewSubject::RevertTo`), and how `→ ⏎` confirms it. Where a
   `PreviewSubject::Operation` preview with `y` could open from inside `J`.
3. How the name prompt is opened with a prefill from an overlay, and where it returns to.
4. The row-tag mechanism from 09 (`✎ changed`): where tags are set, drawn and cleared.
5. Where the command log is drawn (the `$ git` prefix), with the line.
6. Every J capture and walk (m3, m5, m6), with its file, and the hint-bar tests covering `J`.
7. Where `failed: aborted` (M6/68) and the stash-drop names (:847) are produced, and the row-cutting function (:848).

## Signatures (exact)
```rust
// graft (app)
pub fn journal_enter(state: &mut AppState) -> Vec<Command>;
// Command/Product for the outside read and the lost scan (builder names them).
// graft-ui::journal_screen
pub enum RowKind { Graft, Outside, Lost, LostEdit }
// JournalRowView gains: kind: RowKind, git_subject: Option<String>, under: Option<usize>
```

## Acceptance (name the verifying test at tick time; red first where it says so)
- [ ] **Stages:**
  - `j_opens_with_graft_rows_at_once`.
  - `outside_rows_arrive_second`.
  - `lost_rows_arrive_third_and_the_title_says_while`.
  - `a_failed_scan_says_why_in_the_title`.
- [ ] **Preselect:**
  - `the_newest_lost_row_is_preselected` (red first).
  - `no_preselect_after_a_key`.
  - `no_preselect_when_a_graft_row_is_newer`.
  - `n_goes_to_the_next_lost_row`.
- [ ] **Enter:**
  - `enter_on_a_lost_commit_opens_the_put_back_preview`.
  - `enter_on_a_gone_branch_opens_make_it_again`.
  - `enter_on_an_outside_row_puts_that_ref_back`.
  - `enter_on_a_switch_row_switches_back`.
  - `enter_on_a_lost_edit_puts_the_file_back`.
  - `enter_on_a_graft_row_is_the_revert_as_before`.
  - `moved_on_shows_the_words_and_b_prefills_the_name`.
  - `b_on_any_row_opens_the_name_prompt`.
- [ ] **Landing:**
  - `put_back_lands_on_the_restored_commit_with_tags_and_words`.
  - `the_tags_clear_on_the_next_operation`.
  - `j_again_shows_a_graft_row_and_no_lost_row`.
  - `u_undoes_the_put_back`.
  - `u_never_acts_on_outside_rows`.
- [ ] **Carried rows:**
  - `an_aborted_entry_reads_aborted_by_you` (M6/68).
  - `a_cut_row_cuts_at_a_word` (:848).
  - `a_stash_drop_has_one_name` (:847).
  - `a_refusal_in_j_shows_on_its_status_row` (:217).
  - `the_command_log_uses_the_dotted_rule`.
- [ ] **UI snapshots:** 187×50, 140×40, 100×30, and 187×24 (the title, legend and bar kept).
- [ ] **Walks** (`m7_walk`), starting from the "lost work" fixture:
  - `lost_to_shell_reset`: `J ⏎ y` = 3;
  - `deleted_branch_back`: `J n ⏎ y` = 4 (after 13, as F4);
  - `edit_in_undo_backup`: `J n ⏎ y` = 4.

  Each asserts the landing, and `len(keys) <= noted_keys(id)`.
- [ ] **Captures**, each ≤ 200 KB, each with a Right line. They lead the contact sheet (unjudged in rev 2):
  - `docs/shots/m7/journal-187x50.txt`: `●`/`$`/`✗` rows, the preselected `✗` row, the legend, the muted last line.
  - `journal-140x40.txt`: no `git:` column; the selected `$` row's subject under it.
  - `journal-100x30.txt`: no source word.
  - `put-back-preview-187x50.txt`: the preview box with `now` / `after y` / `your files:`.
  - `put-back-landed-187x50.txt`: `↺ put back`, `○ main was here`, the `✓` line.
  - `journal-after-put-back-120x40.txt`: the new `●` row; the D row gone.
  - `journal-expanded-120x40.txt`: `┆ git …` under `commands Graft ran`.
- [ ] MAP.md (graft, graft-ui) entries. NOTES: the scout answers used; every J capture re-shot, by file.

## Touches (check each, report how)
- **Every J capture and walk in m3/m5/m6** (scout Q6): the legend and command-log changes re-shoot them. Check: each
  one named in NOTES as re-shot or unchanged, with why. The M6 contact-sheet captions that quote them get updated.
- **The revert-to path (`●`):** unchanged. Check: the m5 revert walks stay green.
- **The name prompt:** a new caller. Check: its tests, plus the `b` test above.
- **Row tags (09):** a new tag kind. Check: 09's tag tests, plus the clearing test.
- **`land_on_result` (03):** a new kind. Check: 03's tests stay green.
- **Hint-bar tests:** the per-row bar. Check: updated and named.
- **The stash screen**, for the one drop name (:847). Check: its snapshot.

## Rules
Worktree only. `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. `b`, `/` and `n` in `J`
are given by the 2b keymap diff; take no other key.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
~80k. If it compacts, split:
- **journal-screen-a:** stages, preselect, keys, `journal_enter`, landings, the carried rows, and their tests.
- **journal-screen-b:** drawing (`RowKind`, breakpoints, legend, the command log), snapshots, walks, captures, and the
  re-shoot of the old J captures.

## Carried (conductor, 2026-09-26)
- ADR-0048 (`┆ git …` for Graft's commands): `docs/shots/m3-journal-expanded.txt` and `docs/shots/m3-contact-sheet.txt` still show `$ git`. This ticket draws the command log, so it re-takes them. Add a tick naming the capture test.
- 22b's question (fixture): the first commit's reflog reads `commit (initial): Start the api` — not in the note's J1 words table, so git's text shows. Decide the row's words (or confirm git's text) and assert them.

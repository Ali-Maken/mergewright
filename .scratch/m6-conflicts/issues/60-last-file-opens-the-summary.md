# last-file-opens-the-summary: finishing a merge's last file opens the summary; otherwise a finish row says `c`

Status: ready
From: P11 (approved 2026-09-24, picks item 2; supersedes batch 11); field notes :60, :63 (in the demo the summary
opened by itself and he went on to `y`); KNOWN_GAPS M6/31 :755 (`c finish` shown only after the next key).
Build with: *(blank: brownfield; the conductor sets it after the scout, ADR-0035)*
Scout brief: (1) graft `update::resolver`: `resolved_advance` and its three landings (`to_next`, `close_after_last`,
`stay_after_last`), `every_file_done_words`, `set_notice`, `settling` / `in_flight` / `queued`, `write_not_landed`, and
the status-read hook (`status_read::owed`, `CHECKING`) that `continue_pressed` waits on — exactly where "the last
`Stage` has landed **and** the status read after it has landed" becomes true. What clears resolver state on `Esc`,
file switch, `d`, `e`, `M`, `u`. (2) graft-ui: how `DocView.notice` is drawn (the notice row under the title), what the
list's column head and strip label are built from, and the `Resolver { finishable }` hint order. (3) Every test and
capture that asserts "the resolver stays" after the last file (`every file done ✓ · c finishes the merge`).
Needs: `finish-summary-is-a-preview` merged (the summary's `y`; the counts below assume it).
Blocked by: 59, 64 (64 marks `d start over` in the done list before `c finish` joins it).
Crates: graft, graft-ui.
Lane: wave 2, second.

## Read first
- `docs/design-notes/m6-conflicts.md` Amendment 2026-09-24: J13 (:896-903), "Where each event lands him" (:916-934),
  "The finish row" (:1027-1048), "Card and file list" last bullet (:1022-1025), hard-to-reverse #2 (:1103).
- `docs/design/02-keymap.md` §2.6 the `c` and `Enter` rows (edited 2026-09-24).
- `crates/graft/MAP.md` §update::resolver (:315-346), §update::banner `continue_pressed` (:301), §update::finish
  `open_finish` (:359).
- `crates/graft-ui/MAP.md` lines 12-13 (`DocView.notice`, the file list), 22 (`Resolver { finishable }`).

## Scope
**In — graft:**
- **The trigger is the event, never the state.** `stay_after_last` (the last file done by `⏎`, `O`/`T`, or `o`/`t` on a
  whole-file card), for a **merge** (begun in Graft or in the shell), sets `ResolverState.finish_owed`. It is spent by
  `finish_if_owed` at the first moment all hold: no take `settling`, `in_flight` or `queued`; no write out; the status
  read after the last `Stage` has landed and shows nothing unmerged. Spending it calls
  `banner::continue_pressed(state, commands, ReturnTo::Resolver, next_id)` — the one `c` path, same refusals.
- **Cleared, never spent,** by: `Esc`, a file switch (`j`/`k`, a click, `n`/`N` crossing), `d`, `e`, `M`, a failed or
  refused write (`write_not_landed`), a status read that still shows a conflict, the resolver closing.
- **No open:** a rebase, cherry-pick or revert (the finish row instead; `c` writes); a stash pop (closes to the graph,
  unchanged); after `u`; on relaunch; the last file done outside the resolver (`space` on a `CONFLICTS` row → the
  banner, unchanged).
- **Status after the last file:** summary opening → `<file> done ✓ · every file done`; no summary (rebase, …) →
  `<file> done ✓ · every file done · c continues the rebase` (the kind's words, `banner::continue_words`).
- **The finish row** (the resolver, every file done, the summary not open: after `Esc` from it, a rebase, cherry-pick or
  revert): replaces `every file done ✓ · c finishes the merge` on the notice row. A click on it does what `c` does.
- **Summary `Esc`** (from `ReturnTo::Resolver`) → the resolver with the finish row; the draft kept.

**In — graft-ui:**
- **Finish row:** full `✓ every file done — c review and finish the merge` (merge) / `✓ every file done — c continue
  the <kind>`; when the row is narrower than the full text: `✓ every file done — c review & finish` /
  `✓ every file done — c continue`; then cut with `…`. Style: `accent` BOLD on a full-width `selection` fill; `NO_COLOR`
  REVERSED. Width-driven, not size-tier-driven.
- **All-done head:** column `FILES · all <n> done ✓`, strip label `files · all done ✓`.
- **Bar:** `Resolver { finishable: true }` puts `c finish` (merge) / `c continue` **first**.
- **Hit test** for the finish row.

**Out:**
- The summary itself (previous spec). `c` from the pane (ticket 52).
- The banner's `every file done ✓ · c finishes the merge` (unchanged).

## The failing test first
graft `tests/it/resolver/`: `the_last_file_of_a_merge_opens_the_summary`.
- Setup: two-files merge begun in the app, `gone.txt` done, `notes.txt` open with one block left. Press `o`, then `⏎`;
  drive the stage and the status read to landing.
- Assert: `finish.is_some()`, `return_to == ReturnTo::Resolver`, focus `Files`, cursor on `README.md` (first row git
  combined); status `notes.txt done ✓ · every file done`.
- Red today: the resolver stays, notice `every file done ✓ · c finishes the merge`.

## Signatures
```rust
// graft/src/update/resolver.rs (ResolverState) and child `resolved`
pub finish_owed: bool,                                                   // new field
pub(crate) fn finish_if_owed(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut dyn FnMut() -> JobId);
//   (argument shapes as `continue_pressed` takes them; called from the Stage answer and the status-read answer)
// graft-ui/src/conflict_resolver.rs
pub struct FinishRow { pub full: SanitizedText, pub short: SanitizedText }
pub struct DocView { /* existing …, */ pub finish: Option<FinishRow> }   // Some → drawn on the notice row instead of `notice`
#[must_use] pub fn finish_row_at(doc: &DocView, placement: Placement, area: Rect, col: u16, row: u16) -> bool;
```
If `DocView` is built by struct literal in `graft` (it is, in `view::resolver`), the new field is set there in this
same ticket (both crates are in the set).

## Touches (each entry, and how the builder checks it)
- **Takes still landing cannot open it early.** Check: `a_take_still_landing_holds_the_summary` (`T` then `⏎` typed
  faster than the disk: opens once, after both land), `a_failed_last_stage_opens_nothing`.
- **Clears.** Check: `esc_before_the_read_lands_cancels_the_open`, `switching_file_cancels_the_open`.
- **No open where it must not.** Check: `the_last_file_of_a_rebase_shows_the_finish_row`,
  `a_stash_pop_still_closes_to_the_graph`, `undo_after_finish_opens_no_summary` (ticket 41's landing: banner focused),
  `relaunch_on_a_resolved_merge_opens_no_summary`, `space_on_the_last_conflicts_row_lands_on_the_banner`.
- **Shell-begun merge:** opens too (J6b). Check: `the_last_file_of_a_shell_merge_opens_the_summary`.
- **Finish row.** Check: `finish_row_words_by_width` (187, 120, 80: full / full / short), `finish_row_style`
  (buffer: `accent` BOLD on `selection`; `NO_COLOR` REVERSED), `click_on_the_finish_row_is_c`.
- **All-done head.** Check: `list_head_when_every_file_is_done` (column and strip).
- **Hints.** `c` first when `finishable`. Check: `every_context` coverage, `every_key_the_bar_draws_is_answered`.
- **Journal and undo:** nothing new (the summary writes nothing). Check: the finish journal tests green.
- **KNOWN_GAPS M6/31 :755** — closed by this (the conductor's row; name the test).
- **The walk:** `one-file-one-conflict`, `three-conflicts-all-theirs`, `two-files-in-order`, `two-files-out-of-order`
  now land on the summary (assert its frame, not "the resolver stays"); `a-finish` and `b-finish` start on the open
  summary and send `y`; new scenario `last-block-to-finished` (two-files, `gone.txt` done, the cursor on `notes.txt`'s
  last open block: `o ⏎ y`, 3). Update `.scratch/m6-conflicts/pty-walk-m6.py` keys (`m6-14` no longer needs `c`).
  - Check: `journey_counts_match_the_note` and `every_scenario::journey` with the rows below applied.
- **Captures:** m6-14 (now the summary, opened by itself), m6-15 (now the finish row after `Esc`), `fix-finish-from-the-
  resolver-*`, `fix-every-file-done-footer-*`, `fix-file-list-*` (the head). Regenerate what the tests own; the walk's
  m6-1x captures are the acceptance ticket's.
- **Docs:** `crates/graft/MAP.md` §update::resolver (`finish_owed`, `finish_if_owed`, the landings);
  `crates/graft-ui/MAP.md` lines 12-13 (`FinishRow`, `finish_row_at`, the head).

## Acceptance (name the verifying test at tick time)
- [ ] `the_last_file_of_a_merge_opens_the_summary`: red first, then green.
- [ ] every test named under Touches.
- [ ] `journey_counts_match_the_note` with the rows below.
- [ ] Capture (rule 6; colour **unverified**, rule 5): `docs/shots/m6/fix-last-file-opens-summary-187x50.txt` (two
      frames: before the last `⏎`; after it, the summary) and `fix-finish-row-120x40.txt` (after `Esc` from it).
  - **What right looks like:** frame 2's top border is `╭ Preview: finish merging feature into main`, the status row
    `notes.txt done ✓ · every file done`. The finish row capture: under the identity row
    `✓ every file done — c review and finish the merge`, list head `files · all done ✓`, bottom row starting `c finish`.
- [ ] MAP.md updated (doc, no test).

## For the conductor (exact text, in this spec's merge commit)
In the `journey-counts` fence, set these `keys` (comments unchanged) and un-comment the last row:
```
a-finish                      1
b-finish                      1
a-whole-one-file              4
a-whole-two-files             5
b-whole-two-files             4
last-block-to-finished        3                     # Amendment 2026-09-24 J13: o ⏎ y (P11+P12) · JetBrains ~4 (?) · VS Code 3–4 · lazygit 2 (bare, rejected)
```

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium-large. One context. The trigger is the one decision point; the rest is words and a row.

## NOTES

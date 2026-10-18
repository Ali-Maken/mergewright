# 05t — merge screens: lower 05b's ratchet rows (review ease-pass-05-10 M2), one name per state (Minor 9), capture names (Minor 10)

Status: done (2026-10-18; Captures box open, see NOTES)
Blocked by: none. **Disjoint by file from 05s and 08u** — either order. Shares only registries, row by row: `crates/graft/tests/it/over_400.txt` (05s and 08u lower other rows), graft-ui `MAP.md` (one row) — the conductor merges rows by hand.
Build with: Sonnet — moves of whole tests, one rename and two file renames; nothing to decide, the ratchet test and the test count judge it.
Scout: none — the ratchet file and `git diff` name every file.
Needs: none.

## Read first
- `docs/reviews/ease-pass-05-10.md`: M2, Minors 9, 10.
- `crates/graft/tests/it/over_400.txt` header (:1-2). `CONTEXT.md` "Done (file)" (:147-150). graft-ui `MAP.md` `FileState` row (:280).
- `.scratch/ease-pass/issues/05s-merge-screens-fixes.md` Scope "Out" and Decided "M2, this ticket's three rows" — the files you must not touch.

## Scope
**Never edit** 05s's files (its Scope "In" list, including `update/finish/tests.rs`, `update/banner/tests.rs`, graft-ops `tests/it/finish.rs`, `tests/it/merge_preview.rs`, `tests/it/merge_one_file.rs`, `update/resolver/tests/review_at_y.rs`) or 08u's (`update/commit_editor/`, `view/frame`, graft-ui `commit_editor`, `graph_screen/{layout,mod}.rs`). Need one → commit at the seam, `blocked: needs <file>`, stop. **Do not edit `tests/it/main.rs`**: every new file is a submodule of the file it leaves.

1. **M2 — every row 05b raised, back to at most its pre-05b value.** The rows: `git diff 4ed45107^ 4ed45107 -- crates/graft/tests/it/over_400.txt` (22; minus 05s's three). Target per row = its value at `4ed45107^`; the two rows 05b added are dropped (file ≤400). The review's named splits:
   - `tests/it/m5_walk/conflict_journeys.rs` (1396): 05b's added scenarios → `m5_walk/merge_review_journeys.rs` (≤1355).
   - `tests/it/resolver.rs` (419, new row): the `two_file_cherry_pick` fixture and its users → `tests/it/resolver/fixtures.rs` (≤400, row dropped).
   - `tests/it/resolver/file_list/start_over.rs` (407, new row): one file per merge origin, e.g. `start_over/graft_begun.rs`, `start_over/shell_begun.rs` (each ≤400, row dropped).
   - The rest, each by moving one cohesive block (whole tests and the helpers only they use) into a sibling submodule: `tests/it/m5_walk/run.rs` (820 → ≤781), `tests/it/finish/choices.rs` (442 → ≤430), `tests/it/conflict_interim.rs` (→ ≤ pre-05b), `tests/it/resolver/file_list.rs` (717 → ≤705), graft-ops `tests/it/history_ops.rs` (→ ≤ pre-05b), and every other row the diff shows.
   - Rules: fn names unchanged; no line-joining, no deleted comments, no `#[rustfmt::skip]`, no new file over 400. Shell moves (`sed -n`/`sed -i` by line range) are fine; read only the block's bounds.
2. **Minor 9 — one name per state.** graft-ui `conflict_resolver::FileState` → `FileRowLook` (it is how a row looks; `graft_ops::conflict::FileState` keeps the name). Every user (grep, both crates); `conflict_resolver/files.rs` stays ≤397 (it sits at the ceiling; 11 splits it). graft-ui `MAP.md` row renamed. `CONTEXT.md` gains, after "Done (file)":
   ```
   **Solved (file)**:
   A conflicted file of a merge with every clash picked (or edited by hand), not yet
   staged: git still calls it unmerged until the review's `y` stages it. Shown `✓ solved`
   today (11 asks the user whether it reads `done`, as the mock).
   _Avoid_: resolved, done
   ```
   Add one doc line on the app's `RowState::AllTaken`: "a merge's solved file (CONTEXT `Solved (file)`)".
3. **Minor 10 — capture names say what they show.** `git mv docs/shots/m6/fix-finish-row-120x40.txt docs/shots/m6/fix-solved-row-120x40.txt`; `git mv docs/shots/m6/fix-last-file-opens-summary-187x50.txt docs/shots/m6/fix-y-opens-the-review-187x50.txt`. Change the stem in the test that writes each (grep the old stem) and every live reference (`TESTING.md`, `docs/design*`, contact-sheet lists, `.scratch/ease-pass/`). History (`docs/log/`, `docs/reviews/`) keeps the old names. Don't regenerate contact sheets (14's).

Out: any behaviour, any word on screen, 05s's and 08u's files, contact sheets.

## Signatures
```rust
// graft-ui conflict_resolver — renamed, variants and derives unchanged
pub enum FileRowLook { /* was FileState: AllTaken, Solved, Done, … */ }
```

## Touches (check each, report how)
- Moved tests: their module paths change. Every moved fn that a tick names in `.scratch/ease-pass/issues/*.md` or `TESTING.md` — list old path → new path in NOTES (grep each fn name). Don't edit other tickets' ticks.
- Test count equal before and after (`cargo nextest list` line count, both in NOTES).
- pty capture paths inside moved tests: unchanged strings (grep `docs/shots` in each moved block).
- Rename: `rg -n 'conflict_resolver::FileState|FileState::' crates/graft-ui crates/graft` — every hit is the ops type or renamed; `graft_ui` re-exports (grep `pub use`).
- Captures renamed: no file content changes (`git diff -M --stat docs/shots` shows renames only).
- Keymap, journal/undo, other screens: none (no behaviour) — say the grep.

## Acceptance
Each tick names its test inline.
- [x] Every row 05b raised (but 05s's three) is ≤ its `4ed45107^` value; `tests/it/resolver.rs` and `…/start_over.rs` rows gone; no new row — graft's ratchet test in `tests/it/file_size.rs` (name the fn). NOTES: table `path · pre-05b · before · after`. — `file_size::every_file_over_400_lines_is_listed_with_its_count` green.
- [x] The four named splits exist as named (`m5_walk/merge_review_journeys.rs`, `resolver/fixtures.rs`, two `start_over/` files) — by `ls` in NOTES; all ≤400 by `file_size::every_file_over_400_lines_is_listed_with_its_count`.
- [x] Test count unchanged (`cargo nextest list`, both numbers in NOTES); full nextest green. — `cargo nextest run --workspace`: 6853/6853 passed.
- [x] `FileRowLook`: the grep above shows no graft-ui `FileState`; `CONTEXT.md` `Solved (file)` entry as written — by reading, quote it; `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` green (in the 6853).
- [ ] Captures: `git diff -M --stat` shows two renames, 100 % similar; the writing tests green and write the new names (name them); `rg 'fix-finish-row|fix-last-file-opens-summary' -g '!docs/log/**' -g '!docs/reviews/**'` empty. — renames 100 % and writer `resolver::opens_summary::y_on_a_solved_shell_merge_opens_the_review_over_the_resolver` green with new stems; **open**: rg not empty (see NOTES).

## NOTES (builder 05t, 2026-10-18)
Scout: none — ticket named rows; `git diff 4ed45107^ 4ed45107` gave the rest.

Ratchet (path · pre-05b · before · after):
- graft-ops src/operation/tests.rs · 2010 · 2011 · 1981
- graft-ops it/absorb_staged.rs · 642 · 643 · 609
- graft-ops it/finish_merge.rs · 814 · 819 · 787
- graft-ops it/history_ops.rs · 2705 · 2712 · 2631
- graft-ops it/set_aside.rs · 1352 · 1356 · 1327
- graft-ops it/stop_hooks.rs · 916 · 917 · 857
- graft src/update/banner/tests/resume_words.rs · 536 · 537 · 510
- graft src/update/palette/tests.rs · 425 · 428 · 404
- graft it/conflict_interim.rs · 1994 · 2005 · 1906
- graft it/finish/choices.rs · 430 · 442 · 370 (row dropped)
- graft it/m5_walk/conflict_journeys.rs · 1355 · 1396 · 1320
- graft it/m5_walk/run.rs · 781 · 820 · 700
- graft it/resolver/file_list.rs · 705 · 717 · 669
- graft it/resolver.rs · — · 419 · 397 (dropped)
- graft it/resolver/file_list/start_over.rs · — · 407 · 57 (dropped)
- merge_set_aside.rs (953) and m5_walk/conflicts.rs (670) already under pre-05b; untouched.

New files: m5_walk/merge_review_journeys.rs (`#[path]` child of conflict_journeys.rs), m5_walk/run/secrets.rs, resolver/fixtures.rs, resolver/file_list/start_over/{graft_begun,shell_begun}.rs, resolver/file_list/many_files.rs, conflict_interim/clicks.rs, finish/choices/handover.rs; graft-ops it/{absorb_staged/continued_elsewhere, finish_merge/autostash_abort, history_ops/refusals, set_aside/relaunch, stop_hooks/resume}.rs, src/operation/tests/shell_merge.rs; graft src/update/banner/tests/resume_words/finish_merge.rs, src/update/palette/tests/placeholder.rs.
Edits beyond moving: `pub(super)` on moved fns the parent calls; `super::super::restart` → `crate::resolver::restart` in start_over children; explicit `use super::{…}` in the two m5_walk files (the m5_walk example compiles them outside cfg(test), where `clippy::wildcard_imports` fires).

Test count: nextest list 6853 before, 6853 after. Moved tests gain one module segment (e.g. `start_over::y_restores_…` → `start_over::graft_begun::y_restores_…`); m5_walk scenarios are string-named, unchanged. Grep of every moved fn in `.scratch/ease-pass/` and `TESTING.md`: no hits.

Rename: `rg '\bFileState\b' crates/graft-ui crates/graft` → none. Also renamed in graft MAP.md `file_list_view` row (it names the UI type). `files.rs` 397. CONTEXT entry added verbatim from spec; `RowState::AllTaken` doc line added.

Captures, open box: remaining rg hits are history or sheets: `docs/shots/m6/m6-contact-sheet-3.txt` (no regenerating, 14's), `.scratch/m6-conflicts/issues/{60,63}`, `.scratch/ease-pass/issues/{05b,05c}` + `scouts/05c-bar-and-rows.md` (records of what was regenerated then). Captions tsv updated. Conductor decides.

Touches: keymap/journal/undo none — no behaviour change; only `src` edits are the rename (view mapping, graft-ui) and a doc line. 05s/08u files: none in diff.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k. Hard stop past ~80k at a file boundary: hand off the rows left, by name.

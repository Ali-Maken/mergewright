# 05t — merge screens: lower 05b's ratchet rows (review ease-pass-05-10 M2), one name per state (Minor 9), capture names (Minor 10)

Status: ready (planner, spec-only pass, 2026-10-18, from `docs/reviews/ease-pass-05-10.md`)
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
- [ ] Every row 05b raised (but 05s's three) is ≤ its `4ed45107^` value; `tests/it/resolver.rs` and `…/start_over.rs` rows gone; no new row — graft's ratchet test in `tests/it/file_size.rs` (name the fn). NOTES: table `path · pre-05b · before · after`.
- [ ] The four named splits exist as named (`m5_walk/merge_review_journeys.rs`, `resolver/fixtures.rs`, two `start_over/` files) — by `ls` in NOTES.
- [ ] Test count unchanged (`cargo nextest list`, both numbers in NOTES); full nextest green.
- [ ] `FileRowLook`: the grep above shows no graft-ui `FileState`; `CONTEXT.md` `Solved (file)` entry as written — by reading, quote it; `map_coverage::map_lines_are_short_and_carry_no_ticket_ids` green.
- [ ] Captures: `git diff -M --stat` shows two renames, 100 % similar; the writing tests green and write the new names (name them); `rg 'fix-finish-row|fix-last-file-opens-summary' -g '!docs/log/**' -g '!docs/reviews/**'` empty.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~65k. Hard stop past ~80k at a file boundary: hand off the rows left, by name.

# 81 — an untracked file's diff shows its content

Status: partial — graft-git side done; body blocked on graft-ui
Blocked by: none (70 merged)
Crates: graft-git (`diff/read.rs`, tests); graft only if the job must ask differently (`jobs/detail.rs`), else none.
Source: usability-final n1 (`docs/reviews/m6-usability-final.md`); ticket 70 NOTES: `crates/graft-git/src/diff/read.rs:330-332`
returns `empty_diff` for `Untracked`, so selecting a new file in the changes pane shows no body. The entry itself is kept
(`crates/graft/src/jobs/detail.rs:272`). Every tool he uses (VS Code, JetBrains, lazygit) shows a new file as all added lines.
Build with: builder-sonnet
Scout: none; 70's builder named the line. Re-check it on main first.

## Acceptance
- [x] An untracked file's diff reads its working-tree content as one all-added hunk, through the same sanitising and size
      limits as any diff (binary → the binary line; over the size cap → the cap's line). Test on a real repo, red first.
      — `diff::read::tests::untracked_file_diff_is_one_all_added_hunk` (red before the fix, seen).
- [x] Binary untracked file, empty untracked file, untracked file over the size cap: one test each.
      — `untracked_binary_file_gives_the_binary_note`, `untracked_empty_file_gives_no_hunks_and_no_error` (passed red
      too: empty was already hunk-less), `untracked_file_over_the_line_cap_is_truncated`. Byte cap (64 MiB → `TooLarge`)
      shares the code path, not tested (laptop-safe).
- [x] No git call on the UI thread; the read stays in the detail job. Checked by grep: `file_diff(` callers in graft are
      `jobs/detail.rs:126` (run from `jobs/runner.rs:279`, the job thread) and `jobs/finish.rs:47` (`Staged`, no untracked).
      No new caller.
- [x] The capture that shows an untracked row selected (grep `docs/shots` for one; if none, add one at 120×40 through an
      existing capture test) shows the body. Rebuild that milestone's sheet, each part ≤ 200 000 B.
      — `crates/graft/tests/it/conflict_interim/tab_walk.rs::capture_fix_tab_words_120x40` (and `_80x24`), frame 6 of
      `docs/shots/m6/fix-tab-words-120x40.txt`: `1 + hello from new`. Sheet parts max 179 698 B.

## Touches
- Every caller of the untracked read (grep). MAP.md graft-git. TESTING.md row for the changes pane if one names it.
- KNOWN_GAPS and `docs/design/` are the conductor's.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit. No machine paths.

## NOTES
- Re-checked on base 6a43fff9: `read.rs:330` returned `empty_diff` for `Untracked`. Now `untracked_file_diff`:
  `git diff --no-index --no-color --no-ext-diff -U<n> [-w] -- /dev/null <path>`, `run_raw`, exit 0/1 ok, else `Exited`;
  same `parse_unified`, `OUTPUT_CAP`, `apply_truncation`. Test helper asserts `.git/index` bytes unchanged by the read.
- **Blocked: needs graft-ui.** `crates/graft-ui/src/detail_pane/body.rs:92` draws `untracked · no diff yet (M3)` and
  returns whenever `DiffView::untracked` is set (`crates/graft/src/jobs/detail.rs:455`), before hunks. Next step: drop
  that early return (the flag is still needed by `update/operate.rs` for stage/discard), then tick 4's capture and
  TESTING.md:1136 (`untracked · no diff yet (M3)` wording) and the sheet. Tick 4 not done.
- Round 2 (graft-ui freed): `body.rs` draws the hunks; `empty file` only when untracked with no hunks and no binary
  note. Tests `detail_pane::tests::an_untracked_file_with_lines_draws_its_lines`,
  `an_empty_untracked_file_says_empty_file` (both red first). TESTING.md:1136 updated. The existing capture
  `docs/shots/m6/fix-tab-words-{80x24,120x40}.txt` (frame 6, untracked `new.txt`) failed against the change and was
  regenerated: body now `1 + hello from new`. Caption row 149 updated, sheet rebuilt (4 parts, max 179 698 B). No graft
  src or test file edited. Tick 4 left for the conductor to judge (a capture of the untracked row now exists and shows
  the body; the brief reserved tick 4 until 80 merges).
- Scout: none (brief said 70's builder named the line; re-checked by reading it).

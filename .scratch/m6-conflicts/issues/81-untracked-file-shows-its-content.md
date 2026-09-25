# 81 — an untracked file's diff shows its content

Status: ready
Blocked by: none (70 merged)
Crates: graft-git (`diff/read.rs`, tests); graft only if the job must ask differently (`jobs/detail.rs`), else none.
Source: usability-final n1 (`docs/reviews/m6-usability-final.md`); ticket 70 NOTES: `crates/graft-git/src/diff/read.rs:330-332`
returns `empty_diff` for `Untracked`, so selecting a new file in the changes pane shows no body. The entry itself is kept
(`crates/graft/src/jobs/detail.rs:272`). Every tool he uses (VS Code, JetBrains, lazygit) shows a new file as all added lines.
Build with: builder-sonnet
Scout: none; 70's builder named the line. Re-check it on main first.

## Acceptance
- [ ] An untracked file's diff reads its working-tree content as one all-added hunk, through the same sanitising and size
      limits as any diff (binary → the binary line; over the size cap → the cap's line). Test on a real repo, red first.
- [ ] Binary untracked file, empty untracked file, untracked file over the size cap: one test each.
- [ ] No git call on the UI thread; the read stays in the detail job. Say how you checked.
- [ ] The capture that shows an untracked row selected (grep `docs/shots` for one; if none, add one at 120×40 through an
      existing capture test) shows the body. Rebuild that milestone's sheet, each part ≤ 200 000 B.

## Touches
- Every caller of the untracked read (grep). MAP.md graft-git. TESTING.md row for the changes pane if one names it.
- KNOWN_GAPS and `docs/design/` are the conductor's.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit. No machine paths.

## NOTES

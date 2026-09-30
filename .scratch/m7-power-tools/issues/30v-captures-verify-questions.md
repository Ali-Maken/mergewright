# captures-verify — name the cause of every pty drift no ticket owns, before anything is re-shot

Status: ready
Build with: **Opus, dispatched as `debugger`** (diagnosis first: a SIGTERM that may hang, a SHA that may be unpinned, a
marker nobody may have designed). `diagnosing-bugs` is allowed for `debugger` only. Re-shoots here are mechanical and
happen only after a cause is named.
Needs: nothing. Base: `main`.
Runs concurrently with: `captures-reshoot-named` (disjoint capture files), `f-recover-set-aside-bytes`,
`f-guard-refusal-at-80`.

## Why
26 ran every m3–m6 pty walk in compare mode: 26 capture files drifted (m4 4, m5 5, m6 17). `captures-reshoot-named` owns
the 15 whose change is named (m4-03/04, m5-01, m6-06, m6-07, m6-19). **This part owns the other 11**, three of them
questions with no line (rule 7):
- **m5-11-diverged-pull-sticky** (`docs/shots/m5-11-diverged-pull-sticky-{80x24,160x48}.txt`): line 4 drawn blank;
  the app was `not gone 10 s after SIGTERM — killed`. A slow read (`reading history…` where the capture had `184 rows so
  far`, 160x48 :3) or a hang on SIGTERM.
- **m6-18-launch-shell-begun** (`docs/shots/m6/m6-18-launch-shell-begun-{80x24,120x40,160x48,187x50,200x50}.txt`):
  `diff vs HEAD 1e78110` → `a301516`. A changed fixture or an unpinned date.
- **m6-21-stash-clash-done** (`docs/shots/m6/m6-21-stash-clash-done-{80x24,120x40,187x50}.txt`): a `▎` drawn after
  `+1 −1`. A designed marker (whose?) or a stray.
- **m5-07-branch-row-hints-160x48** (`docs/shots/m5-07-branch-row-hints-160x48.txt` :50): the hint bar differs past
  column 150; 26 did not read further. No cause named.
If Step 1 finds a drifting file in neither list, it is this part's too.

## Files (exact)
- The 11 capture files above, **only** once their cause is named and cleared (below).
- `TESTING.md`: only lines that quote a re-shot capture's changed text.
- `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py`, `.scratch/m6-conflicts/pty-walk-m6.py`: only for a cause that
  is the walk's own (a date not pinned, a wait on the wrong frame), by the rule below.
- This ticket's NOTES.
- Not: any crate source. Not: contact sheets or caption tsvs (`m7-acceptance-finish` rebuilds them). Not: m6-25 (26
  re-took it). Not: the 15 named files.

## Step 1: list (context guard: logs to files, read summary lines only)
- Build as TESTING.md :2173 says (`-p graft --example m5_walk --bin graft`, `-p graft-fixtures --bin graft-fixtures`,
  `GRAFT_TARGET`). Run `pty-walk-m5.py` at 80x24 and 160x48 and `pty-walk-m6.py` at every size it has captures for
  (80x24, 120x40, 160x48, 187x50, 200x50), **compare mode** (no `GRAFT_UPDATE_SHOTS`). Logs outside the repo
  (`$CARGO_TARGET_DIR/pty-logs/`): raw streams hold home paths and are never committed
  (`published_repo::nothing_a_commit_would_carry_names_this_machine`).
- NOTES table: every drifting file, whole frame (not the first line only): file · lines that differ · owner
  (`reshoot-named` / this part).

## Step 2: for each file this part owns, the cause **with a line**, before any re-shoot
Each row in NOTES: cause · the file:line or commit (`git log -S'<drawn text>'`) that proves it · verdict.
- **m5-11:** time the read and the exit apart.
  - Run the scenario alone; note when `reading history…` gives way, and whether the app exits within 10 s of SIGTERM
    when idle and when mid-read.
  - Find the SIGTERM handling in graft (grep `SIGTERM`, `signal`) and the walk's own wait (the script's line).
  - The app does not exit on SIGTERM mid-read → **stop, finding**: `graft hangs on SIGTERM while <what>`, file:line.
    No re-shoot. The conductor files the fix.
  - The walk captured before the read finished → the walk's cause: its wait must be on a frame condition (the
    capture's own `N rows so far`, or the read done), never a longer sleep or timeout (rule 2). Change the wait, then
    re-shoot. A fix that needs a sleep or a raised timeout → **stop and ask**.
- **m6-18:** run the scenario twice.
  - SHA differs between runs → unpinned date. Pin it the way `pty-walk-m6.py:60` pins m6-25's (used at :111): cite both
    lines. Re-shoot.
  - SHA stable, fixture changed → name the commit that changed it (`git log` on the fixture's source, found by grep in
    graft-fixtures). Re-shoot, citing it.
- **m6-21:** find `▎` in graft-ui/graft (grep), `git log -S'▎'` for the commit, and the design line that asks for it
  (grep `▎` in `docs/design/`, `docs/design-notes/m7-power-tools.md`, the ticket the commit names).
  - Designed → re-shoot, citing the ticket and the design line.
  - No design line → **stop, finding**: `stray ▎ after +1 −1`, the drawing file:line and the commit. No re-shoot.
- **m5-07 (160x48 :50) and any other owned file:** the full differing line; the commit that changed the drawn text
  (`git log -S`), and the ticket that commit belongs to. Named and intended (a keymap or wording ticket; cite the line in
  `docs/design/02-keymap.md` or the ticket) → re-shoot, citing it. Otherwise → **stop, finding**, with the line.

## Stop conditions (each stops that file, not the whole part)
- No line or commit found → the file is not re-shot; NOTES say `cause not found` and what was tried.
- A product defect found → not re-shot; a finding with file:line for the conductor.
- **Re-shoot only after** the cause row is in NOTES. Re-shoot with `GRAFT_UPDATE_SHOTS=1`, then run compare mode again:
  green for that file.

## Acceptance (name the verifying command at tick time)
- [ ] Step 1's table in NOTES, every drifting file listed with its owner.
- [ ] m5-11: cause row with its line; re-shot or finding.
- [ ] m6-18: cause row with its line; re-shot or finding.
- [ ] m6-21: cause row with its line; re-shot or finding.
- [ ] m5-07 and every other owned file: cause row; re-shot or finding.
- [ ] Every re-shot file: compare mode green on the second run (the command and its summary line). Old vs new: every
      changed line is explained by the cause row (said per file).
- [ ] `TESTING.md`: every line quoting a re-shot file's old text updated (grep the old text: 0 hits left).
- [ ] Each capture ≤ 200 KB (`wc -c`).

## Touches
- The walk scripts: a change is to the named scenario only; every other scenario's captures stay byte-identical
  (compare run green for them).
- TESTING.md: `git diff --stat TESTING.md` shows only the quoted lines.
- Contact sheets and caption tsvs: untouched here (`git diff --stat` on them empty).
- No crate source changed (`git diff --stat crates/` empty). A cause in source is a finding, not a fix here.

## Gates
No Rust changed → the two walks' compare runs. If a walk script changed, also `python3 -m py_compile` on it.

## Size
~60k. If it compacts, say so; the conductor splits by file.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md`. Raw logs never
committed. Rule 7: every verdict carries its line.

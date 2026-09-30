# captures-verify — name the cause of every pty drift no ticket owns, before anything is re-shot

Status: done
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
- [x] Step 1's table in NOTES, every drifting file listed with its owner — `target/pty-logs/step1.sh` (7 compare runs,
      rc=1 each), summarised by the scratchpad `summ.py`; 22 drifting + m6-25, same set as 26's diffs.
- [x] m5-11: cause row with its line; re-shot — the walk's own (`pty-walk-m5.py` `Run::end` unread pty; `when:
      Appears` frame, `remotes.rs:95`), proven by `probe.py` (exit 0.00–0.02 s read, hang + `sample` in `write` unread).
- [x] m6-18: cause row with its line; re-shot — `dc7da822`, `view/detail.rs:216-233`; SHA stable over 3 runs.
- [x] m6-21: cause row with its line; re-shot — `130dd529` (20b), `line_render.rs:144-145`, design `20b…md:20,:25-27`.
- [x] m5-07 and every other owned file: cause row; re-shot — m5-07 `daef05e6`, `hints/lists.rs:194`, keymap :204;
      m6-06 (conductor's add) `dc7da822` + `ca2695b0` + `20df04d2`.
- [x] Every re-shot file: compare mode green on the second run — `MODE=compare RUN=2 sh target/pty-logs/reshoot.sh`,
      summary: every targeted line `rc=0` (12 runs). Old vs new per file in NOTES "Re-shoot".
- [x] `TESTING.md`: every line quoting a re-shot file's old text updated — `grep -n "main ↑1\]\|1e78110\|2e2d51c\|REBASING
      step 1 of 2" TESTING.md` → 0 hits.
- [x] Each capture ≤ 200 KB — `wc -c` on the 13: largest 7797 B.

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

## NOTES (debugger, 2026-10-01)

Base `0efb6867` (main, 30t merged). No scout: diagnosis ticket, every cause found by running the scenario and reading
the drawing code. Logs under `target/pty-logs/` (worktree target, never committed).

### Step 1: compare runs (m5 80x24, 160x48; m6 80x24, 120x40, 160x48, 187x50, 200x50)
Command: `GRAFT_TARGET=<wt>/target python3 <walk> <WxH>`, all seven rc=1. Script `target/pty-logs/step1.sh`.
Not drift, left out of the table: 39 `could not be read` (the m6 walk walks every stem at every size; those sizes have
no capture) and `m5-05 … not on screen` at 80x24/160x48 (m5-05 is taken at 38x10 only). Same 22 + m6-25 as 26's diffs.

| File (sizes) | Lines that differ (first shown by the compare; whole frame after the re-shoot below) | Owner |
|---|---|---|
| m4-03/04 (4 files; m4 walk, from 26's diffs, not re-run) | — | reshoot-named |
| m5-01-preview-rebase (80x24 :23, 160x48 :47) | `git rebase -i … --empty=drop main` | reshoot-named |
| m5-07-branch-row-hints-160x48 | :50 bar gains `  C` at the end | **30v** |
| m5-11-diverged-pull-sticky (80x24 :4, 160x48 :3) | graph row / header | **30v** |
| m6-06-relaunch-sticky (80x24 :4, 120x40 :4) | `REBASING  step 1 of 2 · pick …` | reshoot-named |
| m6-07-all-resolved (80x24 :3, 120x40 :3) | same | reshoot-named |
| m6-18-launch-shell-begun (80x24 :6, 120x40 :28, 160x48 :4, 187x50 :4, 200x50 :4) | `diff vs HEAD 1e78110` → `a301516` | **30v** |
| m6-19-after-esc-esc (80x24 :26, 120x40 :42, 160x48 :50, 187x50 :52, 200x50 :52) | bar `c finish` first | reshoot-named |
| m6-21-stash-clash-done (80x24 :8, 120x40 :29, 187x50 :9) | `▎` | **30v** |
| m6-25-menu-fixed-slots (80x24, 120x40, 187x50 :6) | `Cherry-pick 401cfbe (o)` | 26 (re-took it; not on this base) |

No file in neither list.

### Step 2: causes

| File | Cause | Proof (line / commit) | Verdict |
|---|---|---|---|
| m5-11 exit | **The walk's own wait, not graft.** After taking the frame the script stopped reading the pty; the run was still drawing (the post-fetch graph lands after the sticky), the pty buffer filled, graft's main thread blocked in `write` inside a ratatui draw, so the loop never took the SIGTERM wake. | `sample` of the stuck pid: main thread `ratatui … Terminal::draw → crossterm SetColors → StdoutLock::write_all → write`; `graft-signals` idle in `recvfrom` (it had sent the wake). Probe (scratchpad `probe.py`): SIGTERM with the pty read → exits 0.00 s idle, 0.02 s at the sticky's first draw (mid-reload), status 15, 3/3 runs; with the pty unread after the frame → not gone in 10 s, 1/1. Script: `pty-walk-m5.py` `Run::end` waited with `time.sleep(0.05)` and closed `raw` first. | Walk fix: `end()` pumps the pty while it waits. Not a graft hang (`event_loop/signals.rs:53-81` sends the wake; nothing to fix there). |
| m5-11 frame | **The walk captured a transient.** `when: Appears` (`tests/it/m5_walk/remotes.rs:95`) takes the frame on the sticky's first draw. Timeline (160x48): 0.887 s rows blank + `reading history…` (the reload after the fetch), 0.906 s sticky, 0.908 s the new graph (`the teammate's commit`, `main ↑1 ↓1`, `184 rows so far`), nothing after for 5 s. The committed frame was the other side of the same race (sticky over the pre-fetch graph, `main ↑1`, no `ⓘ` — the radar marker, `graph_screen/render.rs:297`, arrives from its own read). | Probe timelines above; two settled runs per size byte-identical (tmp path masked). The fetched commit is the fixture's `commit_on_remote("the teammate's commit")`, `tests/it/m5_walk/fixture.rs:480`. | Walk fix: the script also waits for `the teammate's commit` (`DRAWN_TOO`), a frame condition, no sleep or timeout raised. `When::Appears` itself is crate source, not changed; with the condition the frame is the settled one. Re-shoot. |
| m6-18 | Run twice (and 26's run): `a301516` every time — stable, not a date. It is the true HEAD: the scratch's `git log` has `a301516 main's notes` (HEAD) and `1e78110 feature's readme` (MERGE_HEAD, the merge's top row). The committed capture named the graph's top row. | `dc7da822` m6-conflicts-85 (m-b, `.scratch/m6-conflicts/issues/85-usability-2-fixes.md:35`): header names HEAD from the refs, `crates/graft/src/view/detail.rs:216-233`. Capture taken at `f3c9b803`, before it (`git merge-base --is-ancestor`). Fixture unchanged since (`git log f3c9b803..HEAD -- tests/it/m5_walk/fixture.rs` empty). | Named, intended. Re-shoot. |
| m6-21 | `▎` is the diff line cursor on the commit diff's first row (the hunk heading, empty here), drawn at the body's x (col 30 at 120x40, gutter 3). | `crates/graft-ui/src/detail_pane/line_render.rs:144-145` (`CURSOR_GLYPH` on a heading row); every spec has the cursor since `130dd529` m7-power-tools-20b (`body_cursor`, `crates/graft/src/update/detail/ops.rs:446-451`). Design: `20b-diff-line-cursor.md:20` (every `DetailSpec` has the cursor), `:25-27` (a glyph carrier for captures); 20b NOTES `:140-143` name this exact change ("`▎` on row 0 of a commit diff (its first hunk heading)") and re-shot the Rust-drawn `stash-clash-done-120x40`, not this pty one. | Designed. Re-shoot. |
| m5-07-160x48 | Bar gains `C` (switch) at the end of the tail, compacted to the key at this width. | `daef05e6` m7-power-tools-10: `crates/graft-ui/src/graph_screen/hints/lists.rs:194` `hint("C", "switch")`; ticket `10-branch-picker.md:134` (`C switch` in `TAIL`); keymap `docs/design/02-keymap.md:204` (`C` opens the branch picker). | Named, intended. Re-shoot. |

Latent, not acted on: `.scratch/m6-conflicts/pty-walk-m6.py` `Run::end` has the same unread wait (`time.sleep(0.05)`,
raw closed first). No m6 run hung in step 1 (0 `not gone` across 5 sizes), so it is left alone (Touches: named scenario
only). Graft cannot take SIGTERM while its stdout write is blocked (main thread in `write`); a real terminal always
reads, so this is only a harness hazard — a question for the conductor, not a finding.

**m6-06 (conductor's add, 2026-10-01; 30u stopped on it).** Reproduced the kill -9 flow keeping the scratch
(scratchpad copy of the walk with the `rmtree` swapped for a print): HEAD `b0eb655 main rewrites it` (the rebase's
detached HEAD, `rebase-merge/onto`), `2e2d51c feature writes again` = feature's tip = `rebase-merge/orig-head`, the
graph's top row. Three changes, each with its line:
- `diff vs HEAD 2e2d51c` → `b0eb655`: same cause as m6-18, `dc7da822` (m-b), `view/detail.rs:216-233`.
- 120x40 gains the `still to apply · not commits yet` box (`pick 2e2d51c feature writes again`), graph rows pushed down
  three: `ca2695b0` m7-power-tools-07c, `crates/graft-ui/src/still_to_apply.rs:1-20`; design
  `docs/design-notes/m7-power-tools.md:634-635` (R4). Not drawn at 80x24 (Detail tab only).
- `REBASING step 1 of 2` → `REBASING  step 1 of 2 · pick c3b01ea feature writes`: `20df04d2` m7-power-tools-07b,
  `crates/graft-ui/src/graph_screen/banner.rs:96-99` (30u's named cause, row :18 of its ticket).
Named, intended. Re-shot here on the conductor's word.

**m5-11, second walk fix.** The first re-shoot (wait for `the teammate's commit` only) was taken **mid-frame**: that
row drawn, the rows under `Commit 199` blank. graft wraps each frame in DEC 2026 (`event_loop/running.rs:27-31`), so
the walk now tracks the frame markers and `wait_for_whole` takes the text only when no frame is open. Used for the
`DRAWN_TOO` wait only; every other wait is unchanged.

### Re-shoot (13 files) and second compare
- Update: `MODE=update sh target/pty-logs/reshoot.sh` (+ m5-11 and m6-06 by hand, `GRAFT_UPDATE_SHOTS=1`).
- Compare: `MODE=compare RUN=2 sh target/pty-logs/reshoot.sh` — every targeted run rc=0: m5 80x24, m5 160x48, m6-18 ×5,
  m6-21 ×3, m6-06 ×2. The same script's full m5 walk at both sizes: every other m5 capture `settled` (unchanged by the
  script edit), m5-11 green a second time, only `m5-01` drifts (reshoot-named, not touched) and `m5-05` not on screen
  (38x10 only). No `not gone` line anywhere after the `end()` fix (it printed at both sizes before).
- Old vs new, per file (`git diff -U0`):
  - m5-07-160x48: :50 only, `  C` appended — `daef05e6`.
  - m6-18 ×5: one line each, `1e78110` → `a301516` — `dc7da822`.
  - m6-21 ×3: one line each, `▎` on the diff's first row — `130dd529` (20b).
  - m6-06 80x24: :4 banner, :7 SHA. 120x40: :4 banner, box rows :5-7, three blank rows gone at the bottom, SHA — causes
    above.
  - m5-11 ×2: every graph row (the post-fetch graph: `[◆ main ↑1 ↓1]`, `ⓘ` the radar marker, `the teammate's commit
    0657dff` on `origin/main`, `●─╯ Commit 200`, one commit fewer at the bottom), header unchanged (`184 rows so far` at
    160x48). The frame the walk now waits for; two probe runs per size byte-identical.
- Privacy: `graft-fixtures redact <13 files>` → `0 of 13 files changed`. Sizes: largest 7797 B (m5-11-160x48).
- TESTING.md: :2106 quoted `[◆ main ↑1]` (m5-11's old frame) → updated. `grep -n "main ↑1\]\|1e78110\|2e2d51c\|REBASING
  step 1 of 2" TESTING.md` → 0 hits. :2105 (m5-07) quotes the bar up to `q …`, still true; :2359, :2615, :2618 quote
  nothing that changed.

### Touches
- Walk scripts: only `pty-walk-m5.py` (`end()` drains; `DRAWN_TOO` + `wait_for_whole` for m5-11). Checked by the full
  m5 compare above. `pty-walk-m6.py` not changed (same latent `end()`, never hung). `python3 -m py_compile` ok.
- `git diff --stat crates/` empty. Contact sheets, caption tsvs: not touched (`git status` lists none).
- 30u's list: m6-06 re-shot here (conductor); none of its other 13 files touched.

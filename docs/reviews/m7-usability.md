# M7 stop — whole-app usability walk

**Counts: Critical 0 · Major 6 · Minor 9 · Questions 2.** Plus 10 known rows observed still standing (cited, not re-filed).

Reviewer, read-only, against `78f29ea1`. CLAUDE.md "Every milestone ends with a whole-app usability walk". Scored against
`docs/research/ux-principles.md` (**UX §n**; heuristics **H1–H10**) and `docs/design/02-keymap.md` (**§n**).

## How this was checked, and what was not

- **Driven:** the release `graft` binary built in a throwaway worktree at `78f29ea1` (own `CARGO_TARGET_DIR` outside
  the repo), run under a pty (pyte, 187×30 to 187×44 — his width, field notes :57), Unicode tier forced by config. One
  scratch world: a bare origin, his clone, a teammate's clone that pushed a commit; branches `main`, `fix` (conflicts
  with `main` on `notes.md`), `feature-a` with `feature-b` built on it; a stash; uncommitted edits (his usual state).
  The screen was dumped after each key group. "Observed" below means seen in those dumps. Driver, repos, worktree and
  target dir were deleted afterwards; nothing from them is committed.
- **Behaviour:** each behavioural finding names the file:line that proves it, read at `78f29ea1` (rule 7).
- **Not run:** gates, heavy tier, release perf. The concurrent reviewer of 30p–30w owns those; two heavy runs at once
  would break PROMPT.md 7.1. This walk is not evidence that the gates pass.
- **Colour and look: not judged** (rules 5, 6). No finding rests on colour.
- **Not reached:** 80/120-column layouts; mouse and drag; a real network remote and credentials; the `e`/`$EDITOR`
  round trip; a rewrite conflict stop (read from `docs/shots/m7/rewrite-chain-stop-187x50.txt` only); absorb; `F` file
  history from a file row; bisect (not built).

## Daily tasks, walked

Counts per UX §1.3 (typed string = 1, repeated motion = 1). "Note" = the design note's `journey-counts` fence.
✓ holds · ✗ fails (finding id) · – n/a.

| Task | Keys observed | Note | H1 | H4 | H6 | H9 | 3.1 |
|---|---|---|---|---|---|---|---|
| Stage one file, commit (from launch) | `k ⏎ space c <msg> ⏎` **6** | 6 (m6) | ✓ | ✓ | ✓ | ✓ | ✓ lands on the new commit |
| Push, rejected by a teammate's push | `P y` → `push rejected … · p pull first` | – | ✓ | ✓ | ✗ row 766 | ✗ row 766 | ✓ |
| …recover with a dirty tree | `p` (diverged) `r r P y` **5** | 5 | ✗ m1 badge | ✓ | ✓ | ✓ | ✓ `your edits are back` |
| Merge `fix`, one conflict, finish | `m m o ⏎ y` **5** | 4 (`O`) | ✓ | ✓ | ✓ | ✓ | ✓ | 
| Switch branch with edits | `C feature-a ⏎ y` **4** | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Rebase my branch + the one built on it onto main | `r y` **2** (cursor already on main) | 3 | ✗ **M1** | ✓ | ✓ | ✓ | ✓ |
| Squash the last three | motion `R s s y` **5** (launch is not on HEAD) | 4 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Fix into an older commit, a branch forked from it | motion `a y` **3** | 3 | ✗ **M6** | ✓ | ✗ **M4** | ✓ | ✓ |
| Cherry-pick a commit in view | motion `o y` **3** | 3 | ✓ | ✓ | ✗ **M4** | ✓ | ✓ |
| `edit` stop, continue | `R j e y`, then `c` | 0 to land, `c` 1 | ✓ | ✗ m3 | ✗ m3 | ✗ m3 | ✓ |
| Blame a line → its commit → back | `B ⏎ Esc` **3** (first `B` built a commit-graph) | 2 + 1 | ✓ | ✗ **M3** | ✓ | ✓ | ✓ |
| Line before that commit; history of the function | `^`, `Esc`, `L` | 2, 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Commit lost to a shell `reset --keep` | `J ⏎ y` **3** | 3 | ✗ **M5** (other rows) | ✓ | ✓ | ✓ | ✓ `↺ put back`, `○ feature-a was here` |
| Stash save, apply | `S s <msg> ⏎`, `⏎ y` | – / 3 | ✗ m5 | ✓ | ✓ | ✓ | ✓ |
| What will my PR contain | `x` motion **2** | 2 | ✓ | ✓ | ✓ | ✓ | ✓ |
| Undo a commit, redo it | `u`, `U` | – | ✓ | ✓ | ✓ | ✓ | ✗ m2 |
| Help | `?` does nothing | – | – | – | ✗ row 840 | – | – |

**Fixed since the M6 walks (observed):** a dirty-tree rebase preview now sets edits aside instead of refusing after
`y` (M6 N4); the `.` menu holds Checkout first and Cherry-pick second (N7); commit, merge, rebase, rewrite and
cherry-pick land the cursor on the result (m5); an untracked file reads `new` (n1); stash internals are no longer drawn
(row 846 — a commit only a stash holds draws `□ [only in stash@{0}]`); `(M5)` is gone from the diverged line.

## Critical

None. No path found that loses work without a backup. Every rewrite, rebase, cherry-pick and put-back I ran left a
`refs/graft/backup/**` ref and a journal row, and `u` took back the one I tried.

## Major

**M1 — A rebase preview hides its own result when the target is the top row of the screen — which is where the app
opens.**
- Task: rebase my branch (and the one built on it) onto `main`, weekly (note journey 6).
- Principles: the M5 promise "you see the result before you confirm"; H1; UX §3.6.
- Evidence:
  - `preview.view_start` is the row that was at the viewport's top when the preview opened
    (`crates/graft/src/update/preview/answer.rs:127-131`). The ghosts of a rebase onto `main` hang **above** `main`
    (`docs/shots/m7/rebase-line-preview-187x50.txt`: `after rebase` block over `[main] m2`), so when `main` is that top
    row they are before `view_start` and never drawn.
  - The app opens with the cursor on row 0, the newest tip (row 842) — after a pull that is usually `main`.
  - Observed at 187×44, cursor on `main` (row 0): `r` drew the box (`feature-a → 3 new commits`, `feature-b → 4 new
    commits · built on yours · follows`) over a graph with **no** `after rebase` label and **no** `○` row; the
    uncommitted row scrolled off too. `↑`/`PgUp` do nothing in the preview (it swallows keys, §2.3b).
  - The acceptance capture starts with `main` mid-screen, so it cannot catch this.
- Direction: start the picture at the first ghost row when it is above `top`, keeping the cursor row in view; a view
  test with the target on row 0.

**M2 — M7's daily-row ticket was never built; four daily annoyances stand and their rows expire with M7.**
- Task: every daily task (push, commit, every branch row, every repo opened).
- Principles: rule 4 (never mark half-done work done); H1, H6, H8, H9; the note's R7 (`docs/design-notes/m7-power-tools.md:770-786`).
- Evidence:
  - `.scratch/m7-power-tools/issues/15-daily-wording.md:3` reads `Status: draft`. The log records its respec (`docs/log/m7.md:249`), no build.
  - Observed, all four still as the rows say:
    - `push rejected: origin/main has moved · p pull first · P force with lease`; `p` then answered
      `diverged · m merge it  r rebase onto it` (row 766).
    - A fresh commit editor reads `subject empty` before a key is pressed (row 768).
    - `ⓘ` on `fix`, `feature-a`, `feature-b` — every branch with no upstream — and the bar leads with `. explain` on
      them (row 769).
    - `no commit-graph file: history sorts on every open · B build one …` on a 12-commit repo, one row on every screen
      until answered (row 770). The offer is decided with no timing at all (`crates/graft/src/update/commit_graph.rs:32-34`).
  - The M7 acceptance is marked finished (`b73574e6`, `78f29ea1`) with this ticket open. Nothing I read ticks it, so
    this is not a false tick; it is a milestone stop with part of its own design unbuilt and not listed as a gap.
- Direction: build 15 or move it, with its four rows re-expired, into KNOWN_GAPS by name before the checkpoint.

**M3 — `B` blames in the diff pane, except on any repository without a commit-graph, where the first `B` writes one.**
- Task: blame a line, weekly (journey `blame-line-to-commit`, 2 keys).
- Principles: H4 (same key, same meaning); §2.1 "one meaning per verb".
- Evidence:
  - The pane gives `B` to the offer while it can be acted on (`crates/graft/src/update/detail/keys.rs:77-80` →
    `place::b_key`); the offer's `B` writes (`crates/graft/src/update/commit_graph.rs:45-47`).
  - Observed: `⏎` on a commit, `⇥`, `B` → `commit-graph written: 56 commits · … Ctrl+R to use it`, no blame. The bar
    had left out `B blame` while the offer was up, and drew it once the offer was gone — so the bar is honest, but the
    key he learned in the prototype ("B and enter is enough", field notes :70) does something else on first use.
  - §2.1's `B`/`X` exception was written for M1, before M7 gave `B` to blame and history. With M2's ticket 15 unbuilt
    the offer is up on every repo he opens for the first time.
- Direction: give the offer keys nothing else claims, or land 15's ≥ 300 ms rule; either way amend §2.1.

**M4 — The two M7 commit-row verbs, `o` (take a commit, daily) and `a` (put staged changes into an older commit), are
on no hint bar.**
- Task: cherry-pick (daily); fix into an older commit (weekly).
- Principles: H6; §2.1 "a row or ambient verb that never appears anywhere is a finding, not a default".
- Evidence:
  - Observed bar on a commit not on his branch at 187 columns: `⏎ open b branch r rebase t tag c commit y sha m merge
    u undo S stash J journal / search q quit F2 status F3 renderer . more x + j/k space 1 F H f p P M O C` — no `o`,
    no `a`. The only teacher is the `.` menu's `Cherry-pick e5290a5 (o)`.
  - The guard test `every_bound_action_is_hinted_live_or_is_named_here` reads a hand-written table,
    `crates/graft/src/update/keys/bound.rs`, which lists `o` only as the resolver's take (`:96-98`); the graph's `o`
    is not in it, so the test cannot fail for it.
  - `a` sits in `NEVER_HINTED` as an "open question, not a decision" since m5/33
    (`crates/graft/src/update/tests.rs:5588-5595`); M7 widened what `a` does and left it there.
- Direction: add graph `o` to `bound_actions`; rank `o take` on rows off his branch and `a` on unpushed rows of his
  branch; retire the `a` exemption.

**M5 — The journal shows git's scratch commit from a squash as lost work, owned by a branch that never held it.**
- Task: find lost work (rare, high stakes — note Need).
- Principles: H1, H2; the M7 promise "recover history without leaving the app" rests on `✗` rows being real.
- Evidence:
  - Observed after `R s s y`: `✗ lost 013a232 "# This is a combination of 2 commits." · was feature-b's tip · no
    branch holds it`.
  - `git reflog feature-b` never holds `013a232`; only `HEAD@{8}: rebase (squash): # This is a combination…` does.
  - A reflog record on a detached `HEAD` that is not a checkout falls back to the inferred branch `on`
    (`crates/graft-ops/src/journal_list/rows.rs:207-221`), which becomes `HeldBy::Branch` (`:238-243`) and the words
    `was {name}'s tip` (`crates/graft/src/view/journal.rs:222-223`).
  - Every squash rewrite adds such rows; `n` (next `✗`, §2.11) stops on them, and `⏎` would offer to put `feature-b`
    on a commit titled with git's comment.
- Direction: drop intermediate `rebase (…)` steps of a Graft rewrite from the lost set (the rewrite's backup already
  holds the real before); never name a branch whose own reflog lacks the commit.

**M6 — A rewrite strands a branch forked from a rewritten commit, and the preview does not say so.**
- Task: "I changed a branch that others are built on, now fix the chain" (note Need, 2026-09-19); journeys 2 and 7.
- Principles: H5; the M5 promise; field notes :76 decision 1 ("dependent branch follows when he rewrites his").
- Evidence:
  - `feature-b` forks from `dd21dca` ("feature a: part 1"), which is not `feature-a`'s tip. `a y` on `dd21dca`:
    the preview named `feature-a da9880d → 3 new commits` and nothing else. After: `feature-a` rewritten,
    `feature-b` still on the old `dd21dca`, and the graph draws `feature a: part 1` twice (field notes :38, "in my
    history twice").
  - Only a branch whose first-parent chain reaches the tip follows (`crates/graft-ops/src/rewrite/dependents.rs:56-60`,
    ADR-0047 §4). That rule is decided; the preview's silence about the branch it leaves behind is not.
- Direction: the preview names each branch left on an old copy (`feature-b stays on the old dd21dca`). Whether such a
  branch should follow is a question for the planner.

## Minor

**m1 — The ahead/behind badge overcounts when commits share a timestamp.** Observed `main ↑6 ↓1` where
`git rev-list --left-right --count` says `1 1`, and `main ↑5` where it says `2 0`; the stale badge also rode into the
next rebase preview's ghost row. A commit popped as one side's only (`crates/graft-git/src/history/divergence.rs:108-110`)
is counted at once; the re-visit when the other side reaches it later (`:161-167`) fixes its flags, not the count. My
fixture's commits were made in the same second. Extends row 146 ("badly skewed clocks") to equal committer dates,
which are not skew.

**m2 — Undo and redo do not land.** After `u` on a commit the cursor sat on the stash row and the pane showed the
stash; after `U` it stayed off the redone commit. `land_after_undo` acts only when an operation is in progress
(`crates/graft/src/update/undo.rs:270-277`). UX §3.1; the R7 landing (note :774) covered writes, not undo.

**m3 — At a stop the bar offers verbs that refuse, and `m` refuses for the wrong reason.** At an `edit` stop the bar
reads `⇥ banner a amend it c continue space stage . explain ⏎ open b branch r rebase … m merge u undo`; `u` answered
`finish or abort the rebase first` and `m` answered `m needs a branch checked out`. The live list keeps the refused
write keys, ranked after `c` (`crates/graft-ui/src/graph_screen/hints/lists.rs:445-468`), against §2.1 ("a key whose
verb is not valid at the cursor" never appears). `m`'s words come from the detached-HEAD check
(`crates/graft/src/update/preview/history.rs:105`), which runs before anything asks about the rebase.

**m4 — `m` and `r` on a stash row preview a merge or rebase onto the stash's internal commit, named by sha.** Observed:
`Preview: merge 387c83b into feature-a · no conflicts expected · [ Confirm ]`, and `Preview: rebase feature-a onto
387c83b`; the bar on that row advertises `r rebase  m merge  R rewrite`. Nothing refuses a stash commit as the target
(`crates/graft/src/update/preview/history.rs:92-106`). Undoable, hence Minor. With the launch cursor on row 0, a stash
newer than his branch is where he starts (observed on the first launch; row 842).

**m5 — `now ago` in the stash list.** Observed `stash@{0} "On feature-a: my edits" now ago`. `view/stash.rs:76-83`
has no `now` guard; the journal's copy of the same function does (`crates/graft/src/view/journal.rs:338-341`).

**m6 — `Contained in:` with nothing after it** on a stash commit's detail header (observed). An empty `Known` list
still formats the label (`crates/graft-ui/src/detail_pane/header.rs:211`). H8: show nothing, or say `no branch`.

**m7 — History shows two numbers for one commit.** List row `aacce94 initial app +3`, file-pane header `aacce94 · … ·
this file +12 −0`. The list counts the traced lines, the header the file; only the header says which. Textual (from
the dump).

**m8 — The chain preview's numbers don't add up.** `4 commits replayed` beside `feature-a 6234adb → 3 new commits`
and `feature-b c28b23d → 4 new commits`; each ref's count includes the commits under it
(`crates/graft/src/update/preview/plan_lines.rs:128-131`). Same in `docs/shots/m7/rebase-line-preview-187x50.txt`
(4 replayed; 2, 3, 4 new). Textual.

**m9 — A refused pull is a journal row that reads like a reverted write.** Observed twice, one per `p`: `pull
origin/main  failed: diverged: not possible to fast-forward · 1 ref · the index · the working tree · backup…  undone`;
the rejected push likewise ends `undone`. Nothing was written. Beside row 797 (`failed: aborted`). Textual.

## Questions (no line proves them)

- **Q1.** The default squash message reads `part 1`, `part 3`, `part 2`: the kept row, then melded rows newest first
  (`crates/graft-ops/src/rewrite/todo.rs:44-66`, chosen after pass 1b). Read top to bottom it is neither order. Does he
  want this, now that he sees it on three rows?
- **Q2.** After `/root ⏎`, the search line `/root 1 match (1/1) n/N next/prev Esc close` stayed on the bar through
  `⏎` (pane open), `⇥`, `x` and the compare — the pane's own keys never showed. Is the search meant to hold the bar
  until `Esc`? Observed; not traced.

## Known rows observed still standing (cited, not re-filed)

| Row | What I saw |
|---|---|
| 766, 768, 769, 770 | see M2 |
| 828 | `yours` is the left pane in the merge (`yours · main`), the right in a rebase stop (`yours · a05fa97 b1`, capture) |
| 836 | a stash of 2 files (one untracked): `space files` lists 1 |
| 840 | `?` does nothing (pressed at launch) |
| 842 | launch cursor on row 0: the stash, then `main`, then `main` with `feature-a` checked out |
| 844 | name prompt draws `⏎ create ⇥ toggle esc cancel` in its border and on the bar |
| 845 | one-file merge walked `m m o ⏎ y` = 5 against the note's 4 |

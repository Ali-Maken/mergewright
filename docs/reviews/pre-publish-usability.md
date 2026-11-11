# Pre-publish — whole-app usability walk

Reviewer, 2026-11-11, at **5f0784401** (detached worktree `.claude/worktrees/audit-walk`). His daily tasks (memory
`graft-daily-workflows`: history, diffs, stage and commit, fetch/pull/push, branch and switch, conflicts, stash,
cherry-pick; `docs/field-notes.md` to 2026-11-10) walked through every screen they touch, scored against
`docs/research/ux-principles.md`. Previous walk: `docs/reviews/ease-pass-usability.md` (55cfdbc5).

**How it was walked.** The real `mwr` binary (debug build), in a pty, at 187×45 (his Zed width), 120×30 and 80×24,
`TERM_PROGRAM=vscode` (Unicode tier), config in a temp folder. A team-shaped repo built for the walk: bare `origin`,
`main` 1 behind `origin/main`, a teammate branch merged `--no-ff` (folded), `wallet-release` → `feature/receipts` stacked
and pushed (1 unpushed commit), `integration` (Bilal) clashing with `main` on `report.md`, a stash, a modified file, a
new file and a new `.env`. Frames read as character grids through `pyte`. Also run: `cargo test -p mergewright --test it
ease_walk` — **93/93 passed, 120 s wall**. **Colour: unverified everywhere** (rule 5); look and feel are his (rule 6).
Not walked: Ghostty / Pixel tier, mouse, the rewrite editor, blame, file history, Map, the tour, a real network remote.
`pyte` prints a kitty-graphics probe's tail (`=1`) into the header's first cells; that is the harness, not the app.

Out of scope by the brief: codename / capture-id differences (11d, 12 in flight); the second line on the cursor row (by
design); the file list living in two implementations (known, field notes 2026-11-10).

## Scorecard by daily task

| Task | Keys walked / note / best tool | Lands where | Next step visible? | Recovery | Score |
|---|---|---|---|---|---|
| Open the app | — | HEAD's row, right pane answers the 4 questions | yes, row bar | — | ok |
| See whose line is whose | 0 | — | right pane per row | — | ok; names Mi2 |
| Open a commit's diff | `⏎ j` 2 on a one-file commit / — / SM 1 click | file list on the folder row: `select a file` | `⏎ open` | `esc` | **Q1** |
| Commit everything | `c type ⏎` 3 / 3 / JetBrains 3 | new commit, `✓ Committed 2 files to feature/receipts.` | `u undo the commit… · P push` | `.env` kept out with `!` | ok |
| Undo the commit | `u` 1 | HEAD, `✓ undo: commit "quick"` | — | `J` shows `⇢`/redo | ok |
| Push | `P y` 2 | `✓ pushed feature/receipts → origin/feature/receipts` | box says it can't be undone | — | ok |
| Fetch / pull (ff) | `f` 1, `p` 1 | pull lands on the new tip | — | — | ok; bar U6 still |
| Push rejected, pull diverged | `P y` → `p` → `m` | merge preview of `origin/main` | `p pull, then m or r`; `m merge it  r rebase onto it` | `m` works from HEAD's row | ok |
| Switch branch, unsaved edits | `C type ⏎ y` 4 | edits clashed → clash screen | `esc, u undoes the switch too` | `u` | ok; words Mi4 |
| Merge, one clash | `m y o y y` 5 / 5 / lazygit 4 | first clash → review → merge commit | yes at every stop | — | ok; Mi1, Mi3 |
| Merge, clean (via diverged) | `m` → `y` 2 / 2 / lazygit 2 | preview reached; `After y the merge is committed.` | yes | `esc` | ok (`y` not pressed) |
| Pause, resume | `esc ⏎` 2 / 2 | graph + banner → first open clash | banner `⏎ back to the clashes` | — | ok |
| Relaunch mid-merge | `⏎` 1 | changes pane on raw markers, not the paused place | banner `⏎ resolve` | — | **Mi5** |
| Undo a pick | `u` 1 / 1 / lazygit 1 | the clash, open again | — | — | ok |
| Open a folded merge | `space` 1 / 1 / SM 1 | merge opens, cursor stays | `space hide the commits inside` | `space` | ok; **M3**, Mi2 |
| Cherry-pick | `o y` | — | `o take` | — | **M1** |
| Stash list | `S` 1 | list | `⏎ apply  p pop  s save  d drop…` | `S/esc close` | ok |
| Help on a place | `?` 1 | its keys, grouped | — | `esc` | **M4** on the clash screen |

The other three counted journeys (`commit-with-body` 6, `commit-leave-first` 6, `merge-three-clashes` 8) were not walked
by hand; their `ease_walk` walks passed in my run. No counted journey walked over its note's count.

**Previous walk, rechecked:** U1 (in-sync branch reads "not on a branch") **fixed** — after `P y` the pane reads
`feature/receipts (you're here)` / `✓ pushed`. U3 (route names a merged-in branch) **fixed** on this repo shape —
`built on main ─◆ wallet-release ─◆ feature/receipts`. U2 **partly**: `wallet-release` is now named in the branch
column on its first row but still draws on `feature/receipts`' lane (`║ ● ┊`). U4 still open (Mi1). U6 (no `p`/`f` on
the bar) unchanged, still his question. U7 (two separators) unchanged: commit screen, merge preview, picker, banner use
two spaces; graph, clash and review bars use ` · `.

## Critical

None.

## Major

**M1. `o take` is offered on every commit already in your branch, and `o` then refuses.** On `receipts: view`
(an ancestor of HEAD `feature/receipts`) the bar reads `⏎ open · o take · P push · …`; `o` answers
`e88e832 is already on feature/receipts · esc dismiss` (pty, 187×45). The bar's predicate treats only HEAD's tip as
"on HEAD": `take_check` passes `id == head.oid` as `on_head`
(`crates/mergewright/src/update/context_menu/verbs.rs:177-178`), while the preview refuses any ancestor. The bar's own
doc says it can't happen: "so the bar never hints a row the key refuses"
(`crates/mergewright-ui/src/graph_screen/hints/lists.rs:128-132`). Why it matters: every row of his own branch below the
tip — the rows he looks at most — teaches a key that refuses (H4, H6; same class as M7/u-m3). Fix: decide `on_head`
from the graph's ancestry the app already holds (the commit is reachable from HEAD), so bar, `.` menu slot 2 and `o`
agree; a test on an ancestor row.

**M2. "force-pushed: not checkable — no remote-tracking reflog" on a branch pushed once, whose reflog exists.** The
detail header of every commit on `feature/receipts` reads it (120×30, `⏎` on the HEAD row);
`.git/logs/refs/remotes/origin/feature/receipts` exists. `probe_force_push` reads the entry *before* the current one;
a ref with only its creation entry gives none (`previous_tips_on_a_fresh_branch_with_only_its_creation_entry_is_empty`,
`crates/mergewright-git/src/history/tests.rs:985`) and the probe calls that "no remote-tracking reflog"
(`crates/mergewright-ops/src/mistake_radar/probes.rs:344-346`), drawn by
`crates/mergewright-ui/src/detail_pane/header.rs:118`. Why it matters: the words are false, and a warning-shaped status
when nothing is wrong breaks the CLAUDE.md status-field rule on the daily case (a branch pushed once and not fetched
since). It answers open question M6/u2 Q-a: the right ref is read; the words are wrong. Fix: one previous entry missing
= the remote has not moved since you first saw it → say nothing (or `not force-pushed since first seen`); keep
"not checkable" for a ref with no reflog at all.

**M3. A pushed branch reads `not pushed yet` next to its own remote copy.** Opening the folded merge draws
`fix-toc  toc: anchors  sara/fix-toc is here  not pushed yet  origin/sara/fix-toc is here` — one row, two contradicting
facts. `sara/fix-toc` was pushed with `git push origin sara/fix-toc` (no `-u`), so it has no upstream configured;
`Upstream::None => AheadOfUpstream::NotPushed` (`crates/mergewright/src/refs/ahead.rs:24`, doc `:37` "No upstream
configured, or the ref it names does not exist") draws `not pushed yet` (`crates/mergewright-ui/src/graph_screen/row_text.rs:261`;
the right pane's `crates/mergewright/src/view/branch_pane.rs:221` reads the same). Why it matters: "did I push this?"
is the question he asked the graph to answer (field notes 2026-10-18); a branch pushed without `-u` is common, and the
answer is wrong while the evidence sits on the same row (H1). Fix: with no upstream, fall back to
`<default remote>/<name>` when that ref exists (pushed, ahead/behind against it), else `not pushed yet`; or say
`no upstream set` rather than claim it isn't pushed. Trade-off: the fallback guesses the remote name, as `git push`'s
default does.

**M4. `?` on the clash screen lists `c` twice with two meanings, and banner keys that do nothing there.** The list
reads `IN PROGRESS: c Continue · a Abort · M Launch the mergetool on the first conflicted file · e Open the first
conflicted file in $EDITOR`, then `HERE: … c Finish … e Open $EDITOR at this block · M Launch the mergetool`. On the
clash screen `a` did nothing (pty: frame unchanged). The IN PROGRESS group is the banner's keys whatever the place
(`crates/mergewright/src/update/palette/groups.rs:39-54`), and the de-duplication is by action, not by key (`:55`), so a
key the place answers itself is listed under both. Why it matters: same key, two meanings, in the one list meant to say
what a key does here (H4, H10; UX §2 row 10 "`?` in conflict view lists only conflict-view keys"). Fix: when the place
is modal (resolver, review), drop banner rows whose key the place binds, or omit IN PROGRESS there.

## Minor

**Mi1. "conflict" and "block" on the clash screen, "clash" everywhere else** (U4, still open). Title `▸ report.md ·
1 conflict · 1 open` (`crates/mergewright/src/view/resolver.rs:195-196`); after the take `all blocks taken · y reviews
the merge` (`crates/mergewright/src/update/resolver/take.rs:238,241`); help `n Next block`
(`crates/mergewright-ui/src/keymap/table/resolver.rs:41`, also open row ease/05r-n-label — `n` goes to the next file);
banner `1 file conflicted`. Beside it: `Clashing files`, `Every clash in report.md is solved`, the preview's `You
decide 1 file clashes`. Heuristic 4; the mock's words are "clash".

**Mi2. One branch, two names on one screen.** Opened merge: the branch column and second line say `fix-toc` / `merged 2
from fix-toc` (last segment, `crates/mergewright/src/view/entries/second.rs:76-78`), the fold label and badge on the next
rows `sara/fix-toc`. On a team using `<person>/<topic>` names, `sara/fix-toc` and `bilal/fix-toc` would read alike
(§3.6: different things must look different). Fix: full short name where it fits, last segment only under width
pressure.

**Mi3. After a take the footer shows the took line and drops the keys.** Live, one-file merge, after `o`: footer
`✓ took yours · main in report.md · 0 of 1 open`, no `u undo pick` / `esc pause` (pty 187×45), set by
`crates/mergewright/src/update/resolver/answers/taken.rs:171-175`. The captures show the key bar in that state
(`docs/shots/ease-pass/merge-clash-taken-120.txt`, `merge-file-solved-120.txt` last rows), so the captures don't match
what he sees. `y` is still on screen three times (title, list, solved line), so the next step is visible; undo is
not, at the moment he would want it (H3, H6). Also those three `y review` lines say one thing three times (H8).

**Mi4. Git's plumbing words on the clash screen and the changes pane.** Switch with an edit to a file `main` lacks:
title `src/receipts.rs · deleted on branch · main, modified on yours` (user words) and, under it, `deleted by us · o
deletes it  t keeps yours` (git's words, `crates/mergewright/src/view/resolver/whole_file.rs:36` →
`crates/mergewright-git/src/status.rs:225`). "us" is `main` here, not him (H2, and the fact twice). The relaunch's
`CONFLICTS (1)  report.md  both modified` comes from the same words (`status.rs:220`). Fix: drop the git words where the
title already says it; elsewhere use the title's wording.

**Mi5. A relaunch mid-merge does not land in the paused place.** In-session `esc` → graph, cursor where it was, banner
`merge stopped · 1 file conflicted · ⏎ back to the clashes`. After `q` and relaunch → changes pane open on the
uncommitted row, the diff showing raw `<<<<<<< HEAD` / `>>>>>>> integration`, banner `merge still in progress ·
1 conflicted · ⏎ resolve` (`crates/mergewright/src/update/relaunch.rs:192-216`, the M6 landing). The note says "A
relaunch mid-merge lands in the same paused place" (`docs/design-notes/ease-pass.md`, Journeys, Pause). Still 1 key to
the clash; the difference is what he reads first.

**Mi6. The trunk's pane says one fact twice and is built on itself.** On `main`: `! 1 new commit to pull` and
`! origin/main has 1 new commit since it left` (`crates/mergewright/src/view/branch_pane.rs:224` and `:188-195` — for the
trunk the base is its upstream), then `built on main` with no other stop. Field notes 2026-09-21 "same fact twice".
Fix: for the trunk, the behind line goes (the pull line says it) and the route is not drawn.

**Mi7. The help names the merge in other words than the bar.** Bar `m merge integration into yours`
(`crates/mergewright-ui/src/graph_screen/hints/lists/rows.rs:76`); help `m Merge selected branch into current`, `r Rebase
current onto selection` (`crates/mergewright-ui/src/keymap/table.rs:144,150`). Heuristic 4, CONTEXT.md terms; the
palette's own doc already says "into yours" (`crates/mergewright-ui/src/palette.rs:106`).

## Questions

**Q1. Should `⏎` on a commit show a diff straight away?** A one-file commit opens with the cursor on the folder row
`▾ src/` and the diff side reads `select a file` (187×45 and 120×30); one `j` reaches the file and draws its diff. 2 keys
(`⏎ j`) against Sublime Merge's 1 click. Not traced to a line: whether the first file is meant to be selected on open.
It overlaps his 2026-11-10 note on the folder tree costing rows.

**Q2. `space` folds "sara/fix-toc" by its last segment on purpose (ease-pass 07) — does he want the full name?**
See Mi2; it is a design rule, so asked rather than filed as broken.

**Q3. The `.` menu on HEAD leads with two dimmed rows** (`Checkout — already on main`, `Cherry-pick (o) — already on
main`) and a diverged-pull refusal stays drawn under it after `esc` from the merge preview. Already open as M6/u2 m-g;
observed again, no line traced.

## What changed for the better (for the record)
- U1 and U3 from the last walk are gone on this repo shape; the right pane answers pushed / merged / behind / who on
  every branch row walked.
- Push rejected → `p` → `m`: every refusal named the next key and the key worked from where the cursor was.
- Switch with an edit that clashes lands on the clash with `esc, u undoes the switch too`.
- Every counted journey walked at its note's count; commit and merge land on the commit they made.

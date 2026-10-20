# Ease pass — whole-app usability walk

Reviewer, 2026-10-20, at **55cfdbc5**. His daily tasks (memory `graft-daily-workflows`: history, diffs, stage and
commit, fetch/pull/push, branch and switch, conflicts, stash, cherry-pick; plus `docs/field-notes.md` to 2026-10-19)
walked through every screen, scored against `docs/research/ux-principles.md`.

**How it was walked, and what that cannot show.** Not by hand in a terminal: from the ease-pass captures
(`docs/shots/ease-pass/*.txt`, 55 files, character grids drawn from real repos), the bar words in code, the keymap
(`docs/design/02-keymap.md`), and the `ease_walk` tests, which drive the real app in-process and all passed in my run
(7199/7199). Key decoding in his terminal (Zed, Ghostty) was not exercised — 14's walk is in-process, not a pty
(`ease/14-accept-in-process`). Colour: **unverified** everywhere (rule 5). Look and feel are his (rule 6).

Findings carry their line (rule 7); textual ones stand on the capture.

## Scorecard by daily task

| Task | Steps (built / note / best tool) | Lands where | Next step visible? | Recovery | Score |
|---|---|---|---|---|---|
| See whose line is whose, who did what | 0 keys: second line + right pane (`main-cursor-*-120.txt`) | — | yes: bar per row kind (G13) | — | **broken on two daily shapes** (U1, U2) |
| Open a commit's diff | `⏎` 1 / — / SM 1 click | detail pane | yes (`⏎ open` first on every commit bar) | `esc` | ok |
| Commit everything | `c type ⏎` 3 / 3 / JetBrains 3 | graph, cursor on the new commit, `✓ Committed …` (C13) | yes, `u undo the commit, your edits come back · P push · c commit again` | refused: `⏎ put a file in first`, `type a message first` on the bar (`commit-refused-120.txt`, `commit-open-120.txt`) | ok; bar extras U5 |
| Commit with a body | `c type ⇥ type esc ⏎` 6 / 6 / lazygit 5 | as above | `⇥ more detail` on the bar | `esc` keeps the message (C15) | ok |
| Leave a file out | `c type ⇥ ⇥ space ⏎` 6 / 6 / lazygit 5 | as above | `space leave it out` (`commit-files-focus-120.txt`) | `space` again | ok |
| Push / fetch / pull | `P` 1, `f` 1, `p` 1 | — | **`P push` only**: `p pull` and `f fetch` are no longer on any row bar (built to the mock, `mergewright-main-screen.html:209`); `?` and `.` find them | — | question U6 |
| Switch to a teammate's branch | `C ⏎` 2 (Q1) / — / — | graph on his branch | `C switch to <b>` on the row | — | **broken for in-sync branches** (U1) |
| Merge, one clash | `m y o y y` 5 / 5 / lazygit 4 | first clash, then review, then the folded merge | yes at every stop: `After y you go straight to the first clash.`; `✓ Every clash in report.md is solved.  y review the merge`; review bar | unrelated: `Can't merge: stray and wallet-release share no history.` (`merge-unrelated-120.txt`); hook failure stays on the review (M15) | ok; words U3, U4 |
| Merge, clean | `m y` 2 / 2 / lazygit 2 | folded merge, `✓ Merged …` | `u undo the merge · P push · …` | `u` (M19) | ok |
| Pause a merge, come back | `esc ⏎` 2 / 2 | graph, banner focused → first open clash | banner | relaunch lands paused | ok |
| Undo a pick | `u` 1 / 1 / lazygit 1 | the clash, open again | — | — | ok |
| Stash, cherry-pick, rebase stop | unchanged from M6–M8 (`S…`, `o y`, `r …`) | — | `o take` on teammate rows | rebase clash lands on the clash (Q17 a) | ok; key words U7 |

All 9 counted journeys walk at their note count (`ease_walk::journeys::*`, green). None over.

## Findings

### Major

**U1. A branch level with its remote reads "not on a branch".** Every branch he has just pushed is level with its
remote. For those, the right pane draws `<sha7> · not on a branch`, `C` opens the picker on its first row while the bar
promises `C switch to <b>`, and HEAD's `(you)` is lost. Seen on `ops/ci` (14's walk); HEAD reasoned from the same lines.
Lines and fix: group review Major 1 (`history/seeds.rs:44-51`, `update/branch_pane.rs:204-206`). Principles: heuristic
1 (status), 4 (the bar and the key disagree).

**U2. On a stack of his own branches, every commit says it was made on the top branch.** `feature/receipts` cut from
`wallet-release`: wallet-release's commits draw on feature/receipts' thick `┃` lane with no branch name
(`main-cursor-own-120.txt:8-9`); the mock gives the lower branch its own lane (`mergewright-main-screen.html:110,117`).
He works on dependent branches (field notes 2026-10-18 "stacked branches"). Group review Major 2. Principle: §3.6
different things must look different — two branches drawn as one.

**U3. The built-on route names a branch that was merged in.** `built on main ─◆ sara/fix-toc ─◆ wallet-release ─◆
feature/receipts` (`main-cursor-own-120.txt:12-16`) where sara/fix-toc was merged *into* wallet-release. The pane's
"behind what it was built on?" answer is about the wrong branch. Group review Major 3.

**U11. Scrolling in a repo with many refs lags.** Not walked by hand; measured: with 50 000 refs, p99 key-to-frame is
293 ms scrolling a diff, 292 ms with the detail pane open, 977 ms in the map view, against 16 ms (release run,
`docs/perf/ease-pass-review.txt`). Cause not established — group review Major 6. Heuristic 1: a key with no answer for
a third of a second reads as a missed key.

### Minor

**U4. "conflicts" on the clash screen's file title, "clash" everywhere else.** `merge-file-solved-120.txt:7` reads
`▸ report.md · 2 conflicts · 0 open`; the list beside it says `Clashing files`, the solved line `1 clash left`, the
mock's title is `<file> · N of M clashes open · K left in the merge` / `all solved · K left in the merge`
(`mergewright-merge-screen.html:148`). Heuristic 4 (one word per thing) and the mock (ADR-0063). Built words described
at `mergewright-ui/src/conflict_resolver.rs:96`. Overlaps open row `ease/11-title-words`.

**U5. Landing bars lead with keys the mocks don't have.** Commit landing: `. explain · u undo the commit, your edits
come back · P push · c commit again · a amend · ? keys` (`commit-landed-120.txt:45`) — the mock's three hints
(`mergewright-commit-screen.html:179`) now start second. Merge landing adds `a amend` between `P push` and `space show…`
(`merge-landed-120.txt`), and amending a merge commit he just made is not a next step he is likely to want. Heuristic 8
(every glyph earns its cell), ADR-0063. Group review Minor 3.

**U6 (question for him). Fetch and pull left the bar.** His own row's bar is `⏎ open · P push · c commit · a amend · u
undo · . actions · ? keys` (`main-cursor-own-120.txt`, last line). Fetch and pull are daily
(`graft-daily-workflows`), and the bar is how this app teaches keys (§3.4, heuristic 6). The mock he accepted shows
exactly `⏎ open · P push · c commit` (`mergewright-main-screen.html:209`), so this is built as agreed. One line for him:
"`p pull` / `f fetch` back on your row's bar, or keep the mock?"

**U7. Two separators, two meanings of "take".** Bars use ` · ` (graph, clash screen, review); the merge preview's
footer, the commit screen, the picker and `?` use two spaces (`merge-preview-120.txt` footer `y merge  esc cancel  ⏎
see the 3 commits`, `commit-90.txt`, `picker-on-branch-120.txt`, source `preview_box/merge.rs:256-270`). Small, but it is
the same bar in his eye. Separately: on the graph `o take` means "cherry-pick this commit onto yours"
(`main-cursor-teammate-120.txt`), on the clash screen `o take yours (left)`, and in a rebase clash `o take upstream
(left)` (`rebase-clash-120.txt`) — `o` keeps "left" in both clash cases and the words say which side, so this one is
mitigated; noted for the record, no change asked.

**U8. `b take` at 90 columns.** `merge-clash-90.txt` bar: `o take yours (left) · t take theirs (right) · b take · e
edit` — the short form drops the only word that says what `b` does (`hints/lists/clash.rs:73`). Short form `both`.

**U9. `⏎` commits on one commit screen and looks on the other.** Commit screen: `⏎ commit 3 files`
(`commit-90.txt`); merge review: `y commit the merge · … ⏎ look at it` (`merge-review-120.txt`). Both came from mocks he
accepted, and the review's `⏎` is read-only, so a slip costs nothing (heuristic 5 holds). Recorded because the M6
walk's finding was exactly this kind of key mismatch; no change asked.

**U10. A key pressed during a clash read is refused, not queued.** `c` the instant after a file's last take answers
`still reading the file` (`update/resolver.rs:296-301`); a second press works (14b NOTES). Words are given (heuristic 9),
so minor; queueing the one key would remove a step he cannot see coming.

## Consistency check: same key, same meaning

| Key | Graph | Commit screen | Clash screen | Review / look | Preview box | Verdict |
|---|---|---|---|---|---|---|
| `u` | undo (journal) | — (after landing: undo the commit) | undo pick | undo last pick | — | consistent: always "undo the last thing" |
| `esc` | — | cancel, message kept / back to message | pause | back to the clashes / back to the review | cancel | consistent: leave one level |
| `y` | copy SHA (§2.3) | commit, in the file list only | review the merge | commit the merge | confirm | consistent where it confirms; graph's `y` is the one exception, decided in M1 (ADR-0023) |
| `⏎` | open | commit | — | look at it | see the N commits | U9 |
| `space` | fold / unfold a merge | leave out / put in | — | same as `⏎` in the review | — | consistent: "toggle this row" |
| `n` | next search hit | — | next open clash / next file | — | — | consistent: "next" |
| `c` | commit | commit again | alias of `y` (not on bar) | — | — | consistent |

## What changed for the better (for the record)
- Commit is 3 keys and lands on the new commit with undo offered: the field-notes complaint "≥7 keys in Zed" is gone.
- A merge with one clash is 5 keys, the stop lands on the clash, and the solved file stays on screen with the next step
  written under it — the 2026-10-17 complaint ("immediately going to the next one") is answered as he asked.
- Every refusal seen (nothing to commit, no message, unrelated histories, busy reads) says why and what to do.

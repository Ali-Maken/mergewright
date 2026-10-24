# 27 — Re-check the conflict footer and key row against the user's rule

Status: done · Build with: Sonnet · Blocked by: none
Source: user 2026-10-24 on batch item 6: "drop the origin, keep the count"; triage `.scratch/ease-pass/scouts/user-batch-triage.md`; KNOWN_GAPS M6/21 rows (160x48 count dropped; 80x24 relaunch no words; unread `…`).

## Scope (measure first; fix only a confirmed break of the rule)
1. Capture the merge-stopped-on-conflicts footer, changes pane open and closed, at 80x24, 120x40, 160x48, 187x50, 200x50, for a merge begun in the shell and one begun in the app. Rule: when room is short, the origin words go first; the conflicted-file count stays as long as anything but keys fits. Any width that drops the count while keeping (or before dropping) the origin is a defect: red-first test, fix at cause (`update/banner/stop/wording.rs` and the fit it feeds), re-shoot.
2. 80x24 relaunch into a rebase left from a previous run: does the bottom row say so? Capture; report (question row stays a question unless the rule above covers it).
3. Resolver key row at 120 columns with `B base` shown (batch item 9): capture; does it still cut words after `d` and `esc` since ticket 24? Report with the capture; no fix unless a word is cut — then a question for the conductor.
4. Finish screen: does `Shift+Tab` move back between fields (batch item 8)? Test it; report.
5. Unread-file mark: confirm `•`/`~` (`conflict_resolver/files.rs:254,263`) in the current capture, so the `…` row can close.

## Acceptance
- [x] Footer captures at the five widths x two origins, listed; each verdict against the rule. Captures in one file, `docs/shots/ease-pass/27-footer-matrix.md` (one section per capture: `{shell,app}-{landing,pane-focused,pane-closed,banner}-{80x24,120x40,160x48,187x50,200x50}`; a `.md` so the ease-pass caption/sheet machinery, shared with ticket 26, is not touched) (shell = `b-launch-to-first-conflict`; app = `a-stop-to-first-conflict`, the resolver landing, `app-banner` after one Esc). Verdict, shell landing: 120 `from the shell · 2 conflicted`; 160 `merge in progress` + keys (no count, no origin); 187 `2 conflicted`, origin gone; 200 origin + count, `q quit` yielded; 80 keys only. App banner: `2 files conflicted` at 120/160/187/200, keys only at 80. No width keeps the origin without the count.
- [x] Any defect: none by the ticket's test (count dropped while origin kept / before origin). The ladder order is pinned by `update::banner::tests::stop_wordings_drop_whole_parts_lowest_value_first` (origin short, origin gone, step, count last); the captures above show no width breaking it.
- [x] Items 2-5 answered: see NOTES. Item 2 section `27-relaunch-rebase-80x24`; item 3 test `resolver_bar_at_120_with_base_keeps_the_words_of_d_and_esc` (mergewright-ui `hints/tests/typed_fields.rs`); item 4 tests `update::finish::tests::typing_and_tab_leave_the_review_unchanged` and `update::commit_editor::tests::enter_in_the_subject_commits_the_body_too` (line 153 presses `BackTab`); item 5 section `27-app-landing-120x40` (`notes.txt  •`).
- [x] Captures <=200 KB (one file, 118 KB); colour unverified (pty grids carry none); five gates + `scripts/walks.sh` (counts in the report); Touches: below.

## NOTES
Scout: none; the code read was one file pair (`wording.rs`, `fitting/stop.rs`) and the capture harness, small enough to read directly. Harness: `.scratch/ease-pass/pty-ease27.py` (+ `matrix27.sh`), adapted from `pty-walk-m6.py`; it needed the config dir `mergewright/` and `.git/mergewright/` (the old script wrote `graft/` and drew a rename notice and a `config: 1 problem` row over the footer, so the m6 pty script is stale for these two paths: a gap, not fixed here).

1. Verdict: no break of the rule as the ticket states it. Findings, each with its line:
   - Why 160 loses the count: the footer is drawn inside the graph area, which is narrow once the changes pane sits beside it (>=~150 columns), so 160 has less room than 120 (pane below, full width). `stopped_row` (`mergewright-ui/src/graph_screen/hints/fitting/stop.rs`, `stopped_row`) lets only `q quit` (`QUIT_YIELDS`) give way to a fuller reason, so `e edit`, `M mergetool` keep their cells and the count (15 cells) does not fit. Question for the conductor (footer fit is ease-26's file set, so not touched): should more tail keys yield to the count? Not a defect by the ticket's test (origin is also gone at 160).
   - With the pane focused (Esc) or closed (Esc Esc) the footer carries no stop words at any width, only the key ladder (sections `shell-pane-focused-*`, `shell-pane-closed-*`, `app-pane-closed-*`); doc comment on `stopped_row` calls this accepted.
2. 80x24 relaunch into a rebase from a previous run: bottom row is keys only (section `27-relaunch-rebase-80x24`); the top row says `REBASING  step 1 of 2 · pick c3b01ea feature writes`, no origin words. Stays a question.
3. Resolver bar, `B base` + `d start over`, 120 columns: no word cut after `d` or `esc` (`d start over  esc close` whole); `j/k` and `M` are shortened earlier in the row. Since ease-pass/11 a merge document draws the clash bar, which has no `B`; the `B base` bar shows in a rebase once nothing is open, and with the file's read out. The pty capture could not be taken (a notice sits on the bar after the key that reaches it); the evidence is the test.
4. Finish summary (merge): `Shift+Tab` does nothing, by decision 5 (message read-only, no fields); test above. On the commit screen `Shift+Tab` moves back (`commit_editor/keys.rs:57`, `focus.previous()`).
5. Confirmed: `notes.txt  •` in section `27-app-landing-120x40`; the `…` mark is gone (`conflict_resolver/files.rs:254,263`).

Touches: `crates/mergewright-ui` hints tests (one test added to `tests/typed_fields.rs`, 112 lines, no `over_400` row involved); no source changed. Checked: `git diff --stat` is the test, captures, harness and this ticket.

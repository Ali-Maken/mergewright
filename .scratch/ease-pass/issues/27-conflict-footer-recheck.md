# 27 — Re-check the conflict footer and key row against the user's rule

Status: ready · Build with: Sonnet · Blocked by: none
Source: user 2026-10-24 on batch item 6: "drop the origin, keep the count"; triage `.scratch/ease-pass/scouts/user-batch-triage.md`; KNOWN_GAPS M6/21 rows (160x48 count dropped; 80x24 relaunch no words; unread `…`).

## Scope (measure first; fix only a confirmed break of the rule)
1. Capture the merge-stopped-on-conflicts footer, changes pane open and closed, at 80x24, 120x40, 160x48, 187x50, 200x50, for a merge begun in the shell and one begun in the app. Rule: when room is short, the origin words go first; the conflicted-file count stays as long as anything but keys fits. Any width that drops the count while keeping (or before dropping) the origin is a defect: red-first test, fix at cause (`update/banner/stop/wording.rs` and the fit it feeds), re-shoot.
2. 80x24 relaunch into a rebase left from a previous run: does the bottom row say so? Capture; report (question row stays a question unless the rule above covers it).
3. Resolver key row at 120 columns with `B base` shown (batch item 9): capture; does it still cut words after `d` and `esc` since ticket 24? Report with the capture; no fix unless a word is cut — then a question for the conductor.
4. Finish screen: does `Shift+Tab` move back between fields (batch item 8)? Test it; report.
5. Unread-file mark: confirm `•`/`~` (`conflict_resolver/files.rs:254,263`) in the current capture, so the `…` row can close.

## Acceptance
- [ ] Footer captures at the five widths × two origins, listed; each verdict against the rule.  <- test/capture names
- [ ] Any defect: red-first test named, fixed, re-shot; or "none" with the captures that show it.
- [ ] Items 2–5 answered with a capture or test each.
- [ ] Captures ≤200 KB; colour unverified; five gates + `scripts/walks.sh`; Touches:.

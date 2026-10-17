# fold-captures — every capture the fold default changes, re-shot or fixed by D3's rule; the folded and opened main screen

Status: ready
Blocked by: 06e
Build with: Opus (`builder-opus-xhigh`) — sorting each diff into fold-only or regression is the step where a regression gets shot in as "expected". It needs judgment, not a script.
Needs: from 06d/06e: the fixed parents, the HEAD keep, fixtures (including any unmerged branch 06e added).

## Read first
- `.scratch/ease-pass/issues/06-graph-fold-and-lanes.md`: Handoff 2 items 3–5; **Decisions for continuation 2, D3**; Acceptance box 9.
- 06e's NOTES: the capture tests whose fixture changed.
- `TESTING.md`: the "what right looks like" line for each capture below. ADR-0037 (contact sheet).
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-graph/MAP.md`: the sections 06/06d changed.

## Scope
In: the `capture_*` tests, `lane_cap.rs`, `look_polish`, `docs/shots/**`, the ease-pass contact sheet, capability M1-09's citation, and the MAP.md files.
- **Diff each capture first, then classify it by D3.** Fold-only → re-shoot. Anything else → fix the cause or stop.
- **Expected fold-only** (background graph): m8-look-chrome-80x24, m8-look-help-graph-187x50, m8-help-80x24, m8-keymap-config-problem-120x36, m8-keymap-remapped-120x36, m8-quit-stop-now-80x24, m8-quit-waits-80x24, m8-bar-after-search-120x36, m8/search-hint-120x36, m5-ui-name-prompt-80x24.
- **Check the subject is still shown first:** m7/rebase-line-preview-187x50, m7/rewrite-refused-fork-120x40, m7/refused-box-single-close-120x40, m7/rebase-stop-forecast-80x24, m5-ui-preview-rebase-80x24, m5-ui-preview-scrolled-80x24 — predicted rows never fold. Also `look_chrome::capture_the_cursor_row_in_dark_and_light` and `m8_walk::look::capture_the_cursor_row_at_80x24_and_187x50`.
- **Re-check after 06d** (the Fix menu was the parents bug): m6/fix-preview-verb-120x40, m6/fix-finish-shell-merge-120x40. Their diffs must be fold-only.
- **Lane cap:** `lane_cap::*` and m5-lane-cap-160x24. Make the fixture's topics unmerged branches (D3 route 2). 06b will re-shoot it again.
- **Superseded:** `look_polish::capture_folded_merge_count_for_docs_shots` → retired. `15-merges-folded.txt` / `15-merges-unfolded.txt` → deleted if `grep` finds no other citer. M1-09 is repointed to box 9's captures.
- `m7_walk::pr_contents::capture_compare_since_split` — diagnose it by 06e's finding for `pr_contents`.
- **Box 9:** `docs/shots/ease-pass/main-folded-120.txt` and `main-opened-120.txt`, from 06e's box 5 fixture. Opened = `space` on the PR 16021 merge.
- MAP.md entries for `Row::drawn_parents` / `is_merge`, `View::folded`, `Window.fold`, `FoldMark` / `FoldCount` (`map_coverage` passes).

Out: lane drawing and header words (06b); the opened rows' look (07).

## Signatures
None new.

## Touches
`docs/shots/**` (≤200 KB each), the contact sheet, `TESTING.md` capture lines (only if a path changed, never "what right looks like"), the capability index (M1-09), and MAP.md files.

## Acceptance
- [ ] NOTES has a table: capture → fold-only / regression-fixed / retired, a one-line diff summary, and the test that writes it. Every row on Handoff 2.5's list appears.
- [ ] No re-shot capture's diff has anything outside D3's allowed set (checked by the builder per row; the user judges the look by eye — rule 6; colour unverified — rule 5).
- [ ] Box 9 in 06 ticked: both captures exist, are written by named tests, and are in the contact sheet next to `prototypes/ease-mocks/mergewright-main-screen.html`.
- [ ] M1-09 cites the new captures; the `graft-capabilities` checker passes.
- [ ] The workspace is fully green.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k. Most rows are a diff and a re-shoot. Any regression found → fix it if it is small, or stop and report it with its line.

# ease-acceptance sheet and script — 14's remainder, part 2 (split by conductor, 2026-10-20)

Status: done (2026-10-20): items 4 and 5 done; item 6 left unticked in 14 with its reason. fmt, clippy, nextest 7199/7199, doc tests clean; ease-accept.sh rc=0.
Blocked by: 14b-ease-acceptance-walks (its captures go on the sheet)
Build with: Fable (acceptance)

Split from 14 after its builder compacted. **No new decisions**: scope, acceptance and signatures are 14's.

## Scope
14's handoff "Remains" items **4–6** (`14-ease-acceptance.md`, NOTES → Remains):
4. `ease_walk/shots.rs`; rewrite `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv` (ease-pass captures only, each with its mock path and "what right looks like" line); add the `ease-pass` promise case to `scripts/contact-sheet.sh`; regenerate the ease-pass contact sheet (ADR-0037).
5. `scripts/ease-accept.sh` (one command walking the scenarios); the `TESTING.md` "## Ease pass acceptance" section; the Q4 pixel quote; the m3 waiter.
6. Old milestone sheets: not rebuilt (user, 2026-10-19). Leave that box unticked with the reason.

## Acceptance
Tick 14's remaining boxes **in 14's file**, each naming its test. After this, 14 is done or every unticked box says why.

## Read first
14's Read first, Scope, Acceptance, NOTES (Done, Remains 4–6, Gotchas); 14b's NOTES.

## NOTES (builder, 2026-10-20; no compaction; no scout, the files were named and small)

Worktree was on c5235291, an ancestor of main; fast-forwarded to d6fbef86 (14b) before starting.

**Done**
- `crates/mergewright/tests/it/ease_walk/shots.rs` (3 tests): every file under `docs/shots/ease-pass/` <=200 KB; 55 captions, each `mock <existing path>: …` and saying `unverified` wherever it speaks of colour; the sheet holds every capture's current text, no `NOT RECORDED`.
- `docs/shots/ease-pass/ease-pass-contact-sheet-captions.tsv` rewritten: 55 rows, ease-pass captures only (was 45, 36 pointing at `../m2…m8`). Captions are each capture's own header ("Judge by eye" / "What right looks like") with the mock path in front; six captures had no header (merge-review-120/90, merge-landed-120, merge-look-clashed/combined-120, main-pixel-120) and got hand-written captions from their tests and 12/12b/13. Mock per capture: commit-* -> commit mock; merge-*/rebase-clash -> merge mock; 06h/06i captures (merged-shown, help-graph-mode, graph-mode-switched, config problem) -> show-all-branches mock (the config problem has no mock of its own); the rest -> main mock. Whether that mapping is right is the user's to judge.
- `scripts/contact-sheet.sh`: `ease-pass` promise case. Sheet: `docs/shots/ease-pass/ease-pass-contact-sheet-{1,2}.txt` (177 767 B + 103 489 B).
- `scripts/ease-accept.sh` + `.scratch/ease-pass/accept/.gitignore`; summary `docs/perf/ease-accept.txt`. **It walks the in-process `ease_walk` tests (real repositories, real job pool, drawn frames), not the `mwr` binary in a pty.** 14 as built (14b) is in-process; 14's spec said "in a pty". Said so in the script header and TESTING.md. A pty walk of the nine journeys would be a new ticket.
- `TESTING.md` "## Ease pass acceptance": journeys, G/C/M/Q tables with test names and words, pixel-lag note, 55 capture rows. Colour unverified throughout.
- m3 waiter: `m3_walk.rs` `on_screen` for `m3-05-commit-refused`: `nothing staged` -> `Nothing goes in. Put a file in with space.` (`mergewright-ui/src/commit_editor/words.rs:191`). The in-process scenario asserts `Refusal::NoFile` (bar words `put a file in first`, `words.rs:46`) only after a typed subject and `⏎`; the pty capture presses `c` alone, so the static `Nothing goes in…` line is what is on screen.

**m3 before/after (observed)**
- Before: `pty-walk-m3.py 80x24 commit-refused-nothing-staged` did **not** time out. It reported `expected `nothing staged` is not on screen after the frame settled` and DRIFTED against the old capture (the COMMIT editor).
- After the waiter change: text found, still DRIFTED against the old capture. Re-shot with `MWR_UPDATE_SHOTS=1` at 80x24 and 160x48 (the diff is the whole commit screen of ease-pass 04/08, not an unowned change); re-run: `1337 bytes, settled`, `4215 bytes, settled`.
- Needed `MWR_FIXTURES_BIN=target/debug/mergewright-fixtures`: `pty-walk-m3.py:43` still defaults to `graft-fixtures` (stale after the rename) and `scripts/m7-accept.sh` sets no such variable. **Question, no line proving it:** whether the m7-accept pty step runs today is unverified.
- `docs/shots/m3-contact-sheet.txt` is now older than these two captures (not regenerated: old sheets not rebuilt).

**Not done, and why**
- 14's "Earlier records current": unticked, user 2026-10-19 (old sheets not rebuilt). Beyond the two m3 captures above no m3/m4/m6/m7/m8 walk was re-run.
- Pixel: only the pty lag is quoted. The in-Ghostty run (`docs/perf/ease-pixel-ghostty.txt`) is missing: **unverified**; conductor to log it in `KNOWN_GAPS.md`.
- Nothing judged by eye (rule 6); no capture was looked at for "right".

# fix-acceptance-conflict-journeys: the walk replays every journey in the note, counts its keys, fails over the note's count, and asserts where he lands after every event; both start paths, both arrangements, the finish summary

Status: partial — part 1 (parser, counting, every journey walked) done 2026-09-24; captures, contact sheet, TESTING.md done by ticket 43 (2026-09-24); perf remains (ticket 44). (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 09-ops-conflict-set-and-summary.md, 10-resolver-line-numbers.md, 11-preview-verb-confirms.md, 12-landing-after-a-stop.md, 13-resolver-file-list.md, 14-resolver-three-panes.md, 15-take-advances-to-next-block.md, 16-take-every-open-block.md, 17-resolver-file-keys.md, 18-stop-opens-the-resolver.md, 19-finish-summary-screen.md, 20-continue-a-merge-begun-in-the-shell.md
Was: draft · **rewritten 2026-09-23 (Amendment 2)**. This replaces the Amendment 1 version: that version's count block (`journey-counts-superseded`) is not parsed.
Build with: **Fable**. It is the acceptance ticket and does not count against the Fable build budget. It decides what each landing must prove, and writes the "what right looks like" lines the user judges by.
Needs: every other `fix-*` ticket in `.scratch/m6-conflicts/issues/`:
- `fix-ops-conflict-set-and-summary`, `fix-resolver-line-numbers`, `fix-resolver-file-list`, `fix-resolver-three-panes`, `fix-finish-summary-screen` (Part A; Part B only if P8 is approved);
- `fix-landing-after-a-stop`, `fix-preview-verb-confirms` (P1), `fix-take-advances-to-next-block` (P2), `fix-take-every-open-block` (P3), `fix-continue-a-merge-begun-in-the-shell` (P4), `fix-stop-opens-the-resolver` (P5);
- `fix-resolver-file-keys`, only if P6 or P7 is approved.

A ticket that is gated and not approved is not needed. Its journeys then walk the in-spec route and assert the `keys` column.
Crates: graft (tests only: `crates/graft/tests/it/m5_walk/`), docs (`TESTING.md`, `docs/shots/m6/`, `.scratch/m6-conflicts/pty-walk-m6.py`).

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Journeys (every J, "Where each event lands him", the `journey-counts` block), New UI (all of it: the three-pane and strip mockups, the finish summary and its states).
- `docs/adr/0043-resolver-file-list-and-three-panes-when-wide.md` §1–§5.
- `docs/research/ux-principles.md` §1.3 (the counting rule the walk implements).
- `.scratch/m6-conflicts/issues/07-acceptance-conflicts.md` Scope 1: the harness it built. Reuse it; never rebuild it.
- `crates/graft/MAP.md` §`tests/it/m5_walk/` **M6 additions** (`Step`, `Wait`, `fixture.rs`, `truth.rs`, `every_scenario.rs`, `conflicts_matrix`, `M6_STEMS`).
- CLAUDE.md "The seven that pay" 1, 5, 6 and 7; "Captures stay small"; ADR-0037 (contact sheet).
- TESTING.md "Conflicts walk" (the shape to copy) and "Times I left" rows :104-109.

## Scope
In:
1. **The count is read from the note, not restated.** `journeys.rs` parses the one fence tagged exactly `journey-counts` in `docs/design-notes/m6-conflicts.md`, and ignores `journey-counts-superseded`.
   - Each row reads `id keys [token…] [# comment]`. `P6:n`-style tokens are read and not asserted.
   - A comment of the form `id + id + …` names a whole row's parts.
   - A journey id in the walk but not in the block fails the build, and so does the reverse.
   - Only `keys` is asserted. A whole row must also equal the sum of its parts' `keys`, and the walk checks that sum.
2. **Counting** (UX 1.3). Every `Step` that sends input counts 1: a key, a chord (`Ctrl+T` = 1), a click, or a `Type`. `Wait`s, `Claim`s and truth reads count 0. A journey fails when `keys_sent > note.keys`, and the failure names both numbers and the key list.
3. **Fixtures.** Reuse where the shape exists; add the rest in `fixture.rs`:
   - `one_conflict`: one file, one block.
   - `three_blocks`: `three.txt`, three blocks.
   - `two_files`: `gone.txt` (changed on `main`, deleted on `feature`); `notes.txt` (two one-line blocks); and a clean one-line change to `README.md` made by `feature`.
   - Each fixture also has a **shell-begun** variant: `git merge feature` run by git in setup, then the app opened fresh.
4. **Journeys.** Each is a scenario in the M6 table with its own start and a landing assertion after **every** event. A landing is read from the drawn frame, not only from `AppState`. Routes are the in-spec ones. If the note's `keys` column holds P6 numbers (the conductor moved them), the walk uses the P6 routes.
   - `a-merge-to-stop` (`one_conflict` and `two_files`): the graph, cursor on `feature`; keys `m m`. It lands in the resolver on the list's first file:
     - `file_cursor` 0, cursor block 0, header `MERGING`;
     - the bottom row starts `o yours`;
     - `Status::Stopped` holds `merge stopped · …`.
   - `a-stop-to-first-conflict`: 0 keys. The frame right after the stop is the one above.
   - `b-launch-to-first-conflict` (shell-begun): the launch lands on the banner, pane `CONFLICTS`, with a reason containing `begun outside Graft`. Key `⏎` → the resolver on the first file. There is also a `git pull` variant over `LocalRemote`, with the same count.
   - `one-file-one-conflict`: keys `o ⏎`.
     - Without P8, it lands on the banner with the notice `every file done ✓ · c finishes the merge`.
     - With P8, the resolver stays, with the same notice.
   - `three-conflicts-all-theirs`: `Ctrl+T ⏎`. The bytes equal per-block theirs (truth from git, not `checkout --theirs`), and it lands as above.
   - `modify-delete-keep-yours` (`two_files`, on `gone.txt`): `o`.
     - `gone.txt` equals `HEAD:gone.txt` and is staged.
     - It lands on `notes.txt`, with the notice `gone.txt done ✓ · ▸ notes.txt`.
   - `two-files-in-order`: `o`, `Ctrl+T`, `⏎`. The list shows both files `done ✓`, then the "every file done" landing.
   - `two-files-out-of-order`: on `gone.txt`, `n` → `notes.txt`; `Ctrl+T ⏎` → lands on `gone.txt`; `o` → every file done.
   - `a-finish` (`two_files`, from every file done): `c` → the summary.
     - The cursor is on `README.md` under `COMBINED BY GIT ON ITS OWN · 1`.
     - `RESOLVED BY YOU · 2` lists `kept · yours · main` and `theirs · feature in 2 of 2 conflicts`.
     - Then `⏎` → the graph is focused, the cursor row = `HEAD`, `HEAD^2` = `feature`, and the status reads `merged feature into main ✓`.
     - `u` (a truth check, not counted) restores the pre-merge tip.
   - `b-finish` (shell-begun, both files resolved in the app): `c ⏎` → `HEAD^2` = `feature`, and there is exactly one `FinishMerge` entry. `u` → `MERGE_HEAD` is back (truth from git).
   - `summary-open-git-combined`: in the summary, `space` → `README.md`'s diff rows appear under its row, and the rows after it move down.
   - `back-after-esc`: start after `a-merge-to-stop`, with `Esc Esc` not counted (it is the journey's start). The bottom row leads with `⇥ banner`. Keys `⇥ ⏎` → the resolver on the first file not done.
   - **Whole rows** each run as one continuous scenario, and each is also checked as the sum of its parts:
     - `a-whole-one-file`: `m m o ⏎ c ⏎`.
     - `a-whole-two-files`: `m m o ^T ⏎ c ⏎`.
     - `b-whole-two-files`: launch, then `⏎ o ^T ⏎ c ⏎`.
5. **Arrangements.** `two-files-in-order` runs at 120x40 (strip and card) and at 200x50 (column and three panes), with the same count. A click journey is not counted; the walk only asserts that a list click selects the file at both widths.
6. **TESTING.md "Conflict journeys".** Per journey:
   - the start, the keys, the count, and the best tool's count beside it (from the note);
   - **what right looks like**, in concrete text: the exact notice and bottom-row words, the identity row, the list states, and git's state (`HEAD^2`, `ls-files -u`, bytes).

   Also:
   - One line tells the user to press `Ctrl+T` once in **Zed's terminal**. The pty cannot check whether Zed claims `Ctrl+T`, so P3's key stays **unverified** until he does.
   - "Times I left" :108 is closed, with P4's test name.
7. **Captures** via `pty-walk-m6.py`, one frame per landing, at 120x40, 160x48, 200x50 and 80x24, plus the user's own size if field notes record it. The frames:
   - after the stop, landed in the resolver (strip at 120/160/80; column and three panes at 200);
   - the card on `notes.txt`;
   - after `t`;
   - after `Ctrl+T`;
   - the last file done;
   - the summary, and the summary with `README.md` expanded;
   - after `⏎` commits;
   - the launch into a shell-begun merge;
   - after `Esc Esc`.

   Each goes into `docs/shots/m6/` and into the M6 contact sheet (ADR-0037), under its "what right looks like" line. Cap: 200 KB per file.
8. **Context guard.** Every walk and capture writes to files. The builder reads only `ok`/`FAIL` lines and summary counts, and reports paths, never frame contents.

Out:
- New app behaviour: every fix is its own ticket. A walk that fails on behaviour files a finding against that ticket, with its file and line (rule 7), or as a question.
- Colour: rule 5. Reverse-video focus and the cursor-block reverse stay **unverified**; the text tells are `▸`, `▏` and the bottom row.

## Acceptance (name the verifying test when you tick)
- [x] `every_scenario::journey_counts_match_the_note`: the parse; the id sets equal both ways; whole rows equal the sum of their parts; the superseded fence is ignored. Lives at `m5_walk::conflict_journeys::tests::journey_counts_match_the_note` (ids both ways, whole = parts, every route within its row); the superseded fence and the parse errors by `m5_walk::journeys::tests::{the_block_is_read_and_the_superseded_one_is_not, a_stray_token_is_an_error_not_a_comment, no_block_or_two_blocks_is_an_error}`.
- [x] One `every_scenario::journey_<id>` test per id (the block now has **19**), each green at the note's `keys`: 24 `every_scenario::journey_*` tests (+ `two_files_in_order_at_both_arrangements`) cover all 19 ids, 5 of them with `@variant`s. Scratch run: `j` added before `O` in `one-file-one-conflict`'s route; `journey_one_file_one_conflict` failed with `FAIL one-file-one-conflict: one-file-one-conflict: 2 inputs sent, the note's journey-counts allows 1: j O`; reverted, green again. Kept as a unit test: `conflict_journeys::tests::one_key_over_the_note_fails`.
- [x] `every_scenario::two_files_in_order_at_both_arrangements` (200x50: `FILES · 2` column + `result` pane; clicks select `notes.txt` then `gone.txt`), with `journey_two_files_in_order` at 120x40 (no column, no `│ result`; same clicks).
- [x] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with the new frames — by ticket 43 (see its Acceptance).
- [x] TESTING.md "Conflict journeys" written, every line concrete (no "looks right"), with the Zed `Ctrl+T` line marked **unverified** — by ticket 43; its captures table is checked by `every_scenario::captures_are_named_keyed_and_listed_in_the_matrix` (stems ↔ matrix ↔ files).
- [x] The captures are listed by path in ticket 43's NOTES. The user judges them by eye (rule 6); nothing here says they look right.

## NOTES (builder, 2026-09-24) — **partial; this session compacted once**

Scouts: `scout-21-harness.md` (harness), `scout-21-findings.md` (two frame findings); researcher drafted `captions-fix-draft.tsv`.

**Done (commit `m6-conflicts-21: part 1 …`):** parser + counting + every journey walked (Scope 1, 2, 3, 4, 5). Route table (from `journey_counts_match_the_note --nocapture`), every one equal to the note: a-merge-to-stop 2 `m m` · a-stop-to-first-conflict 0 · b-launch-to-first-conflict 1 `⏎` (+@pull, @one-file, @three-blocks) · one-file-one-conflict 1 `O` · three-conflicts-all-theirs 1 `T` · modify-delete-keep-yours 1 `o` · two-files-in-order 2 `o T` (+@200x50) · two-files-out-of-order 3 `n T o` · a-finish 2 `c ⏎` · b-finish 2 · b-finish-again-after-undo 2 · summary-open-git-combined 1 `Space` · back-after-esc 2 `⇥ ⏎` · open-a-done-file 1 `k` · walk-back-a-done-file 4 `j d → ⏎` · a-whole-one-file 5 · a-whole-two-files 6 · b-whole-two-files 5 · editor-to-finish 5 `Esc ⇥ ⇥ c ⏎`.
Other tickets' journey boxes ticked with test names: 11:97, 16:128 (walked at the note's current 1, not 2), 17:158, 18:38, 19:181, 31:113; 28:204 note marked done.

Where the walk differs from this ticket's Scope text (the note moved on; the walk follows the note): P6 routes (`O`, `T` — no `⏎`); P8 (resolver stays on every file done); P7 `k`/`j` for 28's two rows.

**Findings (for the conductor; not fixed here, out of scope):**
1. *Defect, with lines.* `notes.txt` drawn `…tes.txt` in the 200x50 list column. `column_width` sizes by the widest single row (`crates/graft-ui/src/conflict_resolver/files/column.rs:24`, gone.txt 3+8+2+14 = 27); `column_plan` takes the longest name and longest state from different rows (`column.rs:47-54`) → `name_w = min(9, 27-5-14) = 8`, cut by `fit_left` (`files.rs:330`). The walk's click reads the row as drawn (`journeys.rs::cut_name`), so the arrangement test is green; the cut itself is not asserted.
2. *Defect, with lines.* After a list **click** the bottom row keeps the last `Done` status (`resolved gone.txt (ours) ✓`) instead of the bar: `update/keys.rs:106-108` clears `Status::Done`/`Copied` on the next key; `update/mouse.rs:21 mouse_event` has no such clearing before `resolver_mouse` (`update/resolver/scroll.rs:27`; no `Status` in it). Why `open-a-done-file` routes by `k`.
3. *Question.* `begun outside Graft` is not drawn in the footer at 120x40 (nor, per earlier captures, at 187): the footer draws the fullest `stop_wordings` rung that fits (`update/banner/stop.rs:197-215`), budgeted on the graph side's width (`graph_screen/footer.rs:87`, `hints/fitting.rs:229`), which a launch splits with the changes pane (`update/relaunch.rs:32-33`). Could not confirm the pane's width at 187, or whether sizing to the graph side is intended. The walk asserts the origin on the stop's full reason (state) and `merge in progress` + `CONFLICTS` in the frame.

**Remaining (exact next steps):**
1. Captures (Scope 7): add stems `m6-10-stop-lands-in-resolver`, `m6-11-card-on-notes`, `m6-12-after-t`, `m6-13-after-ctrl-t`, `m6-14-last-file-done`, `m6-15-finish-summary`, `m6-16-summary-expanded`, `m6-17-after-commit`, `m6-18-launch-shell-begun`, `m6-19-after-esc-esc` as `capture:` on the journeys; bump `coverage.rs::M6_STEMS` (fixed-length array, `[&str; 9]` → 19) and the matrix; the launch capture has no keys → exempt it in `captures_are_named_keyed_and_listed_in_the_matrix`. Run `.scratch/m6-conflicts/pty-walk-m6.py <WxH>` at 120x40, 160x48, 200x50, 80x24, 187x50; read only ok/FAIL. Drift: compare the pty frames' landing words with the headless `Frame` facts.
2. Contact sheet (ADR-0037): `scripts/contact-sheet.sh m6 docs/shots/m6` — add a captions-order option, drop the "earlier milestone" label for `fix-*`, split into parts ≤200 KB; merge `.scratch/m6-conflicts/captions-fix-draft.tsv` (47 `fix-*` rows, ticket order) into the captions file; widen `contact_sheet_captions_cover_every_m6_capture` to `fix-*` and exclude `m6-contact-sheet*`.
3. Perf: `.scratch/m6-conflicts/perf-m6-21.sh` → `docs/perf/m6-21.txt` (release `heavy::budgets` `first_paint_us` lines); read summary only.
4. TESTING.md "Conflict journeys" (Scope 6) from the route table above + the note's best-tool counts; the Zed `Ctrl+T` line **unverified**; "Times I left" :108 (log says already closed — confirm).
5. KNOWN_GAPS M6/32 bar-words facts for the user (don't decide).

Gotchas: the worktree guard refuses compound bash naming the repo path with rm/heredocs/vars — use the Edit tool or a script in the scratchpad. `$GRAFT_WALK_FRAMES=<dir>` dumps a failing `Step::Frame`'s whole frame. `Wait::BannerFocused` needs an unmerged path; an all-resolved launch waits on `Wait::OnTheBanner`. `Status` text has no `✓`; the frame draws it.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large: one parser, 15 journeys and one arrangement case on the existing harness, one new fixture, docs, and captures. One context: the captures run from the script, not in context. If it will not fit, split at the seam (parser and journeys first; TESTING.md and captures second) and say so.

# fix-acceptance-conflict-journeys: the walk replays every journey in the note, counts its keys, fails over the note's count, and asserts where he lands after every event; both start paths, both arrangements, the finish summary

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
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
- [ ] `every_scenario::journey_counts_match_the_note`: the parse; the id sets equal both ways; whole rows equal the sum of their parts; the superseded fence is ignored.
- [ ] One `every_scenario::journey_<id>` test per id (15), each green at the note's `keys`. Add a key to one route in a scratch run and confirm it fails; say which route and what the failure line read.
- [ ] `every_scenario::two_files_in_order_at_both_arrangements`.
- [ ] `every_scenario::contact_sheet_captions_cover_every_m6_capture`, green with the new frames.
- [ ] TESTING.md "Conflict journeys" written, every line concrete (no "looks right"), with the Zed `Ctrl+T` line marked **unverified**.
- [ ] The captures are listed by path in the report. The user judges them by eye (rule 6).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium–large: one parser, 15 journeys and one arrangement case on the existing harness, one new fixture, docs, and captures. One context: the captures run from the script, not in context. If it will not fit, split at the seam (parser and journeys first; TESTING.md and captures second) and say so.

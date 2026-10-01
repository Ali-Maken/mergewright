# m7-acceptance-finish — 26's continuation: re-run the walk after the fixes, tick 1/3/4, the sheets and the promise header

Status: done (2026-10-01, re-run 2 on main `a3901dee`) — `scripts/m7-accept.sh` rc=0; see NOTES "Re-run 2"
Build with: **Fable.** It is the acceptance ticket (26) finishing: it judges the whole milestone against the promise and
a false tick is Critical. Doesn't count against the milestone's Fable build expectation.
Needs: `f-recover-set-aside-bytes`, `f-guard-refusal-at-80`, `captures-reshoot-named`, `captures-verify-questions`,
all merged to `main`, and any fix the conductor filed from a `captures-verify` finding merged or gapped.
Base: **26's branch `worktree-agent-aeb403f0c2282df64` with `main` merged in** (the conductor prepares the worktree;
the builder does no merge).
Runs: last, alone.

## Read first
- 26's ticket: "Acceptance", "Decided", "Carry (conductor, 2026-09-30)", and its NOTES (Findings, Questions, pty
  compare, KNOWN_GAPS rows, Open). The ticket file on this branch.
- The four fix tickets' NOTES only (what changed, V5's observation, every finding, every file stopped).
- `scripts/m7-accept.sh` (this branch), `scripts/contact-sheet.sh` :130-145, :184.

## Rulings (planner, 2026-10-01; not open)
- **Q-trace (26 Decided, Trace step 3: "the file pane marks line 19 `+`").** The walk asserts the fixture as drawn:
  Lina's commit adds 18, 21 and 22 (`+3 −2`); 19 (`loop {`) is context (`promise/trace.rs:136-151`). The "19" came from
  the history mock-up (`docs/design-notes/m7-power-tools.md:181`, where Lina adds `let mut attempt = 0;` at 19) and
  F3's "the fixture follows it" (:1074). **The fixture stays; the scenario text is corrected.** Changing the fixture now
  would move every blame and history walk and capture built on it, to match a mock-up whose job was illustration. The
  promise needs "her row selected, her added lines marked `+` where she added them", not the number.
  - 26 Decided step 3 now reads: `Esc`, then `L`: the function's history; Lina's row selected; the file pane marks her
    added lines `+` (in the fixture: 18, 21, 22). Edit that line in the ticket and the `TESTING.md` Trace entry.
  - F3's "The fixture follows it" is not held by the fixture: the conductor amends :1074 (a doc line, not this ticket).
- **Q-push (26 Acceptance "the push-journey gates").** 25's NOTES (25:89) are stale: 25b gave `P y` an id, `push 2`
  (`m6-conflicts.md:522`, "walked in m5_walk (25b)"), and `p r r P y` is `rejected-push-dirty-to-pushed 5` (:521). So the
  gates exist. Claim closed, each only after seeing its test PASS in this run's `accept/nextest.txt` or the workspace
  run: KNOWN_GAPS `M6/u2`, `M6/rw2`, `M6/r47-49`. Tests: grep `push_journey` and `rejected_push_dirty_to_pushed` under
  `crates/graft/tests/it` for the exact names (25b ticks name `every_scenario::push_journey_is_walk_gated` and
  `every_scenario::journey_rejected_push_dirty_to_pushed`). A name that does not exist → not claimed; say so.
- **Q-count:** closed by 26 (29 ids, counted against the note as it is). Nothing to do.

## Scope
In:
1. Workspace gates, then `scripts/m7-accept.sh`. It must exit 0. No walk, count or byte assert loosened to get there.
2. `promise_recover_without_leaving` green **with its byte asserts** (`assert_bytes`, one LF file under
   `core.autocrlf=true`, both `u` steps). `guard_refusal_at_80_names_the_file_and_the_next_step` green.
3. The Q-trace text fix (ticket line and `TESTING.md` Trace entry). The walk is not changed.
4. `scripts/contact-sheet.sh`: an `m7)` arm in the promise `case`, next to `m6)` (:142), with the note's promise verbatim
   (`docs/design-notes/m7-power-tools.md:11`: *you can rewrite, trace and recover history without leaving the app*), then
   ` — ` and one clause naming what proves it, in the m6 arm's form (the three promise scenarios and the 29 journeys,
   `TESTING.md` "M7 acceptance").
5. Contact sheets rebuilt: m7 (with the header now naming the promise, and the new capture from `f-guard`,
   `docs/shots/m7/in-the-way-at-80-80x24.txt`, added to `m7-contact-sheet-captions.tsv` with its ticket's Right line),
   and m4, m5, m6 (their captures were re-shot). Each file ≤ 200 KB (the script splits).
6. Tick 26's boxes 1, 3 and 4, each naming its verifying test or command at tick time.
7. NOTES: the KNOWN_GAPS rows proven closed (each with its test) and the rows still open, with why.

Out: fixing anything a walk finds (a finding goes to NOTES, the conductor files it); re-shooting any capture (a compare
failure is a failing ticket); the release perf run (`docs/perf/m7-review-release.txt`, done).

## Acceptance (name the verifying test at tick time; spot-check your own ticks and say how many held)
- [x] 26 box 1: `scripts/m7-accept.sh` exits 0 (the run's last line and `rc`); the red side is 26's "Red
      demonstration", already shown. First run: rc=1 (NOTES "Run"). **Re-run 2 (main `a3901dee`): rc=0**; last lines
      `accept: 29 journeys walked, 29 at or under count, 0 over` / `summary: docs/perf/m7-accept.txt`.
- [x] 26 box 3: `promise_rewrite_without_leaving`, `promise_trace_without_leaving`, `promise_recover_without_leaving`
      PASS in `accept/nextest.txt`; the recover one's byte asserts present (quote the assert's file:line).
      **Not met:** rewrite and trace PASS; recover FAIL at `recover.rs:123`. Its byte asserts are present and the `u`
      one holds (`recover.rs:103` `assert_bytes(repo, &reset_state)`, helper :145-158). NOTES "F-recover".
      **Re-run 2: met.** `accept/nextest.txt` PASS: `m7_walk::promise::rewrite::promise_rewrite_without_leaving`,
      `…::trace::promise_trace_without_leaving`, `…::recover::promise_recover_without_leaving`. Byte asserts in
      `crates/graft/tests/it/m7_walk/promise/recover.rs`: :103 `assert_bytes(repo, &reset_state)` (the `u` step), :123
      `assert_eq!(on_disk, lost_work::LOST_EDIT, "his edit is back on disk")` (step 4); fixture built under
      `core.autocrlf=true`.
- [x] 26 box 4: every m3–m6 pty walk green in compare mode (step 2's summary line; `accept/pty-diffs/` empty).
      A file `captures-verify` stopped on and nobody fixed → box 4 stays unticked, the file named.
      **Not met:** `2 pty: 13 runs (m3–m6 × sizes), 1 drifted or failed`; `pty-diffs/m4-80x24.txt`. NOTES "pty".
      **Re-run 2: met.** `2 pty: 13 runs (m3–m6 × sizes), 0 drifted or failed`; `ls -A accept/pty-diffs/ | wc -l` = 0.
- [x] Q-trace: the ticket line and `TESTING.md` Trace entry say "her added lines (18, 21, 22)"; `git diff` shows no
      change under `crates/`. (`grep -n '18, 21, 22'` → `26-m7-acceptance.md:70`, `TESTING.md:2989`; `git diff --stat
      crates/` empty.)
- [x] `scripts/contact-sheet.sh` m7 arm; `head -3 docs/shots/m7/m7-contact-sheet-1.txt` shows `# Promise: you can
      rewrite, trace and recover history …` (no `NOT RECORDED`). (`head -4`: the promise is line 4, under the `Part 1
      of 2` line; `grep -c 'NOT RECORDED'` = 0 in both m7 parts; `git diff scripts/contact-sheet.sh` one hunk.)
- [x] m4/m5/m6/m7 sheets rebuilt; every sheet file ≤ 200 KB (`wc -c`); the guard capture is on the m7 sheet under its
      Right line. (`wc -c`: largest `m6-contact-sheet-1.txt` 179 713 B; `m7-contact-sheet-2.txt:669-670` the capture
      under its `what right looks like` line; `NOT RECORDED` 0 in every m4–m7 sheet.)
- [x] NOTES, rows closed (each with its test, seen PASS in this run — `accept/claims.txt` and the workspace run; NOTES
      "Rows closed"): M6/68, M6/77 (both rows; row 2 now also by the
      Recover scenario), :217, :847, :848, :109, M6/u2, M6/rw2, M6/r47-49, `M7/30q-setaside` (by f-recover's tests),
      the 80-column refusal (by the guard scenario and f-guard's tests), and each 2a ticket's rows as 26 listed them.
- [x] NOTES, rows open (NOTES "Rows open"): `M7/30q-putback-fwd` (as f-recover's V5 observed it: broken or holds, with the bytes),
      `M7/30q-drift-eol`, f-guard's resize limit, 30i's M7/13b-120, any `captures-verify` finding; colour everywhere
      (rule 5) and every capture's look (rule 6): **unverified, not broken**.

## Touches (check each, report how)
- `TESTING.md`: only the Trace entry and, if the guard/recover Right lines changed, those. `git diff --numstat
  TESTING.md` against the merged base.
- `scripts/contact-sheet.sh`: one arm added; `m6)` and the other arms byte-identical (`git diff` shows one hunk).
- Captures: `git diff --stat docs/shots/` shows only the four sheets (m4, m5, m6, m7) and the m7 captions tsv.
- The 26 ticket file: the three boxes, the Q-trace line, NOTES appended. Nothing above "Acceptance" changed but
  Decided step 3.
- The walk harness and every crate: unchanged (`git diff --stat crates/` empty).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide; then `scripts/m7-accept.sh`. Context guard as 26's: runs write to files; read summary lines only.

## Size
~45k.

## Rules
Worktree only; `CARGO_TARGET_DIR` in the worktree. Never read `PROGRESS.md` or `KNOWN_GAPS.md` (rows restated above
and in 26's NOTES). No capture's Right is ticked here: the sheets go to the user (rule 6).

## NOTES (builder, 2026-10-01)

No compaction. No scout: every claim below is a test result or a grep run here; 26's scouts cover the rest.
Base: main `e951ef4a` + 26's branch, merged by me (`61029294`; **no conflicts**; the ticket said the conductor would,
the conductor's message said to). Both tickets' NOTES kept as they were.

### Run (summary lines only)
- Gates: `cargo fmt --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0; `cargo test --doc
  --workspace` 0; `cargo nextest run --workspace`: **5004 run, 5003 passed, 1 failed**
  (`m7_walk::promise::recover::promise_recover_without_leaving`), 6 skipped.
- `scripts/m7-accept.sh` → **rc=1**: `1 nextest: 73 tests run: 72 passed, 1 failed` · `2 pty: 13 runs (m3–m6 ×
  sizes), 1 drifted or failed` · `3 sheet: … every file ≤ 200 KB: yes` · `4 perf: 3 files, 0 missing, 0 lines say
  missed` · `accept: 29 journeys walked, 29 at or under count, 0 over`. Nothing loosened.

### F-recover (product; not fixed — the conductor files it)
- The `u` half is **fixed** by 30s: `recover.rs:103` `assert_bytes` passes (every working file, LF `src/a.rs`
  included, byte-equal after `u` under `core.autocrlf=true`).
- **Broken, step 4:** `recover.rs:123` — after `J n ⏎ y` ("your edit to src/a.rs") `src/a.rs` is
  `…{\r\n    2\r\n}\r\n`; his lost edit was `…{\n    2\n}\n` (bytes 13,10 vs 10 in the test's output). The write is
  `RecoverTarget::Files` → `git restore --source=<backup_tree> --worktree -- <paths>`
  (`crates/graft-ops/src/operation/recover.rs:239-247`), which smudges through autocrlf. This is 30s's V5
  (`M7/30q-putback-fwd`; 30s §5: a gap, not 30s). 30s V5 also saw that the entry holding the edit has
  `worktree_raw_before: None` in this fixture (autocrlf set after the fixture is built), so a fix reusing
  `restore::raw` needs a raw source for that case — the fixer's call.

### pty (box 4)
- `docs/shots/m4-03-credential-masked-80x24.txt` :14: committed `▏••••••••`, drawn `▏◦•••••••` — the masked field's
  pulse (ADR-0038; the 160x48 caption already says a run can catch it). The same walk re-run 3× in compare mode
  (`accept/m4-rerun.sh`): 3/3 clean. A race, 1 in 4 here. **Question** (harness, no product line): the walk's
  `settled` wait (`.scratch/m4-remotes/pty-walk-m4.py:213-215`, `wait_quiet(STABLE, CAP)`) can end on a pulse frame.
  No retry added (rule 2). Every other m3–m6 capture `settled`; no capture rewritten (`git status docs/shots` after
  the run: only sheets and tsvs).

### Rows closed (each PASS in this run: `accept/claims.txt`, and the workspace run whose only FAIL is recover)
- M6/68: `graft view::journal::tests::an_aborted_entry_reads_aborted_by_you`.
- M6/77 row 1: `graft-ops::it recover::lost_edits_finds_the_edit_an_undo_over_drift_replaced` and
  `recover::lost_edits_finds_the_same_after_a_redo_over_drift` (26 wrote it `…_after_a_redo_over_drift`; the real
  name is at `graft-ops/tests/it/recover.rs:1018`; workspace run). Row 2: `m7_walk::journal::the_three_lost_work_journeys`
  (journey 15). **Not** by the Recover scenario: red at that very step (the edit comes back, in CRLF).
- :217: `graft::it journal_screen::a_refusal_in_j_shows_on_its_status_row`.
- :847: `graft-ops operation::tests::a_stash_drop_has_one_name`.
- :848: `graft-ui journal_screen::tests::a_cut_row_cuts_at_a_word`.
- :109 (Times I left): `m7_walk::restart_after_done`.
- **M6/u2, M6/rw2, M6/r47-49** (Q-push ruling): `m5_walk::every_scenario::push_journey_is_walk_gated` and
  `m5_walk::every_scenario::journey_rejected_push_dirty_to_pushed` (`every_scenario.rs:73,141`) — both PASS.
  Conductor: close these three.
- `M7/30q-setaside`: `graft-ops::it undo_bytes::u_after_a_set_aside_put_back_brings_back_an_lf_file_under_autocrlf`,
  `…::a_set_aside_put_back_keeps_an_lf_file_under_autocrlf`, `…_with_a_quoted_name`,
  `…::a_switch_with_edits_keeps_an_lf_file_under_autocrlf`, `…::a_set_aside_under_a_one_way_clean_filter_keeps_the_stripped_lines`,
  `…::a_put_back_whose_raw_step_cannot_land_says_so_and_drops_s` (every `undo_bytes::` test PASS, 20), and the
  Recover scenario's `u` step (`recover.rs:103`).
- The 80-column refusal (review 28 Q2): `m7_walk::promise::guard::guard_refusal_at_80_names_the_file_and_the_next_step`,
  `graft::it in_the_way_status::a_refusal_at_80_keeps_the_file_and_the_next_step`, `graft-ops
  operation::tests::in_the_way_rungs` (+ `_untracked_only`).
- Each 2a ticket's rows: as `scouts/26-gap-rows.md` §2 lists them; they rest on their own tickets' tests, all green in
  the workspace run (not re-read one by one here).

### Rows open
- `M7/30q-putback-fwd` — **broken**, observed again (F-recover above, `recover.rs:123`).
- `M7/30q-drift-eol` — not exercised by this run; stays as filed.
- f-guard's resize limit (a resize after a refusal keeps the rung, 30t NOTES) — **unverified**.
- 30i's `M7/13b-120` — its tick unticked (30i:90); not re-checked here.
- 30s's gap candidates (a non-`Exited` git error loses the raw words; a put-back killed before its raw step) —
  unverified (30s NOTES).
- The m4-03 pulse race (question above).
- Colour everywhere (rule 5) and every capture's look, the sheets included (rule 6): **unverified, not broken**.

### Other changes
- `docs/shots/m4-contact-sheet-captions.tsv`: row 5 was stale **and** 30u had joined rows 5 and 6 into one line (no
  newline before `m4-03-credential-masked-160x48.txt`, so the 160x48 capture had no caption row of its own). Split;
  row 5 now quotes the capture: border `⏎ send  ⌃U clear  esc cancel push main → origin/main`, bottom row `⏎ send
  esc cancel` (checked against `m4-03-credential-masked-80x24.txt:16,26`).
- `TESTING.md`: the Trace entry (Q-trace); Recover's "Fails today" re-stated for step 4; Guard's "Fails today" →
  "green since 30t/30t-b". The Right lines themselves unchanged.
- `docs/perf/m7-accept.txt`: rewritten by the script (its summary; the HEAD line says `+uncommitted`, as it was).
- Stale comment, not touched (crates out of set): `crates/graft/tests/it/m7_walk/promise/trace.rs:133-135` still says
  the 19 is "filed as a question in 26's NOTES".

### Touches
- `TESTING.md`: Trace, Recover and Guard entries only (`git diff --numstat` 6/5).
- `scripts/contact-sheet.sh`: one hunk, the `m7)` arm (`git diff | grep -c '^@@'` = 1).
- `docs/shots/`: m4 sheet + tsv, `m5-contact-sheet-1.txt`, `m6-contact-sheet-{1,2,4}.txt`, `m7-contact-sheet-{1,2}.txt`
  + tsv (`git diff --stat`). The m4 tsv is outside the ticket's list: the conductor's carry.
- Crates and the walk harness: `git diff --stat crates/` empty.
- 26's ticket: Decided step 3; a 26b line under boxes 1, 3, 4 (still unticked); a NOTES pointer.
- Spot-check of my own ticks: 5 of 5 re-checked against the command named (grep, wc, git diff) — held.

## NOTES — Re-run 2 (builder, 2026-10-01, main `a3901dee`)

No compaction. No scout: one script run plus greps; the only source read is the recover walk's asserts.
Base: main with 26, 26b part 1, 30s, 30t, 30t-b, 30u, 30v, 30w, 30w-b merged. No product, walk or capture change here.

### Run
- `CARGO_TARGET_DIR=<worktree>/target scripts/m7-accept.sh` → **rc=0**: `1 nextest: … 73 tests run: 73 passed, 4946
  skipped · rc=0` · `2 pty: 13 runs (m3–m6 × sizes), 0 drifted or failed` · `3 sheet: … every file ≤ 200 KB: yes` ·
  `4 perf: 3 files, 0 missing, 0 lines say missed` · `accept: 29 journeys walked, 29 at or under count, 0 over`.
- Workspace fmt/clippy/nextest/doc not re-run: this commit changes no Rust (only `docs/perf/m7-accept.txt` and the two
  tickets). The conductor's log at `a3901dee` records them green; I did not observe that run.
- Nothing loosened: `git diff db4392e7 HEAD -- crates/graft/tests/it/m7_walk/promise/recover.rs` removes no
  `assert`/`within_count`/`TOTAL` line; 30w's change there moves `core.autocrlf=true` from after the fixture build into
  `build(&[("core.autocrlf", "true")])`.
- Captures: no `docs/shots/` file changed after the run (`git status --short`: only the perf summary), so no sheet
  rebuilt.
- m4-03 80x24 pulse: clean in this one run (30w-b normalised the masked pulse). One clean run does not prove the race
  gone; 30w-b's tests are the evidence for that.

### Rows closed (in addition to "Rows closed" above)
- `M7/30q-putback-fwd` — Recover step 4 (`recover.rs:123`) PASS: the LF lost edit comes back LF under
  `core.autocrlf=true`. **Caveat:** 30w moved autocrlf into the fixture build, so the case 30s V5 saw (autocrlf set
  after the build, entry with `worktree_raw_before: None`) is no longer what this walk exercises. Not checked whether
  30w's own tests cover it → close for the from-the-start case; the set-later case stays open unless 30w names a test.
- M6/77 row 2: now also by the Recover scenario (step 4 PASS).
- The m4-03 pulse question: pty step clean this run; close on 30w-b's tests, not on this run alone.

### Still open
- `M7/30q-drift-eol`, f-guard's resize limit, 30i's `M7/13b-120`, 30s's two gap candidates: not exercised here.
- Colour everywhere (rule 5) and every capture's look (rule 6): unverified, not broken.
- Spot-check of my ticks: 6 of 6 (3 here, 3 in 26) re-read against the run's summary lines and `accept/nextest.txt` —
  held.

## NOTES — Post-fix re-run 2026-10-01 (builder, main `e08b8272`, after 36–40)

No compaction. No scout: one script run, `git show`/`blame`/grep on the lines named below.

### Run 1 (before re-take)
- `scripts/m7-accept.sh` → **rc=1**: 77/77 walks, `2 pty: 13 runs, 9 drifted or failed` (m4 80x24/160x48, m5
  80x24/160x48, m6 80x24/120x40/160x48/187x50/200x50), sheets ≤ 200 KB, perf 0 missing, 29/29 journeys.
- **Exit code on drift: 1** (observed). `failed=1` when `pty_bad > 0` (`scripts/m7-accept.sh:98`), `exit "$failed"`
  (:173). Not a defect.

### Every hunk (24 captures, 95 changed lines; the pty-diffs show only each file's first differing line, so every
drifted run was re-taken with `GRAFT_UPDATE_SHOTS=1` into the worktree and each line paired HEAD vs drawn)
- **36, ⓘ column gone** — 20 lines, each a pure `ⓘ`→space change (m5-07 ×2, m6-06-120, m6-17 ×5, m6-18 ×4, m6-19 ×5,
  m6-25 ×3). Cause: 22c3a1e7 removed the `NoUpstream` arm; `crates/graft-ops/src/mistake_radar/rules.rs:325` now
  matches `UpstreamFacts::None` with no finding.
- **36, knock-on of no finding** — `. explain` leaves the bar, `. more` takes its place (m6-19 ×5, m6-25 ×3: the
  `Marked` context, `crates/graft-ui/src/graph_screen/hints/context.rs:63-66`); the menu loses `Explain` and every
  item moves up one row (m6-25 ×3, lines 7–20: `crates/graft-ui/src/context_menu.rs:189-191`, `if marked`). Labels
  re-fit to the freed width (`F2 stat` at 187: the user's short rung, `hints/tests.rs:526`).
- **36, rejected-push wording (15 :774)** — m4-06 80x24 :26, 160x48 :50: `has moved · p pull first` →
  `has commits you don't · p pull, then m or r` (`crates/graft-ops/src/apply/remote.rs:301`).
- **36, offer only after a slow sort (15 :778)** — m6-06-relaunch-sticky-120x40 :26: the commit-graph offer row is
  blank. `crates/graft/src/update/commit_graph.rs:19,23-25` (`SLOW_SORT` 300 ms); 36's NOTES :159 say no small
  fixture's sort raises it now. Not the B/X focus gate: the row is absent, not keyless.
- **37, ghosts above the top row** — m5-02-preview-merge 80x24/160x48 :4-13: the `after merge` label and the ghost
  merge row `◎┈╮ [◆ main] Merge branch 'feature' into main` now draw above `[feature]`; the rest moves down 2.
  `crates/graft/src/view/preview.rs:146` (`Some(below) if below < visible => unpinned(label)`, 37ca8083).
- **40, `o take` on the bar** — m5-07 ×2, m6-25 ×3: `o take` after `⏎ open`
  (`crates/graft-ui/src/graph_screen/hints/lists.rs:124-126`); labels after it drop to keys to fit (`u undo  S stash`
  → `u  S` at 160; trailing `+`/`x` off at 80).
- 37's `left behind:` row and 38's scratch-row removal: no hunk in the m3–m6 pty set.
- **Unexplained: none.**

### Re-take
- Mechanism as 26/26b: each drifted walk re-run with `GRAFT_UPDATE_SHOTS=1` (same plan tsvs, same sizes); 24
  captures rewritten, exactly the drifted set. Sheets rebuilt: `scripts/contact-sheet.sh m4`, `… m5`,
  `… --captions-order m6 docs/shots/m6`; m7 by the script's step 3 (HEAD's m7 sheets lagged 36/40's m7 re-takes).
  Largest file 179 608 B (`m6-contact-sheet-1.txt`); `NOT RECORDED` 0 in every m4–m7 sheet.
- **Run 2: rc=0**: `1 nextest: 77 tests run: 77 passed` · `2 pty: 13 runs (m3–m6 × sizes), 0 drifted or failed` ·
  `3 sheet: … ≤ 200 KB: yes` · `4 perf: 3 files, 0 missing, 0 lines say missed` · `accept: 29 journeys walked, 29 at
  or under count, 0 over`; `ls -A accept/pty-diffs | wc -l` = 0.
- Gates, workspace-wide, after the re-take: fmt 0, clippy 0, `cargo nextest run --workspace` 5061/5061 (7 skipped),
  `cargo test --doc --workspace` 0. No file changed by them (`git status`).

### Findings (conductor)
- **Stale captions (textual)**, `docs/shots/*captions*.tsv` not edited (out of this ticket; 36 left them stale on
  purpose): m4 tsv :11 still quotes `has moved · p pull first` and claims "the whole line" — at 80x24 the reason now
  cuts to `has comm…` (keys kept); m6 tsv :113, :116, :117 still say the bar starts `. explain  m merge`; m6 tsv :173
  says the menu leads `Checkout fix`, `Cherry-pick`, then `Explain`.
- **Question:** the m4-06 80x24 reason now loses `you don't` to the `…` — 36's longer words vs the 80-column fit. A
  wording/space call, not a defect claim.
- **Question (timing):** m6-06-relaunch-sticky-120x40 now relies on the real `graft` binary's sort taking < 300 ms on
  the scratch repo (`commit_graph.rs:23-25`). A loaded machine past 300 ms would draw the offer row and drift.
- Colour and each capture's look (rules 5, 6): unverified, not broken.

### Captions fixed (conductor's follow-up, same day)
Each rewritten to quote its re-taken capture line for line:
- m4 tsv :11 (`m4-06-rejected-sticky-80x24`): **36** wording, `has comm… · p pull, then m or r · P force with lease`. It
  no longer claims "the whole line": the reason is cut with `…` at 80 columns and the keys are kept (capture :26).
- m6 tsv :113 (`m6-19` 120x40): **36**, the row starts `⇥ banner  c finish  m merge  . more` and has no `. explain`
  (capture :42). The old caption also left out `c finish`.
- m6 tsv :116 (`m6-19` 200x50): **36**, `. more` after `F3 renderer` and no `. explain` (capture :52).
- m6 tsv :117 (`m6-19` 80x24): **36**, `⇥ banner  c finish  m merge  . more` (capture :26).
- m6 tsv :173 (`m6-25` 120x40): **36** removed `Explain`, so the menu reads `Checkout fix`, `Cherry-pick 401cfbe (o)`,
  `Copy SHA (y)` (capture :4-6). The `(o)` comes from 40.
- Sheets rebuilt: `scripts/contact-sheet.sh m4` and `… --captions-order m6 docs/shots/m6`. Only m4, m6-2 and m6-4
  changed. Largest file 179 608 B; `NOT RECORDED` 0.
- Run 3 of `m7-accept.sh`: **rc=0**, 77/77 walks, `13 runs, 0 drifted`, sheets ≤ 200 KB, perf 0 missing, 29/29 journeys.
- Not checked here, out of the named five: other tsv rows that still contain `explain`/`Explain` are m5 :26-28 (the
  hand-built marked context, which still has a marker) and m6 :9, :139, :141, :143.

### Captions, second pass (conductor's follow-up)
I grepped every m3–m7 caption tsv for `explain`, `ⓘ` and `no upstream`, and also for `has moved` and `pull first`.
The hits are the rows below, plus the five fixed above. The m3 and m7 tsvs had none.
- **Stale, fixed:**
  - m6 :139 `fix-radar-bookkeeping`: frame 3's item 2 is `Cherry-pick (o) — already on main` (capture :110). Changed
    by **m7-11** (2996546e). Frame 1 still holds: no `ⓘ` on main rows (capture :19).
  - m6 :141 `fix-box-owns-the-bar`: frame 2 shows the **36** wording, `has commits you don't · p pull, then m or r`
    (capture :101).
  - m6 :143 `menu-fixed-slots`: **36** removed `Explain` and the no-upstream `ⓘ`. Item 3 is now `Copy SHA (y)`, and
    item 2 is `Cherry-pick <sha7> (o)` (capture :18-20, :62-64), with the `(o)` from **m7-11**.
  - m6 :144 `fix-undo-drift-way-back`: **36** removed the `ⓘ` the caption claimed on `fix` in frame 2 (capture :68
    `● ▏  [fix]`).
- **Not stale, left alone:**
  - m5 :26-28 `m5-hints-marked-*`: the hand-built marked context still leads `. explain` (captures :47/:63/:71).
  - m6 :9 `m6-05`: it hit on "has moved on", and all four quoted strings are in the capture.
  - m6 :113, :116, :173: these are my pass-1 rows, which say "no `. explain`".
- **Finding, outside my file set (crates):** two capture headers written by test code still state the old behaviour.
  - `docs/shots/m6/fix-undo-drift-way-back-120x40.txt:13` says "its `ⓘ` is on `fix`". The header comes from
    `crates/graft/tests/it/undo_over_drift.rs`.
  - `docs/shots/m6/fix-box-owns-the-bar-120x40.txt:9-10` says "`p pull first`".
  - These are textual (rule 7) and belong to whoever holds `crates/graft`.
- The m6 sheet was rebuilt; only sheets 3 and 4 changed. The largest file is 179 608 B and `NOT RECORDED` is 0.
- Run 4 of `m7-accept.sh`: **rc=0**, `13 runs, 0 drifted`, 77/77 walks, 29/29 journeys.

### Touches
- `docs/shots/`: the 24 re-taken captures + m4, m5 (×2), m6 (×5), m7 (×2) sheets (`git status --short docs/shots`:
  34). Captions tsvs: unchanged. `docs/perf/m7-accept.txt`: the script's summary. Crates: none (`git status` shows no
  `crates/` path).

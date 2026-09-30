# m7-acceptance-finish — 26's continuation: re-run the walk after the fixes, tick 1/3/4, the sheets and the promise header

Status: partial (2026-10-01) — the walk still fails on the product (F-recover moved to step 4; one pty race); see NOTES
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
- [ ] 26 box 1: `scripts/m7-accept.sh` exits 0 (the run's last line and `rc`); the red side is 26's "Red
      demonstration", already shown. **Not met:** rc=1 (`accept/run.txt`; NOTES "Run").
- [ ] 26 box 3: `promise_rewrite_without_leaving`, `promise_trace_without_leaving`, `promise_recover_without_leaving`
      PASS in `accept/nextest.txt`; the recover one's byte asserts present (quote the assert's file:line).
      **Not met:** rewrite and trace PASS; recover FAIL at `recover.rs:123`. Its byte asserts are present and the `u`
      one holds (`recover.rs:103` `assert_bytes(repo, &reset_state)`, helper :145-158). NOTES "F-recover".
- [ ] 26 box 4: every m3–m6 pty walk green in compare mode (step 2's summary line; `accept/pty-diffs/` empty).
      A file `captures-verify` stopped on and nobody fixed → box 4 stays unticked, the file named.
      **Not met:** `2 pty: 13 runs (m3–m6 × sizes), 1 drifted or failed`; `pty-diffs/m4-80x24.txt`. NOTES "pty".
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

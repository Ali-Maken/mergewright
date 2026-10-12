# 48r: review fixes for ticket 48 (names table + on-disk move)

Status: done
Blocked by: none (48 merged at f11ac645)
Build with: **Opus**. The on-disk move: a wrong fix deletes his backups or journal lines unread.
Brownfield: ticket 48 NOTES and `scouts/26-rename-on-disk.md` serve as the scout; the review names every line.
Needs: none. Files: graft-ops `rename/*` and their tests, graft `config/load.rs` (minor 7), MAP.md. Runs beside 49/50 only
if their file lists stay disjoint from these.

## Read first
- `docs/reviews/m8-mergewright-48.md`, whole (Major 1–2, Minor 1–7)
- Ticket 48: Decisions, Part B, NOTES

## Scope (from the review; conductor's routing)
- **Major 1 (data loss):** which generation is scratch is inferred from markers that exist, never recorded or re-checked
  (`strays.rs:75-80`, `scratch.rs:35-49`, `:84-105`, `locks.rs:27-31`). Record the source in the marker (the review's
  suggestion, e.g. `switched tributary`) or an equivalent the builder argues for in NOTES; scratch is deleted only when
  the record says it is scratch. Test-first: each trigger (crash mid stray pass; lock file not yet created) as a
  real-repo test that loses a backup or journal line before the fix.
- **Major 2:** tests for the uncovered windows: Mergewright `switched` from a Tributary source; `switched` with
  scratch; a stray pass cut short; `CleanedUp` beyond `it/rename.rs:242`.
- **Minor 1, 2, 4, 5, 7:** fix as the review says, each with a test where it is behaviour. Minor 2 (stray pass stuck on
  two generations' refs with the same rest) test-first.
- **Minor 3 (question):** answer by test whether `check_journals` must read kept-aside journals; fix only if it loses
  something.
- **Minor 6:** reword the two tick texts in ticket 48.

## Acceptance (each box: the test named at tick time)
- [x] Major 1 trigger A (crash mid stray pass) test: red before, green after; nothing lost; `git fsck` clean.
      Test: `graft-ops it rename::generations::windows::a_stray_pass_cut_short_keeps_what_tributary_wrote_next`.
      Red on base source: second open `CleanedUp { refs: 0, files: 0, kept_aside: None }`, the backup and journal
      line removed unread. Green: backup carried, journal kept in `older-version/1/tributary/`, fsck clean.
- [x] Major 1 trigger B (lock file not yet there) test: red before, green after.
      Test: `…::windows::an_older_build_moving_alongside_is_shut_out` (a real second process, via the git shim at the
      mirror transaction, takes Tributary's lock as a Tributary build would). Red on base: "the older build moved,
      and its backup was removed unread". Green: it is refused.
- [x] Major 2: one named test per window listed above.
      (a) `…::windows::switched_from_tributary_cleans_up`; (b) `…::windows::switched_from_graft_removes_scratch`;
      (c) `…::windows::a_stray_pass_cut_short_keeps_what_tributary_wrote_next`; the re-check
      `…::windows::a_newer_generation_with_a_marker_is_never_scratch`. All are `CleanedUp` tests beyond
      `it/rename.rs:242`.
- [x] Minor 2 test: red before, green after. `…::generations::minors::two_generations_one_name_carry_the_newer`
      (red on base: `LeftOld(... multiple updates for ref 'refs/mergewright/x' not allowed)`).
- [x] Minor 1, 4, 5, 7: test or grep named per item.
      - Minor 1: grep `rg -n 'remove_file|remove_dir_all' crates/graft-ops/src/rename/` — the lock is filtered out of
        `scratch::remove_marker` and kept by `scratch::remove_all_but`; only `locks::Alone::release` removes a lock,
        after `drop(locks)`. Lock held during the move: `…::windows::an_older_build_moving_alongside_is_shut_out`.
      - Minor 4: `…::generations::minors::staying_old_reports_the_names_on_disk` (red on base: `Graft`, want
        `Tributary`).
      - Minor 5: grep `grep -rn 'refs/tributary/\*\*\|refs/tributary/backup/…\|`refs/graft/` or' crates` → 0 doc
        hits; 10 comment lines reworded (list in NOTES).
      - Minor 7: `graft config::folder::tests::each_earlier_folder_alone_moves` (now writes `<dir>.lock`;
        `only_the_new` asserts it is gone; red on base load.rs: "Tributary's lock").
- [x] Minor 3: test named, outcome stated. `…::generations::minors::a_kept_aside_journal_naming_a_missing_backup_keeps_old`.
      Outcome: **nothing lost**; the move refuses before its switch on every open (`KeptOld(ReadBack …)`), Tributary
      names stay in force. Not fixed (ticket: fix only on loss). See NOTES for the proposal.
- [x] Every older-generation scenario from 48 still green: full `cargo nextest run` workspace (see NOTES), including
      every `rename::generations::*`, `rename::*`, `rename_status`, `rename_opens`.

## Touches
Callers of `settle_names`; journal reading (kept-aside); config folder; status words; MAP.md; ticket 48 NOTES.

## NOTES
No compaction. Scout: none new — ticket 48 NOTES, the review's line references and the rename module (read whole,
~1.7k lines) stood in for one, as the ticket says.

**Major 1 — the fix (cause).**
- `rename/marker.rs`: the switch writes `switched <source dir>`; `clean_up` takes the source from the marker.
  `scratch::was_in_force` (inference from earlier markers) is gone. A bare `switched` (written by ticket 48's builds,
  e.g. a dev run on his repo) reads as source unknown: **no generation is scratch**, everything goes through the
  stray pass (compared, never removed unread; worst case a false "older version wrote" row).
- `rename/scratch.rs`: a generation newer than the source is scratch only if its marker is absent **now**, checked
  under the locks; a marked one goes to the stray pass (`a_newer_generation_with_a_marker_is_never_scratch`).
- `rename/strays.rs`: three loops — every ref folder, then every marker, then every folder — so markers go after
  everything that held data (the review's third point).
- `rename/locks.rs` (trigger B): every earlier generation that moves names (`has_marker`, i.e. Tributary) has its
  lock **made if missing** and taken alone; `CURRENT`'s is taken first. Graft's is taken only if present (its builds
  never lock). Cost: a Graft repository gets a `.git/tributary/lock` for the length of the move; removed on release.
- Minor 1: lock files are never removed while held. `Alone::release` drops the locks, then removes each earlier
  generation's lock file and empty folder; called only after `clean` is written (migrate, clean-up, older-wrote).
  A release failure is reported as `LeftOld` (rule 3), and the next open retries it. A run that ends before `clean`
  (KeptOld/LeftOld) leaves a created lock file; the next successful run removes it.
  Remaining window, by design: an earlier build opening the lock file between drop and unlink holds an unlinked
  inode; whatever it writes then is a stray the next pass compares. No loss path found.
- `rename.rs` was 426 lines after the change: `older_wrote`/`clean_up` moved to new `rename/reopen.rs` (375 now).

**Minor 2:** `refs::carry_or_set_aside` walks old refs newest generation first; a target this pass carries counts as
the new ref for an older generation's same rest (equal → delete only, differs → kept aside).
**Minor 4:** `stay_old` re-reads `names_in_force` after the marker check; on a read error it keeps the handle's
answer and says both on the row.
**Minor 5 (comment lines reworded, 10):** `graft-ops/src/snapshot.rs:172`, `graft-git/src/history_reads/unreached.rs:27`,
`graft-ops/tests/it/{aborted_fetch.rs:237, journal_list_oracle.rs:301, rename/review.rs:16}`,
`graft/tests/it/{repo_truth.rs:17, m3_walk.rs:207,364, radar_bookkeeping.rs:3, m7_walk/rewrite_editor.rs:85}`.
These are outside the ticket's file list (graft-git, graft tests) — comment-only; flag if 49 collides.
**Minor 7:** `config/load.rs::remove_earlier_lock` removes `<config_root>/<dir>.lock` once that folder is settled
(Nothing/Moved/OlderWrote), without taking it; a failure is a `ConfigFolder` warning.
**Minor 6:** ticket 48's two ticks reworded.

**Minor 3 — answer.** Reachable only if a backup named by a kept-aside journal is gone (no code prunes backups;
by hand). Then every open is `KeptOld`: nothing lost, but the move never happens. The **live** journal has the same
property. Proposal (not done, ticket says fix only on loss): `check_journals` skip paths under `older-version/` —
a kept-aside journal is an archive, never read as live.

**Red-before evidence:** fixed source set aside, base `rename/*` checked out from 9263a173, the new tests run, the
fix restored (same for `config/load.rs`). Outcomes quoted in the ticks.

**Touches:**
- Callers of `settle_names` (graft `repos.rs`, workers): signature and outcomes unchanged; full nextest green.
- Marker readers: graft-git `names_in_force` stats only (`repo/names.rs:31`); graft `repos.rs:544` test reads
  `clean` only — unaffected by the new `switched` line.
- Journal reading: unchanged (Minor 3 answered by test only).
- Config folder: `load.rs` + `folder/tests.rs`.
- Status words: no new outcome; `rename_status.rs` green.
- MAP.md: graft-ops `settle_names` and `LOCK_FILE` lines (`map_coverage` green).
- `file_size` ratchet: new test files under 400 (`windows.rs` 328, `minors.rs` 121); `rename.rs` 375.

**Gates (workspace, after the last code/MAP edit):** fmt ok; clippy `-D warnings` ok; nextest 6408/6408 (16 skipped); doctests ok (3 passed).

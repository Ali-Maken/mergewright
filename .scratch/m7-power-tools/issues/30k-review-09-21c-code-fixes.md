# 30k: code fixes from review 09..21c (Major 2, Minors 3-8, 12, 13)

Status: done
Build with: Opus. Minor 13 journals a config removal via 30j's `SetBranchConfig` (undo path); the rest are small.
Needs: nothing new.
Blocked by: nothing (all merged). Concurrent: 07b (holds graft `update/operate.rs`, `jobs/runner.rs`, `update/events.rs`,
`view/rows.rs`, graft-ui `graph_screen/mod.rs`, `graph_screen/banner.rs`, graft-ops `operation.rs`, `apply/fold.rs`),
30e (graft-ops `apply.rs`, `apply/error.rs`, `rewrite/dependents.rs` doc comments). Don't touch those. Minors 1 and 2
(in `operate.rs`) went to 07b.
Scout: none. The review names every line (`docs/reviews/m7-09-to-21c.md`, Major 2, Minors 3-8, 12, 13). 30h split
files after the review: re-find each line on main first (`graft-ui/src/history.rs` is now `history/{list,file_pane}.rs`).

Files: graft-ui `src/history/file_pane.rs` (or wherever `binary_words` now lives), `src/graph_screen/render.rs`;
graft `src/update/amend_into.rs`, `tests/it/m7_walk/amend_into.rs`, the preview file drawing "files set aside" (name it);
graft-ops `src/operation/history.rs` (branch delete), tests beside each; the fixture/corpus clock for Minor 7 (name it);
captures their tests re-take; MAP.md per crate. Anything else → NOTES, with the line.

## Fixes (each decided)

1. **Major 2** — `binary_words` re-matches the pane its caller matched. Bind the fields in the outer arm (or pass the
   `Binary` payload) to a function that takes only that; no unreachable arm.
2. **Minor 3** — refs unread: `a` on HEAD's row must take today's amend, not the older-commit path (`amend_into.rs:108`
   vs comment :149). Return `None` when refs are unread.
3. **Minor 4** — `absorb_chosen` gets the same early `live_operation` (P14) check `a_key` has, with `a`'s words.
4. **Minor 5** — footer reveal subtracts the row tag's width (`render.rs:464-468`) so a subject cut only by the tag is
   revealed.
5. **Minor 6** — preview's set-aside count: count what is set aside and put back (the unstaged part); agree with the
   landing's "your unstaged edits". Re-take `amend-into-preview-187x50`, `absorb-preview-120x40`.
6. **Minor 7** — landed capture shows "futu…": pin the rewritten commits' dates to the corpus clock. Re-take
   `amend-into-landed-120x40`.
7. **Minor 8** — binary header: drop `+0 −0` for binary files. Re-take `history-binary-140x40`.
8. **Minor 12** — move the two `#[ignore]` put-in perf tests (`m7_walk/amend_into.rs:567`, `:741`) into the heavy tier
   (`heavy-tests` feature), so every review runs them. Don't run them yourself (measurement never runs in a helper).
9. **Minor 13** — deleting a branch leaves `branch.<b>.*` config: remove the section on delete and journal it through
   30j's `SetBranchConfig`, so undo restores it and a later branch of the same name inherits nothing.

## Acceptance

- [x] Major 2: no fallback arm; `history_binary_*` snapshots unchanged — `binary_words(old, new, sizes_failed)` takes only
      the `Binary` fields (`graft-ui/src/history/file_pane.rs`); `history::tests::history_binary_140x40` and
      `history_binary_sizes_failed_140x40` pass unchanged.
- [x] Red first: Minor 3 test (refs unread, `a` on HEAD → amend path) — `update::amend_into::tests::a_on_head_with_refs_unread_is_todays_amend`
      (red at the `plan_job().is_none()` assert, then green).
- [x] Red first: Minor 4 test (absorb during a live op refused with `a`'s words) —
      `update::amend_into::tests::absorb_during_an_operation_keeps_p14` (red: a plan job started; green: status equals `a`'s).
- [x] Red first: Minor 5 test (subject cut only by the tag is revealed) —
      `graph_screen::tests::a_subject_cut_only_by_the_tag_is_revealed` (red at the footer assert).
- [x] Minor 6: red first on the count — graft-ops `absorb_staged::the_preview_counts_only_the_unstaged_edits_set_aside`
      (red: 3 ≠ 1) and `the_preview_counts_nothing_set_aside_with_nothing_unstaged` (red: 1 ≠ 0); app side
      `m7_walk::amend_into::a_put_in_with_nothing_unstaged_has_no_set_aside_row`, `the_edits_rows_agree_in_the_put_in_preview`,
      `the_edits_rows_agree_in_the_absorb_preview`. Captures re-taken: `amend-into-preview-187x50.txt:44` now
      `your edits: 1 file set aside, put back after`; `absorb-preview-120x40.txt` loses its `your edits: 3 files…` row
      (nothing unstaged) — `capture_amend_into`, `capture_absorb`.
- [x] Minor 7: capture shows a date, not `futu…` — `amend-into-landed-120x40.txt:3-5` read `GF 2d   <sha>`;
      `m7_walk::amend_into::capture_amend_into` (asserts no `futu`; runs its pinned child
      `capture_amend_into_git_clock_pinned`).
- [x] Minor 8: capture line 2 without `+0 −0` — `history-binary-140x40.txt:2`; `history_place::capture_history_binary_140x40`;
      red first `update::place::tests::history::a_binary_header_has_no_line_counts`.
- [x] Minor 12: both tests under `heavy-tests`, not `#[ignore]` — moved to `tests/it/m7_walk/amend_into/perf.rs`
      behind `#[cfg(feature = "heavy-tests")] mod perf;`; `cargo nextest list --workspace --features heavy-tests` lists
      `m7_walk::amend_into::perf::perf_dependents_check` and `::perf_dependents_spawns`; `cargo clippy -p graft --all-targets
      --features heavy-tests -D warnings` clean. Not run (measurement). `scripts/m7-09-perf.sh` updated to the new paths.
- [x] Red first: Minor 13 (real git) — graft-ops `history_ops::branch_delete_takes_its_config_and_undo_puts_it_back`
      (red: `branch.feature.remote` still `origin` after the delete; green: gone, `u` restores both keys, redo + new
      same-name branch has no upstream). Unit: `operation::history::tests` plan line `git branch -D feat`.
- [x] MAP.md each crate touched (graft: `a_key`, `absorb_chosen`; graft-ops: `PreviewPlan::set_aside`, `RefDeleteParams`;
      graft-ui: no public item changed). Gates workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest 4623/4623,
      doctests pass.

## Touches (check each, say how)

Other callers of `binary_words`; every capture with a binary pane, a row tag in the footer, or the set-aside count;
every undo of a branch delete; journal read of old entries. KNOWN_GAPS rows this closes (list; don't edit).

## NOTES (builder)

Scout: none, per the ticket (the review names every line); each line re-found on main first.

Fixes, cause or symptom:
- Major 2 — cause: the second match is gone; the function takes only the Binary fields. `draw_file` hit clippy's
  60-line cap, so Reading/Binary share `header_and_line` (same drawing).
- Minor 3 — cause: `older_target` returns `None` when refs are unread (`state.refs.as_deref()?`).
- Minor 4 — cause: `refused_by_live_operation` shared by `a_key` and `absorb_chosen`.
- Minor 5 — cause: `drawn_tag` is the one rule for whether the tag draws; `draw_subject` and `subject_truncated` both use
  it. `subject_truncated` gained an `ascii` argument (footer.rs passes `view.ascii`).
- Minor 6 — cause, at the count: `graft-ops/src/preview.rs::shown_aside` counts unstaged files for a put-in/absorb
  (`Rewrite` with `made` or `index_base`); `Operation::sets_aside` untouched (apply still stashes staged edits — it
  must). `update/preview.rs` draws no row when the count is 0. Cost: absorb's staged hunks that **stay staged** are also
  set aside and put back, but are not in the count; the box already names them (`stays staged · …`).
- Minor 7 — cause in the test, not the app: git's clock was wall time while the fixture clock is `CORPUS_NOW`. The
  capture now runs in a child of the test binary with `GIT_COMMITTER_DATE` pinned (the `set_aside_kill` pattern;
  `std::env::set_var` is `unsafe`, denied). Rewritten SHAs are now stable, so the `#######` mask is gone.
- Minor 8 — cause: the header skips line counts when `view.binary` is set.
- Minor 12 — moved, not gated: neither test asserts a budget (they print). A failing gate on 16.7 ms would need
  `--release` (ADR-0021) — question for the conductor whether to add one. `perf_dependents_spawns` re-runs itself as a
  child with its own `GIT_TRACE2_PERF` dir when the variable is unset, so the heavy tier can run it without the script.
- Minor 13 — cause: a branch's delete is now `git branch -D <b>` (removes `branch.<b>.*`; absent section is no error);
  the journal already records config before/after for touched refs (30j), so undo writes it back via
  `SetBranchConfig` with no new code. Cost: `branch -D` has no expected-id compare-and-swap; `check_delete` still
  refuses a moved branch against facts read just before apply (ms window). Also: git refuses to delete a branch checked
  out in another worktree, which `update-ref -d` did not. Tags keep `update-ref -d` CAS. `update-ref -d` + a separate
  config removal was not possible without touching `operation.rs` (07b) or the executor (30e): git has no
  idempotent section removal, and `plan` sees no config.

Touches:
- `binary_words` callers: one (`grep -rn binary_words crates/`).
- Captures with a binary pane: `history-binary-140x40` (re-taken), `history_binary_*` snapshots (unchanged, pass).
- Row tag in the footer: `subject_truncated` has one caller (`footer.rs:60`); graft-ui 552/552 incl. reveal tests.
- Set-aside count: only reader of `PreviewPlan::set_aside` for drawing is `update/preview.rs:1031`; `put_back` still
  keys on `Some` (unchanged). Every m6 set-aside capture passed unchanged (rebase/checkout counts untouched).
- Branch delete undo: `history_ops::branch_delete_applies_undoes_redoes`, `deleting_a_ref_draws_it_going_away` pass;
  the only test naming the old command line was the unit test (updated). Old journal entries: `config_before/after`
  empty → no config steps, undo as before.
- Docs: `m7-contact-sheet-captions.tsv` rows 7, 9, 10 and TESTING.md (binary capture, fix-into-older row) updated.
- KNOWN_GAPS rows closed: not checked — builders do not read KNOWN_GAPS.md; conductor to map (review says Minor 5 had
  no row).
- No compaction.


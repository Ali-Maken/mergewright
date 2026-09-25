# Review: m6-conflicts tickets 77 and 79 (undo over drift, and the HEAD re-read after an undo)

Reviewer: `reviewer` (Opus). Fixed hash `18e88d8c`, in a detached worktree with its own `CARGO_TARGET_DIR`.
Diffs reviewed: 77 is `1bef5053^1..1bef5053`, and 79 is `18e88d8c^1..18e88d8c`.
Colour is **unverified** (rule 5). How the captures look is the user's call (rule 6).

**Count: 0 Critical · 2 Major · 13 Minor.**

## Gates (run one at a time, workspace-wide, at `18e88d8c`)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test` (fast tier) | pass: 3320 passed, 0 failed. **247 s wall.** Peak RSS 352 MB (largest child, `time -l`) |
| `cargo nextest run` (for per-test times) | pass: 3317 passed, 1 skipped. 179 s wall. Peak RSS 273 MB |
| heavy tier, `cargo test --features heavy-tests -- --test-threads=2` | pass: 3353 passed, 0 failed. 408 s wall, under the 10-minute budget. Peak RSS 392 MB |
| release perf run (`frames::` + `heavy::budgets_hold`, `--release`, 1 thread) | pass: 13 of 13, 74 s. Worst frame p99 is **6.34 ms** (pixel tier, 50k refs); every Unicode-tier p99 is 4.3–4.6 ms. First paint on the 100k fixture: 18.8 ms with a commit-graph, 68.3 ms with 50k refs, 104 ms realistic with a commit-graph, and 120–121 ms without one. Peak RSS 106 MB |
| fuzz, 4 targets, 60 s each (`-rss_limit_mb=1024`) | no crash: conflict_parse 578 636 runs, parse_file_list 567 039, parse_porcelain_v2 564 663, parse_unified 581 379. Neither ticket touches these parsers |
| benchmarks | not run. The diff touches neither graft-graph nor graft-ui (`git diff --stat` is empty for both) |

- **The 5 slowest tests** (nextest):
  1. `graft-ops::it undo_replaced::replaced_names_exactly_what_the_run_took`, 15.55 s (**new in 77**; 12.54 s run alone)
  2. `undo_cancel::cancel_at_every_step_boundary_is_clean`, 11.47 s
  3. `restart_whole_file::gate_and_apply_agree_on_every_shape`, 10.13 s
  4. `conflict_ops::whole_file_shapes_resolve_like_git`, 10.02 s
  5. `m5_walk::every_scenario::merge_resolved_with_the_keys`, 9.00 s
- **`target/`:** 11 GB for this review's own target dir (debug, heavy and release), plus 856 MB for fuzz. The shared `target/` is 7.2 GB.
- **These times are contaminated.** Load average was 7.5–8.3 throughout, because ticket 70's builder was running at the same time. The fast suite's 247 s (`cargo test`) and 179 s (nextest) are over, or at, the 3-minute budget. That is **unverified on a quiet machine**, the same as the 68+69 review found (183–204 s).

## Answers to the conductor's questions

- **Is the inverse proven on real repos, including the staged cases?** Yes, for everything except the whole-tree case (m8).
  - graft-ops `undo_replaced::replaced_names_exactly_what_the_run_took` covers 11 scenarios, counted in `scenarios()`, `tests/it/undo_replaced.rs:144-229`: unstaged, staged, staged plus disk, inside plus outside, three files, untracked (two ways), a delete by hand, untracked outside the paths, staged with the disk copy put back, and a redo.
    - Each one checks the named set against a set computed with git alone (`taken`, :111-127).
    - It checks the backup commit holding the pre-`y` blob (`backup_keeps`, :236-270).
    - It checks that `U` restores `state_of` exactly (:347).
  - At app level, `a_staged_edit_comes_back_staged` and `a_staged_and_unstaged_edit_both_come_back` compare `symbolic-ref`, `ls-files -s`, porcelain v2 with untracked files, `diff --cached --raw` and the file bytes (`tests/it/undo_over_drift.rs:266-340`).
  - All of these passed in both of my runs.
- **Does the box's promise match what `U` does?** Yes, for `u`: unstaged, staged, both, and the time machine's `U (2 times)` (`the_time_machine_names_the_presses`, :457-503). For a redo, the box promises only the backup, which is honest (see the next answer and m3).
- **Is the 77 builder's deviation acceptable?**
  - **5 git calls:** acceptable in kind, because it runs on the drift path only. **The count is wrong, though: the worst case is 8** (m1).
  - **A redo names no key back:** acceptable. The spec's `u` was wrong about the journal: `next_undo` passes over `Redo` entries (`crates/graft-ops/src/journal.rs:259-272`, verified). No key brings the edit back without a design change. The gap row is mis-cited and misclassified, though (m3).
- **79: was every `Refresh` choice audited?** The graft-ops half is complete. I checked each verdict in the NOTES against `restore.rs:275-282`, `apply.rs:338`, `apply/resume.rs:375` and `apply/fold.rs:161`, and they hold.
  - The graft side was not audited.
  - `jobs/undo.rs:215-232` is sound: `None` only before `execute` is entered, and `Everything` otherwise.
  - `jobs/operate.rs:174-178` uses the same "refs stand in for HEAD" test that 79 removed (m10, a question).
- **Captures and the settle-wait:**
  - Frames 2 and 3 now wait for the reload and the Radar, and they assert `[◆ main]` and `[◆ fix]` (`undo_over_drift.rs:547`, `:557`).
  - Frame 1 was not settled and still shows `sorting…` (m6).
  - The contact sheet's part 4 matches the capture byte for byte in the 80x24 block. `KNOWN WRONG` appears 0 times in it.
- **Swallowed errors:** none in the new code.
  - A failed `replaced` read aborts the undo as `UndoError::Facts` (`undo/execute.rs:175`), so it is not swallowed.
  - The `map_or_else` in `redo_waiting` (`jobs/undo.rs:506-515`) falls back to a name. It is not a failure.
  - `left_cuts`'s `unwrap_or(path)` cannot be reached.
- **A status that reads as an error when nothing is wrong:** none. `nothing to undo ✓ · U redoes checkout fix` is `Status::Done`.
- **Terminal sanitising:**
  - Paths reach the box through `sanitize_line` in `with_consequence` (`update/undo.rs:578-596`).
  - The status is sanitised at draw time (`view/frame/bars.rs:288-291`, `view/stash.rs:109`).
- **Git on the UI thread:** none. The `replaced` read and `redo_waiting` run in the undo job, and `update::undo` stays pure.
- **`unwrap`/`expect` outside tests:** none added.
- **Feature pipeline:** 77 cites `docs/research/checkout-cherry-pick.md` §1 and §4, and the file exists with the JetBrains and GitHub Desktop rows. The ticket has Journeys, and the step counts are asserted: `u y` = 2 (`:175`), `U` = 1 (`:194`), `u U` = 2 (`:254`) and `U U` = 2 (`:492`). These match the ticket's Journeys table.

## Ticks spot-checked: 9 checked, 9 held

- **77, all 6 ticks.**
  - (1) Every named test exists and passes. The reds are taken on the NOTES' word.
  - (2) The `Keys` counter asserts are at the lines above.
  - (3) `the_drift_box_line_is_never_cut` loops `(80,24),(160,48)` × long and short (`view/tests.rs:3108-3143`).
  - (4) There are 11 scenarios, counted. The `a_clean_undo_spawns_no_diff_tree` test is green.
  - (5) Every capture test is green in the suite, so the files match what the code draws.
  - (6) The pin asserts `Done { "nothing to undo", Some("U redoes checkout fix") }`, then `U` restores byte-exact (`undo_over_drift.rs:246-262`).
- **79, all 3 ticks.**
  - (1) The table test and both graft-ops tests exist and pass.
  - (2) The audit verdicts hold for graft-ops, which is the tick's scope (`restore.rs`).
  - (3) Held, but its wording is stale (m11).

## Critical

None.

## Major

### M1. A new fast-tier test takes over 10 s: `replaced_names_exactly_what_the_run_took`
- **Where:** `crates/graft-ops/tests/it/undo_replaced.rs:286-350`, one `#[test]` looping 11 scenarios. Each scenario builds a fresh repository and runs a checkout, two undo runs and a redo.
- **What:** it took 15.55 s in the full nextest run, which makes it the slowest test in the suite, and 12.54 s run alone. PROMPT.md 7.1 (:532): "A fast-tier test that takes over 10 seconds … is a Major review finding."
- **Caveat (rule 1):** both runs were at load average 7.5 with a builder running at the same time. On a quiet machine it may come in under 10 s. That is not measured.
- **Why it matters:** the fast tier is the gate every builder runs, and the suite as a whole is already at or over its 3-minute budget under load.
- **Fix:** keep the table, but give each scenario its own `#[test]` that calls a shared `check(scenario)`, so nextest can spread them over workers. Or build `main_and_feature()` once and `cp -r` it per scenario. Re-measure on a quiet machine either way.

### M2. Readability: 77 grew two files that were already over the size limit, with a concern that stands on its own
- **Where:**
  - `crates/graft/src/update/undo.rs` went from 1511 to 1755 lines, and its production part is now 855 lines (`mod tests` at :856). 77 added the consequence-words unit (`with_consequence`, `CONFIRM_COLUMNS`, `CONFIRM_LINES`, `drift_consequence`, `replaced_keys`, `Named`, `named_replaced`, `done_with`, `applied_is_redo`, `way_back_key`, `left_cuts` and `wrapped_at`) at :578-760. That is about 190 lines, none of which touch `AppState`.
  - `crates/graft/src/jobs/undo.rs` production went from about 500 to about 593 lines (`mod tests` at :594).
- **What:** PROMPT 6.2 says files stay under about 400 lines, and this review's brief makes readability findings Major. A sibling directory `update/undo/` already exists (`box_answers.rs`), so the natural home was there.
- **Why it matters:** the words and the fit rule are the part the next ticket will change (m5, m7), and they are buried in the middle of the answer handling.
- **Fix:** move :578-760 into `update/undo/consequence.rs` as a pure move, with the suite count unchanged. Refresh the stale size rows: `KNOWN_GAPS.md` :816 (M6/r69) still says 1511, and :597 (M6/07) says graft-ops `undo.rs` is 603 lines when it is now 731.

## Minor

### m1. The worst case is 8 spawns, not 5
- **Where:** 77 NOTES D5 says two `diff-tree` plus three index comparisons, 5 spawns in all.
- **What:** `snapshot::index_paths_changed` spawns **two** `ls-tree -r` when the two `IndexState`s differ in shape (`crates/graft-ops/src/snapshot/index_paths.rs:58` then `:79-80`, one spawn each, per `stage_tree.rs:108-137`). `in_index` calls it up to three times (`undo/replaced.rs`, `in_index`). So the worst case is 2 + 3×2 = **8**. It happens when an undo inside a live merge (m6-conflicts/57) meets a merged index against an unmerged one.
- **Why it matters:** each of those `ls-tree -r` calls lists the whole index tree. On a 100k-file repository the cost is unmeasured.
- **Fix:** correct D5 and the module doc (`replaced.rs:16-17`). Either measure one drift undo on the 100k fixture, or accept the cost by name.

### m2. The Command Log's "every command it ran" is not true, and the clean-cost test cannot see the index half
- **Where:** `restore.rs:224-226` says of `reads`: "the log is every command it ran". But the index comparisons log nothing (`replaced.rs:33-35`).
- **What:**
  - A forced undo with index drift runs up to 6 git commands that its entry's Command Log does not show.
  - `a_clean_undo_spawns_no_diff_tree` (`undo_replaced.rs:357-405`) filters the log for `diff-tree`, so it cannot see an index read on the clean path. The builder's own mutation note R11 says it cannot see the rail's gate either.
- **Fix:** pass `commands` through `index_paths_changed`, or reword the doc to say "the working-tree reads". Note in MAP.md that the no-cost promise rests on the early returns at `replaced.rs:104` and `:129` as well as on the gate.

### m3. The redo gap row cites the wrong file and is misclassified
- **Where:** `KNOWN_GAPS.md:818` cites `crates/graft-core/src/journal.rs:264`. That line is a struct field (`JournalEntry::refs_before`). `next_undo` is at **`crates/graft-ops/src/journal.rs:264`**.
- **What:** the row is marked `unverified (design)`. It is the same shell-only reach as :812, which is marked **broken**. The redo box says `a backup keeps it.` and then there is no in-app way to it: the q4 dead end, honestly worded.
- **Fix:** correct the path and mark the row **broken**, like :812.

### m4. The D8 limit was proposed and never filed
- **What:** 77's NOTES proposed 5 KNOWN_GAPS rows. Three landed: :812, :813 and :818. "D8: when the drift line takes four lines, graft-ui cuts the consequence line" did not. The graph-HEAD row is moot, because 79 fixed it.
- **Fix:** add the D8 row.

### m5. The time-machine landing does not fit in 80 columns
- **Where:** `update/undo.rs:462` builds `reverted to before “checkout fix” (2 steps)`, and `replaced_keys` adds `U (2 times) brings back your edit to fix.txt`. `done_line` joins them into **92 characters**, 93 with the footer's indent. It is drawn through `graft_ui::fit::fit` with no status-specific fitting (`view/frame/bars.rs:48-55`, :288-291).
- **What:** at 80×24, the test's own scenario loses the end of the path. A longer entry description (for example `commit "…"`) pushes `U (2 times)` itself off the row. 77 gave the box a fit rule and gave the landing none. No 80×24 capture shows this landing.
- **Fix:** apply the box's left-cut to the status path, or drop `your edit to <path>` before the key. Add an 80×24 frame of the time-machine landing to `fix-undo-drift-way-back`.

### m6. Frame 1 of the new capture still shows a reload in flight
- **Where:** `docs/shots/m6/fix-undo-drift-way-back-{80x24,120x40}.txt:19` reads `date order, sorting…`.
- **What:** 79's settle-wait covers frames 2 and 3 only. Frame 1 comes from `drift_box_after_a_checkout` (`undo_over_drift.rs:533`, `:80-90`), which pumps only until the box opens, while the checkout's graph reload is still running.
- **Question:** is frame 1 deterministic? It depends on the undo job's drift read losing a race with the 6-commit reload. It passed in 3 of 3 runs here, which is not proof.
- **Fix:** call `settled_frame` (and `radar_answered`) after the checkout, before `u`. Retake the captures.

### m7. The redo landing puts a sentence in the `keys` slot
- **Where:** `replaced_keys` returns `a backup keeps your edit to …` for a redo (`update/undo.rs:679-697`), and `undid` stores it as `Status::Done.keys` (:300-301).
- **What:** `keys` is documented as "The next key, worded as it reads in the hint bar" (`update/state.rs:902-903`).
- **Fix:** fold it into `what` for a redo, or document the exception.

### m8. Question: does "the whole working tree" fit an index-only restore, and does the whole-tree `U` promise have a test?
- **Where:** `undo/replaced.rs:72-75` sets `whole_tree` for `IndexPaths::Everything` as well, and `named_replaced` (`update/undo.rs:707-709`) always calls it "the whole working tree".
- **What:** no real-repo test drives a whole-tree case through `y` and then `U`. All 11 scenarios expect `whole_tree: false` (`undo_replaced.rs:324-327`). The `m3-undo-drift` capture shows the box only.
- **Question:** is the wording right for an index-only restore, and is the promise tested? The code was read, not run on this case.

### m9. An unreachable match arm in `run_revert_to`
- **Where:** `jobs/undo.rs:463`, `Err(UndoError::Drift(drift)) => RevertedTo::Drift {..}`.
- **What:** `revert_to` returns only `Nothing` before its loop (`undo/execute.rs:97`) and wraps every step's error in `PartWay` (:111-117). A bare `Drift` never arrives. The arm reads as a second live path.
- **Fix:** remove it, or comment that it is kept for a future `revert_to` that stops wrapping.

### m10. Question: `jobs/operate.rs:174-178` uses the same refs-for-HEAD test that 79 removed
- **What:** `ApplyError::Git` picks `WorkingTree` when `touches.refs` is empty, and a checkout's `touches` names no ref.
- **Why it is probably harmless:** a failed `git checkout` or `git switch` does not move `HEAD`, so this is likely fine.
- **Question:** 79's audit covered only graft-ops, and the conductor asked for every choice. Does this site need the same fix, or a comment saying why not?

### m11. 79 tick 3 reads "which now fails"
- **What:** at `18e88d8c` it passes. The follow-up in the NOTES retook the capture.
- **Fix:** reword the tick to name the passing test and the retake.

### m12. Test files over the size limit, and a stale size row
- **Sizes:**
  - `tests/it/undo_over_drift.rs` 589 (new in 77)
  - `graft-ops/tests/it/undo_replaced.rs` 405 (new)
  - `view/tests.rs` 4724 (+99)
  - `graft-ops/tests/it/history_ops.rs` 2624 (+55)
  - `graft-ops/src/restore.rs` 472 (+~90 across both tickets)
- **What:** these are over PROMPT 6.2's ~400 lines, with no rows. The graft-ops `undo.rs` row (M6/07) is stale.
- **Fix:** file one row, or accept them by name.

### m13. Question: after the first of `U (2 times)`, nothing says a second press is owed
- **Where:** the first redo of a two-step jump runs clean, so `replaced` is empty and `replaced_keys` returns `None` (`update/undo.rs:300`, :684-686). The status becomes `redo: … ✓`, with no hint.
- **What:** the count was named once, on the landing, and it is gone after one key (UX: visibility of the next step).
- **Question:** should the first redo say `U brings back your edit to fix.txt`? That needs `replaced` carried on the Undo entry, not on the run. It is a design call for the conductor.

## Touches, checked

- **77's list:**
  - Every `Applied`/`Drift` construction site compiles under both clippy lines.
  - Every `Undone::Nothing` and `RevertedTo` match site was grepped.
  - The captures were regenerated, and each passes its own test.
  - The contact sheets are all ≤ 200 KB: m3 130 KB, and m6 parts 180/177/163/86 KB.
  - The TESTING.md rows are at :1344 and in walk 13.
  - `graft-core/src/journal.rs:25-29` was reworded.
  - graft-ui is untouched (`git diff --stat` is empty).
- **79's list:**
  - `refresh_scope`'s callers were read and not edited.
  - The capture was retaken and the captions row updated.
  - The graft-side `Refresh` choices are not listed (m10).

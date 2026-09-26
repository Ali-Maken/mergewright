# Review — m6-conflicts/86: the stash list previews before it writes

Reviewed read-only at `f68f0857` (diff `83735e1d..f68f0857`, 43 files, +1806/−155). Every cargo command ran in a
throwaway clone at that hash with its own target dir. The git probes below ran in scratch repos. Nothing ran against
the main repo's stash list.

**Counts: Critical 0 · Major 2 · Minor 7.**

## Gates (run by me, one at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test` (workspace) | 3523 passed, 0 failed; 180 s wall; peak RSS 350 MB |
| `cargo nextest run` (workspace) | 3520 passed, 1 skipped; 163 s (load average ≈5 from other agents, so not comparable with the 50 s baseline) |
| heavy: `cargo test --features heavy-tests -- --test-threads=2` | 3557 passed, 0 failed, 1 ignored; 424 s; peak RSS 434 MB |
| release perf: `frames:: heavy::budgets_hold` | 13/13 pass. Input-to-frame p99 is 4.8–5.6 ms against a 16 ms budget (max 7.2 ms, pixel tier). First paint is 15–70 ms; the realistic commit-graph fixture is the slowest at 69.7 ms. Peak RSS ≤ 103 MB |
| fuzz / bench | none touched by this ticket; not run |

Slowest five (nextest): `m5_walk::every_scenario::merge_resolved_with_the_keys` 8.3 s,
`conflict_ops::whole_file_shapes_resolve_like_git` 7.8 s, `diff_fixture::parse_unified_matches_numstat_for_every_shape`
7.8 s, `undo_cancel::cancel_at_every_step_boundary_is_clean` 7.3 s,
`layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.6 s. None of them is 86's.
`target/`: 7.0 GB in the clone, 9.1 GB in the main repo.

## Ticks — spot-checked 25 of 27 named tests; 25 held (1 weakly), 0 false

- **graft-ops, 8 of 8.** `a_stash_that_applies_cleanly_forecasts_clean`, `a_stash_that_clashes_forecasts_its_paths`,
  `unsaved_edits_in_the_way_refuse_the_preview`, `an_untracked_file_in_the_way_refuses_the_preview` (the ignored-file
  case included), `an_untracked_only_stash_lists_its_files`, `the_stash_preview_writes_no_ref_index_or_file` (index
  bytes after a `touch` that would force a refresh, `for-each-ref`, the stash list, `status --no-optional-locks`, the
  top-level names of the git dir) and `an_old_git_leaves_the_stash_forecast_not_checked` each assert what the tick
  claims. `the_stash_preview_honours_stop` holds, but weakly (m6).
- **Oracle, 1 of 1.** It holds for the six rows it has. The tracked-in-the-way row checks that the tree is unchanged.
  One reachable disagreement has no row (M1).
- **App, 6 of 6.** `enter_opens_the_apply_preview_and_writes_nothing` asserts that `PlanPreview` is the only command.
  `y_in_the_preview_applies` asserts `Stoppable::Yes`, which proves the route is `launch`.
  `esc_from_the_preview_returns_to_the_same_stash` and `enter_twice_writes_nothing` also hold. So do
  `p_opens_the_pop_preview` (`p p` is inert) and `y_in_the_preview_pops`.
- **Walks, 4 of 4.** `a_stash_{pop,apply}_that_clashes_is_five_keys` count `S p y o ⏎` and `S ⏎ y o ⏎` against
  `noted_keys`. `stash_look_and_leave_writes_nothing` counts `S ⏎ esc` = 3 and compares the stash list, the index and
  the status before and after. 85's walk is `S p y esc ⏎ o esc`: the builder disclosed this and it is sound (the
  pop.rs doc explains why).
- **Captures, 3 of 3.** Every "right" item in the ticket is in its capture, and the three are under 2.2 KB each.
- **Sheet and MAPs.** The five sheet parts are ≤ 179 772 B. The MAP rows are present.
- **Not checked:** the six key-sequence rows in TESTING.md; I read the diff only. The hint-bar tests are green in the
  run, but I did not read what they cover.

## Brief checks

- **Forecast.** `ours` comes from `write-tree` over a `TempIndex` copy (`preview/stash.rs:138`,
  `snapshot/plumbing.rs:18-45`, removed on drop). Only loose objects are written; no ref, no index, no file. The writes
  test proves it. `NotChecked` is reached on git < 2.40 (`stash.rs:116-119`), on an unmerged index (`:141`) and on any
  read failure (`:144,:150,:156`). It draws as `conflicts: not checked (…)` through the shared `conflict_line`
  (`update/preview.rs:911`), never as clean. **Stop:** `stopped()` runs before every spawn (`stash.rs:80,89,94,135,
  146,152`), and snapshot/run stops map to `Stopped` (`plumbing.rs:301`, `preview.rs:233`). **Verified by probe:** I
  widened the stop test's loop in the clone. Polls 1–21 all return `Stopped`, and the plan completes at poll 22
  (`Likely{notes.md}`, deterministic over 3 runs).
- **Landing.** `y` → `confirm_preview` → `operate::launch` for stash kinds (`update/preview.rs:743-754`). The
  landing and done captures differ from before only in their headers (`git diff` shows 1–2 header lines each; frames
  identical), so the outcome lands where `⏎`/`p` landed.
- **Refusals.** Tracked unsaved: `in_the_way` (`stash.rs:49-63`) is mapped from `check`'s `TouchesChangedFiles`
  (`graft-ops/src/preview.rs:353-367`). The oracle shows git refuses with the tree unchanged. Untracked in status: same
  path (`now.untracked`). Ignored or untracked-on-disk: `occupied` (`stash.rs:99-104`). I also checked a case the
  oracle lacks, an untracked file where the stash adds a *tracked* file: git refuses ("untracked working tree files
  would be overwritten by merge"), and the tree was untouched. That matches `StashInTheWay`.
- **The `--index` refusal is reachable** — M1.
- **Touches.** Every test that presses `S p`/`S ⏎` was updated: `m3_walk` (`stash_pop`, `stash_apply`, the refused
  scenario), `stash_by_key`, `stash_clash` (via `put_back`), `m5_walk/{stash_journeys,resolved_operations}`, and
  `m6_stash_put_back{,/pop}`. I grepped every `'S'` press under `crates/graft/tests`. The remaining ones (`name_and_drag`,
  `refs_and_names`, `pull_over_edits`, `m3_walk:1327,1666`, `resolver/modal`) press `b`, `s` or `esc` after `S`, never
  `⏎`/`p`. The fence (`m6-conflicts.md:515-517`) reads 5 / 5 / 3, matching the walks. `stash-pop-clean` is not
  fenced (m7).

## Major

**M1. The forecast can promise `no conflicts expected` for a put-back that git refuses ("conflicts in index").**
`preview/stash.rs:128-160` forecasts only the three-way merge. Graft runs `stash apply|pop --index`. When the stash
has a staged part, git first applies that part's diff (`stash^2^..stash^2`) to the index with `apply --cached`. If
that fails, git exits 1 with `conflicts in index. Try without --index.` and writes nothing.
*Reproduced (scratch repo, git 2.54):* stage a change to line 5 of `f`, `stash push`, then commit a change to line 3.
`merge-tree --write-tree --merge-base=stash^1 <index-commit> stash` exits **0**, which is what the forecast runs. Then
`git stash apply --index` exits **1** with the words above. It is reachable from the UI: Graft's own `s` keeps the
index in the stash, and `check_apply` (`operation/stash.rs:104-121`) passes, because the committed change leaves no
dirty path. The user sees `no conflicts expected`, presses `y`, and lands on failure words.
`a_failed_apply_with_nothing_unmerged_is_not_a_clash` (`graft-ops/tests/it/stash_clash.rs:131`) shows the same
refusal exists. For its shape (a whole-file change), `merge-tree` reports `CONFLICT (content)` in `a.txt` (reproduced), so the preview says `may conflict: a.txt` and promises
the resolver.
*Why it matters:* the promise of the preview is "you see the result before you confirm". The ticket says "a row where
they disagree is a failure, not a NOTE", but this disagreement ships as a NOTE (ticket NOTES; `graft-ops/MAP.md:219`)
with no oracle row. It has no `KNOWN_GAPS.md` row either (rule 4). Nothing is lost: git writes nothing and the stash
stays.
*Fix:* use git's own rule. When `stash^2`'s tree differs from both `stash^1`'s and the index-now tree, run
`diff-tree --binary stash^2^ stash^2 | apply --cached --check` with `GIT_INDEX_FILE` = the same `TempIndex` copy. On
failure, return a refusal (`the staged part of stash@{n} no longer applies`) or `NotChecked`, never `Clean`/`Likely`.
Add the oracle row (staged line 5 / committed line 3). Until then, file a KNOWN_GAPS row marked **broken**.

**M2. The stash list's bar stays on screen under the preview, and it advertises keys the box does not answer.**
The capture `stash-apply-preview-120x40.txt:52` shows, beneath the box whose cursor sits on `[ Cancel ]`:
`⏎ apply  p pop  s save  d drop  space files  esc close`. The same bar is at `stash-pop-preview-80x24.txt:35` and
`stash-preview-in-the-way-120x40.txt:52` (textual; the captures show it). While the box is up, `preview_key` holds
every key (`update/preview.rs:422-435`): `⏎` acts on Cancel, `p`/`s`/`d`/`space` fall to `_ => {}` (:433), and `esc`
goes back to the list instead of closing it. The bar says `⏎ apply` at the moment `⏎` means Cancel, which is the very
misreading of `⏎` this ticket exists to fix (walk 2 C). Every other preview ends in its own footer, with no bar under
it (`m5-02-preview-merge-80x24.txt`, last row). The bar comes from `graft-ui/src/stash_screen.rs:175`.
`every_key_the_bar_draws_is_answered` did not catch it because it sweeps only the graph's contexts
(`update/tests.rs:3686`).
*Fix:* while `state.preview` is `Some`, have the stash screen draw no bar, or let the box take the bar row as it does on
the graph. Re-take the three captures.

## Minor

**m1. An index-only stash draws an empty `working tree:` line.** `preview/stash.rs:89-113` lists `stash^1→stash` plus
`stash^3`. When the working-tree file equals HEAD but the index differs, that diff is empty. *Reproduced:*
`add`, restore the worktree, then `stash push`: `diff-tree stash^1 stash` prints nothing, while `stash^2` holds the
change. `stash_box` (`update/preview.rs:903-908`) then prints `working tree: ` followed by nothing.
`worktree_line`'s own comment (`:1094-1095`) says an empty `Updated` "reads as nothing happens to your files". *Fix:*
add `stash^2`'s paths, or say `index only: <paths>`.

**m2. Past 50 files the count is wrong.** `changed_paths` caps at `PATHS_CAP` = 50 (`graft-ops/src/preview.rs:50`).
`stash_box` then prints `list(paths, 3)` plus `, more` (`update/preview.rs:903-908`), for example
`a, b, c (+47), more`, when there are more than 47 others. *Fix:* when `more` is set, drop the `(+n)`, or print
`a, b, c and more`.

**m3. The refused box offers keys it doesn't need.** `stash-preview-in-the-way-120x40.txt:51` shows
`←→ choose  ⏎ act  esc cancel` over a single `[ Close ]`. The design note says the keys are `esc back`. Textual.
*Fix:* when the box is refused, drop `←→ choose` and say `esc back`.

**m4. `StashInTheWay` calls every blocker "your unsaved edits".** `operation.rs:634-636` uses the same words for a file
found by `occupied` (`stash.rs:99-104`), including an **ignored** file such as a build artefact, which is not his edit.
The words were decided in the ticket, so this is filed as a question for the conductor. *Suggestion:* use
`<path> is in the way` for the on-disk case.

**m5. A mixed refusal names only half the blockers.** `graft-ops/src/preview.rs:353-367`: when some stashed paths have
unsaved edits and others have staged-only edits, `StashInTheWay` names only the unsaved ones. The staged ones are
dropped from the words. After fixing the first set, the user meets a second refusal (`TouchesChangedFiles`).
*Fix:* name both sets, or append `and <n> staged`.

**m6. `the_stash_preview_honours_stop` stops short of the end.** `stash_preview.rs:277-292` loops `1..20`, but the
plan needs 22 polls (my probe), so `Ok(_) => break` (:287) never runs. Polls 20–21, the last spawn, are never
exercised, and the arm that would catch a stop turned into `NotChecked` is dead. The code is right (the probe returns
`Stopped` at every poll 1–21), so the defect is in the test. *Fix:* loop until `Ok`, assert every earlier answer is
`Stopped`, and assert the final `Ok` is not `NotChecked`.

**m7. The note's `stash-pop-clean` journey (3, `S p y`, `m6-stash-preview.md:32`) is neither fenced nor counted.**
`m3_walk::stash_pop` walks it but checks it against no `noted_keys`. The ticket's fence edit left it out, so the gap
belongs to the conductor. The pipeline's step 7 wants every journey counted. *Fix:* add
`stash-pop-clean 3` to the fence and count it in `m3_walk::stash_pop` or `stash_by_key`.

## Not findings (checked)

- No `unwrap`/`expect` outside tests.
- Box text goes through `sanitize_line`: `stash_box` and `closing_box` (`update/preview.rs:849-853`, :897-924).
- `let _opened: bool` (`stash.rs:468`) is safe as its comment says: `open_preview`'s `false` paths either already show a
  preview or set `Status::Sticky` (`update/preview.rs:319-324`).
- The plan runs in the preview job, so no git runs on the UI thread.
- Other preview kinds: the check mapping matches only `StashApply`/`StashPop`, so no new spawn for them.

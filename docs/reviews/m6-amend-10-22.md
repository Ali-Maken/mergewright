# Review: m6-conflicts tickets 10 and 22 (group)

Range `136edb2..0b5019c` (merges of `e019835` = 22 and `48e4ef6` = 10), read-only. Reviewer: Opus, 2026-09-23.

Tickets: `.scratch/m6-conflicts/issues/10-resolver-line-numbers.md`, `22-ops-finish-message-and-step-bound.md`.

**Count: 1 Critical · 3 Major · 8 Minor.**

## Gates (run one at a time, on `main` at `0b5019c`)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo nextest run` (workspace) | 2716 passed, 1 skipped. **97.2 s** wall (`/usr/bin/time`). Two re-runs for per-test times took 118 s and 128 s (other agents running on the machine). |
| `cargo test --doc` | 3 passed, 0 failed |
| Heavy-feature clippy, heavy tier, release perf run, fuzz | **not run**: the conductor limited group reviews to the four standard gates. They run at the milestone review. |

- 5 slowest tests: `graft-git diff_fixture::parse_unified_matches_numstat_for_every_shape` 11.7 s; `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 11.7 s; `graft-ops conflict_ops::whole_file_shapes_resolve_like_git` 10.4 s; `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 8.3 s; `graft-fixtures corpus::two_builds_produce_the_same_object_ids` 7.2 s.
- Peak RSS 272 MB. That is `time -l`'s max-RSS over the nextest process tree, not per test.
- `target/` is 16 GB.

## Tick spot-check

- **Ticket 10:** checked 10 of 10 ticks; 9 held, **1 false** (C1).
- **Ticket 22:** checked 8 of 8 ticks; 8 held.
- One claim in ticket 22 was not reproduced: that `a_rebase_lists_only_the_current_steps_files` failed on the old code. I did not check out and run the old code. It is consistent with the M1 probe below, which found step 1's link on step 2's chain holding `a.txt` at stage ≥ 1. That link is exactly what the old, unbounded walk would have listed.

## Pipeline

- Amendment 2 of `docs/design-notes/m6-conflicts.md` has Prior art rows for both tickets: "Names per side" at :427 and "Line numbers" at :428, citing `docs/research/conflict-layout-audit.md`.
- It also has a Journeys section at :434.
- Neither ticket adds or changes a journey step: 10 is display only, and 22 is ops only. There was no step count to check.

---

## Critical

### C1. A false tick: `one_name_per_side` never draws stash pop at 80 columns

- **Where:** `crates/graft/src/view/resolver.rs:454-461`. Ticket 10, Acceptance line 6.
- **What's wrong:** The tick says "verified by that test, drawn through `view::view` at 80 and 120 … merge, rebase, cherry-pick, stash pop".
  - The loop over `[80, 120]` (`:446`) covers only merge, rebase and cherry-pick.
  - The stash-pop case is drawn once, at 120 columns (`:457`).
- **Did it hold anyway?** Yes. I checked in a throwaway worktree: the stash case drawn at 80 columns reads `yours · main` / `stashed`, with no raw label and no `stashed ·`. The test still does not check what the tick says it does.
- **Why it matters:** A false tick is Critical whatever its size (CLAUDE.md, the gate). The stacked label path, below 100 columns, is where stash pop's lone role word is drawn differently from the paired one. That path is the one left untested.
- **Fix:** Run the stash block under `for columns in [80, 120]` and assert `stashed` on its own label row. That is a one-loop change.

## Major

### M1. Ruling on the builder's question: yes, `d` at step 2 restores a finished step-1 conflict

**The answer.** `apply::stage_source` walks the whole chain, so `d` at step 2 of a rebase, on a file step 1 conflicted on, brings step 1's conflict back into step 2. I verified this with a test, not only by reading the lines.

**The code path:**

1. `check_restart` admits a stage-0 path without looking further: `crates/graft-ops/src/operation/conflict.rs:238-247`.
2. `stage_source` falls through to `chain_states` for a live operation: `crates/graft-ops/src/apply/restart.rs:78-81`.
3. `chain_states` returns **every** link, whatever its kind: `restart.rs:108-112`.
4. The first `Unmerged` state that holds the path at stage ≥ 1 wins (`restart.rs:82-89`). Step 1's `Step` link is such a state: it was written before `a.txt` was staged.
5. `prepare_restart` then sees `UU`, which has markers, and returns `Restore`.

**The probe.** A throwaway test in a scratch worktree, since deleted, run on `0b5019c`:

- Setup: `conflict_set.rs::at_step_two()`, then `try_run(RestartConflict { path: "a.txt" })`.
- It returned **`Ok`**.
- Before: `a.txt` held `1\nboth\n3\n`, step 1's committed resolution.
- After: `a.txt` held `1\n<<<<<<< ours\nM\n=======\nF\n>>>>>>> theirs\n3\n`.
- `git status` then showed `UU a.txt` and `UU b.txt`.

**Why it matters:**

- Step 2's index now carries step 1's sides (`M` against `F`), not step 2's.
- Taking "ours" writes `M` into step 2's commit. That silently undoes the user's step-1 resolution (`both`) in the rebased history.
- Nothing is lost for good: the chain link is written before the restore, and `HEAD` still has `both`. But the result is wrong content, reached through a public ops entry point.

**Reachable from the app?** Not through the paths I read:

- Both ways into the resolver (the changes pane `⏎`, and the banner) open only paths the status calls unmerged: `crates/graft/src/update/banner.rs:548-556`.
- At step 2, `a.txt` is stage 0 and clean.
- Not checked: whether a resolver left open across a resume can still be on `a.txt`.

**Fix:** Give `chain_states` the same bound `chained_conflicts` now has: stop at the first `LinkKind::Resume`. Pin it with the probe above as a real test, expecting `Refusal::NoStagesKept`. It is its own ticket (the ticket 22 scope said so). It belongs before ticket 13 lists files from the set.

### M2. Two status lines still name the sides `ours`/`theirs`

- **Where:**
  - `crates/graft/src/update/resolver/restart.rs:39`: `d` on a whole-file card, `keys: Some("o ours  t theirs")`.
  - `crates/graft/src/update/resolver/walk.rs:63`: `n` past the last card, "no more conflicts", `keys: "o ours  t theirs"`.
- **What's wrong:** Ticket 10 scope 3 says "**Every** place the resolver names a side now uses `SideNames`", notices and hint verbs included. `whole_file_both_refused` was converted (`whole_file.rs:109-112`); these two were missed.
- **Why it matters:** In a rebase with a whole-file card, the bar reads `o upstream  t yours` while the status line directly above it reads `o ours  t theirs`. That is the very swap this ticket exists to stop showing: in a rebase, git's `ours` is upstream.
- **Fix:**
  - Build both from `side_names(state).roles`, as `whole_file_both_refused` does.
  - Add these two cases to a rebase variant of `took_line_uses_the_side_name`, or add a grep-style test that no resolver status carries a bare `ours`/`theirs`.

### M3. Three files already over the ~400-line rule grew, with no gap row

- **Where:**
  - `crates/graft-ui/src/conflict_resolver.rs`: 929 → 1003 lines.
  - `crates/graft-git/src/conflict.rs`: 1195 → 1245.
  - `crates/graft-ui/src/graph_screen/hints.rs`: now 1256.
- **What's wrong:** None of the three is named in `KNOWN_GAPS.md`. The size rows there cover other files (M5/18, M6/06, M6/07).
- **Why it matters:** PROMPT.md 6.2 is a hard rule. Growing an untracked over-limit file hides the debt.
- **Fix:**
  - Add a gap row naming all three.
  - Split with `codebase-design`. A natural seam: `SideRoles`/`SideNames`/`BlockNumbers`/`number_width` move to `conflict_resolver/names.rs`, and the per-row drawers to `conflict_resolver/draw.rs`.

## Minor

1. **A refused resume still bounds the walk.**
   - Where: `crates/graft-ops/src/apply/resume.rs:176` writes the `Resume` link before git runs. If a hook refuses (merge `commit`, rebase `--continue`), the operation stays at the same step, but `chained_conflicts` (`conflict/set.rs:157-160`) now ends at that link.
   - The comment at `set.rs:155-156` ("A merge has … no `Resume` link before its finish") is untrue after a refused `ContinueWith`.
   - Impact is small: git's `Conflicts:` list still names the files, as the builder noted.
   - Fix: record the step with the trailer (e.g. `Graft-Link: resume 2/5`) and bound only where the step changed, or correct the comment and add a gap row.
2. **Reading theirs' name can block `--abort`.**
   - `incoming` (`crates/graft-git/src/state.rs:197-225`) turns an unreadable `*_HEAD` commit into a failure of all of `in_progress_detail`.
   - That fails the whole working-tree read (`crates/graft/src/working_tree.rs:61`) and `live_entry` (`crates/graft-ops/src/apply/resume.rs:90`). So a label lookup can stop `--abort` from running.
   - It follows the ticket's Decision 4, which forbade `None`, but that blast radius was not weighed.
   - The `rev_list` call also passes `&()` (`state.rs:217`), so it cannot be stopped.
   - Fix: read `theirs` in its own call that only the view path makes, or keep it as `Result` inside the detail so one failure does not sink the rest.
3. **The number width is recomputed for every row.** `numbered` calls `number_width(self.doc)` (`crates/graft-ui/src/conflict_resolver.rs:547-548`), which walks every region, so each frame costs rows × regions. Fix: compute it once per `ResolverScreen` or per `DocView`.
4. **`checkout -m` is labelled `stashed`.** See `crates/graft/src/update/resolver/names.rs:65`. This is the builder's stated deviation: the reused predicate cannot tell `checkout -m` from a stash pop, so the table's "none" row is unreachable while the resolver is open. Fix: a gap row, or tell the two apart (e.g. by whether `refs/stash`'s reflog moved) in a later ticket.
5. **Stale doc comments.** `crates/graft-ui/src/graph_screen/hints.rs:958` and `:990` still document the lists as `o ours  t theirs`. Fix: reword to "`o`/`t` read as `SideRoles`".
6. **A doc comment on the wrong item.** `crates/graft-ui/src/conflict_resolver/tests.rs:514-517` repeats `:585-587`'s test description on top of `starts_with_number`. Fix: delete the first copy.
7. **Two lines of the capture checklist don't match the captures (textual).**
   - "No header row carries a digit" is false as written: the header reads `── conflict 1 of 2 · open ──`.
   - "labels read `yours · main │ theirs · feature`" holds only in the 120×40 capture. At 80×24 the labels stack, on `fix-card-names-numbers-80x24.txt` lines 6 and 8.
   - The other lines match: `2 two from main │2 TWO from feature` (120), `3 three` (both), and the bar `o yours  t theirs  b both` (both).
   - Fix: reword to "no header row carries a line number", and give each width its own label line.
8. **`no_marker_row_draws_a_number` is weak.** It checks only each row's leading characters (`conflict_resolver/tests.rs:504-507`). A number drawn in the right half of a paired `Labels` row would pass. Fix: also check the text after the divider on `Labels` rows.

## Builders' stated deviations: rulings

**Ticket 10:**

- **The capture builds its own `notes.txt` merge.** Accepted: `StoppedMerge` has no such file, and NOTES say so, not substituting silently. The ticket's own acceptance for the numbering test still ran on both named fixtures.
- **Stash pop versus "none".** Accepted as the only reading consistent with "reuse the predicate". The ticket contradicted itself. Tracked in Minor 4.
- **Whole-file keys use full names** (`t keeps theirs · feature`), where Decision 1 said role words. Accepted: the acceptance line requires `theirs · feature` on a `DU` card, and a card has no label row. Planner note: in a cherry-pick this becomes `t keeps picked · 3f2a1b7 <subject>`, which is cut at 80 columns.
- **`Product::WorkingTree` boxed.** Accepted: it fixes the cause (`large_enum_variant`) and changes no behaviour.
- **`StateError::Io` carries the object-read failure.** Accepted given the crate set. The consequence is in Minor 2.
- **Role-word rows in `bound_actions`.** Accepted: needed so the bar sweeps can see them.

**Ticket 22:**

- **An unknown `Graft-Link:` value is `SnapshotError::UnknownLink`, not `Step`.** Accepted, and right under rule 3: guessing would bound the set wrongly. The one cost is that a set read after a downgrade fails loudly.
- **One `cat-file commit` per link replaces `rev-list --parents`.**
  - The parent comes from the first `parent ` header, which is the first parent. That is correct.
  - Splitting at the first `\n\n` is safe: a signature header never contains an empty line.
  - The spawn count is unchanged.
- **`TwoStopRebase` goes in `STEPPED`, not `STOPPED`.** Accepted: `graft-git`'s `STOPPED` consumers expect the marker files, and `graft-git` was outside the crate set.
- **Tests fetch the fixture's history so the rebase is Graft's own.** Accepted: the corpus repo was begun in the shell and has no chain.
- **The `stage_source` question.** Ruled in M1: yes, with a probe.

## Git-data safety of the `Graft-Link:` change (ticket 22)

- **Writes.** Only the backup commit's message changes: `chain.rs:73` appends the trailer. No ref is removed or rewritten, and the ref still moves last, after `commit-tree` (`chain_backup`).
- **Old chains.** A link with no trailer, or the pre-operation snapshot, reads as `Step` (`link_of`, `chain.rs:203-218`). A rebase begun by a pre-22 build therefore keeps 09's behaviour: the set over-lists earlier steps' files. That is an over-list, not a loss. Pinned by `a_link_without_a_kind_reads_as_step`.
- **Restart and fold.**
  - Fold writes `Step` (`fold.rs:108`); resume writes `Resume` (`resume.rs:176`).
  - Restart still reads every link and drops the kind (`restart.rs:108-112`). Its behaviour is unchanged by this ticket, and that unchanged behaviour is M1.
- **Commit message.** `--cleanup=whitespace` keeps the user's `#` lines. git's own comment lines are removed once, when `finish_summary` reads the message, and `gits_comment_lines_never_reach_the_summary` pins that for unset and `;`. The unedited path (`Resume::Continue`) lands byte for byte as shown.
- **Verdict:** this ticket adds no new data-loss path. M1 is a wrong-content path that existed before it, and the new bound makes it visible.

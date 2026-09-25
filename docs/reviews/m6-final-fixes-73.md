# Review — m6-conflicts 73: set-aside around rebase and switch

Range `8e5d825e..b98fc040` (73 merged at `b98fc040`). Reviewer: read-only, detached worktree at `b98fc040`, own
`CARGO_TARGET_DIR`. Ticket `.scratch/m6-conflicts/issues/73-set-aside-around-rebase-and-switch.md`; ADR-0045 §1, §4–§7;
`docs/design-notes/m6-stash-put-back.md`.

**Counts: Critical 1 · Major 2 · Minor 9.**

## What I verified, and how (rule 1)

- **The inverse is proven** on real repos, by the builder's tests and by my own scratch probes (added to my worktree
  only, run, then deleted with it; never committed):
  - rebase whose put-back **clashes**, with an **older user stash** in the list: apply → `Clashed`; `u` → refs,
    `HEAD`, index, every file and the stash list (ids and messages) equal to before; redo → state and stash ids equal
    to after; `u` again → equal to before. Passed.
  - put-back **refused** (untracked file in the way, entry `Aborted`, `S` kept): `u` → index, `HEAD`, stash list and
    both edits back. Passed.
  - stop before each step 1–6, rebase and checkout, then `u`: equal to before in all 12 cases (1–3: nothing to undo,
    tree untouched; 4–6: undo restores).
  - builder's `set_aside_round_trips` (8 trees × kinds), `undo_after_a_clashed_put_back_…`, `redo_after_that_undo_…`:
    green.
- **Where each stop lands** (my probe printed the entry, the stash list and `git status` per stop):
  1–2 `Stopped`, nothing journaled, tree as it was · 3 entry `Aborted`, `S` not stored, tree as it was · 4 `S` listed,
  tree as it was · 5 `S` listed, tree **clean**, the operation **never ran** (the entry's commands end at
  `git reset --hard`) · 6 the operation ran, the put-back ran, `S` dropped. Never "neither". So the promise holds.
- **Recover/resume act on `put_back`**: `a_rebase_that_stops_…` resumes from a freshly opened journal
  (`set_aside.rs:202-210`), so `put_back` comes from the Pending line on disk; continue, abort (original tip), final
  skip all put back. `relaunch_after_a_kill_…` shows the closed entry keeps `put_back`. Read `recover.rs`/`file.rs`
  via the scout (entry cloned); not re-read.
- **82 interaction (on main, outside the range)**: 82 rewrote this range's one `live_clash` call to read the index now
  (`git diff b98fc040..main -- crates/graft-ops/tests/it/set_aside.rs`); the set-aside branch in `closed` is untouched
  by 82. I checked by reading whether a set-aside clash *after a stopped rebase* would break 82's tie: steps inside a
  stopped span fold into the entry and write no line (`apply/fold.rs:5-9`), so the set-aside entry stays the newest
  index-recording entry, and `expect` (main `clash.rs:71-115`) is its own `index_after`. **No conflict found.** Not
  run on main.
- **git behaviour of the put-back** replayed by hand (`stash create` → `store` → `reset --hard` → `checkout` →
  `stash apply <S>`): `MM base.txt` → ` M`; `A  n.txt` → `A `; `D  a.txt` → ` D`; `R  a→b` → `A  b.txt` + ` D a.txt`.
- **Ticks**: 4 acceptance ticks checked, **3 held, 1 false** (C1). NOTES file:line refs: 9 checked
  (`apply.rs:222`, `:231`, `:290-296`; `set_aside.rs:56`, `:94`, `:108`, `:122`, `:370`, `:415`; `resume.rs:370-376`;
  `undo.rs:343`, `:362`, `:440`), all held.
- **Rule 3** on every new failure path (`create`, `put_aside`, `abandon`, `close`, `put_back`, `drop_by_id`,
  `listed_at`, `stash_step`): no error is swallowed. Three paths lose the *words* that say where the edits are (m1–m3).
  The one `unwrap_or_default` (`set_aside.rs:87`) is on a string already checked non-empty; nothing fails there.
- One reviewer slip, for the record: I ran `git stash push` in my worktree to set my probe aside before the perf run,
  then popped it. `refs/stash` is shared across worktrees, so for about 3 minutes the main checkout's stash list
  held one extra entry. It is gone (`git stash list` in the main checkout: empty).

## Gates (worktree at `b98fc040`)

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo test` | 3401 passed, 0 failed, 1 ignored; **170.7 s** wall, peak RSS 341 MB |
| `cargo nextest run` | 3398/3398 (1 skipped), 175.4 s |
| `cargo test --doc` | 3 passed |
| heavy: `cargo test --features heavy-tests -- --test-threads=2` | 3434 passed, 0 failed; 439 s wall incl. build; peak RSS 1.0 GB |
| release perf (`frames:: heavy::budgets_hold`) | 13/13 passed. Frame p99: pixel 50k refs 6.7 ms, unicode 5.4 ms, diff 4.9 ms, detail 5.2 ms, map 4.8 ms, 100k pixel 5.9 ms, 100k unicode 5.6 ms. First paint 15–103 ms (worst: realistic, no commit-graph). Peak RSS in budgets 31–113 MB |
| `target/` | 7.1 GB (review worktree); main checkout 8.0 GB |

Five slowest (nextest): `set_aside::set_aside_round_trips` 10.6 s (new, this ticket) · `undo_cancel::cancel_at_every_step_boundary_is_clean`
10.1 s · `restart_whole_file::gate_and_apply_agree_on_every_shape` 8.9 s · `m5_walk::every_scenario::merge_resolved_with_the_keys`
8.1 s · `conflict_ops::whole_file_shapes_resolve_like_git` 7.9 s. No fuzz or bench targets touched by this range; none run.

## Critical

### C1. False tick: "`--index`-free put-back verified by `set_aside_round_trips` (staged/unstaged split, staged new file, deletion, rename, all back byte for byte and in the index)"
- **Where:** ticket acceptance tick 3 (`73-…md:144-145`) and NOTES "Verified, and how" → "Round trip: `--index`-free
  put-back restores the staged/unstaged split, a staged new file, a deletion and a rename (`set_aside_round_trips`,
  index read back)" (`:227-228`).
- **What's wrong:** the named test never looks at the state the put-back leaves. After apply it asserts only that
  *something* changed and the stash list is unchanged (`crates/graft-ops/tests/it/set_aside.rs:232-234`). The
  byte-for-byte and index equality it does assert is after **undo** (`:238`), which comes from the backup, not the
  put-back. And the claim is false as behaviour: a put-back without `--index` loses the split (my replay above:
  `MM` → ` M`, a staged deletion comes back unstaged, a staged rename comes back as `A b` + ` D a`). ADR-0045
  Consequences says so ("A put-back loses the staged/unstaged split on the clean path too"), as does the builder's own
  doc on `SetAside` (`operation.rs:274-276`). The code is right; the tick is not.
- **Why it matters:** CLAUDE.md: a false tick is Critical, whatever its size. And 74 builds the preview's
  `… come back unstaged` words on what this ticket says it verified.
- **Fix:** correct the tick and NOTES. In `set_aside_round_trips`, after apply, assert per tree: every edited file's
  bytes are back (`base.txt`, `n.txt`, `a.txt` gone, `b.txt` present), and the index shape git actually gives
  (`git status --porcelain` per row above). That is what proves "`--index`-free is enough": the edits come back, and
  exactly this much staging is lost.

## Major

### M1. `SetAside::staged` counts files that come back staged; its doc says they come back unstaged
- **Where:** `crates/graft-ops/src/operation.rs:274-276` (doc: "how many of those are staged — they come back
  unstaged"), the count in `Operation::sets_aside` (`operation.rs:843`),
  and `sets_aside_table` (`operation/history/tests.rs`, `b.rs` `staged: Some(Change::Added)` counted, `staged: 2` at `:660`).
- **What's wrong:** a staged new file comes back **staged** (`A  n.txt` → `A  n.txt`, replayed); a staged rename
  comes back half staged (`A  b.txt` + ` D a.txt`). `is_staged()` counts both as "staged, comes back unstaged".
- **Why it matters:** 74's row is `your edits: N files set aside, put back after · M staged come back unstaged`
  (`74-…md:33`, `:124-125`). With a staged new file it will tell the user something is unstaged that is not — a
  wrong promise on the one screen that stands in for a confirm.
- **Fix:** define `staged` as "staged entries whose staging the put-back loses": staged modifications and deletions;
  not `Added`; for a rename, count its old-path deletion. Put the table's `b.rs` in the "stays staged" column, and add
  a real-repo row that compares `sets_aside(facts).staged` with what `git status` shows after the put-back, per tree
  in C1's table. Tell 74 before it words the row.

### M2. The stop-injection test cannot tell which step it stopped at
- **Where:** `crates/graft-ops/tests/it/set_aside.rs:626-665` (`a_stop_before_each_step_loses_nothing`); step 6 is
  accepted without firing (`:641-644`).
- **What's wrong:** each case asserts only "edits somewhere" and "no Pending entry unless git is live". The builder
  showed the stops land on their steps with a probe that was then removed (NOTES `:220-226`). Nothing in the suite
  would notice a stop drifting to a neighbouring step, so the ADR §7 proof ("a stop before each step 1–6") is only as
  good as a removed probe. The NOTES' own row 5 is wrong: "reset done, op killed". My probe shows the operation
  **never ran** at 5 (`reset --hard` reports the stop, `apply.rs:239-242` → `abandon`) and the tree is left clean,
  edits in `S` only.
- **Why it matters:** this is the milestone's riskiest write, and the test is the one guard on its step order.
- **Fix:** per step, assert what that step leaves: entries written (0/1) and status, stash list length, the entry's
  last command (`stash store` / `reset --hard` / the op / `stash drop`), tree dirty or clean. Drop `|| step == 6` (it
  fires; my probe saw it every time). Correct NOTES row 5.

## Minor

- **m1. A journal failure after step 6 drops where the edits are.** `set_aside.rs:211-218`: when `finish` fails,
  `ApplyError::Journal { after: ended.failure }` carries git's failure of the op, not the put-back's outcome. A kept
  `S` or a clash is not named. The next launch's recover names it (74), so nothing is lost. Fix: carry the kept words
  in the error.
- **m2. An unreadable status after the put-back returns `Facts` with no stash words.** `set_aside.rs:278`: `(None,
  Err(why)) => ApplyError::Facts(why)`. ADR §5 says the error names `stash@{n}`; only the entry's reason does. Fix: a
  `Git`-shaped error, or put the words into the `Facts` message.
- **m3. Wrong words when the stash list can't be read after a clean put-back.** `set_aside.rs:400` maps
  `Dropped::Unlisted` to `Kept::Unread`, whose words (`:362-363`) say "the status after putting your edits back
  could not be read". The status was read and the edits are back; the list wasn't read. Fix: a `Kept` arm of its own
  ("your edits are back; the set-aside copy could not be found to drop").
- **m4. Early `?` exits after step 4 leave the edits only in `S`, and the error doesn't name it.**
  `apply.rs:278` (`stopped_on_conflict(...)?`, an `in_progress` read failing), `apply/resume.rs:361`
  (`before_commit_of(...)?`), `apply/resume.rs:230` (a stop during `--continue`). The entry stays Pending, `S` is
  listed, and relaunch recovery names it (ADR §5 kill row). Not lost; just not said at the time.
- **m5. Question: should a stop landing after `reset --hard` still put back?** `set_aside.rs:122-162`
  (`abandon`): at the step-5 stop the op never ran, and the tree is left clean with the edits in `S` (probe). ADR §5
  row "4 or later" permits this (`u` restores). But §4's rule that step 6 "never stops" would put them straight back
  on the unchanged tip, one fewer recovery for the user. Filed as a question, not a defect.
- **m6. Question: `Aborted` for a switch or rebase that took effect.** `set_aside.rs:232-234`: a put-back git
  refuses, after the op went through, closes the entry `Aborted` (test `:405`). HEAD did move; the journal browser
  will say "aborted". ADR §4/§5 don't fix the status. It's still in force and `u` works (probe). Decide the status
  and state it in ADR §5.
- **m7. README still says a dirty rebase is refused.** `README.md:133-134`: "A rebase on a dirty tree is refused
  with `S stash 1 changed file` on the line." Untrue since this ticket; not in 74's Touches (grep of
  `74-…md` for README: none). Textual.
- **m8. No test for `u` after a kept put-back or a stop at 4/5.** ADR §5's "`u` restores the backup" rows have no
  test in the suite. My probes show they hold. Add two cases: the hook test (`set_aside.rs:376`) followed by `u`,
  and M2's step 4/5 cases followed by `u`.
- **m9. Fast-tier headroom.** `set_aside_round_trips` is now the slowest fast test (10.6 s). `cargo test` took
  170.7 s against the 180 s budget (PROMPT 7.1). Consider splitting the table per kind so it parallelises.

## Architecture and readability

- Crate rules hold: graft-git untouched; stash args built in graft-ops as the existing restore steps do (scout Q1).
  graft touched in tests only. `set_aside.rs` (470 lines) is one module with the order in its header doc, and
  `Outcome`/`Returned`/`Kept` are named plainly. `apply` grew about 20 lines but still reads top to bottom. No finding.
- Pure core kept: `sets_aside` is pure over the facts; the preview and `apply` ask the same question (`preview.rs:363`).

## The builder's deviation (m5 walk, TESTING row 12)

**Accepted.** `rebase-dirty-tree-refused-offers-stash` would go red under the new `check`. The replacement
(`crates/graft/tests/it/m5_walk/merge_rebase.rs:301-354`) asserts the box opens with `Confirm`, no clean-tree status,
and `Esc` moves no ref or file. A regression to the refusal fails on the box check, not on a timeout. TESTING row 12
matches it word for word. It stays within the crate set (graft, tests only) and was flagged in NOTES. What it no
longer covers is a *confirmed* dirty rebase end to end in the app. 74's walks own that (`74-…md:113-120`). Nothing
to fix here; 74 must not drop them.

## Pipeline

Design note has Need, Prior art (citing `docs/research/commit-stash.md` A1–A4) and Journeys. This ticket has no UI;
the journeys (`switch-dirty` 3, `switch-dirty-clash` 5, `pull-rebase-stop-then-clash` 8) are walked in 74, not here.

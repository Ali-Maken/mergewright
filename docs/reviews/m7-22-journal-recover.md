# Review — M7 group: journal list + recover (tickets 22, 22b, 23)

Reviewer, 2026-09-26. Read-only against `b09274ea`. Merges reviewed: `72699ce4` (22), `d1ce35a9` (23, with the
conductor's `files: None,` seam in five graft test literals), `e6d620e8` (22b), each as `git diff <m>^1 <m>`.
Checked against ADR-0046, ADR-0048, the M7 note's pass 2b (F1, F4, F5, J1, hard choice 5, the words table,
Interfaces 2b) and the three tickets.

**Counts: Critical 1 · Major 4 · Minor 9.**

How I verified (rule 1): every gate and tier was run on a `git archive` snapshot of `b09274ea` in my scratchpad with its
own `CARGO_TARGET_DIR`, so no worktree was touched. Behavioural findings C1 and M1 were **reproduced**, each with a
throwaway probe test run against the real code in a second scratch copy (bodies quoted below; not committed). M2 was
reproduced by a mutation run. All else is by reading, with lines.

---

## Gates and tiers

| Check | Result |
|---|---|
| `cargo fmt --check` | ok |
| `cargo clippy --all-targets -- -D warnings` | ok (19 s warm) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | ok |
| `cargo nextest run` (workspace) | **3764/3764 passed**, 1 skipped, **453 s** wall |
| `cargo test --doc` | ok (3 doctests) |
| Heavy tier `cargo test --features heavy-tests -- --test-threads=2` | graft `it` binary **520 passed, 3 failed** (8 m 35 s), see below |
| Release perf run | see below |

**The machine was contended.** Two or three builders ran cargo in their own worktrees throughout. The load average
reached 63 during the heavy run. The fast suite's 453 s is over the 3-minute budget, but I cannot attribute it: the same
suite on a quiet machine is not measured here. **Unverified against the budget.**

- **5 slowest (nextest):**
  - `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 46.6 s
  - `graft-ui …hints::a_tail_hint_gains_its_verb_only_once_the_whole_list_is_drawn`: 22.3 s
  - `graft-ui …hints::no_hint_loses_ground_as_the_room_for_it_grows`: 22.1 s
  - `graft::it published_repo::nothing_a_commit_would_carry_names_this_machine`: 19.5 s
  - `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys`: 19.2 s

  None is this group's.
- **Peak memory:** max RSS of any one process 274 MB (`/usr/bin/time -l`).
- **`target/`:** the shared `target/` is 12 GB; my snapshot's is 16 GB (debug + heavy).
- **Heavy failures (not this group's code):**
  - `many_refs::reading_50k_real_refs_is_within_budget`: 3.58 s against a 1.5 s budget.
  - `many_refs::first_paint_with_50k_real_refs_is_within_budget`: 4.31 s against 1.5 s.
  - `heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion`
    (`crates/graft/tests/it/heavy.rs:217`, "the second answer is in the new order"; the 12-series promotion work).

  None of the three touches `journal_list`, `reflog`, `recover` or the fixture. The rerun alone and the other heavy
  binaries are in the addendum at the end.
- **Fuzzing:** not run. No fuzz target covers this group: `reflog` reads through gix, and the subject parsing in
  `steps.rs`/`words.rs` is `strip_prefix`/`split_once` with no indexing (no panic path found by reading).
- **Benchmarks:** none for this group.

## Acceptance ticks, spot-checked

- All **60** test names ticked across 22/22b/23 exist at the files named (grep).
- All pass in the nextest run above.
- I read the bodies of 30, per group:

| Group | Read | Held |
|---|---|---|
| 22 pure `build` | 3 (`a_graft_op_is_one_row_not_two`, `head_and_branch_records_pair_within_the_window`, `later_stages_keep_earlier_rows_in_order`) | 3 |
| 22 `words` | 1 (`a_rebase_finish_names_its_upstream`) | 1 |
| 22 oracle | 3 (`shell_reset_hard…`, `a_deleted_branch_once_checked_out…`, `a_branch_deleted_and_never…`) | 3 |
| 22 `recover_for` | 2 (`moved_on_offers_main_restored`, `an_existing_restored_name_gets_a_suffix`) | 2 |
| 22 format / `records` | 2 (`a_recover_entry_round_trips`, `records_caps`) | 2 |
| 22b fixture | 2 (`the_lost_work_fixture_draws_every_row_in_place`, `the_lost_rows_are_the_fixtures_commits_and_his_edit`) | 2 |
| 23 targets | 9 (all eight named + `…refuses_over_an_untracked_file…`) | 9 |
| 23 undo | 2 (`u_undoes_each_target`, `u_after_gc…`) | 2 |
| 23 lost edits | 3 | 3 (the tick is honest that it is not red first) |
| 23 preview / journal | 3 (both preview tests, `touches_match_what_changed`) | 3 |

**30 checked, 30 held. No false tick.**

- The "red first" claims (22's `a_graft_op_is_one_row_not_two` by stubbing `match_entries`; 23's untracked refusal) are
  history I cannot re-observe. They are **unverified, not doubted**.
- 23's mutation claim for the **drift** condition I re-ran: dropping it fails all three lost-edit tests. **Held.** The
  other condition is M2.

## Feature pipeline

- The note has Need (`m7-power-tools.md:20`), Prior art (`:32`, citing `docs/research/reflog.md`) and Journeys (`:69`,
  2b counts `:1242-1253`).
- This group builds no screen, so no journey is walked here. Step counts land with 24 and 26. See M4: the fixture and F1
  already disagree about journey 13's count.

---

## Critical

### C1. A put-back of the checked-out branch overwrites an ignored file, and `u` does not bring it back

- **Where:**
  - `crates/graft-ops/src/preview/recover.rs:98-112` (`untracked_in_the_way`): it takes only `facts.status` entries with
    `untracked`.
  - `facts.status` is `git status --porcelain=v2 -z --untracked-files=all --renames` (`crates/graft-git/src/status.rs:471`).
    It has no `--ignored`, so ignored files are never in it.
  - `crates/graft-ops/src/apply.rs` `guard_untracked` (`:465`) asks the same read at `y`.
  - `crates/graft-ops/src/operation/recover.rs:218-222` then runs `reset --hard <to>`, which writes over **any** file
    not in the index at a path `to` tracks, ignored ones included.
- **What happens (reproduced):**
  - Raw git (git 2.54.0): with `local.env` in `.git/info/exclude` and holding `HIS SECRET`, `git reset --hard <lost>`
    (where `<lost>` tracks `local.env`) exits 0. The file then holds the commit's bytes, and status listed nothing
    before.
  - Through Graft, with this probe in a copy of `tests/it/recover.rs`:

    ```rust
    let repo = three_commits();                    // c.txt tracked in "three"
    let three = id(&repo, "HEAD");
    repo.git(&["reset", "-q", "--hard", "HEAD~1"]);
    let two = id(&repo, "HEAD");
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", b"HIS IGNORED FILE\n");
    let op = move_ref("main", three, two);
    // preview files: Some(Changes { come_back: ["c.txt"], go: [], set_aside: 0 })  — no refusal
    // try_run: Ok
    // c.txt after y: "c\n"
    // run_undo: Ok;  c.txt after u: "c\n"
    ```

    His bytes are gone after `y`, and `u` does not restore them.
  - The backup cannot hold them: `snapshot::worktree_tree` is `add -A` over a temp index
    (`crates/graft-ops/src/snapshot.rs:198-206`), which skips ignored files. Nothing in git has them either.
- **Why it matters:**
  - This is the M3 promise, "you cannot lose work", broken on the milestone's own recovery path.
  - It also breaks 23's "`u` after a put-back leaves the repo byte-for-byte as before `y`" for this input.
  - The trigger is narrow but real: a local config file (`.env`, `config/local.yml`) that the lost commit tracked, and
    that he has since ignored and kept a private copy of.
  - The codebase already guards this class for stash apply: `StashInTheWay` checks the disk, "ignored files included"
    (`crates/graft-ops/MAP.md`, the `preview::stash` line; `preview/stash.rs:120-126` `occupied`).
- **Fix (cause):**
  - Decide "in the way" from the disk, not from status. For each path `tree_diff(expected, to)` adds or changes that the
    index does not track, check whether a file exists there. The stash preview's `occupied` is the precedent.
  - Refuse `UnsavedEditsTo`, in the preview and in `guard_untracked` alike.
  - Add the probe above as a test, red first.
- **Open question:** whether Graft's own `Checkout` has the same exposure. `git checkout` also treats ignored files as
  expendable. **Unverified.** Not checked here; worth one probe.

---

## Major

### M1. A detached checkout of a remote-tracking branch or tag is read as a branch; `⏎` then makes `refs/heads/origin/main`

- **Where:**
  - `crates/graft-ops/src/journal_list/steps.rs:257-258`: `after_head_record` takes any non-hex `to` of
    `checkout: moving from a to <to>` as the branch `HEAD` is now on. git writes what he typed, so `origin/main`, `v1`
    and `HEAD~3` all pass.
  - `crates/graft-ops/src/journal_list/rows.rs:213-218`: when the leaving record is `moving from <full hex> to main`
    (git's proof that `HEAD` **was detached**), `record_hint` skips `SwitchedFrom` and falls through to `on`. That is the
    wrongly inferred `origin/main`.
  - `crates/graft-ops/src/journal_list/rows.rs:234-240`: `held_by` makes that `Branch { name: "origin/main", gone: true }`.
  - `crates/graft-ops/src/journal_list/recover.rs:202-206`: → `Recreate`.
  - `recover.rs:252-253`: `because` is "the shell's branch -D".
- **Reproduced** (a probe in a copy of `tests/it/journal_list_oracle.rs`, real git): `update-ref refs/remotes/origin/main
  HEAD`; `checkout origin/main`; commit "Detached work"; `checkout main`. The list:

  ```
  "$ switch from 3e8236f to main",
  "✗ Detached work · origin/main gone · under 0",
  "$ committed \"Detached work\" on origin/main",
  "$ switch from main to origin/main",
  ```

  `recover_for` on the `✗` row returned `Run(RecoverParams { target: Recreate { name: "origin/main", at: 3e8236f… },
  because: "the shell's branch -D" })`.
- **Why it matters:**
  - Three wrong statements on the screen he opens under stress: "origin/main gone", "committed … on origin/main", and
    "the shell's branch -D".
  - Worse, `y` creates `refs/heads/origin/main`. git resolves `refs/heads/` before `refs/remotes/`, so from then on his
    own `git rebase origin/main` or `git log origin/main` uses the stale local copy. On a team with dependent branches
    (memory: works-in-a-team) that is a silent wrong base.
  - Checking out a teammate's `origin/<b>` detached is ordinary daily use.
- **Fix (cause):**
  - `after_head_record`: a `to` is a branch only when the logs prove it: a later `moving from <to> …` by name, or a
    branch record paired with a later `HEAD` record.
  - `record_hint`: a hex `from` means detached, so return `Hint::Head`.
  - Defence in depth in `recover_for`: never `Recreate` a name that exists as `refs/remotes/<name>` or
    `refs/tags/<name>`. Add both to `recover_scope`'s refs and offer `restored` instead.
  - Oracle tests: detached at `origin/main`, and detached at a tag.

### M2. `lost_edits`' "the undo wrote there" condition has no test; removing it leaves every lost-edit test green

- **Where:** `crates/graft-ops/src/journal_list/edits.rs:99-103`, the `written` side and its intersection.
- **Reproduced by mutation:**
  - I replaced `:103` with `(Some(drifted), Some(_written)) => drifted`.
  - All six lost-edit tests still passed: `recover.rs`'s three and `lost_work.rs`'s three.
  - A probe on the mutated code produced a false row. He edits `b.txt`, which the checkout never touched, the undo is
    forced over drift, and he edits `b.txt` again. `lost_edits` then listed `["a.txt", "b.txt"]`.
  - The real code lists `["a.txt"]` (probe run on the unmutated code too).
- **Why it matters:**
  - The ticket asks whether the mutations are enough. They are not. The builder mutated the drift condition (verified:
    it fails 3/3) and the on-disk filter, but not this one.
  - A regression here would draw a `✗ your edit to b.txt` for a file whose edit is on disk. `⏎` would then be refused
    (`b.txt has unsaved edits`) or, once clean, write an older copy over it.
  - The one-side-unknown arm (`edits.rs:102`) is also untested.
- **Fix:**
  - Add the probe as `an_edit_to_a_file_the_undo_never_wrote_is_not_lost`.
  - Add one test for a reversed entry with no recorded worktree (`:102`).

### M3. Readability: `recover_scope`'s `Option<Option<&str>>` and `lost_commit`'s `unwrap_or(tip)` trick

- **Where:**
  - `crates/graft-ops/src/journal_list/recover.rs:78-94`: the outer `Option` means "this row has a scope", the inner
    "and it names a branch".
  - `recover.rs:213`: `Some(now) if now == left_at(…).unwrap_or(tip)` relies on the arm above having already taken
    `now == tip`, so a missing `left_at` can never match.
- **Why it matters:** PROMPT.md 6.2, plain over clever. Both need the reader to hold the other arm in mind to see they
  are correct, and this is the file M8's journal builds on.
- **Fix:**
  - A three-way match in `recover_scope`: no scope; a branch and its `-restored` names; `restored` names only.
  - In `lost_commit`, `let left = left_at(…)` first, then `Some(now) if Some(now) == left`.

### M4 (question, Major weight). F1's preselect rule and the lost-work fixture disagree about journey 13

- **Textual, from the documents:**
  - F1 (`m7-power-tools.md:1057`, ADR-0048 §5) preselects "the newest `✗` row whose event is newer than every `●` row".
  - In 22b's fixture (`crates/graft-ops/tests/it/lost_work.rs:129-147`) the `●` Fetch, Undo and Discard rows (10:43–10:44)
    are all newer than the `$ reset` that holds `✗ Add rate limiter`.
  - The note's own mock-up (`:239-241`) has `● fetch 10:44` above the preselected `✗`.
  - Read literally, F1 does not preselect in this fixture. Journey `lost-to-shell-reset` becomes `J n ⏎ y` = 4 against
    the note's 3 (`:1250`), and 26's walk would fail its count.
  - The `✗` lost edit sits under a `●` row, so F1 can never preselect it.
- **Why it matters:** 24 builds the preselect and 26 walks it. Better decided now than found red there.
- **Question for the conductor:** does F1 mean "newer than every *undoable* `●`" (a fetch is not undoable), or should the
  fixture's Graft steps move before the reset? Either choice is a spec edit, not a 24 build decision.

---

## Minor

- **m1.** `.scratch/m7-power-tools/issues/23-recover-op.md:3`: the Status still reads "blocked: needs graft (five test
  literals…)". The seam landed in `d1ce35a9`. Set it to done.
- **m2.** ADR-0046 is not amended.
  - Ticket 22 (`:79-80`) says the conductor records hard choice 5 (the `recover` serde tag, written for good) as an
    ADR-0046 amendment. `docs/adr/0046-…md` has none.
  - ADR-0046 §3 (`:31-32`) still says the put-back "restores only refs". The `Files` target and J1's lost edits
    contradict it.
- **m3.** The KNOWN_GAPS M7/22 row says "the three new Refusals". There are four: `MovedOnSince`, `HeldAgain`,
  `NoPutBack` and 23's `UnsavedEditsTo`. By reading, graft's catch-alls are benign:
  - `crates/graft/src/update/operate.rs:1105` `other => Status::sticky(other.explain())`. `explain` is exhaustive in
    graft-ops, so the new refusals already have their own words.
  - The `OperationKind` `_` arms (`update/preview.rs:267` → "the result"; `update/journal.rs:282`;
    `view/journal.rs:148`) fall to generic paths, and a Recover row shows its `describe()` text.

  Still unverified by test until 24, as the row says.
- **m4.** `crates/graft-ops/src/journal_list/recover.rs:252-253`: `because` is "the shell's branch -D" whenever the
  branch is gone now. That is also said when the branch was deleted later by something else (M1's case, or a later
  Graft delete). Name the deleting step, or say "spike is gone".
- **m5.** The preview's `go` list is never non-empty in a test (`recover_preview_lists_your_files` asserts
  `go: vec![]` twice). A put-back whose `to` lacks a file `expected` has goes untested.
- **m6.** A checked-out `MoveRef` with **staged** edits is not tested (only unstaged, `recover.rs:182`, `:427`). It
  rests on ADR-0045's set-aside tests. One case in `u_undoes_each_target` would close it.
- **m7.** `crates/graft-ops/src/journal_list/steps.rs:322-330` (`claim`) has no time window for (ref, old, new)
  matches. A Graft entry whose own reflog record fell past `RECORDS_CAP` can claim a shell step with the same values from
  any time, hiding a `$` row. Rare. A generous window (like `CHECKOUT_WINDOW_MS`) would bound it.
- **m8 (question).** `MoveRef` off `HEAD` runs `update-ref` even when the branch is checked out in **another linked
  worktree**. That worktree's files then read as reverse edits. The rewrite path refuses this case
  (`crates/graft-ops/src/operation.rs:644`, `:904`). Should the put-back refuse it too?
- **m9 (question, for 24).**
  - `build` is O(entries × steps × members) in `claim` and O(records²) in `pair_head_records`/`fold_one`
    (`steps.rs:137-146`, `:175-190`, `:322-330`), and the app calls it three times.
  - `reflog::records` takes no `Stop`, and `read_outside` does up to 100 commit reads per reset or fast-forward record
    (`journal_list.rs:186-198`).
  - Fine off-thread. 24 should measure `build` at 2 000 records × a large journal before calling it on the UI thread.

## Focus questions from the conductor, answered

- **Can a put-back lose work?**
  - Untracked in the way: refused (tested).
  - Dirty `Files` paths: refused, staged, unstaged and untracked (tested).
  - Ref moved: CAS in `check` (tested).
  - Staged state: set aside by ADR-0045 (untested here, m6).
  - **Ignored file in the way: lost (C1).**
- **Undo inverse proven by test:** yes for all four targets, as a whole-state comparison (`u_undoes_each_target`), plus
  the `gc` pin. It fails for C1's input.
- **22's signature deviations:** justified.
  - No `git` param: graft-ops has no `Git` type, and the reads are gix.
  - `branch_exists`: `gone` must be the ref store's answer.
  - `entries` in `recover_for`: it needs the after-value and the words.
  - `recover_scope`, `Recover`, `NoPutBack`: the spec had no way to say "offer a branch" or "nothing here".

  23's `lost_edits` moving into `journal_list` (operation stays pure) is also justified.
- **`lost_edits`' third condition:** justified, and it is the right rule. Without it, undoing a checkout lists the
  checkout's own files. `lost_edits_finds_the_edit_an_undo_over_drift_replaced` asserts `o.txt` absent, and my mutation
  confirms the test bites. Its sibling (the undo wrote there) is unguarded (M2).
- **I checked one suspicion and dropped it:** does a redo after the forced undo hide the edit? No. The probe showed the
  redo restores `a.txt` to `"a on other, and his edit"`, and `lost_edits` correctly lists nothing.
- **Errors swallowed (rule 3):** none found.
  - Every `?` maps into `FactsError`/`ApplyError`.
  - `reflog.rs`'s `NotFound => {}` for a missing `HEAD`, and its `Ok(None)` for a pruned commit, are commented and
    correct.
  - `rows.rs:121-122`'s subject fallback text is display-only, for a subject the caller could not read.
  - `rows.rs:126-130`'s unplaceable tip is argued unreachable in a comment.
  - No `unwrap`/`expect`/`let _ =` outside tests in the new files.
- **Escape sequences:** reflog subjects and branch names flow raw into `words` and `because`. That is correct for
  graft-ops: ARCHITECTURE.md (`:195-198`) makes `term::SanitizedText` the only way into `ui`, applied by `app`. It
  becomes 24's to honour.
- **Knock-on to existing journal and undo paths:**
  - `undo::replaced::raw_diff` now delegates to `tree_diff`; the `undo_replaced*` tests are green.
  - `kind_name` and `pins_for` are exhaustive and extended.
  - `sets_aside` and `put_back::forecast` are extended, with set-aside tests green.
  - No graft `match` on `Operation` is exhaustive.
  - The old journal goldens are green, and `a_recover_entry_round_trips` pins the tag.

## Architecture

- Crate rules hold: graft-fixtures depends on no Graft crate (its `Graft` trait is the seam, with two implementors),
  graft-git has no ops dependency, and `operation` stays pure.
- `lost_edits` and `find_lost` read the repo from `journal_list`, beside `read_outside`, as the module doc says.
- MAP.md lines match the code for every new public item I read.
- ARCHITECTURE.md has no line for `journal_list` or `reflog`. The module docs are good, but the crate tables should
  name them (Minor, folded into m2's doc pass).

---

## Addendum — heavy rerun and release perf run

**The rest of the heavy tier** (`--no-fail-fast --workspace --exclude graft`): **1959 passed, 0 failed** (5 m 35 s).
With the graft `it` binary's 520 passing and 3 failing, the heavy tier is 2479 passed and 3 failed on the first run.

**The three graft heavy failures, rerun alone:**
- `many_refs::reading_50k_real_refs_is_within_budget`: passes alone, 740 ms.
- `many_refs::first_paint_with_50k_real_refs_is_within_budget`: passes alone, 946 ms.

  Both were load, not code.
- `heavy::the_table_and_the_order_stay_at_the_commit_count_after_g_and_after_promotion` **fails alone too**, and
  deterministically: `heavy.rs:217`, "the second answer is in the new order".
  - It is outside this group (the history worker's promotion, 12-series).
  - I found it in no KNOWN_GAPS row and no review (grep).
  - **For the conductor:** it is red at `b09274ea`, and its owner is whichever of 12b/12c/12e last touched promotion.
    Not diagnosed here.

**Release perf run** (`cargo test --release -p graft --features heavy-tests --test it -- frames:: heavy::budgets_hold
--test-threads=1 --nocapture`, run after the heavy tier, alone from my side; load average 17–20 from other builders):
**13 passed, 1 failed** in 96 s.
- Input to frame, p99 against the 16 ms budget, 420 keys each, `over_budget=0` in all four:

  | Tier | Refs | p50 | p99 | max |
  |---|---|---|---|---|
  | Pixel | Fixture | 1.64 ms | 2.88 ms | 3.83 ms |
  | Unicode | Fixture | 0.76 ms | 1.09 ms | 1.33 ms |
  | Pixel | 50k refs | 1.23 ms | 3.00 ms | 4.70 ms |
  | Unicode | 50k refs | 0.74 ms | 1.36 ms | 3.72 ms |

- First paint:

  | Fixture | First paint | Bottom | Peak RSS |
  |---|---|---|---|
  | 100k + 50k refs + commit-graph | 96 ms | 127 ms | 118 MB |
  | 100k commit-graph | 30 ms | 89 ms | 32 MB |
  | 100k no commit-graph | 136 ms | 959 ms | 97 MB |
  | realistic commit-graph | 125 ms | 105 ms | 36 MB |
  | realistic no commit-graph | **180 ms, failed the 150 ms budget** | 982 ms | 104 MB |

  The failing test rerun alone twice: **40 ms and 41 ms**, pass. That is load at the time, not a regression. Nothing in
  this group is on the first-paint path: `journal_list` runs only when `J` opens, and 24 is not built.

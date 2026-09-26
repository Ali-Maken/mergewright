# Review — M7 group 02: the rewrite engine (02a `66eaf8aa`, 02b `2c167a23`)

Reviewer, 2026-09-26. Read-only against `2c167a23`, checked out detached in a scratch clone (own `CARGO_TARGET_DIR`)
so the conductor's merges could not move the tree under the run. **Every file:line below is at `2c167a23`.** (Since then
`f84cfea6` (absorb, 18) grew `rewrite/plan.rs` to 795 lines — see Major 3.)

Scope: `.scratch/m7-power-tools/issues/02-rewrite-engine.md` + `02b-rewrite-engine-b.md`, against
`docs/design-notes/m7-power-tools.md` (hard choices 2–3, R1, R2, R4, "Interfaces"), ADR-0047, ADR-0045 §4.

**Counts: Critical 0 · Major 3 · Minor 11 · Questions 3.**

## Gates and tiers (run one at a time)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo test --no-fail-fast` (fast suite, doc tests included) | pass, 3586 tests, **279 s** wall, max RSS 779 MB |
| `cargo nextest run` (run 1, for the regression-net tick) | pass, 3583 run, 1 skipped, 339 s |
| `cargo nextest run` (run 2, for per-test times) | **1 fail**: `graft::it status_under_a_box::capture_fix_status_under_a_box` (Minor 11); 412 s |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | pass, 3621 passed, 1 ignored, **790 s**, max RSS 1.03 GB |
| Release perf (`frames::` + `heavy::budgets_hold`, alone) | pass, 14/14, 95.5 s |
| Fuzz / benches | not run: no fuzz target or bench touches `graft-ops::rewrite`/`apply::engine` |

Times were taken while other builders' `nextest` runs were loading the machine (seen in `ps`). Treat them as upper bounds.

- **5 slowest tests (nextest run 2):** `conflict_ops::whole_file_shapes_resolve_like_git` 34.5 s ·
  `diff_fixture::parse_unified_matches_numstat_for_every_shape` 32.9 s ·
  `layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 24.7 s ·
  `corpus::two_builds_produce_the_same_object_ids` 21.6 s · `m5_walk::every_scenario::merge_resolved_with_the_keys` 18.3 s.
  None of them is in this group.
- **Perf numbers:**
  - Frames p99: Pixel with 50k refs 3.10 ms; Unicode with 50k refs 0.81 ms; diff scroll 0.80 ms; detail pane 0.66 ms;
    map 0.42 ms; 100k fixture 2.40 ms (Pixel) and 0.77 ms (Unicode).
  - First paint: 23–139 ms. The worst is the realistic fixture with no commit-graph.
  - Peak RSS at the bottom: 31–112 MB.
- **`target/`:** the scratch target (debug + release) is 15 GB. The shared `target/` is 10 GB.

## Acceptance ticks — spot-checked

| Group | Checked | Held | How |
|---|---|---|---|
| Pure (`rewrite/tests.rs`) | 6 of 6 named + 2 extra | 8 | read each; the assertions are what the tick says. Red-first claims not re-run |
| Oracle (`rewrite_oracle.rs`) | 11 of 11 rows | 11 | read each: every row runs the hand route on a `copy()`, `agree` compares branches' tree+message logs, symbolic HEAD, porcelain v2, and asserts a change (`:199-207`) |
| Record and undo (`rewrite_run.rs`) | 5 named + 4 "also" | 9 | read each. Note: every run test drives `Operation::Rebase`, except the edit-stop test. No test undoes an `OperationKind::Rewrite` entry (Minor 7). My probe did, and it held (see "Probes") |
| Refusals (`rewrite_plan.rs`) | 7 of 7 + plain-amend | 8 | each asserts `explain()` words, except `a_dependent_in_another_worktree…` (prefix/suffix + path) and plain-amend (the variant only) |
| Dependents | 4 + backup | 5 | read |
| Progress | 3 + `no_rebase_no_progress` | 4 | read; `feature-b, 2 of 3` asserted at `:630-637` |
| Regression net | 1 | 1 | my nextest run 1: 3583 passed, 1 skipped. Cargo test: 3586 passed |
| MAP.md | 1 | 1 | `Operation`, `Context`, `apply::engine`, `Resume`, `AfterState`, `Recovered`, `rewrite` all updated |

No false tick found.

## Probes (throwaway tests in the scratch clone, deleted after; not in the repo)

- **P1:** `a` into `feature~3` with one staged file plus one unrelated unsaved edit, then `u`.
  - The staged change landed in the target.
  - The unsaved edit came back, and no stash was left.
  - `u` restored refs, index and every file byte-for-byte (`state_of` equal).
  - **No work lost on this path.**
- **P3:** `r` on **`main`** onto another branch, with `feature` started on `main`'s tip. `dependents(main)` returns
  `[main, feature]`, and **`feature` moved** (Question 1).
- **P5:** `R` on a branch with no commits of its own, pressed on its base row. The plan's one row is `main`'s commit
  `m2`, and it is editable (Minor 3).
- **P6:** `r` with a fork above his branch (`feature-b` and `experiment` both on `feature-a`) is now refused with
  `experiment and feature-b are both built on yours · Graft moves one line`. Before this group it rebased (Major 1).

## Critical

None.

## Major

### 1. `r` now moves (or refuses over) branches the preview never showed — "you see the result before you confirm" is broken at HEAD
- **Where:**
  - `crates/graft-ops/src/apply/engine.rs:151-166` finds `r`'s line of dependents when `y` runs.
  - The preview it confirms still moves one branch: `crates/graft-ops/src/preview/kinds.rs:151-155`
    (`ref_moves = vec![his branch]`).
  - The preview's command line lacks the top: `operation/history.rs:609-611` builds it with `top = None`, while the
    run passes `<top>` (`engine.rs:165-166`).
- **What goes wrong, observed in probes P3/P6:**
  - A teammate's branch built on his is rewritten with no line in the preview.
  - `r` on `main` drags a feature branch started on `main`'s tip.
  - With two branches on top, `r` is refused **after `y`**. Before this group it worked.
- **Why it matters:**
  - This is M5's promise.
  - The user works on dependent branches daily (memory: works-in-a-team-on-dependent-branches).
  - `u` does put everything back (`undo_puts_every_branch_in_the_line_back`), so nothing is lost. But the act and the
    picture disagree.
- **Known to the builder** (NOTES -b item 2: "Costs until 05"). The conductor's ruling moved `RebaseParams.line` to 05.
  But nothing stops the gap from shipping in between, and it has **no KNOWN_GAPS row**.
- **Fix:** either
  - (a) until 05 merges, run `r` with a one-branch line (the pre-M7 behaviour, still through the engine); or
  - (b) file a **broken** KNOWN_GAPS row now, and make 05 close it with a walk that asserts the preview names every
    branch that moves.

  (a) is the cause-level fix. (b) accepts a known lie in the preview for the interval.

### 2. `a` off his branch is refused with `R`'s words
- **Where:**
  - `crates/graft-ops/src/rewrite/plan.rs:190-193`: `AmendInto` refuses with `Refusal::NotOnYourBranch`.
  - Its words, at `crates/graft-ops/src/operation.rs:835-836`, are `R rewrites your branch · <short> isn't on it`.
- **Spec:** the note R2 (`m7-power-tools.md:565-567`) gives `a` its own words:
  `a puts staged changes into a commit on your branch · 9c8d7e6 isn't on it`. The ticket says the words must match R1/R2
  exactly.
- **Test:** none. `a_commit_off_your_branch_is_refused` covers `Open` only.
- **Why it matters:** the user pressed `a`, and the status line names `R`. That fails the consistency heuristic
  (ux-principles), and it breaks the ticket's "Display = the note's words exactly".
- **Fix:** carry the verb in the refusal: `NotOnYourBranch { commit, by: RewriteKey }`, or a second variant. Add a
  test asserting the `a` words.

### 3. File size over the 6.2 limit (readability is Major)
- **New files:**
  - `crates/graft-ops/src/rewrite/plan.rs` is **707 lines**, a new file against a ~400 limit.
  - It mixes three jobs: planning (`plan`/`open`/`amend_into`); the dependents search (`dependents`, `chain_above`,
    `local_branches`, `checked_out_elsewhere`, `default_branch_of`); and raw history reads (`read_range`,
    `rebase_rows`, `messages_of`, `parents_of`, `commit_index`, parsers).
- **Growth past the limit in this group:**
  - `apply/resume.rs` 420 → 511;
  - `apply.rs` 544 → 610;
  - `operation.rs` 1281 → 1410, already far over.
- **Fix:** split `plan.rs` into `plan.rs`, `dependents.rs` and `reads.rs`, with no behaviour change. Move
  `resume_invocation`/`stopped_on_edit`/`started_on_its_line` into `apply/engine.rs` or a `resume/rewrite.rs`.

## Minor

1. **The note's gaps are not filed.** NOTES -b item 8 lists gaps "for KNOWN_GAPS, by the conductor", and none are in
   `KNOWN_GAPS.md` at `2c167a23`:
   - The kill window between git finishing and `git switch <home>`. `recover.rs:84-94` closes the entry and removes the
     plan, but leaves HEAD on the **top (teammate's) branch**, so his next commit lands on it.
   - `--empty=drop` has no oracle row.
   - `Recovered.plans_left` is unread by graft (`graft/src/jobs/undo.rs:583` carries only `live`).
   - `Context.default_branch` / `plan(…, default_branch)` are never set by graft, so the user's `graph.default_branch`
     override is ignored by `r` and `R`.
   - Stale M5/M6 captures.

   Fix: conductor files them. The first is **broken (unverified)**, the rest **unverified**.
2. **`EverythingDropped` words drift from R1.** `crates/graft-ops/src/rewrite.rs:242` says
   `every commit dropped · the branch would be its base again`. R1 (`m7-power-tools.md:521`) says
   `… the branch would be main again`. `validate_refuses_everything_dropped` (`rewrite/tests.rs:121-125`) asserts only
   the prefix, so the drift is invisible. Fix: either use the note's words (naming the base branch), or record the
   generalisation in NOTES for the conductor, and assert the whole string.
3. **`R` pressed on the base row makes the base editable** (probe P5).
   - `plan.rs:129-139`: when `from` *is* the base, `ancestor(from, base)` is true, so the rows run down to `from`
     itself, and main's tip becomes an editable row.
   - R1 (`:470`) says the base is shown as `─ main` and is not editable. "Deeper" should mean strictly below the base.
   - Fix: treat `from == base` as `onto = base`, and test it.
4. **Orphan plan dirs, and `progress` picks any plan by top.**
   - The plan is saved before the Pending line (`apply.rs:285-287`, by design), so a kill between `save` and `begin`,
     or a partial `saved::save` failure (`rewrite/saved.rs:81-97`), leaves a dir no entry owns.
   - `recover.rs:89-93` removes only the dirs of the entries it closes.
   - `progress.rs:57-58` takes the first plan whose `top()` equals `head-name`, in `read_dir` order. So a later shell
     rebase of that branch reads `begun_in_graft: true`, with another rewrite's chain words.
   - Fix: key `progress` by the live `Pending` entry's id (as `resume` does, `resume.rs:145-159`). Sweep plan dirs with
     no Pending entry in `recover_interrupted`. Clean up a partial save.
5. **Rule 3: silent defaults.**
   - `apply/engine.rs:193-198`: `message_of` falls back to `unwrap_or_default()`, so a commit `messages_of` did not
     return becomes an empty part of a squash's default message, with no error.
   - `rewrite/plan.rs:148-152` and `:206-209` do the same with `map_or_else(String::new, …)`. Both are unreachable today.
   - `engine.rs:177-179` returns `Ok(None)` if `Rewrite`'s `plan()` were ever empty, which would silently take the
     non-engine path.
   - Fix: return `FactsError::History(Unparsable)` for a missing message, and make the empty-plan case an error. Or
     add the one-line "why safe" comments rule 3 asks for.
6. **`CLEANUP` clobbers the user's `GIT_CONFIG_COUNT`.**
   - `rewrite/saved.rs:32-36` sets `GIT_CONFIG_COUNT=1`. Any `GIT_CONFIG_COUNT`/`KEY_n` the user exports (a documented
     git feature) is replaced for the whole rewrite, hooks included.
   - Fix: pass `-c commit.cleanup=whitespace` on the `rebase` and resume command lines (the sequencer's commits
     inherit it). Or append at index `$GIT_CONFIG_COUNT`.
7. **Test gaps.**
   - (a) No test undoes an `OperationKind::Rewrite` entry. Every record and undo test uses `r`. The MAP for
     `OperationKind::Rewrite` still says "unverified until rewrite-engine-b's undo test"
     (`graft-core/MAP.md` journal line). My probe P1 passed, so add it as a test, and fix the MAP line.
   - (b) `progress` is only tested against a hand-written `plan.json` (`rewrite_plan.rs:607-616`), never against the plan
     `apply` actually saves. A format drift between `saved::save` and `progress` would pass.
   - (c) `amend_into_head_with_nothing_built_on_it_is_a_plain_amend` asserts the variant, not the words.
8. **The preview and the Command Log now read `git rebase -i …`.** Six regenerated captures show it
   (`docs/shots/m5-ui-preview-rebase-80x24.txt` etc.). To a git user, `-i` means "an editor will open". Nothing opens,
   and the box doesn't say so. Textual. Fix: 05's preview could say `(todo written by Graft)`, or show the command
   without `-i`. The conductor decides.
9. **ARCHITECTURE.md is silent on the engine.**
   - It has no "where to start reading" line for rewrites (`rewrite` → `apply::engine` → `apply::resume`).
   - It doesn't mention that Graft now owns executable files under `<git dir>/graft/rewrite/`.
   - Fix: one bullet in "Where to start reading".
10. **The 02b builder regenerated six graft captures while graft was held by 04.** See NOTES -b item 9 and CLAUDE.md
    "A builder never edits outside its crate set". The captures aren't a crate, and the change is one command line each,
    so this is flagged, not reversed. The risk the builder named (04 regenerating the same files) is the conductor's to
    check at merge.
11. **A new intermittent capture failure, not this group's code.**
    - `graft::it status_under_a_box::capture_fix_status_under_a_box` failed once in 3 full runs, under load:
      ```
      line 150:
        committed   graft /code/api  148 rows so far  date order, sorting…
        drawn       graft /code/api  148 rows so far
      ```
    - It passed in my `cargo test` run and in nextest run 1. The shape looks timing-dependent (the sort job's status
      racing the capture), like KNOWN_GAPS M7/04b, but it is a different test.
    - Not retried into green. Fix: debugger ticket; add it to the M7/04b row.

## Questions (no line proves a defect; the conductor or the user decides)

1. **Should the default branch ever have dependents?** `dependents` excludes the default branch *as a follower*
   (`plan.rs:296-300`), but not *as the branch being rewritten*.
   - So `r`/`R` on `main` treats every local branch started from `main`'s tip as built on it (probe P3).
   - One such branch moves. Two are refused as a fork.
   - ADR-0047 §4 doesn't cover this case. A one-line rule (`branch == default → Line([branch])`) would settle it.
2. **`r` with a fork above has no way through.** ADR-0047 §4 refuses a fork, and §6 routes every rebase through the
   engine. So the plain "rebase just my branch", which worked before, is now impossible from Graft whenever two branches
   sit on his (probe P6).
   - A stale experiment branch is enough to block it.
   - Is that the intended trade, or should `r` offer "only yours moves" with the fork named in the preview?
3. **`from_utf8_lossy` on commit text.** `read_range` (`plan.rs:540`) and `messages_of` (`:646`) decode lossily. git
   re-encodes `%B` to UTF-8 by default, but a repo with `i18n.logOutputEncoding` set otherwise would feed a squash's
   default message with U+FFFD, and that message is then written. Worth a one-line guard?

## Focus items the conductor asked about

- **Undo inverse — can work be lost?**
  - Yes cases: none found. Every branch of the line is in `refs_before`/`refs_after` (`apply.rs:299-307`), and the
    generic inverse restores them with CAS.
  - `made` objects are pinned (`apply/record.rs:345-347`).
  - Probe P1 (`a` into an older commit + an unsaved edit + `u`) restored the exact state.
  - The one hazard is the unverified kill window (Minor 1): HEAD is left on the teammate's branch. That misplaces work;
    it does not lose it.
- **Refusal words:** 7 of 7 ticked refusals assert R1's words. `a` uses `R`'s words (Major 2).
  `EverythingDropped` drifted (Minor 2). The words invented for `GitTooOld`/`PlainAmend`/`RootCommit` still await the
  conductor's confirmation (NOTES -a item 2).
- **Dependents following:**
  - Proven by `a_three_branch_line` (oracle), `a_three_branch_rewrite_records_every_ref`,
    `a_stopped_line_continues_home_with_every_branch_moved`, `an_edit_row_stops_and_the_resume_takes_the_line_home`,
    and `abort_returns_head_to_his_branch_and_every_ref`.
  - The update-refs apply only at the end, so an abort leaves every branch put (asserted with `state_of`).
- **Errors swallowed:** Minor 5.
  - `go_home` ignoring a failed `head_ref` read (`engine.rs:270`) and the `canonicalize` fallback (`plan.rs:441-443`)
    are commented, and acceptable.
  - Failed plan removal and failed switch home are surfaced as `AfterState::{PlanLeft, NotHome}`. `plans_left` from
    recovery has no reader yet (Minor 1).
- **Knock-on to existing rebase paths:**
  - Every M5/M6 rebase test is green on the new engine.
  - Behaviour changes: Major 1, Questions 1 and 2, and the preview's command text (Minor 8).
  - `live_entry` now accepts `head-name` equal to the plan's top (`resume.rs:133-135`, `:145-159`). The M6 banner and resolver don't
    read `head_name` (grep in `crates/graft*/src`: tests only), so the top-of-line run doesn't mislabel them.

## Pipeline

- The design note has Need (`:20`), Prior art (`:32`, citing `docs/research/interactive-rebase.md` and
  `stacked-branches.md`, both present) and Journeys (`:69`, counts at `:809-830`).
- This group is ops-only and builds no journey. Its journeys (`squash-last-three`, `amend-with-dependents`,
  `rebase-line-onto-main`, …) are counted by the app groups' walks. Nothing to count here.
- A scout was used (`.scratch/m7-power-tools/scouts/02.md`).

# Review: M8 ticket 14, daily ops fixes

Reviewed: merge `5a0dfc59` ("merge: m8 14 daily ops fixes (item 2 split off)") against its first parent `27330d46`.
3 commits (`9603139a`, `8db1fc99`, `f54435c3`), 47 files, +1117 −55. Read-only against that hash.
Reviewer: Opus, 2026-10-02.

**Counts: 0 Critical · 5 Major · 9 Minor.**

## Gates (run against `5a0dfc59`, own `CARGO_TARGET_DIR`, one at a time)

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` | **5218 passed, 9 skipped**, 601.8 s run (673 s with the 1 m 09 s build). Load average 24–32 the whole time, so the time is not a laptop number |
| `cargo test --doc` | pass, but **not evidence for `5a0dfc59`**: it ran at ~22:59. From 22:55 another agent's edits (ticket 14b's `preview/forks.rs` and 23 other files) were in the worktree the gates ran in. fmt, both clippy runs and the nextest build finished by 22:49:40, before the first edit, so those four are clean |
| Heavy tier, release perf run | **skipped** on the conductor's instruction (machine loaded) |
| Fuzz, benchmarks | not run (ticket touches neither) |

- Peak RSS of the nextest run: 1.36 GB (`/usr/bin/time -l`).
- The 5 slowest tests were not captured. The run was quiet, and no test passed nextest's 60 s SLOW mark.
- `target/` (the review's own): 5.0 GB, deleted at hand-back.

## Spot-check of the ticks

**Checked 5 of 5 ticked boxes. 4 held as written. 1 (J19) held in its behaviour but not in its headline count, which the tick discloses itself.**

| Tick | Test named | What I verified |
|---|---|---|
| J19 reword-head | `m7_walk/amend_into.rs` `reword_head_with_a_line_built_on_it`; ops `rewrite_run.rs` ×3 | Read: the editor opens on `HEAD`'s subject. Subject, tree, `feature-b~1 == feature-a` and `HEAD` home are all read with git; `u` → `for-each-ref refs/heads` equals the refs from before. The ops test's `state_of` also matches after `run_undo`. Holds. The count: see Minor 1 |
| `m`/`r` on a stash row | `preview/tests.rs` `m_and_r_on_a_stash_row_are_refused_in_words` | **Seen red** with the check disabled (`history.rs:104` → `if false && …`): panics at `tests.rs:917`. Holds |
| Put-back names the file | `edits_words.rs` …`named_on_the_landing_line`, `set_aside.rs` …`named_as_back_from_the_index` | Read: real git, `notes.md` back on disk, `Applied::came_back == [notes.md]`, `kept.md` not named. Holds for the paths it covers (see Major 3) |
| Equal-date ahead/behind | `equal_dates.rs` `ahead_behind_on_one_date_matches_rev_list`, unit `one_date_*` ×3 | **Seen red** with the old `count`: all three units fail (`ahead: 2, behind: 1` against `1, 1`), and the rev-list test fails on `main...merged` (`5 4` against git's `0 4`). Holds |
| `rebase: fast-forward` | `journal_list_oracle.rs` `a_rebase_that_fast_forwards_is_one_row` | **Seen red** with the old `is_rebase`: the list shows `$ rebase: fast-forward` ×2 and `$ rebase (start)` as separate rows. Holds |

**J19 tick: is it honest?** Yes in content, no in form. The box reads "= 3 (`a` edit `⏎`)", which the build does not do: the route is `a <edit> ⏎ y` = 4. But the same tick names the gap ("Count question (NOTES): route is … = 4; the walk allows the note's 3 + the preview's `y`"), names its tests, and the behaviour holds. Nothing was hidden, so this is **not a false tick**. Two things were wrong:
- The box should have stayed `[ ]` until the question was answered.
- The walk loosened the pipeline step-7 gate itself (`allowed + 1`) instead of failing and asking.

The note now says 4 (Pass 2c, :395), so the built route matches the note. The test is weaker than the note (Minor 1, gap row 1102).

## Feature pipeline

- J19 has a Need ("Two daily rulings", note :29), a Prior-art row (lazygit, `docs/research/stacked-branches.md` §1.9) and a Journey (now 4).
- The count in the walk: 4 (`a`, `<edit>`, `⏎`, `y`), equal to the note's current 4.
- Items 3–6 are fixes against rulings and bugs, not features. Item 4's wording is the ruling's own.

## Critical

None.

## Major

### 1. The reword mode carries into the next `c`: a commit becomes a reword of `HEAD` with its line
- **Where:**
  - `crates/graft/src/update/commit_editor/keys.rs:180`: `Esc` keeps the whole editor as the draft, `reword_line: true` included.
  - `crates/graft/src/update/commit_editor.rs:298`: `c` reuses that draft as it is.
  - `crates/graft/src/update/commit_editor/keys.rs:309`: `⏎` branches on `reword_line` before anything else.
- **Observed:** a throwaway probe test, not committed. State: `a.rs` staged, a reword editor open. Pressed `Esc`, then `c`, then `⏎`. Commands: `[PlanRewrite { request: RewordHead { message: "old subject\n" }, head: None }]`. No `Operate`. The editor `c` reopened has `amend=true reword_line=true`.
- **Why it matters:** `c` is "commit my staged changes". Here it plans a rewrite of `HEAD` and its dependents, and the staged file is not committed. `reword_line` describes the repository when `a` was pressed, and it goes stale with the draft: once `HEAD` moves or the line goes, the reword plan refuses `PlainAmend` and his text is gone. Nothing is written without `y`, so no data is lost, but it is the wrong operation.
- **Fix:** do not carry `reword_line` into a draft. Clear it in `close`, or don't keep a reword editor as a `c` draft. Add a test: `Esc` on a reword editor, then `c` `⏎`, gives `Operate(Commit)`.

### 2. `a` opens the editor for a reword the plan will refuse; his typed message is then lost
- **Where:** `crates/graft/src/jobs/rewrite.rs:103-118`.
  - The early return (`:103`) catches only `Dependents::Line` with no dependents. A `Fork` falls through to `amend_into`.
  - There, `NothingStagedFor` is the **first** refusal (`crates/graft-ops/src/rewrite/plan.rs:233`), checked before the line, merge and root checks.
  - `:113-118` maps it to `RewordHead` without asking whether a reword could run.
  - After the edit, `reword_head` (`plan.rs:293`) refuses `RootCommit`, `MergeInRange`, `ForkedDependents`, `DependentElsewhere` or `GitTooOld` (`line_of`, `rewrite/dependents.rs`).
  - `keys.rs:313` has already closed the editor (`state.commit_editor = None`) before the plan runs, and nothing keeps the text.
- **Why it matters:** in these cases he edits the message, presses `⏎`, gets a sticky refusal, and his text is gone. Before ticket 14 he got the refusal at `a`, before typing anything. The same loss happens on the plain path when he presses `Esc` in the reword preview (UX: recovery from errors; don't make him redo work).
- **Fix:** check whether the reword can run before answering `RewordHead`. In `jobs/rewrite.rs::plan`, plan `RewordHead` with `HEAD`'s current message, or run `line_of` and the root/merge checks, and answer its refusal at `a`. Also keep the typed text as the draft when the plan refuses or the preview closes, so `a` reopens it.

### 3. Put-back naming covers only the clean path, and its gaps are not in KNOWN_GAPS
- **Where:**
  - `StagingNotKept`: `crates/graft/src/update/operate/outcome.rs:297` builds the line from `paths` only. The `Applied` it is answered with already carries `came_back`, which `with_came_back` filled (`crates/graft-ops/src/apply/set_aside.rs:356`, its `AfterState` arm).
  - Git failed after `Back`: `set_aside.rs:367` (`Err(_) => {}`, commented) drops the paths.
  - The outcome line could not be written: the `ApplyError::Journal` branch of `close` (`set_aside.rs:310`) builds `kept` words without them.
- **Why it matters:** his ruling was "keep git's result and **say it**". On these three paths a file he deleted is back on disk and nothing says so: the symptom the ticket closes. The first is cheap to fix and was left. The builder's NOTES name two of the three, but rule 4 puts open items in KNOWN_GAPS, and there is no row (grep `came back`/`StagingNotKept` in `KNOWN_GAPS.md`: none).
- **Fix:**
  - Pass `applied.came_back` into `staging_not_kept_rungs` (the longest rung).
  - Carry `came_back` on `ApplyError::Git` (or on `EditsAt`) and append `came_back_words` in `explain`.
  - Name the files in the `Journal` branch's `kept`.
  - File a gap row for whatever stays open.

### 4. No capture of the new drawn states, and no words test for the reword box
- **Where:** `crates/graft/src/jobs/rewrite.rs:303` `reword_words`. It builds a new box (title `reword <sha7> "<old>"`, row `<sha7>  new message  <new>`, confirm `reword`) and the landing line `reworded "<new>"`. A grep of `crates/graft` finds no test asserting these strings. A grep of `docs/shots/` for `new message`, `came back from the index` and `stash isn't a branch` finds nothing.
- **Why it matters:** rule 6. The user judges new screens by eye, and these three states are new to the screen. The walk asserts only `state.preview.is_some()` (`tests/it/m7_walk/amend_into.rs:379`), so J19's "(preview: the branch follows)" is not shown to be on screen. (The title is prefixed `Preview: ` and sanitised by `update/preview/boxes.rs:26`, as the put-in's is: no finding there.)
- **Fix:** add the reword preview and the came-back landing line to the walk's captures (`docs/shots/m8/…`, ≤ 200 KB), with what-right-looks-like lines. Add a unit test pinning `reword_words`.

### 5. Readability: one file pushed past 400 lines, two already over it grew (PROMPT.md 6.2)
- **Where:**
  - `crates/graft-ops/src/apply/set_aside.rs`: 385 → 441, no tests in the file. Crossed in this ticket.
  - `crates/graft/src/jobs/rewrite.rs`: 458 → 494 non-test lines.
  - `crates/graft/src/update/amend_into.rs`: 431 → 467 non-test lines.
- **Fix:**
  - `read_came_back`, `came_back_or_unread` and `with_came_back` belong in the new `set_aside/came_back.rs`, which is about them.
  - `reword_words` and `REWORD` go with the other box words: a `jobs/rewrite/words.rs` taking `amend_into_words`, `absorb_words` and `stays_words` too.

## Minor

1. **J19 walk tolerates one step more than the note.** `crates/graft/tests/it/m7_walk/amend_into.rs:383` asserts `route.len() <= allowed + 1`. With the fence now at 4, that allows 5. Assert `route.len() <= m8_count("reword-head")` and drop the +1 comment (`:351-353`). Tracked: `KNOWN_GAPS.md:1102`.
2. **The editor's options are silently dropped on the reword path.** `keys.rs:309-316` passes only `editor.message()`, so `Ctrl+S` sign-off (`keys.rs:92`), no-verify, allow-empty and the amend toggle do nothing. The title still reads `AMEND <sha>` (`crates/graft/src/view/commit_editor.rs:72`), and nothing tells him that `⏎` opens a preview and his line follows (UX: visibility of the next step). Hide the options he can't use, or honour them; title it `REWORD <sha> · <n> branches follow`.
3. **`reword_head` is written for many rows but only ever has one.** `crates/graft-ops/src/rewrite/plan.rs:313-318` marks every row `Reword` and copies the message into every row. Merges are refused, so the range `HEAD^..HEAD` is exactly one commit, and the doc says "One `Reword` row". Build the one row directly, or `debug_assert_eq!(todo.len(), 1)`, so a reader doesn't wonder whether several commits get one message.
4. **The module doc on cost is stale.** `crates/graft-git/src/history/divergence.rs:9-10` ("the cost is the size of the two sides' difference plus a little"). The walk now runs on through every commit sharing the floor's date. In a history imported with one date, that is the whole history, per diverged branch. Say so in the doc. It is not measured here: the synthetic fixtures use distinct dates (`graft-fixtures/src/stream.rs:91`).
5. **`equal_dates.rs:14-34` copies `session::git` and drops part of it.** It leaves out the removal of `GIT_COMMON_DIR` and `GIT_OBJECT_DIRECTORY` and the `GIT_TERMINAL_PROMPT=0` that `crates/graft-git/tests/it/session.rs:21-38` sets. Build on `session::git` and add the two date variables.
6. **Edits outside the file set: judged acceptable.**
   - All were enumerated by the compiler and disclosed in NOTES.
   - `restore.rs` `opening_of` is a pure extraction; I checked the fields one by one.
   - The `Applied` literals: `came_back: Vec::new()` in ~15 places.
   - `rewrite_open.rs:118` sends `RewordHead`, which never reaches `R`'s editor, through `base_label`. The put-in job answers the same kind of case with an explicit error (`jobs/rewrite.rs:197`, "R is not a put-in"). Make it an error here too.
   - Why so many sites changed: `Applied` is built by struct literal. An `Applied::new(entry, refresh)` would have kept this to one line, and to the ticket's own files.
7. **Redo of a reword is untested.** The ticket's Touches lists "journal entry shape for a reword (undo/redo of it)". `rewrite_run.rs` `reword_head_rewords_moves_the_line_and_undoes_as_one` covers undo only.
8. **The came-back test was never seen red.** The ticket says so. A quick red check: `with_came_back` with `paths` ignored should fail `a_staged_new_file_deleted_from_disk_is_named_as_back_from_the_index`'s `applied.came_back` assert.
9. **Question, no defect claimed:** is `rebase: ` too broad a prefix? `crates/graft-ops/src/journal_list/steps.rs:100-104` now folds any `rebase: …` record. Git 2.54's `--apply` backend writes `rebase (pick): …` (checked here), so I found no modern record it would wrongly fold. Reflogs written by older git may differ. Not verified.

## Process note (not a finding against the build)

Ticket 14b's builder is editing in worktree `.claude/worktrees/agent-aa4abba36c4d3ab03`. I had switched that worktree to branch `review/m8-14` at `5a0dfc59` (~22:46), before its first edit (22:55). So:
- 14b's work sits on `5a0dfc59`, not on `2e8e77ce`.
- Its commits will land on branch `review/m8-14`.

I did not touch its files, and wrote this review from a separate worktree (branch `review/m8-14-daily-ops`).

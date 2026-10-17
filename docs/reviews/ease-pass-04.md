# Review: ease-pass group 04 — commit-what-goes-in

Range `ea7206cb^1..ea7206cb` (merge of `0e670f64`), graft-ops plus compile
fixes in graft. Ticket `.scratch/ease-pass/issues/04-commit-what-goes-in.md`,
scout `.scratch/ease-pass/scouts/04-commit-what-goes-in.md`. Reviewed
read-only against `ea7206cb`. All line numbers are at `ea7206cb`.

**Verdict:** the write path is careful. On a single process the real index
and working tree survive every failure I traced: a failed copy, a failed hook,
or a failed step-4 reset (that last one has a test, `u` included). The copy
is dropped on every path. No Critical findings. **Two Major.** The first: the
scratch index has a fixed name, so two windows on one worktree can corrupt each
other's commit. The second is readability.

## Gates

**Not run by me.** The conductor said not to run cargo: it is running the
gates itself, and two builds are using the CPU. The builder reports workspace
nextest 6511/6511. I have not verified that. There are no heavy-tier,
release-perf, fuzz or memory numbers from me for this group. The conductor's
run is the record.

## Feature pipeline

- Design note `docs/design-notes/ease-pass.md` has **Need** (`:20-26`),
  **Prior art** (rows "Commit start", "In vs out", "Secrets file", citing
  `docs/research/commit-stash.md` §1.4–1.5, which exists) and **Journeys**
  (`commit-all-subject 3`, `commit-with-body 6`, `commit-leave-first 6`).
- This group is the ops backend only. No screen, no keys. You can't count journey
  steps here; the commit screen group (S) owns the walk.

## Ticks spot-checked

**9 checked, 9 held.** I read each one against its named test, but did not
run them (no cargo):

| Tick | Test read | Held |
|---|---|---|
| first_choice cases | `choice.rs` `nothing_staged_…`, `something_staged_…`, `a_partially_staged_…` | yes |
| looks_like_secrets table | `looks_like_secrets_table` (`.env.example` false, `id_rsa.pub` false, `ID_RSA` true) | yes |
| Whole / StagedPart / out file's `ls-files -s` line | `commit_chosen.rs:108-148` (`staged_line` = `ls-files -s -- <path>`) | yes |
| deleted + untracked, status clean | `commit_chosen.rs:150-172` | yes |
| Chosen == staged → one line | `tests/chosen.rs:49-56` (asserts `IndexFile::Real` too) | yes |
| `u` after chosen commit | `commit_chosen.rs:174-190` (`state_of`: refs, `ls-files -s`, files) | yes |
| amend chosen / amend staged rewords | `commit_chosen.rs:192-241` | yes |
| NothingChosen + failing hook | `tests/chosen.rs:136-173`, `commit_chosen.rs:243-257` | yes |
| `command_line` leads `GIT_INDEX_FILE=` | `tests/chosen.rs:119-133` | yes; "existing tests unchanged" taken on the builder's word |

## Data-loss walk (what I traced)

- **Failed copy** (`run.rs:222-230`): returns before any git runs. The real
  index has not been touched. A partly written copy is removed by
  `TempIndex`'s `Drop` (`plumbing.rs:73-76`: `temp` exists before
  `io::copy`/`set_modified`). Holds.
- **Hook fails at step 3**: only the scratch copy was written.
  `run_plan` removes it (`run.rs:179-185`). Has a test
  (`commit_chosen.rs:243`). Holds.
- **Step 4 fails after the commit** (`commit.rs:121`): the entry is closed
  `Aborted` and `u` restores everything (`commit_chosen.rs:342-368`). Holds.
  Wording: see Minor 3.
- **Every executor honours `index`**: plan lines run only through
  `run_plan` (`apply.rs`, `fold.rs:148`, `pull.rs:88-154`). The engine takes
  only a Rewrite's first line (`engine.rs:242`), and Rewrite never makes
  `Scratch` lines. So no path runs steps 1–2 on the real index. Checked by
  grep for `plan_git`/`run_plan(`/`.plan(`.
- **Path is absolute**: `ctx.scratch()` comes from `git_path`, which
  makes the path absolute (`graft-git/src/repo/mod.rs:221-227`). The
  in-process copy and git's `GIT_INDEX_FILE` therefore name the same file
  whatever the process cwd.
- **Crash between steps 3 and 4**: unverified, as the builder says. Minor 2.

## Critical

None.

## Major

### 1. One fixed scratch name, shared by every process on the worktree

`crates/graft-ops/src/apply/run.rs:194` (`SCRATCH_INDEX = "commit-index"`),
`:222` (copy to it), `:181` (remove it); `snapshot/plumbing.rs:49`
(`copied_as`).

- **What's wrong.** Two processes writing to one worktree at once is a
  supported case. Writers take the journal lock *shared*
  (`journal/file.rs:14-17`). Recovery's own comment names "an operation
  running in another job or another window" (`apply/recover.rs:32`, `:42-44`). Every
  one of those processes copies to, runs on and removes the same
  `<names dir>/commit-index`. The copy and the removal are plain `fs` calls,
  not git lockfiles, so nothing serialises them. One interleaving: window A
  finishes and removes the file (`run.rs:181`) while window B is between step 1
  and step 3. B's `git commit` then runs with `GIT_INDEX_FILE` pointing at a
  missing file. The code itself says git reads a missing index as the empty
  index (`plumbing.rs:61-62`), so B commits a tree holding only its `add -A`
  paths: **every other tracked file is deleted in that commit**, and the
  entry closes `Committed`. In a second interleaving, A's `File::create`
  (`plumbing.rs:74`) truncates B's copy, and B commits A's choice. `u` can take
  either back, because the backup is of the real index. But nothing tells him
  the commit is wrong, and he may push it.
- **Why it matters.** "You cannot lose work" (M3), and a wrong commit that
  looks right. The snapshot code avoided this on purpose: "a name no other
  snapshot in this process or any other can take" (`plumbing.rs:24-25`). This
  group gives that up. The ticket's Scope asked for "one scratch file", so the
  spec carries the cause as much as the build does.
- **Not reproduced.** I worked this out from the code and did not run it. It
  needs two processes inside one commit's window, which is narrow but not
  zero: a hook that runs for seconds widens it.
- **Fix (cause).** Name the copy per process: `commit-index-<pid>-<n>`, as
  `TempIndex::unique_name` already does. Compute the name once per `apply` and
  pass it to both `logged_line` and `run_plan`, so the Command Log still
  names the real path. Then sweep `commit-index-*` leftovers whose pid is dead
  in `recover_interrupted` (this also closes Minor 4). If one name must stay,
  hold an exclusive `FileLock` on a sibling such as `commit-index.mwr-lock`
  (not `commit-index.lock`, which is git's) for the whole plan. The cost of the
  lock: the second window waits or refuses. That is fine for a rare race.

### 2. Readability: clever code where plain code would do (PROMPT.md 6.2)

- `crates/graft-ops/src/snapshot/plumbing.rs:103-115` `TempIndex::remove`:
  `mem::take` the path, then `mem::forget(self)` so `Drop` does not try
  `remove_file("")`. It is correct, but a reader has to work out three
  things: why `forget`, why the empty path leaks nothing, and that `Drop`
  would otherwise print a bogus error. **Fix:** `path: Option<PathBuf>`
  (`Drop` removes only `Some`), and `remove` does `self.path.take()`. That
  needs no `forget` and no comment explaining one.
- `crates/graft-ops/src/operation/types.rs:203-211` `command_line_in`:
  it quotes one path by building a fake `git <path>` line and stripping
  `"git "`, and falls back silently to the unstripped line if the prefix is
  ever missing. **Fix:** expose graft-git's shell-word quoting as its own
  function, `graft_git::run::quote_word(&[u8]) -> String`. That is a
  graft-git seam, so it goes through the conductor.
- `crates/graft-ops/src/apply/run.rs:211-217` `run_lines`: five parameters,
  two of them `&mut` out-parameters (`scratch`, `cleanup`). Then `:233` checks
  `scratch.as_ref()` right after making sure it is `Some`. **Fix:** a small
  `ScratchIndex` holder (lazily copied, `.env_for(git)`, `.finish(&mut
  cleanup)`) owned by `run_plan`, or return `(Option<PlanFailure>,
  Option<TempIndex>)`.

## Minor

1. **A latent swallowed cleanup** (rule 3) at `crates/graft-ops/src/apply/fold.rs:148`:
   `let PlanRun { failure, .. } = run_plan(…)` throws `cleanup` away. An amend
   with `Include::Chosen` at an `edit` stop folds here (`fold.rs:108-111`;
   `check.rs` admits it through `amends_at_edit_stop`). A failed scratch
   removal there would be dropped silently. Today the only edit-stop amend is
   `Include::Staged` (`graft/src/update/commit_editor.rs:341-344`), so nothing
   can reach it yet. **Fix:** carry `cleanup` into the folded `Applied` or its
   `unread` before S wires a choice into the edit-stop amend.
2. **Unverified paths have no `KNOWN_GAPS.md` row** (rule 4). The builder's
   NOTES mark two unverified items: a crash between step 3 and step 4, and
   `ScratchIndexLeft`. `KNOWN_GAPS.md` at `ea7206cb` has neither. The
   conductor should add them.
   - **Crash, as a question (rule 7).** HEAD would have moved but the real
     index would not have been reset (`commit.rs:121` never ran). Each chosen
     path would then read as staged *back to its old content*. If he then
     opens the commit screen, `first_choice` would put those paths in as
     `StagedPart` (`choice.rs:41-48`, "something staged"), and the next
     commit would undo his change in history. Nothing is lost on disk; this
     is reasoned, not observed.
   - **A fixture route for both.** A `post-commit` hook that replaces
     `<names dir>/commit-index` with a non-empty directory (`rm -f X; mkdir
     X; touch X/k`) makes `remove_file` fail, which reaches `ScratchIndexLeft`.
     The next chosen commit's `File::create` on that directory then fails the
     copy, which exercises Minor 3's wording too. Nothing else in the names
     dir is touched.
3. **The words are wrong at the symptom level** at `crates/graft-ops/src/apply/run.rs:227-229`. A
   failed copy is reported as `RunError::Spawn`, so the entry's reason reads
   "could not run git: the index could not be copied…". Git ran fine; the
   copy failed. The builder flagged this. **Fix:** a typed `PlanFailure`
   source, such as `RunError::Io` if graft-git has one or a new
   `ApplyError` arm, worded "could not copy the index for the commit: …". It
   is also untested; see the route in Minor 2.
4. **A leftover file stays silently after a crash.** If the process dies
   mid-plan, `commit-index` (index-sized, MBs on a big repo) stays until the
   next chosen commit overwrites it. `recover_interrupted` doesn't look for it.
   **Fix:** sweep it at recovery (it folds into Major 1's fix).
5. **Question (rule 7): what does he read when step 4 fails after the
   commit landed?** `commit_chosen.rs:342-358` asserts the entry is `Aborted`
   with git's `index.lock` words while HEAD has moved. I did not read the app's
   rendering of `ApplyError::Git` for a commit. Does it say the commit *landed*
   and that `u` takes it back, or does it read as "commit failed"? If it reads
   as failed, a retry commits the stale staged content (see Minor 2). Check
   when S builds the landing (C13).
6. **The size ratchet was raised for production files**
   (`crates/graft/tests/it/over_400.txt`: `apply/error.rs` 560→570,
   `operation/refusal.rs` 422→425, plus six test files). The file header says
   rows are lowered as files shrink. **Fix:** move `ScratchIndexLeft`'s
   neighbours, or the `AfterState` enum, into its own module, or record the
   raise as a decision.
7. **Gaps in fixture coverage.** The unborn-branch choice
   (`rm --cached -f`, then step 4 on the first commit) is plan-tested only
   (`tests/chosen.rs:101-116`). `Chosen([])` with `allow_empty` and something
   staged (held reset + `--allow-empty`) has no fixture. **Fix:** add one
   fixture for each to `commit_chosen.rs`.

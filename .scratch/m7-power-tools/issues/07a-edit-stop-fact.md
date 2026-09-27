# 07a — part of 07 rewrite-stops (split by planner, filed by conductor 2026-09-27)

Status: ready. Shared context (settled scout findings, order, Decided, Signatures) is `07-rewrite-stops.md` lines 1-118; read it first. This file holds only part 07a plus Touches and Rules.

## 07a — the edit-stop fact and P14's exception (graft-ops)
Build with: **Opus**. Small, but it verifies git's behaviour first and chooses between a rule and its fallback.
Needs: `rewrite::progress` (02). Blocked by: 03b (holds `operation.rs`), which is after 10b and 11a.
Files: `crates/graft-ops/src/operation.rs` (`RepoFacts` :312, `check_conflict_state` :1435-1446 only);
`crates/graft-ops/src/facts.rs` (MAP `## facts`: `read_facts` fills `edit_stop`; if the module is another file, name
it in NOTES); `crates/graft-ops/src/rewrite.rs` (`EditStop`); `crates/graft-ops/src/rewrite/progress.rs`
(`edit_stop`); `crates/graft-ops/tests/it/edit_stop.rs` (**new**; the conductor adds the `mod` line in the seam
commit); every `RepoFacts {` literal in graft-ops, literal-only (`edit_stop: None`); `crates/graft-ops/MAP.md`.
Conditional: `crates/graft-ops/src/apply/engine.rs` only if check 2 below fails. A `RepoFacts {` literal in graft →
grep first; literal-only edits there are granted **unless** the file is one 09 holds while 09 runs → `blocked`.

Steps, in order:
1. **First test, against real git:** `git_writes_rebase_merge_amend_at_an_edit_stop` — a rebase with one `edit` row;
   at the stop, `rebase-merge/amend` exists and holds HEAD's oid; after `--continue`, gone. Record git's version.
   - **Holds →** `edit_stop` = `Some(EditStop { stopped: <amend's oid> })` iff that file exists and the last pick-like
     line of `rebase-merge/done` is `edit` (rules out any other writer of the file).
   - **Doesn't →** fallback: `edit_stop` = `Some` iff the last pick-like `done` line is `edit`, `stopped` = HEAD at
     read time; and the check drops the parent test (amend allowed at any edit stop — `git commit --amend`'s own
     shell behaviour). Say which branch was taken in NOTES; the conductor tells the planner.
2. **Second check:** `an_edit_stop_comes_back_as_a_stop` — `apply` of a rewrite plan with an `edit` row returns the
   outcome variant a conflict stop returns (the one graft's `operated` `Conflicted` arm takes), with no paths. If it
   comes back as a finish, fix it in `apply/engine.rs` (`PlanRun.edit_stop`, `:121`): the stop must reach the app.
3. The exception in `check_conflict_state`, then `read_facts`.

Acceptance:
- [x] the two tests above — **rule branch taken** (git holds): `edit_stop::git_writes_rebase_merge_amend_at_an_edit_stop`
  (git 2.54.0 (Apple Git-157): `amend` = HEAD at the stop, unchanged by an amend, gone after `--continue`);
  `edit_stop::an_edit_stop_comes_back_as_a_stop` (`ApplyError::Conflicted`, no paths; already so — no `engine.rs` edit);
- [x] `operation::tests::amend_is_allowed_only_at_an_edit_stop_on_the_stopped_commit` (pure table, incl. merge /
  cherry-pick / revert, conflict stop, commit on top) + against real repos
  `edit_stop::amend_passes_check_at_a_real_edit_stop_until_a_commit_lands_on_top` (HEAD = stopped, amended once,
  commit on top) and `edit_stop::edit_stop_reads_the_stopped_commit_and_whether_head_is_still_on_it`;
- [x] `edit_stop::edit_stop_is_none_outside_a_rebase`, `edit_stop::edit_stop_is_none_at_a_conflict_stop`;
- [x] existing amend-refusal tests unchanged and green: `operation::tests::mid_rebase_stage_and_unstage_are_allowed_and_the_rest_wait`,
  `every_kind_passes_the_baseline`, `rewrite_run::an_edit_row_stops_and_the_resume_takes_the_line_home`;
- [x] MAP.md graft-ops (`EditStop`, `edit_stop`, `RepoFacts.edit_stop`, `read_facts`, `Operation::check` P14 exception).

Status: done (2026-09-27).

## NOTES (builder, 07a)
- Scout: used `scouts/07.md`; no new scout (small, lines confirmed by hand). Lines after 03b: `RepoFacts` :307
  (was :312), `check_conflict_state` :1481 (was :1435). `read_facts` is in `facts.rs` as MAP says.
- **Signature deviation:** `EditStop { stopped: Oid, head_on_it: bool }`. `check` is pure over `RepoFacts`, and the
  rule needs HEAD^1 and stopped^1, which no fact holds; `edit_stop` computes the comparison at read time (one
  `rev-list --no-walk=unsorted --parents HEAD <stopped>` spawn, only at an edit stop). `edit_stop(repo: &RepoHandle,
  stop: &dyn Stop) -> Result<_, FactsError>` — `progress`'s own parameter/error types (the spec said `Git`,
  `StopFlag`, `RewriteError`; none exist on `progress`).
- Exception also requires `in_progress == Some(Rebase)` (defensive; a merge with an edit fact cannot occur).
- **Finding for 07b / conductor (probed, not kept as a test):** `apply` of `Operation::Amend` at an edit stop in a
  Graft-begun rewrite now **folds into the live rewrite span** (`apply/fold.rs:40` `folded_step`: `in_progress` is
  set and amend's `touches.worktree` is false) — `Applied` came back with kind `Amend`, `Committed`, but the journal
  held only the `Rewrite` `Pending` entry. So the Touches line "the edit-stop amend is an ordinary `Amend` entry" is
  **not** what happens; `u` after the finish would take the amend back with the whole rewrite. Either is defensible;
  a decision for 07b's owner. Also `fold.rs:63` doc ("`check` admits Stage, Unstage and the two conflict
  operations") is now stale — not my file.
- App behaviour unchanged by 07a: graft's `a` refuses a live op in its own layer (`update/commit_editor.rs:133`
  `operation_refusal`) before `check` is asked. Unverified end-to-end; 07b wires it.
- Touches: `RepoFacts {` literals — graft-ops only (4 test fixtures, `edit_stop: None`); `graft/src/map.rs:249` and
  `view/radar.rs:46` are `mistake_radar::RepoFacts`, a different struct (checked by reading). P14 for merge /
  cherry-pick / revert / conflict stop: pinned by the table and the unchanged tests above. `read_facts` cost: one
  extra file read mid-rebase only; none otherwise. Mutation check: disabling the exception turned the table test
  and the real-repo check test red.
- Cause vs symptom: new feature, not a fix. `esc_never_aborts` is 07b's.
- No compaction.
Size: ~20k.


## Touches (each part checks its own, reports how)
- **M6 banner and resolver** for merge, pull, stash clash: words and focus unchanged. Check: every m6 banner/resolver
  test and capture byte-identical (`check_shot`), or each change listed in NOTES. (07b, 07c)
- **Footer `stop_wordings`:** unchanged. Check: its tests. (07b)
- **`u`/`U` refusal while live:** unchanged; the amend at an edit stop is refused for `u` like any live step. Check:
  its test. (07b)
- **P14 for merge / cherry-pick / revert and conflict stops:** unchanged. Check: existing amend-refusal tests. (07a, 07b)
- **`resume_target` / `live_entries`:** an amend entry landing inside a live rewrite must not hide the rewrite's
  `Pending` entry. Check: `c_after_an_amend_at_an_edit_stop_continues_the_rewrite`. (07b)
- **`pane_c_acts`** (one place; `commit_key` and `view::rows::live_c` read it): now true for shell-begun rebase and
  cherry-pick. Check: `live_c_tests.rs`, `shell_begun.rs`. (07c)
- **Relaunch after kill -9:** reads the saved plan through `progress`. Check: `a_relaunch_mid_rewrite_shows_the_chain`. (07b)
- **Keymap:** §2.6a already amended; no doc edit here. Check: hint-bar tests.
- **Journal/undo:** the edit-stop amend is an ordinary `Amend` entry; the shell hand-over writes none (git's
  operation, begun outside, as M6). Nothing else new.
- **KNOWN_GAPS :545:** closed by the conductor (settled 1), not by a builder.

## Rules / Gates
As rewrite-engine. Workspace-wide gates. Each part commits at its seam.

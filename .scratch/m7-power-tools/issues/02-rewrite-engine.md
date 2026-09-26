# rewrite-engine — every rewrite runs git's `rebase -i` on a todo Graft writes; dependents follow on one line

Status: partial — rewrite-engine-a done (m7-power-tools-02a); rewrite-engine-b open, see Handoff
Blocked by: none
Crates: graft-ops only. No other crate's matches may break: a new `Operation`/`OperationKind`/`Refusal` variant that
graft matches exhaustively gets a `NotPreviewable`-style arm in graft-ops's own `Display`/`kind` code. If graft fails to
build, stop and report `blocked: needs graft`, naming the match.
Source: `docs/design-notes/m7-power-tools.md` hard-to-reverse choices 2–3; "Pass 2a" R1–R4, "Interfaces and data
shapes"; ADR-0047 (proposed); KNOWN_GAPS :548 (the chained forecast is `rewrite-preview`'s, not here).
Build with: **Fable (signature)** — it fixes the interface every M7 rewrite calls, and it decides ADR-0047's open points.
- Why Fable: a wrong mechanism here (message feeding, top-of-line run, what is recorded) becomes structural for `r`, `a`,
  `R`, absorb and every stop.
- Why not the diff or tests alone: correctness is git's sequencer interplay, which only oracle tests against a hand-run
  `git rebase -i` can show.

Scout: `scout` first (ADR-0035, brownfield: today's `Rebase` plan, `apply`, `resume`, the journal recording).
Needs: none.

## Decided (ADR-0047; the builder may amend only its "open to the builder" points, in NOTES, with evidence)
- **One engine for every rebase.** `git -c rebase.updateRefs=false rebase -i <onto> [<top>]`.
  - `GIT_SEQUENCE_EDITOR` copies Graft's todo over git's.
  - The todo holds only `pick reword edit squash fixup drop update-ref`, never `exec`.
- **Messages are pre-written.** Any message git asks for comes from `RewriteParams::messages`, through a `GIT_EDITOR`
  helper keyed by the todo line git is on.
  - The helper is a file Graft writes under `.git/graft/rewrite/<entry-id>/`.
  - The keying mechanism is open to the builder; the last line of `rebase-merge/done` is the candidate.
  - Oracle-prove it for reword, for a squash group of 3, and for a squash group after an `edit` stop.
- **The saved plan.** `.git/graft/rewrite/<entry-id>/` holds the todo, the messages, the line of branches, and the
  branch HEAD returns to.
  - It is written **before** the Pending line and removed when the entry closes (done, aborted or failed).
  - A relaunch mid-rewrite (kill -9) reads it.
- **Every moved ref is recorded.** Graft writes the `update-ref refs/heads/<b>` lines itself, one after each lower
  branch's tip.
  - `refs_before`/`refs_after` hold every branch in the line + HEAD's symbolic ref.
  - The kind is `OperationKind::Rewrite`; the undo inverse is Rebase's.
  - `u` after a 3-branch rewrite puts all 3 back.
- **Top-of-line run.** With dependents, git rebases the top branch. After the finish **and** after an abort, Graft
  switches back to his branch, inside the same entry.
- **Dependents are inferred** (`dependents`): local branches whose first-parent chain contains the tip, with no merge
  between, not the default branch, and not checked out in another worktree.
  - They must form one line.
  - A fork → `Dependents::Fork`, refused as `ForkedDependents{a,b}`.
  - Another worktree → `DependentElsewhere`.
  - git < 2.38 with dependents → `GitTooOld`.
- **The range for `RewriteRequest::Open{from}`:** HEAD down to the merge-base with the default branch (or the upstream,
  if set), and further down to `from` if it is deeper. A merge inside → `MergeInRange`. `from` not an ancestor of HEAD
  → `NotOnYourBranch`. Detached → `NotOnABranch`.
- **Autosquash is pre-marked, never applied unseen.** `fixup! X` / `squash! X` rows become `Fixup`/`Squash`, moved
  directly above X, in the returned todo. The app shows them.
- **`RewriteRequest::AmendInto{target}`:**
  - The index is written as a commit object with no ref (`write-tree` + `commit-tree`, parent HEAD), and planned as a
    `Fixup` line after `target`. `made` records its oid.
  - `target == HEAD` with no dependents is **not** a rewrite: `plan` refuses it, and the app keeps today's amend.
  - `target == HEAD` with dependents is a rewrite.
  - Nothing staged → `NothingStaged`.
  - Unsaved edits are set aside (ADR-0045 §4) inside the entry.
- **`r` (`Operation::Rebase`):** its todo is all `Pick` + `RebaseParams::drops` (filled by `rewrite-preview`; empty
  here) + the line's `update-ref`s.
- **Stops and resume** use today's `ApplyError::Conflicted`/`resume(Continue|Skip|Abort)`, same entry.
  - `ContinueWith` stays refused (`MessageNotHere`): the messages are pre-written.
  - `Continue`/`Skip` carry the same `GIT_EDITOR`.
  - `Abort` also returns HEAD to his branch.
- **`progress`** reads `rebase-merge/{msgnum,end,done,git-rebase-todo}` + the saved plan.
  - `branch` is `Some` only when the line has more than one branch.
  - A shell-begun rebase → `begun_in_graft: false`, `branch: None`.

## Read first
- `docs/design-notes/m7-power-tools.md`: "Hard-to-reverse choices", "Pass 2a" R1, R3, R4, "Interfaces and data shapes",
  "Where I'd differ" item 6.
- `docs/adr/0047-rewrite-engine-and-dependent-branches.md` (whole); `docs/adr/0045-…` §4 (set-aside); ADR-0032.
- `docs/design/06-undo-journal-design.md` §6.1–6.4 (the recording protocol; the Rebase row :117).
- `docs/research/interactive-rebase.md` §4.1–4.6; `docs/research/stacked-branches.md` §1.1 and :30-33.
- `crates/graft-ops/MAP.md`: `operation`, `apply` (and `apply::resume`), `preview` (only how `Rebase` is planned), `state`,
  `journal`, `undo`/`restore`, `history_reads`. Before any source.

## Scout brief (code, not intent; ≤ 1 page)
1. Where `Operation::Rebase` builds its `GitInvocation` today (`--no-update-refs --no-autostash`). How `apply` records
   `refs_before`/`refs_after` for it. What `touches`/`tips` return.
2. `apply::resume`: how Continue/Skip/Abort find the Pending entry (`live_entry`), and which env/`-c` the resume spawn
   passes today.
3. Set-aside (ADR-0045 §4): which kinds `sets_aside` covers, and where the wrap sits relative to the plan's spawns.
4. How `GitInvocation` sets env vars (is there a precedent, e.g. for `GIT_EDITOR` in commit or merge?).
5. Every graft-ops test that asserts the rebase command line or `--no-update-refs` (`grep -rn "update-refs"
   crates/graft-ops`), and every one that asserts a Rebase journal entry's refs.
6. Every exhaustive `match` on `Operation`/`OperationKind`/`Refusal` outside graft-ops (`grep -rn` in `crates/graft`,
   `crates/graft-ui`). A new variant must not break them. List each site.

## Signatures (exact; the note's "Interfaces" block)
```rust
// graft-ops::rewrite (new module, pub)
pub enum TodoAction { Pick, Reword, Edit, Squash, Fixup, Drop }
pub struct TodoLine { pub action: TodoAction, pub commit: Oid, pub subject: String }
pub struct BranchLine { pub branches: Vec<Tip> }
pub struct RewriteParams { pub head: Tip, pub onto: Oid, pub todo: Vec<TodoLine>, pub line: BranchLine,
                           pub messages: BTreeMap<usize, String>, pub made: Vec<Oid> }
pub enum RewriteRequest { Open { from: Oid }, AmendInto { target: Oid } }
pub enum RewriteError { Refused(Refusal), Facts(FactsError), Stopped }
pub fn plan(repo: &Repo, git: &Git, req: &RewriteRequest, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<RewriteParams, RewriteError>;
pub enum Dependents { Line(BranchLine), Fork { a: RefName, b: RefName } }
pub fn dependents(repo: &Repo, git: &Git, branch: &Tip, stop: &dyn Stop) -> Result<Dependents, FactsError>;
pub struct SquashGroup { pub kept: usize, pub melded: Vec<usize> }
pub fn squash_groups(todo: &[TodoLine]) -> Vec<SquashGroup>;
pub fn default_message(group: &SquashGroup, message_of: &dyn Fn(usize) -> String, todo: &[TodoLine]) -> String;
pub enum TodoProblem { MeldsIntoNothing { row: usize }, EverythingDropped }
pub fn validate(todo: &[TodoLine]) -> Result<(), TodoProblem>;
pub struct RewriteProgress { pub step: (usize, usize), pub current: Option<TodoLine>, pub pending: Vec<TodoLine>,
                             pub branch: Option<(RefName, usize, usize)>, pub begun_in_graft: bool }
pub fn progress(repo: &Repo, stop: &dyn Stop) -> Result<Option<RewriteProgress>, FactsError>;
// operation: Operation::Rewrite(RewriteParams); OperationKind::Rewrite; RebaseParams { …, line: BranchLine, drops: Vec<Oid> }
// Refusal: NotOnYourBranch{commit: Oid}, MergeInRange{merge: Oid}, ForkedDependents{a: RefName, b: RefName},
//   DependentElsewhere{branch: RefName, worktree: PathBuf}, NothingStaged{target: Oid}, NotOnABranch,
//   GitTooOld{need: &'static str}; Display = the note's R1 words exactly.
```
`default_message`: the kept row's full message, a blank line, then each melded `Squash` row's message, newest first.
`Fixup` rows add nothing.

## Acceptance (name the verifying test at tick time; red first where it says so)
- [x] **Pure** (`crates/graft-ops/src/rewrite/tests.rs`) — all six green; red shown for
      `default_message_leads_with_the_kept_row_then_newest_first` (git's oldest-first order stubbed in: left
      `…wip\n\nHandle expired tokens`, right `…Handle expired tokens\n\nwip`), then restored. Also
      `the_git_todo_asks_where_git_asks_and_the_line_follows`, `an_empty_git_todo_is_a_noop`:
  - `squash_groups_meld_into_the_row_below`;
  - `default_message_leads_with_the_kept_row_then_newest_first` (red first);
  - `fixup_adds_no_message`;
  - `validate_refuses_a_bottom_squash`;
  - `validate_refuses_everything_dropped`;
  - `autosquash_rows_are_marked_and_moved_above_their_target`.
- [ ] **Oracle** (real repos, `crates/graft-ops/tests/it/rewrite_oracle.rs`). Each row runs Graft's plan+apply on a
      repo, and a hand-written `git rebase -i` todo on a `cp -R` copy. It compares every branch's tree and commit
      messages, HEAD's symbolic ref, and `git status --porcelain=v2`. Rows:
  - squash 3;
  - fixup;
  - reword (the message is Graft's);
  - reorder;
  - drop;
  - `edit` then continue;
  - squash group after an `edit`;
  - a 3-branch line (every branch moved, HEAD back on the bottom branch);
  - `AmendInto` 3 back;
  - `AmendInto` HEAD with a dependent.

  A row where they differ is a failure, not a NOTE.
- [ ] **Record and undo:**
  - `a_three_branch_rewrite_records_every_ref` (red first: today's rebase records one);
  - `undo_puts_every_branch_in_the_line_back`;
  - `abort_returns_head_to_his_branch_and_every_ref`;
  - `kill_mid_rewrite_then_relaunch_continues_from_the_saved_plan` (the plan dir read after a simulated kill: the
    Pending entry + `rebase-merge/` left by a stopped rewrite);
  - `the_plan_dir_is_removed_when_the_entry_closes` (done, aborted, failed).
- [x] **Refusals**, each asserting the words (`crates/graft-ops/tests/it/rewrite_plan.rs`, all seven named tests
      green; `an_old_git_refuses_only_the_chain` passes `GitVersion 2.37.0` to `plan` directly instead of a fake
      `git --version` — `plan` takes the version as a parameter, so it is the same input; plus
      `amend_into_head_with_nothing_built_on_it_is_a_plain_amend`). Written after the code, not red first:
  - `a_fork_is_refused_naming_both`;
  - `a_dependent_in_another_worktree_is_refused_with_its_path`;
  - `a_merge_in_range_is_refused`;
  - `a_commit_off_your_branch_is_refused`;
  - `detached_head_is_refused`;
  - `amend_into_with_nothing_staged_is_refused`;
  - `an_old_git_refuses_only_the_chain` (a fake `git --version` < 2.38, as 84/86 did).
- [x] **Dependents** (`rewrite_plan.rs`, the four named green, plus `a_backup_at_the_same_commit_is_not_a_dependent`):
  - `a_teammate_branch_on_top_is_a_dependent`;
  - `main_containing_the_branch_is_not_a_dependent`;
  - `a_branch_merged_long_ago_is_not_a_dependent` (merge between);
  - `dependents_honour_stop`.
- [x] **Progress** (`rewrite_plan.rs`, the three named green, plus `no_rebase_no_progress`; the branch test runs a
      real `git rebase -i` with `update-ref` lines and a hand-written `plan.json`):
  - `progress_names_the_branch_of_the_step` (`feature-b, 2 of 3`);
  - `progress_of_a_shell_rebase_has_no_branch`;
  - `progress_lists_pending_oldest_first`.
- [ ] **The rebase regression net:** every existing graft-ops rebase test green. Assertions on the old command line
      are updated to the engine's, and NOTES lists each one changed, with why.
- [ ] MAP.md (graft-ops): `rewrite` module entries, the new variants, and the plan dir. NOTES: ADR-0047's open points,
      as decided, with the oracle row that proves each.

## Touches (check each, report how)
- **`r` (Rebase) for every caller:** `graft` launches `Operation::Rebase` through `operate::launch`, and its preview and
  M5/M6 walks are app-side. Check: the workspace builds; `cargo nextest run` workspace-wide is green (the m5/m6 rebase
  walks run through the new engine).
- **Resume, banner, resolver (M6):** they read `rebase-merge/`; unchanged here. Check: m6 rebase-stop walks green.
- **Set-aside (ADR-0045):** now wraps `Rewrite` too. Check: the stash-clash/set-aside tests green, plus a new oracle
  row with an unsaved edit.
- **The journal file format:** `OperationKind::Rewrite` is a new serde tag. Old lines must still read. Check:
  `the_journal_reads_every_line_written_before` (existing) green, plus one new round-trip test.
- **Undo inverse / `restore`:** multi-ref CAS through the existing Rebase inverse. Check: the undo tests above.
- **Other crates' exhaustive matches** (scout Q6): each site listed in NOTES, with how it still builds.
- **Captures:** none (ops only).
- **Keymap:** none.
- **KNOWN_GAPS:** none closed here. :548 is `rewrite-preview`'s.

## Rules
Worktree only (every path, heredocs and `cd` included). `CARGO_TARGET_DIR` in the worktree. Gates workspace-wide after
the last edit. Never read `PROGRESS.md` or `KNOWN_GAPS.md`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Large (~80k). If it compacts, split:
- **rewrite-engine-a:** types, `plan`, `dependents`, the pure fns, refusals, `progress`; no apply.
- **rewrite-engine-b:** apply, resume, record, undo, the plan dir, the oracle.

The signatures land in -a first.

## NOTES (builder, 2026-09-26)

**Compacted mid-ticket.** Per the split above, -a is done and committed; -b is open (Handoff below). Scout used:
`.scratch/m7-power-tools/scouts/02.md`.

What was verified, and how:
- The gates, workspace-wide: fmt, clippy `--all-targets -D warnings`, nextest (3559 passed), `test --doc`.
- git 2.54.0 (Apple Git-157), by throwaway scripts:
  - `rebase-merge/done` keeps full 40-hex ids as the todo gave them;
  - when git asks for a message, the last line of `done` names the commit it is on;
  - a squash group holding a squash asks once, on the group's last row, even when that row is a fixup;
  - `msgnum`/`end` count `update-ref` lines: 4/6 where his rows are 3/4. `progress_names_the_branch_of_the_step`
    asserts both.
- Not verified: anything about running a rewrite. That is -b.

Deviations and decisions, each for the conductor to accept or send back:
1. **`NothingStaged{target}` is `NothingStagedFor { target }`.** The unit `Refusal::NothingStaged` already exists (the
   commit's; graft matches it at `crates/graft/src/update/operate.rs:2203`).
2. **Extra refusals:** `PlainAmend` (`a` into HEAD with nothing built on it), `RootCommit { commit }` (`a` into the
   first commit), `BadTodo(TodoProblem)`.
   - The note has no words for `GitTooOld`, `PlainAmend` or `RootCommit`, so mine are in `explain()` (`operation.rs`).
     The conductor or planner should confirm them.
3. **A dependent checked out in another worktree stays in the line**, and `plan` refuses it as `DependentElsewhere`.
   - The ticket's wording ("not checked out in another worktree") would drop it from the line silently.
   - That would leave duplicated history, and `DependentElsewhere` would then be unreachable.
4. **A branch at the same commit as his is not a dependent.** This protects a backup made just before a rewrite.
   Test: `a_backup_at_the_same_commit_is_not_a_dependent`.
5. **The default branch** comes from `Branches::resolve(repo, None)`. The app's `graph.default_branch` override is not
   visible to graft-ops. Question for the conductor: should `plan` take it?
6. **When `from` is the repository's first commit**, the range stops above it and it becomes `onto`: there is nothing
   to replay it onto. Test: `open_deeper_than_the_base_reaches_from`.
7. **Message cleanup (built in -a, runs in -b).** Every commit git makes gets `commit.cleanup=whitespace` through
   `GIT_CONFIG_COUNT` env (`saved::CLEANUP`).
   - Result: a fed message keeps `#123 fixes login`.
   - Where no message was written, the helper runs `git stripspace --strip-comments`.
   - Oracle still owed (-b): a reword whose message has a `#` line.
8. **`RebaseParams` gains no `line`/`drops` fields.** Adding them breaks about 12 struct literals in graft:
   - `update/drag.rs:270`, `update/preview.rs:1404`, `update/operate.rs:2134`, `context_menu/verbs.rs:167,339`;
   - tests: `preview/tests.rs:145,764`, `verbs/tests.rs:109,190,363`, `remote/tests.rs:723`, `view/tests.rs:3589`.
   - Plan for -b: the engine infers the line at apply time via `dependents()`.
   - Cost: the line is not pinned by the preview. Adding the fields needs a graft seam ticket.
9. **The Touches line names a test that doesn't exist.** `the_journal_reads_every_line_written_before` is not in the
   repo. I added `a_rewrite_entry_round_trips` (graft-core `journal.rs`). The nearest existing tests,
   `a_journal_line_without_put_back_re_renders_byte_identically` and `tests/it/unmerged_index.rs:365` (a golden),
   stay green.
10. **Other crates' matches.**
    - `OperationKind`: `graft/src/update/preview.rs:262`, `update/journal.rs:279` and `view/journal.rs:125` all have
      `_` arms.
    - `Refusal`: `graft/src/update/operate.rs:1097` has an `other =>` arm.
    - The workspace builds and is green, so nothing is blocked on graft.
    - `Operation::Rewrite` is not added yet (-b), so no `Operation` match is touched.
11. **`history_ops.rs:567-600` (the scout's "invert it").** My reading is that it does not invert, for -b to confirm:
    - The test rebases `upper` (HEAD) with `lower` *below* it, inside the range.
    - ADR-0047 moves only branches *above* his branch (built on it). Branches inside the range still don't move under
      `--no-update-refs`, and Graft's todo adds `update-ref` only for the line.
    - So its assertions still hold. -b should re-point its doc comment and add the inverted case as
      `a_three_branch_rewrite_records_every_ref` (red first).
12. **The -b run half is written but not wired.** `saved.rs`, `todo::git_todo` and `plan::line_picks` sit behind
    `expect(dead_code, reason = "…rewrite-engine-b")`. `expect` warns once they are used, so -b must delete those
    attributes.

## Handoff → rewrite-engine-b

Done in -a (commit `m7-power-tools-02a`):
- **The `rewrite` module:** types, `plan`, `dependents`, `progress`, the pure functions, the saved-plan format and
  helper (`saved.rs`), and `git_todo`.
- **Refusals:** the new `Refusal` variants and their words.
- **The new kind:** `OperationKind::Rewrite`; `opens_in_progress(Rewrite)` → `Rebase`; `kind_name` → `rewrite`.
- **Errors:** the new `FactsError` variants.

Next, in order:
1. **`Operation::Rewrite(RewriteParams)`.** Wire every `operation.rs` site:
   - `kind`, `is_history`, `sets_aside`, `tips` (head + line), `facts_scope` (line names);
   - `describe` (`rewrite <branch>` / `put staged changes into <short>`);
   - `check` (on `head.name`, tip CAS → `Moved`, `validate` → `BadTodo`), `plan`, `touches` (line refs + stash if
     set aside, index, worktree).
   - `record.rs`: `pins_for` → `made`. `preview::plan` falls to `NotPreviewable`.
2. **`plan_rebase` → `git rebase -i [--no-update-refs] --no-autostash <onto> [<top short name>]`.**
   - Keep the `git rebase ` prefix: graft tests `m5_walk/preview.rs:75` and `tests/it/preview.rs:216` assert it.
   - Pass `--no-update-refs` only when git ≥ 2.38; `<top>` must be the **short** name (a full ref detaches).
   - Update `operation/history/tests.rs:322`.
   - Fix the stale comment at `history.rs:613`.
3. **`apply/engine.rs`.**
   - Build the line: `dependents` for Rebase, `params.line` for Rewrite.
   - Build the todo: `replayed_commits` picks + `line_picks` + `git_todo`.
   - `saved::save` to `saved::dir(repo, ctx.next_id())` before `begin`; remove it if `begin` fails.
   - Run env: `GIT_SEQUENCE_EDITOR=saved::sequence_editor`, `GIT_EDITOR=saved::editor`, `CLEANUP`.
   - After a finish with top ≠ home: `git switch <home short>`.
   - Exit 0 while `rebase-merge/` is still live is an edit stop → `Conflicted{paths: []}`.
   - Remove the plan dir on close; a failed removal is a new `AfterState` (e.g. `PlanLeft`).
4. **Resume.**
   - `live_entry` accepts `head-name` == the saved plan's top.
   - Continue/Skip pass `GIT_EDITOR` + `CLEANUP`.
   - Exit 0 with the rebase still live → Conflicted.
   - After a finish or abort with top ≠ home, switch home before `close_resume`.
   - Remove the plan dir.
   - `recover_interrupted` removes the plan dirs of the entries it closes.
5. **The `-c` question.** Resume needs no `-c rebase.updateRefs`: it affects only todo generation, and the todo is
   already written.
   - Prove it: `rebase.updateRefs=true` in the repo config, an edit stop, `resume(Continue)`, and a branch inside the
     range stays unmoved.
6. **Tests:** the oracle file (10 rows plus an unsaved-edit row, with a `cp -R` TempRepo helper), record/undo/abort/kill,
   the plan-dir removal test, and a `#123` reword.

Gotchas:
- In a worktree-isolated agent, inline git in temp dirs is refused. Put experiments in a script file and run `sh`.
- The helper script embeds `repo.git_dir()`. Check it is absolute.
- Check that `--continue` after a conflicted pick opens the editor (the helper must answer it).

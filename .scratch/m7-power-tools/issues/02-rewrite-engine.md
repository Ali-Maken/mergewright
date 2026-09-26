# rewrite-engine — every rewrite runs git's `rebase -i` on a todo Graft writes; dependents follow on one line

Status: ready
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
- [ ] **Pure** (`crates/graft-ops/src/rewrite/tests.rs`):
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
- [ ] **Refusals**, each asserting the words:
  - `a_fork_is_refused_naming_both`;
  - `a_dependent_in_another_worktree_is_refused_with_its_path`;
  - `a_merge_in_range_is_refused`;
  - `a_commit_off_your_branch_is_refused`;
  - `detached_head_is_refused`;
  - `amend_into_with_nothing_staged_is_refused`;
  - `an_old_git_refuses_only_the_chain` (a fake `git --version` < 2.38, as 84/86 did).
- [ ] **Dependents:**
  - `a_teammate_branch_on_top_is_a_dependent`;
  - `main_containing_the_branch_is_not_a_dependent`;
  - `a_branch_merged_long_ago_is_not_a_dependent` (merge between);
  - `dependents_honour_stop`.
- [ ] **Progress:**
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

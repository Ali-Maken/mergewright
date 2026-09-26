# 28 ignored-file-guard — no write goes over a file git doesn't keep

Status: **done** — 28a (2026-09-26), 28b (2026-09-26). ADR-0049 accepted by the user 2026-09-26 (scout done: `.scratch/m7-power-tools/scouts/28.md`). Split into **28a** then **28b**. Both are graft-ops,
so they run one after the other, never at the same time.
Blocked by: 23b (merged 4654b7e1); 22c while it holds graft-ops.
Decisions: ADR-0049 (proposed, planner 2026-09-26). The spec-only pass replaced "no new decision / `blocked: needs
wording`": the words are below.

## Why
23b's probe: `main` doesn't have `c.txt`, `other` tracks it, `c.txt` is ignored and holds his bytes. Checkout to `other`:
the preview doesn't refuse, `y` overwrites `c.txt`, and `u` puts back git's copy. His bytes are gone. The scout's probe
(git 2.54.0): switch, merge (ff and no-ff), pull, rebase, cherry-pick, revert and `reset --hard` all overwrite with rc 0.
Only stash apply refuses. The cause is review C1's: "in the way" is read from `git status`, which never lists ignored
files, and the backup (`add -A`) skips them. This breaks M3's promise, "you cannot lose work", on a daily key.

## Decisions (ADR-0049)
1. **Refuse; don't back up.** In `preview::plan` and in `apply` before anything is written (before set-aside's
   `stash create`, before `begin`): `ApplyError::Refused`, no journal line. The backup stays `add -A`.
2. **Covered:** Checkout (branch, detached), BranchCreate with `checkout`, BranchFromStash, Merge, Pull, Reset `Head`
   `Hard`, Recover `MoveRef` checked out (28a); Rebase, Rewrite (`R`, `a`, absorb), CherryPick, Revert (28b). StashApply
   and StashPop keep `StashInTheWay` and its words, fed by the same read. **Not covered** (they write only paths the
   index tracks, so the read would find nothing): Stage, Unstage, Discard, ResolveWholeFile, RestartConflict, soft and
   mixed resets, `ResetTarget::Branch`, and every ref-only kind. `c` continuing a stopped operation is not re-checked
   (out of scope).
3. **`stash branch` is covered** (question (c)). It is `switch -c <name> stash^1`, then `stash pop --index`. The switch
   half is exposed like any switch. The scout didn't probe it; 28a's test probes it first.
4. **The paths an operation writes**, uncapped:

   | Kind | Paths |
   |---|---|
   | Checkout, BranchCreate+checkout, Reset Hard, Recover MoveRef, Pull, Merge `--ff-only` / fast-forward | `diff-tree -r --name-only --no-renames -z HEAD <landed>` |
   | Merge, not a fast-forward | the same between `merge-base HEAD <source>` and `<source>` (the incoming side) |
   | BranchFromStash | `HEAD`→`stash^1`, plus `stash^1`→`stash`, plus `stash^1`→`stash^2`, plus `ls-tree -r --name-only stash^3` when there is one |
   | StashApply / StashPop | `stash^1`→`stash`, plus `stash^1`→`stash^2`, plus `stash^3`'s paths |
   | Rebase, Rewrite (28b) | `HEAD`→`<onto>`, plus each replayed commit's own diff against its first parent (the todo's rows, `made` included) |
   | CherryPick, Revert (28b) | each commit's own diff against its first parent (`-m n`: against parent `n`) |

   A path counts as **in the way** when the index doesn't track it (the read 23b's `in_the_way` uses, `tracked_paths_now`)
   and `symlink_metadata` finds something there. A file at an ancestor (`ENOTDIR`) counts, and so does a directory. The
   path we report is the one on disk that holds his bytes: the path itself, or the first ancestor that is a file.
   **Ignored** = in the way, and not in `facts.status`'s untracked list. For a directory: no untracked entry under it.
   Spawns: 1 for the diff, 1 for the index, and for a replay 1 `diff-tree --stdin` over every commit. **No spawn per path.**
5. **Pull only:** its plan becomes `merge --ff-only --no-overwrite-ignore --no-edit <tracking>`. The fetch runs
   between Graft's read and git's write, so the read can be stale; git's own flag is the guard at write time. Switch and
   merge plans are unchanged.
6. **One read, and where it lives:** a new module `graft-ops/src/in_the_way.rs`. It sits at the top level, not under
   `preview/`, because `apply` calls it too. 23b's `preview::in_the_way` and `preview::stash::occupied` go through its
   `on_disk`, so there is one `symlink_metadata` loop left.

### The words (question (b))
The new `Refusal::InTheWay`, in `explain()`. `+N more` is the same form as Recover's `put back your edit to <path> +N more`:

| Case | Words |
|---|---|
| any ignored path, 1 path | `<path> (ignored) is in the way of <doing> · move it aside first` |
| any ignored path, several | `<path> (ignored) +N more are in the way of <doing> · move them aside first` |
| only untracked, 1 path | `<path> is in the way of <doing> · S stashes it first` |
| only untracked, several | `<path> +N more are in the way of <doing> · S stashes them first` |

`<path>` is the first ignored path in path order, or the first untracked one if none is ignored. Ignored files decide
the next step: `S` (`stash push -u`) doesn't take an ignored file, and `c` can't stage one.

| Operation | `<doing>` |
|---|---|
| Checkout `Branch(b)` / `Detached(oid)` | `switching to <b>` / `switching to <sha7>` |
| BranchCreate `checkout`, BranchFromStash | `switching to <name>` |
| Merge | `merging <b>` (`Source::Branch`) / `merging <sha7>` |
| Pull | `pulling <remote_side>` (`Operation::remote_names`) |
| Reset `Head { branch, Hard }` | `resetting <branch short> to <sha7>` |
| Recover `MoveRef` | `putting <b> back to <sha7>` |
| Rebase | `rebasing onto <b>` / `rebasing onto <sha7>` |
| Rewrite | `rewriting <head short>` |
| CherryPick | `picking <sha7>`; several: `picking <n> commits` |
| Revert | `reverting <sha7>` |

`refusal_status` in graft already words any refusal other than `DirtyTree` through `explain()` (graft MAP.md:241). So
graft needs no change unless it matches `Refusal` exhaustively somewhere: grep `Refusal::` in `crates/graft`. If it
does, commit at the seam and report `blocked: needs graft`, naming the variant.

## Signatures (graft-ops)
```rust
// crates/graft-ops/src/in_the_way.rs — `pub mod in_the_way;` in lib.rs.
// `repo` and `stop` take exactly the types 23b's `preview::in_the_way` takes today.

/// Paths on disk that git doesn't keep and that the operation would write over. Each list is sorted and unique.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blocking { pub ignored: Vec<PathBuf>, pub untracked: Vec<PathBuf> }
impl Blocking { pub fn is_empty(&self) -> bool; }

/// The one read. Empty for a kind decision 2 doesn't cover.
pub fn blocking(repo: &_, op: &Operation, facts: &RepoFacts, stop: &dyn Stop) -> Result<Blocking, FactsError>;

/// `Some(InTheWay { .. })` when not empty, `None` when empty. Stash kinds never call this; they build `StashInTheWay`.
pub fn refusal(op: &Operation, blocking: Blocking) -> Option<Refusal>;

/// `<doing>` from the table above. `None` for a kind that isn't covered.
pub fn doing(op: &Operation) -> Option<String>;

/// Decision 4's table. `Ok(None)` = the kind writes no path the index doesn't track.
pub(crate) fn written_paths(repo: &_, op: &Operation, facts: &RepoFacts, stop: &dyn Stop)
    -> Result<Option<BTreeSet<PathBuf>>, FactsError>;

/// The one `symlink_metadata` loop (NotFound skipped, ENOTDIR → the ancestor file). Other errors → `FactsError::OnDisk`.
pub(crate) fn on_disk(repo: &_, paths: &BTreeSet<PathBuf>) -> Result<Vec<PathBuf>, FactsError>;

// crates/graft-ops/src/operation.rs
pub enum Refusal { /* … */ InTheWay { doing: String, ignored: Vec<PathBuf>, untracked: Vec<PathBuf> } }

// crates/graft-ops/src/apply.rs — `guard_untracked` renamed; it now runs for every covered kind.
fn guard_in_the_way(ctx: &Context, op: &Operation, facts: &RepoFacts) -> Result<(), ApplyError>;
```
Callers: `preview::plan` (after `check`, before any forecast) and `apply::guard_in_the_way`, at 23b's call site
(`apply.rs:224`), before the set-aside. For StashApply/StashPop: `preview::stash` and `apply` take `blocking`'s paths
into `StashInTheWay.untracked`, ignored ones included.

---

## 28a — the read, the one-shot writes, the migrations
Build with: **Opus**. These are git writes that lose his bytes, and each is judged by a probe against real git. The
signatures are fixed above, so no Fable signature ticket is needed.
Crates: graft-ops. Needs: none.
Read first: this file; ADR-0049; scout 28; review m7-22 §C1; `crates/graft-ops/MAP.md` §operation (`Refusal`,
Recover), §apply, §preview (the stash line, `plan`). Then source.
Scope in: the module; `Refusal::InTheWay` and its words; decision 4's rows for the 28a kinds; `--no-overwrite-ignore`
on pull; 23b's check and `occupied` moved onto `on_disk`; StashApply/StashPop now guarded at `y` too. The replay
kinds' arms in `written_paths` return `Ok(None)` with a `// 28b` comment. **Not** a stub marked done: 28b closes them.
Scope out: the replay kinds; changing any stash words; a key that moves a file aside.

Tests live in `crates/graft-ops/tests/it/in_the_way.rs`. Each test sets an ignored `c.txt` (`.git/info/exclude`)
holding `MINE`, and asserts all four of these:
- the preview is `Err(Refused(..))` with the exact words;
- `apply` is `Err(Refused(..))`;
- `c.txt` is still `MINE`;
- the journal entry count is unchanged.

"Red" = the result before the fix, **recorded in NOTES**.
- [x] `switch_over_an_ignored_file_is_refused` (in `crates/graft-ops/tests/it/in_the_way.rs`): 23b's probe. Red: preview Ok, `c.txt` overwritten.
- [x] `switch_detached_over_an_ignored_file_is_refused`. Red as above.
- [x] `new_branch_switch_over_an_ignored_file_is_refused` (BranchCreate `checkout: true`). Red as above.
- [x] `branch_from_stash_over_an_ignored_file_is_refused` (probed: git overwrites, see NOTES): probe first; the stash base tracks `c.txt` and `HEAD` doesn't.
  If git already refuses, the test stays and NOTES say "safe by git", with its stderr.
- [x] `merge_fast_forward_over_an_ignored_file_is_refused` and `merge_over_an_ignored_file_is_refused` (`--no-ff`). Red as above.
- [x] `pull_over_an_ignored_file_is_refused`: the tracking tip already tracks `c.txt`. Refused before the fetch.
- [x] `pull_whose_fetch_brings_an_ignored_path_is_refused_by_git` (plan-args: `operation::remote::tests`, `git merge --ff-only --no-overwrite-ignore --no-edit …`): the tracking tip lacks `c.txt` and the remote has
  it. The fetch runs, then git refuses the merge (the flag). `c.txt` is still `MINE`, and the error names `c.txt`. If
  `classify_failure` doesn't match git's `untracked working tree files would be overwritten by merge`, it maps it to
  `WouldOverwrite`. Red: `c.txt` overwritten. Update the plan-args tests of pull and `Operation::preview` too.
- [x] `hard_reset_over_an_ignored_file_is_refused`; `soft_and_mixed_resets_are_not_refused` (no false refusal).
- [x] `put_back_over_an_ignored_file_is_refused_in_the_way` (and `recover::move_ref_checked_out_refuses_over_an_{untracked,ignored}_file_it_would_overwrite` moved to `InTheWay`): 23b's C1 test, words moved from `UnsavedEditsTo` to `InTheWay`.
- [x] `stash_apply_over_an_ignored_file_is_refused_at_y`: the preview's `StashInTheWay` is unchanged. `apply` now
  refuses before `begin`. Red: git refused after `begin`, and an `Aborted` entry was journaled.
- [x] `stash_whose_staged_new_file_meets_an_ignored_file` (probed: git overwrites, so red-first): probe (the stash's index adds `c.txt`). Red-first if git
  overwrites it; if not, a proof test, with NOTES as above.
- [x] `untracked_file_in_the_way_is_refused_before_git`: `c.txt` not ignored, `S stashes it first`. Red: an `Aborted` entry.
- [x] `a_directory_or_a_file_above_is_in_the_way` (+ no-false-refusal: `a_tracked_file_above_is_not_in_the_way`, `ignored_build_output_elsewhere_is_not_in_the_way`): the target tracks `dist` and an ignored `dist/` holds a file; the
  target tracks `a/b` and an ignored file `a`. Both refused, naming `dist` and `a`.
- [x] `in_the_way_words` (unit, `crates/graft-ops/src/operation/tests.rs`): all four word rows, a mixed case, and one `<doing>` per kind in
  the table.
- [x] One read: NOTES name the grep proving it. `symlink_metadata` appears only in `in_the_way::on_disk`, and the
  callers of `blocking` are `preview::plan`, `preview::stash` and `apply::guard_in_the_way`.
- [x] `crates/graft-ops/MAP.md`: the module, `Refusal::InTheWay`, `plan`'s read-refusals, pull's plan, the Recover line
  and the `preview::stash` line.
- [x] Gates run workspace-wide: `cargo fmt --all --check` ok; `cargo clippy --workspace --all-targets -- -D warnings`
  ok; `cargo nextest run --workspace` 3818/3818 passed (1 skipped); `cargo test --workspace --doc` ok.
Size: about 70k.

### 28a NOTES (builder, 2026-09-26)
Scout: used scout 28 (`.scratch/m7-power-tools/scouts/28.md`); no second one.

**Red, before the fix** (each tuple is `(preview, apply, bytes after, new journal entries)`, observed by running the
tests against the unchanged code):
- switch, switch detached, new-branch switch, merge `--ff-only`, merge `--no-ff`, hard reset: `("preview drew a plan",
  "apply ran", "theirs\n", 1)` — his `MINE` overwritten, rc 0.
- branch from stash: the same — **not safe by git**: `stash branch`'s switch overwrote `c.txt` ("theirs").
- stash whose index adds `c.txt`: `("preview drew a plan", "apply ran", "staged\n", 1)` — **git overwrites** an
  ignored file from the stash's staged part (no refusal). Red-first, not a proof test.
- stash apply of a `-u` stash: `("c.txt is in the way", "apply failed: apply stash@{0}: c.txt already exists, no
  checkout · u restores the backup", "MINE\n", 1)` — git refused, after `begin`: an `Aborted` entry.
- untracked (not ignored) `c.txt`, switch: `("preview drew a plan", "apply failed: checkout other: error: The
  following untracked working tree files would be overwritten by checkout: · u restores the backup", "MINE\n", 1)`.
- `dist/` ignored dir where `other` tracks a file `dist`: `("preview drew a plan", "apply ran", "", 1)` — git
  **deleted** his ignored directory. `a` ignored file where `other` tracks `a/b`: overwritten likewise.
- put-back (23b C1): refused already, in `UnsavedEditsTo`'s words (`c.txt has unsaved edits · c commits or S stashes
  them first`).
- pull, tracking tip tracks the file: `("pull has no preview", "apply ran", "adds a file\n", 1, <tracking moved>)`.
- pull, fetch brings the file: red observed by removing `--no-overwrite-ignore` from the plan and running the test
  (fixture fixed first; its first run failed in setup): his file overwritten ("adds a file"). With the flag git refuses;
  `classify_failure` already matched `untracked working tree files would be overwritten`, so no graft-git change:
  `WouldOverwrite { paths: [<file>] }`, entry `Aborted`, file `MINE`.
- `soft_and_mixed_resets_are_not_refused`, `ignored_build_output_elsewhere_is_not_in_the_way`,
  `a_tracked_file_above_is_not_in_the_way`: green before and after (guards against false refusal).

**One read — the grep:** `grep -rn symlink_metadata crates/graft-ops/src` → `in_the_way.rs` (`on_disk`, and its
`file_above` helper for the `ENOTDIR` ancestor walk) plus `merge_state.rs:110`, which stats `MERGE_AUTOSTASH` in the
git dir, not a worktree path. `grep -rn "in_the_way::blocking\|in_the_way::refused" crates/graft-ops/src` →
`preview.rs` (`refuse_in_the_way`, called by `plan` after `check`) and `apply.rs` (`guard_in_the_way`) only.
`preview::stash`'s `occupied` and `preview::recover::in_the_way` are deleted; `plan`'s read covers both before any
forecast, so `preview::stash` does not call `blocking` itself (the ticket listed it as a caller; one call site fewer).

**Deviations, for the conductor:**
- Hard reset and checked-out `MoveRef` read `diff-index --cached <to>` (index vs target), not `diff-tree HEAD <to>`: a
  superset. It also catches a file he `git rm --cached` (e.g. `.env`), which `reset --hard` writes back and no `HEAD`
  diff names. 23b's own read had a partial version of this (status-untracked paths held in `to`).
- `on_disk` takes a third argument, `tracked`: a directory counts only when it holds something the index doesn't track
  (a tracked `dist/` becoming a file `dist` is git's to empty; without this, a false refusal), and an `ENOTDIR`
  ancestor the index tracks (`a` tracked on `HEAD`, `a/b` on the target) isn't his file.
- Merge reads `diff-tree --merge-base HEAD <source>` in one spawn (git ≥ 2.30; minimum is 2.38).
- `doing` also answers the 28b kinds (rebase, rewrite, pick, revert), since the words table is fixed; `written_paths`
  returns `Ok(None)` for them with a `// 28b` comment, so they are **not guarded yet**.

**Questions (not defects, no line proves loss):**
- The set-aside's `reset --hard` (checkout, rebase over a dirty tree) writes paths where the index differs from `HEAD`.
  A `git rm --cached` file that the *target* doesn't have is outside every row of the table, so it is not read. Worth a
  probe in a later ticket.
- Deletions are in the switch diff (`--name-only` includes `D`): a path `HEAD` tracks, the target deletes and the index
  doesn't track (staged `rm --cached`), with the file on disk, is refused although plain `git switch` would leave it.
  The set-aside runs first there, so refusing is the safer side.

**Touches:** `apply.rs:224` (renamed guard, runs for every covered kind, before `set_aside::create` and `begin`,
checked by the `entries` count in every test); `preview::plan` doc; 23b's tests (`recover.rs`, both moved to
`InTheWay`; `grep -rn "UnsavedEditsTo\|has unsaved edits" crates` → only Recover `Files`, its test, MAP); pull's plan
args (`operation/remote/tests.rs`); captures: `grep -rn "merge --ff-only" docs/shots` → none; graft: `refusal_status`
words every non-`DirtyTree` refusal via `explain()` and `grep -rn "Refusal::Bare\|Refusal::HeldAgain" crates/graft`
finds no exhaustive match, so no graft change. **Not edited (outside graft-ops):**
`docs/capabilities/m4-remotes.md:56` still says `merge --ff-only --no-edit <tracking_ref>`; DESIGN §6.3 and
`docs/design/02-keymap.md:425` are the conductor's per the ticket.

## 28b — the replays
Build with: **Opus** (reasons as 28a). Crates: graft-ops. Needs: 28a's `written_paths`, `blocking`, `doing` and
`Refusal::InTheWay`.
Read first: this file; ADR-0049 §2; 28a's NOTES; MAP.md §rewrite and `apply::engine`.
Scope in: the replay rows of decision 4 (they replace 28a's `Ok(None)` arms), with one `diff-tree --stdin` for all
commits. The engine's `switch <home>` needs no guard of its own: every path it writes is in the union (say so in
MAP.md). Scope out: `c` after a stop.
All tests below are in `crates/graft-ops/tests/it/in_the_way.rs`, asserting 28a's four checks (preview words, apply
words, bytes still `MINE`, 0 new journal entries). Red tuples in 28b NOTES.
- [x] `rebase_over_an_ignored_file_is_refused` (`onto` tracks `c.txt`). Red: overwritten.
- [x] `rebase_whose_middle_commit_writes_an_ignored_path_is_refused`: commit A adds `c.txt` and B deletes it, so the
  landed tree lacks it. Red: `c.txt` gone after `y`.
- [x] `rewrite_over_an_ignored_file_is_refused`: `R` over a branch whose commit X removed `.env` from git, with `.env`
  ignored on disk. `onto` is below X. Red: `.env` gone. (X's row made a reword, see NOTES.)
- [x] `amend_into_over_an_ignored_file_is_refused` (`a`, through the same `Rewrite`; fixture changed, see NOTES).
- [x] `cherry_pick_over_an_ignored_file_is_refused` (one commit, and two commits: `picking 2 commits`).
- [x] `revert_of_a_delete_over_an_ignored_file_is_refused` (the scout's probe).
- [x] `replay_with_ignored_build_output_elsewhere_runs`: an ignored `target/` that no commit writes. `Ok`, no refusal.
- [x] MAP.md updated (`crates/graft-ops/MAP.md` §in_the_way: replay rows, spawns, the engine's `switch <home>`); gates
  run workspace-wide: `cargo fmt --all --check` ok; `cargo clippy --workspace --all-targets -- -D warnings` ok;
  `cargo test --workspace --doc` ok; `cargo nextest run --workspace` — see NOTES (one flaky graft capture).
Size: about 45k.

### 28b NOTES (builder, 2026-09-26)
Scout: used scout 28 and 28a's NOTES; no second scout.

**Red, before the fix** (`(preview, apply, bytes after, new entries)`, observed by running the tests against 28a's
`Ok(None)` arms):
- rebase, `onto` tracks `c.txt`: `("preview drew a plan", "apply ran", "theirs\n", 1)`.
- rebase, middle commit adds then deletes `c.txt`: `("preview drew a plan", "apply ran", "", 1)` — his file gone.
- rewrite (`R`, X reworded): `("rewrite has no preview", "apply ran", "", 1)` — `.env` gone. (`preview::plan` refuses
  a `Rewrite` as `NotPreviewable`; since 28a's read runs before that, it now answers `InTheWay`.)
- amend-into: `("rewrite has no preview", "apply ran", "", 1)` — `c.txt` gone.
- cherry-pick, one commit and two: `("preview drew a plan", "apply ran", "theirs\n", 1)` each (two-commit red observed
  by disabling the arm for `len > 1`, then restoring it).
- revert of a delete: `("preview drew a plan", "apply ran", "theirs\n", 1)`.
- `replay_with_ignored_build_output_elsewhere_runs`: green before and after (no false refusal).

**What was built:** `in_the_way::replayed_paths` — rebase: `diff-tree HEAD <onto_tip>` + `rev-list --no-merges
<onto_tip>..HEAD`; rewrite: `diff-tree HEAD <onto>` + non-`drop` todo rows + `made` + dependents' commits
(`rev-list --no-merges <head>..<top>`, only when the line has a top above his branch); pick: its commits; revert: the
commit against `parent` (else parent `mainline`, via `rev-parse ^@`). Every commit's own change in **one**
`diff-tree --stdin --no-commit-id --root` (`<commit>` alone = first parent; `<commit> <parent>` for revert).

**Deviations / findings, for the conductor:**
- **Ticket fixtures changed.** (1) `R` with every row a plain pick: git's rebase skips unchanged leading picks (it
  never checks out `onto`), so `.env` survived and the red could not show. The test makes X's row a reword so git
  must replay it. (2) `a` into X in the `.env` fixture: the same skip means git never writes `onto`'s tree, and the
  first run conflicted (`y.txt` absent at X). The amend test now uses T/U/V: `a` into T, U adds `c.txt`, V deletes
  it — the replay above the fixup writes his path. Red observed as above.
- **Question (false refusal, safe side):** because of that skip, `HEAD`→`onto` over-reads when the leading rows are
  unchanged picks: `a` into the first commit of a branch that `git rm --cached .env` is refused although git would
  not write `.env`. Modelling git's skip was not done (it is git-internal behaviour, not in ADR-0049's table).
- **Dropped rows are left out** of the rewrite union (git applies nothing for them). The table says "the todo's rows";
  I read `drop` as writing nothing.
- **Changed a 23b-era test:** `set_aside::a_refused_rebase_puts_the_edits_back_on_the_same_tip` used an untracked
  file in the way to make git refuse after the set-aside; Graft now refuses that before the set-aside (correct per
  ADR-0049 §1). The test now makes git refuse with a `pre-rebase` hook, keeping what it proves (edits put back on the
  same tip, `S` dropped). Cause, not symptom: the premise changed.
- **Flaky, not mine (question):** second full nextest run, `graft::it status_under_a_box::capture_fix_status_under_a_box`
  failed once (`docs/shots/m6/fix-status-under-a-box-120x40.txt` line 151: drawn has a Mistake Radar `ⓘ` the capture
  lacks); it passed in the first full run and alone. No graft code touched here.

**Gates:** fmt ok; clippy ok; doctests ok; nextest run 1: 3886/3887 (the `set_aside` test above, then fixed; its
module re-run 50/50); run 2: 3886/3887 (only the flaky capture above; passes alone).

**Touches:** `apply::guard_in_the_way` — unchanged, now fires for the four replay kinds (every test's entry count
proves it runs before `begin`); `preview::plan` — unchanged, same read; `apply::engine`'s `switch <home>` — no guard,
its paths are in the union (MAP.md); set-aside order — `set_aside.rs` test above; `Refusal::InTheWay` words — 28a's,
unchanged (`doing` already answered the replay kinds); graft — no `Refusal` match change (`grep -rn "Refusal::" crates/graft/src`
shows no exhaustive match; full nextest green but for the flaky capture); captures — no drawing code changed;
`docs/capabilities/m4-remotes.md:56` — updated to `merge --ff-only --no-overwrite-ignore --no-edit` (carried line).

## Touches (both; the Touches line of each report says how each was checked)
- `apply`'s guard site (`apply.rs:224`), the set-aside order (`apply/set_aside.rs`), and `preview::plan`'s
  "`Err(Refused)` exactly when `check`" doc.
- 23b's tests and words: grep `UnsavedEditsTo` and `has unsaved edits` workspace-wide. Recover `Files` keeps
  `UnsavedEditsTo`.
- `StashInTheWay`: the words are unchanged. If its next step for an ignored path is `c` or `S`, report it in NOTES and
  don't change it.
- Pull's command line: the preview, the Command log (`┆ git merge --ff-only --no-overwrite-ignore …`),
  `tests/it/remote_ops.rs`, and captures (grep `docs/shots` for `merge --ff-only`).
- graft: `refusal_status`/`explain` path, and any exhaustive `Refusal` match (see "The words").
- DESIGN §6.3 line 95 and `docs/design/02-keymap.md:425` ("git refuses those"). Both go to the conductor with
  ADR-0049; a builder doesn't edit them.
- Captures: no drawing code changes. The refusal line on a switch is a scenario for the milestone's acceptance ticket
  (the conductor adds it).

## Carried to 28b (conductor, 2026-09-26)
- `docs/capabilities/m4-remotes.md:56` still shows the old pull line. 28a changed pull to `merge --ff-only --no-overwrite-ignore`; update that doc line.

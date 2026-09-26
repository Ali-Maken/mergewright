# 28 ignored-file-guard — no write goes over a file git doesn't keep

Status: ready (scout done: `.scratch/m7-power-tools/scouts/28.md`). Split into **28a** then **28b**. Both are graft-ops,
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
- [ ] `switch_over_an_ignored_file_is_refused`: 23b's probe. Red: preview Ok, `c.txt` overwritten.
- [ ] `switch_detached_over_an_ignored_file_is_refused`. Red as above.
- [ ] `new_branch_switch_over_an_ignored_file_is_refused` (BranchCreate `checkout: true`). Red as above.
- [ ] `branch_from_stash_over_an_ignored_file_is_refused`: probe first; the stash base tracks `c.txt` and `HEAD` doesn't.
  If git already refuses, the test stays and NOTES say "safe by git", with its stderr.
- [ ] `merge_fast_forward_over_an_ignored_file_is_refused` and `merge_over_an_ignored_file_is_refused` (`--no-ff`). Red as above.
- [ ] `pull_over_an_ignored_file_is_refused`: the tracking tip already tracks `c.txt`. Refused before the fetch.
- [ ] `pull_whose_fetch_brings_an_ignored_path_is_refused_by_git`: the tracking tip lacks `c.txt` and the remote has
  it. The fetch runs, then git refuses the merge (the flag). `c.txt` is still `MINE`, and the error names `c.txt`. If
  `classify_failure` doesn't match git's `untracked working tree files would be overwritten by merge`, it maps it to
  `WouldOverwrite`. Red: `c.txt` overwritten. Update the plan-args tests of pull and `Operation::preview` too.
- [ ] `hard_reset_over_an_ignored_file_is_refused`; `soft_and_mixed_resets_are_not_refused` (no false refusal).
- [ ] `put_back_over_an_ignored_file_is_refused_in_the_way`: 23b's C1 test, words moved from `UnsavedEditsTo` to `InTheWay`.
- [ ] `stash_apply_over_an_ignored_file_is_refused_at_y`: the preview's `StashInTheWay` is unchanged. `apply` now
  refuses before `begin`. Red: git refused after `begin`, and an `Aborted` entry was journaled.
- [ ] `stash_whose_staged_new_file_meets_an_ignored_file`: probe (the stash's index adds `c.txt`). Red-first if git
  overwrites it; if not, a proof test, with NOTES as above.
- [ ] `untracked_file_in_the_way_is_refused_before_git`: `c.txt` not ignored, `S stashes it first`. Red: an `Aborted` entry.
- [ ] `a_directory_or_a_file_above_is_in_the_way`: the target tracks `dist` and an ignored `dist/` holds a file; the
  target tracks `a/b` and an ignored file `a`. Both refused, naming `dist` and `a`.
- [ ] `in_the_way_words` (unit, in operation's tests): all four word rows, a mixed case, and one `<doing>` per kind in
  the table.
- [ ] One read: NOTES name the grep proving it. `symlink_metadata` appears only in `in_the_way::on_disk`, and the
  callers of `blocking` are `preview::plan`, `preview::stash` and `apply::guard_in_the_way`.
- [ ] `crates/graft-ops/MAP.md`: the module, `Refusal::InTheWay`, `plan`'s read-refusals, pull's plan, the Recover line
  and the `preview::stash` line.
- [ ] Gates run workspace-wide.
Size: about 70k.

## 28b — the replays
Build with: **Opus** (reasons as 28a). Crates: graft-ops. Needs: 28a's `written_paths`, `blocking`, `doing` and
`Refusal::InTheWay`.
Read first: this file; ADR-0049 §2; 28a's NOTES; MAP.md §rewrite and `apply::engine`.
Scope in: the replay rows of decision 4 (they replace 28a's `Ok(None)` arms), with one `diff-tree --stdin` for all
commits. The engine's `switch <home>` needs no guard of its own: every path it writes is in the union (say so in
MAP.md). Scope out: `c` after a stop.
- [ ] `rebase_over_an_ignored_file_is_refused` (`onto` tracks `c.txt`). Red: overwritten.
- [ ] `rebase_whose_middle_commit_writes_an_ignored_path_is_refused`: commit A adds `c.txt` and B deletes it, so the
  landed tree lacks it. Red: `c.txt` gone after `y`.
- [ ] `rewrite_over_an_ignored_file_is_refused`: `R` over a branch whose commit X removed `.env` from git, with `.env`
  ignored on disk. `onto` is below X. Red: `.env` gone.
- [ ] `amend_into_over_an_ignored_file_is_refused` (`a`, through the same `Rewrite`).
- [ ] `cherry_pick_over_an_ignored_file_is_refused` (one commit, and two commits: `picking 2 commits`).
- [ ] `revert_of_a_delete_over_an_ignored_file_is_refused` (the scout's probe).
- [ ] `replay_with_ignored_build_output_elsewhere_runs`: an ignored `target/` that no commit writes. `Ok`, no refusal.
- [ ] MAP.md updated; gates run workspace-wide.
Size: about 45k.

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

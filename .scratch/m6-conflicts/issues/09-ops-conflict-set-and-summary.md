# fix-ops-conflict-set-and-summary: git tells us which files this operation conflicted on, and what a merge will commit; an edited message reaches git through `commit -F`

Status: done (2026-09-23; built with Opus after scout-09). Filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md
Blocked by: none
Was: draft · new 2026-09-23 (Amendment 2). Interface-first: the file list, the finish summary and P4 call what this exposes.
Build with: blank — brownfield (`apply/resume.rs`, `Refusal`) (ADR-0035): **scout first**. Expected **Opus**: every decision is made in the note, but git's behaviour has to be verified in throwaway repos.
Needs: none.
Crates: graft-ops; graft only where an exhaustive `match` on `Resume` or `Refusal` stops compiling (arms only, no behaviour).

## Read first
- `docs/design-notes/m6-conflicts.md`, Amendment 2: Need (his words on finishing), Interfaces (the `fix-ops-conflict-set-and-summary` block), Hard choices 2 and 3, New UI "Finish summary" (what each field is for).
- `crates/graft-ops/MAP.md` §apply (`Resume`, `resume`, `live_entry`), §snapshot (`backup_chain`, `recorded_index_state`, `stage_entries`), §conflict, §facts (`RepoFacts`); `crates/graft-git/MAP.md` §diff (`DiffSpec::Staged`), §state (`InProgress`, `in_progress_detail`).
- **Scout questions:**
  1. Every exhaustive `match` on `Resume` and `Refusal` across the workspace.
  2. How `resume` builds its invocation, and where `GIT_EDITOR=true` is set.
  3. Whether a name-status and numstat helper over `DiffSpec::Staged` already exists in graft-git.

## Scope
In:
1. **`operation_conflicts(repo, facts)`**: the operation's set of conflicted paths. Take the union, deduplicated, in this order:
   - (a) the paths unmerged now, in git's status order;
   - (b) the `Conflicts:` list git writes as comment lines into `$GIT_DIR/MERGE_MSG`, in file order. Use the comment character from `core.commentChar`, default `#`; for `auto`, try `#` and say so in NOTES;
   - (c) the paths holding a stage ≥ 1 in any `IndexState::Unmerged` on the live entry's backup chain.

   `recorded = (b) found || (c) non-empty`. **Verify (b) in throwaway repos** for merge, pull, rebase (merge backend), cherry-pick, revert and stash pop, and write in NOTES which kinds write it. Do not assume.
2. **`finish_summary(repo)`**, for merges only. Otherwise `NotMerging`. With anything unmerged, `StillUnmerged`.
   - `into`: the current branch's short name, or a short sha when detached.
   - `from`: the name in `MERGE_MSG`'s first line (`Merge branch 'feature'` → `feature`; `… of <url>` kept whole), else `MERGE_HEAD`'s short sha.
   - `by_you`: `operation_conflicts`' paths, each `ResultIs`, found by comparing the staged blob with `HEAD:path` and `MERGE_HEAD:path` (absent in the index → `Deleted`).
   - `by_git`: `DiffSpec::Staged`'s changed paths minus `by_you`, with change kind and added/removed line counts (binary → 0/0).
   - `message`: `MERGE_MSG` with comment lines stripped by the same comment-character rule.
   - `recorded`: as in item 1. When it is false, `by_you` is empty and every staged path is in `by_git`.
3. **`Resume::ContinueWith { message }`**:
   - A merge runs `git commit --cleanup=strip -F <scratch>/MSG` with `GIT_EDITOR=true`, and the entry is closed the same way `Continue` closes it (`Committed`, or `Conflicted` again is impossible for a merge; any failure goes through `ResumeFailed`).
   - Any other kind is refused before anything runs: `Refusal::MessageNotHere { kind }`, worded `a <kind> keeps each commit's own message · c continues`.
   - `Continue` is unchanged, byte for byte.
   - No write to `MERGE_MSG`, ever.

Out: drawing anything; the app's state; P4's `FinishMerge` (it reuses the `-F` path; its own ticket).

## Signatures
```rust
// graft-ops/src/conflict.rs
pub struct ConflictSet { pub paths: Vec<PathBuf>, pub recorded: bool }
pub enum ConflictSetError { Io(io::Error), Run(RunError) }
pub fn operation_conflicts(repo: &RepoHandle, facts: &RepoFacts) -> Result<ConflictSet, ConflictSetError>;
// graft-ops/src/finish.rs (new; re-exported from lib.rs)
pub struct FinishSummary { pub into: Vec<u8>, pub from: Vec<u8>, pub by_you: Vec<ByYou>, pub by_git: Vec<ByGit>, pub message: Vec<u8>, pub recorded: bool }
pub struct ByYou { pub path: PathBuf, pub result: ResultIs }
pub enum ResultIs { Yours, Theirs, Deleted, Combined }
pub struct ByGit { pub path: PathBuf, pub change: Change, pub added: usize, pub removed: usize }
pub enum Change { Added, Modified, Deleted, Renamed { from: PathBuf }, TypeChanged }
pub enum SummaryError { NotMerging, StillUnmerged { count: usize }, Facts(FactsError), Run(RunError), Io(io::Error) }
pub fn finish_summary(repo: &RepoHandle) -> Result<FinishSummary, SummaryError>;
// graft-ops/src/apply/resume.rs
pub enum Resume { Continue, ContinueWith { message: Vec<u8> }, Skip, Abort }
// graft-ops/src/operation.rs
pub enum Refusal { /* existing */ MessageNotHere { kind: InProgress } }
```

## Acceptance (name the verifying test when you tick; truth from git)
- [x] `tests/it/conflict_set.rs::a_merge_lists_resolved_files_too` — Graft's merge stops on `a.txt`+`b.txt`, `a.txt` staged through Graft; `paths == [b.txt, a.txt]` (unmerged first, then git's list), `recorded`.
- [x] `conflict_set.rs::a_relaunch_lists_the_same_files` — resolved in the shell, read twice, each with a fresh `RepoHandle` and fresh `read_facts`; same paths, `recorded`.
- [x] `conflict_set.rs::each_kind_says_what_git_records` — merge, pull, rebase, cherry-pick, revert each: `a.txt` resolved in the shell, set = `[b.txt, a.txt]`, `recorded`; stash pop: no `MERGE_MSG`, set = `[a.txt]`, `recorded: false`. Extra: `a_path_with_a_space_is_read_whole`, `the_comment_character_is_git_s` (`core.commentChar=;`), `a_graft_step_is_remembered_without_git_s_list` (source (c) alone), `nothing_in_progress_is_an_empty_unrecorded_set`.
- [x] `tests/it/finish.rs::summary_splits_yours_from_gits` — fixture has two conflicting files (`a.txt` theirs, `b.txt` hand-combined) and one clean (`clean.txt`): `by_you == [a.txt Theirs, b.txt Combined]`, `by_git == [clean.txt Modified +n −m]` with n, m from `git diff --cached --numstat -- clean.txt`; also `into`, `from`, `message`. Extra: `yours_and_deleted_are_read_from_the_index`, `a_shell_merge_names_its_branch_and_a_pull_its_source`, `detached_into_is_a_short_id`, `unrecorded_puts_every_change_under_git`.
- [x] `finish.rs::summary_refuses_with_anything_unmerged` (`StillUnmerged { count: 2 }`) and `finish.rs::summary_is_merge_only` (nothing in progress, and a stopped Graft rebase → `NotMerging`).
- [x] `finish.rs::an_edited_message_is_the_commit_message` — a `pre-commit` hook refuses first: `ResumeFailed { kind: Merge }`, `MERGE_MSG` bytes and mtime unchanged; hook removed, `ContinueWith` again: `Committed`, command `git commit --cleanup=strip -F -`, `git log -1 --format=%B` == `git stripspace --strip-comments` of the message, `HEAD^2` == `feature`, `MERGE_MSG` gone (git removed it).
- [x] `finish.rs::continue_is_unchanged` — `Continue` records `git merge --continue` and commits git's own message; and the existing resume tests (`history_ops.rs`, `chained_backup.rs`, `shell_begun_merge.rs`) are unedited (`git diff` shows no change to them) and green in the workspace nextest run.
- [x] `finish.rs::a_message_on_a_rebase_is_refused_before_anything_runs` — `Refused(MessageNotHere { kind: Rebase })`, words `a rebase keeps each commit's own message · c continues`, `state_of` and every ref (backup refs included) unchanged, still rebasing.

## NOTES (builder, 2026-09-23)
- **Scout used**: `.scratch/m6-conflicts/scout-09.md`. No compaction.
- **What git records (item 1(b)), measured in throwaway repos, git 2.54.0 (Apple Git-157)**: merge, pull (`--no-rebase`), rebase (merge backend), cherry-pick, revert all write `<c> Conflicts:` then `<c>\t<path>` per file into `$GIT_DIR/MERGE_MSG`, path unquoted (`b c.txt` stays as is). Stash pop writes no `MERGE_MSG`. `core.commentChar=;` → git writes `; Conflicts:`, and we read it. `auto` → git wrote `#` for a merge; we read `auto` as `#` (not a general implementation of git's `auto` choice). `core.commentString` (git ≥ 2.45 synonym) is **not** read.
- **Scout contradictions, resolved in scope:**
  - `Resume` dropped `Copy` (keeps `Clone, PartialEq, Eq`). `resume` still takes it by value; `close_resume` now takes `aborting: bool`. Cost outside graft-ops: arms in `graft/src/update/banner.rs:464` and `graft/src/jobs/runner.rs:650` (`ContinueWith` → verb `continue`), **and one non-arm line** in `graft/src/update/banner/tests.rs:729` (`matches!(&confirm.then, … if *what == want)`) — it moved out of a non-`Copy` value and would not compile. Test code only, no behaviour.
  - `Continue` on a merge still runs `git merge --continue`; only `ContinueWith` runs `git commit`.
  - `lib.rs` has no `pub use` style: `finish` is `pub mod finish;` like every other module; `conflict`'s new items are `pub use`d from `conflict/set.rs` so their path is `graft_ops::conflict::…` as specced.
- **Deviations from the spec's signatures (say so, not silent):**
  1. `-F -` with the message on stdin instead of `-F <scratch>/MSG`: same `commit -F` route (hard choice 3 intact, `MERGE_MSG` untouched), no temp file to write, fail or clean up. P4's `FinishMerge` can reuse it the same way. Conductor: say if the file matters.
  2. `ConflictSetError` gained `Facts`, `Journal`, `Snapshot`; `SummaryError` gained `Conflicts(ConflictSetError)`, `Diff(DiffError)`. Reading the live entry needs the journal file and its parse, and (c) walks the chain; the spec's two variants could not carry those failures without mislabelling them.
  3. (b) and (c) are read only while git has an operation in progress (a `MERGE_MSG` with nothing in progress describes no current operation). The spec did not say either way.
  4. `from` for a pull: `Merge branch 'main' of <url>` → `main of <url>` (the reading of "kept whole" I took).
- **Test-fixture cause found and fixed**: pull tests first built the downstream as a second `two_file_conflict()`. Commit ids match only when both repos are built in the same second, so `pull` intermittently refused "unrelated histories" (seen 3/6 runs). Fixed at the cause: `conflict_fixtures::downstream_of` fetches upstream's history; stress-run 15× clean.
- **Not verified**: `ContinueWith` from the app (no caller yet — its ticket); `Renamed`/`TypeChanged`/binary rows in `by_git` (mapping only, no test); a non-UTF-8 path.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: two read functions, one resume variant, ~9 integration tests on throwaway repos. One context.

# fix-ops-conflict-set-and-summary: git tells us which files this operation conflicted on, and what a merge will commit; an edited message reaches git through `commit -F`

Status: draft · new 2026-09-23 (Amendment 2). Interface-first: the file list, the finish summary and P4 call what this exposes.
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
- [ ] `tests/it/conflict_set.rs::a_merge_lists_resolved_files_too`: a two-file merge stops, one file is resolved and staged, and `paths` = both, in order, with `recorded`.
- [ ] `…::a_relaunch_lists_the_same_files`: the same, read with a fresh `RepoHandle` and no app state.
- [ ] `…::each_kind_says_what_git_records`: one case per kind in item 1, asserting what NOTES says.
- [ ] `tests/it/finish.rs::summary_splits_yours_from_gits`: `feature` changes `a.txt` (conflicting) and `b.txt` (clean). After resolving `a.txt` with theirs, `by_you` = `[a.txt Theirs]` and `by_git` = `[b.txt Modified +n −m]`, where n and m equal `git diff --cached --numstat`.
- [ ] `…::summary_refuses_with_anything_unmerged` and `…::summary_is_merge_only`.
- [ ] `…::an_edited_message_is_the_commit_message`: `ContinueWith` → `git log -1 --format=%B` equals the message after `--cleanup=strip`, `HEAD^2` = `feature`, `MERGE_MSG` was never written (mtime and bytes unchanged until git removes it).
- [ ] `…::continue_is_unchanged`: the existing resume tests are green with no edit.
- [ ] `…::a_message_on_a_rebase_is_refused_before_anything_runs`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: two read functions, one resume variant, ~9 integration tests on throwaway repos. One context.

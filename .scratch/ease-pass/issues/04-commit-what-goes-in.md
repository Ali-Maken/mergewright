# commit-what-goes-in — "nothing picked means everything goes in", secrets left out, one undoable write

Status: done (built 2026-10-17; was ready, revised 2026-10-17, fitted to `.scratch/ease-pass/scouts/04-commit-what-goes-in.md`; its file:line facts stand)
Blocked by: none
Build with: Opus — scout recommends builder-opus: one struct field, two pure functions and a small executor change (scratch index), every index state checked by fixture tests (`git ls-files -s`), not by eye.
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (commit-*), Mock behaviours C4–C7, C13–C14, Prior art rows "Commit start", "In vs out", "Secrets file", Questions 9 (answered a).
- `docs/research/commit-stash.md` §1.4–1.5.
- `crates/graft-ops/MAP.md`: `CommitParams`, `Operation`, `Operation::plan`, `Invocation`, `apply`, `RepoFacts`, `Refusal`, journal/undo entries. `crates/graft-git/MAP.md`: `WorkingTreeStatus`, `StatusEntry`.
- The scout's file (above).

## What the scout changed (decided here)
- `CommitParams.message` stays `String` (`operation/types.rs:57`); `subject()`/`message_bytes()` unchanged.
- There is no amend params struct: `Operation::Commit(CommitParams)` and `Operation::Amend(CommitParams)` share it (`operation.rs:80,82`). Adding `include` covers both.
- `u` already restores HEAD **and** index (`apply/record.rs:120,266-275`; `undo.rs:55-62,300-335`, `RestoreIndex` over `ChangedBetween`), worktree untouched by design (`touches` `worktree=false`, `check.rs:207-208`). **No undo work here** — the tests below prove the new write is covered by today's steps.
- `Invocation` has no env (`types.rs:102-110`), so a plan cannot point git at a scratch index today. **This ticket adds the executor change** (below). `GIT_INDEX_FILE` is already set by the snapshot code (`snapshot/plumbing.rs:117,132`, `snapshot/raw.rs:172,211`): reuse its path and env handling, don't write a second one.
- `Refusal::NothingChosen` is new (`refusal.rs:33` has `NothingStaged`), with words in `explain.rs` beside `:55`.
- `StatusEntry` carries no index object ids (graft-git MAP.md: `path`, `original_path`, `staged`, `unstaged`, `untracked`), so the scratch index is seeded as a **copy of the real index**, not `read-tree HEAD` + ids: staged parts need no ids that way.

## Scope
In: graft-ops; plus, in graft, only the two production `CommitParams` literals and the four graft test literals listed under Touches (add `include: Include::Staged`, a compile fix — S gives it meaning later). graft-git only if the git runner `apply` uses cannot take an environment variable and the snapshot code's route can't be reused — then stop and report `blocked: needs graft-git <signature>`.
- `Include` on `CommitParams`; `first_choice` (pure); `looks_like_secrets` (pure).
- **Executor:** `Invocation.index: IndexFile`. When a plan has a `Scratch` line, the executor copies the real index (the path git reports for `index`, so linked worktrees work) to one scratch file in the app's names dir before the first `Scratch` line, runs every `Scratch` line with `GIT_INDEX_FILE=<scratch>`, and removes the file when the plan ends, success or failure. A failed removal is reported in the outcome, never ignored; the next run overwrites a leftover. `command_line` spells a `Scratch` line with a leading `GIT_INDEX_FILE=<scratch> ` so the shown line is runnable. Every existing `Invocation` gets `IndexFile::Real` (behaviour unchanged).
- **Plan for `Include::Chosen`** (commit and amend; every path `--literal-pathspecs`):
  1. Scratch: `git reset -q -- <paths staged but not chosen>` (unborn HEAD: `git rm --cached -q --`); skipped when none. A staged rename names both paths.
  2. Scratch: `git add -A -- <paths chosen Whole>` (modified, deleted, untracked alike); skipped when none. `StagedPart` paths stay as the copy has them — the staged content.
  3. Scratch: today's `git commit -F -` line (plus `--amend`, `--signoff`, `--no-verify`, `--allow-empty` as set). Hooks see the scratch index, as with `git commit <paths>`.
  4. Real: `git reset -q -- <chosen paths>` — the real index now equals the new HEAD for the chosen paths; every other path's entry untouched.
  A hook failure stops at 3: HEAD and the real index untouched.
- **`Include::Staged`** = today's plan exactly (real index, one line). A `Chosen` set equal to the staged set (every staged path, each `StagedPart`, nothing else) is planned as `Staged`, so a commit of what he staged himself runs today's line and today's hooks.
- **Amend** with `Include::Chosen`: new HEAD = old HEAD's tree + chosen paths, old HEAD's parents (step 1 resets to the HEAD being amended). `Include::Staged` = today's amend (nothing staged → message-only).
- **Checks:** `Include::Staged` keeps `NothingStaged` (`commit.rs:19-27`). `Include::Chosen(vec![])` with `allow_empty` off → `NothingChosen` (commit and amend). Words: `nothing goes in · put a file in first` (the commit screen's C9 bar words; S refuses before ops is asked, so this is the backstop). Amend keeps `UnbornHead`, `EmptySubject`.
- Touches (`check.rs:207-208`) and describe words (`queries.rs:284-285`) unchanged.
Out: the commit screen (S); keys (V); the message draft kept after `u` (S, Q9 a).

## Signatures
```rust
// graft-ops operation/types.rs
pub enum Include { Staged, Chosen(Vec<Chosen>) }
pub struct Chosen { pub path: PathBuf, pub part: Part }
/// The note's `Take`; renamed so it never meets `graft_git::Take` in one file.
pub enum Part { Whole, StagedPart }
pub struct CommitParams { pub message: String, pub sign_off: bool, pub no_verify: bool, pub allow_empty: bool, pub include: Include }
/// Which index a git line reads and writes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IndexFile { #[default] Real, Scratch }
pub struct Invocation { /* existing: args, stdin, remote — types unchanged */ pub index: IndexFile }
// operation/refusal.rs
pub enum Refusal { /* existing */, NothingChosen }
// operation/commit.rs (or a new `operation/choice.rs` — builder's call, MAP.md updated)
pub enum OutWhy { LooksLikeSecrets, NotStaged }
pub struct FileChoice { pub path: PathBuf, pub goes_in: bool, pub part: Part, pub why_out: Option<OutWhy> }
/// Nothing staged → every change goes in (`Whole`) except new untracked files where
/// `looks_like_secrets`; something staged → only staged files (`StagedPart`), rest `NotStaged`.
/// A partially staged file (`staged` and `unstaged` both `Some`, `graft-git/src/status.rs:142-162`) counts as staged.
pub fn first_choice(status: &WorkingTreeStatus) -> Vec<FileChoice>;
/// Case-insensitive basename: `.env`, `.env.*` (not `.example`/`.sample`/`.template`), `*.pem`, `*.key`,
/// `*.p12`, `*.pfx`, `id_rsa`/`id_dsa`/`id_ecdsa`/`id_ed25519` (not `.pub`), `.npmrc`, `.pypirc`, `.netrc`,
/// `credentials.json`, `*.keystore`, `*.jks`, `secrets.*`, `.htpasswd`.
pub fn looks_like_secrets(path: &Path) -> bool;
```

## Touches
- Every `CommitParams` construction (scout list): production `graft/src/update/commit_editor.rs:153` (`params()`) and `:340` (amend at an `edit` stop); tests `graft-ops/src/operation/tests.rs:111,228,238,563,1208`, `operation/remote/tests.rs:99`, `graft-ops/tests/it/{aborted_fetch,edit_stop,error_rows,journal_tie,ops_execution_hard,ops_execution,remote_ops,undo_gate}.rs`, `graft/src/update/operate/tests/{landing.rs:132,lands_on_head.rs:27,staging_not_kept.rs:102,166}` — all `Include::Staged`.
- Every `Invocation` construction (all plans): gains `index: IndexFile::Real`, or the existing helper sets it; count in NOTES.
- Matchers on Commit/Amend (`queries.rs:17-18,87-88`, `types.rs:209`, `graft/src/update/operate.rs:205-206`, `operate/outcome.rs:60`): checked, unchanged unless they destructure `CommitParams`.
- Journal/undo: `index_before`/`index_after` recorded as today. Recovery of a `Pending` commit stopped between steps 3 and 4 (HEAD moved, real index not yet reset): NOTES says what today's recovery does there; tested if a fixture can stop the plan there, else NOTES says **unverified**.
- Snapshot code (`snapshot/plumbing.rs`, `snapshot/raw.rs`): shared helper only; its tests pass untouched.
- `explain.rs` refusal words; graft-ops MAP.md (`Invocation`, `IndexFile`, `Include`, `first_choice`, `looks_like_secrets`, `NothingChosen`).

## Acceptance
- [x] `first_choice`: nothing staged, 3 modified + new `.env` → 3 in `Whole`, `.env` out `LooksLikeSecrets`; one staged → that one in `StagedPart`, rest `NotStaged`; a partially staged file counts as staged. — **graft-ops/src/operation/choice.rs: `nothing_staged_puts_every_change_in_but_a_new_secrets_file`, `something_staged_puts_only_staged_files_in`, `a_partially_staged_file_counts_as_staged`**
- [x] `looks_like_secrets` table test incl. `.env.example` false, `id_rsa.pub` false, `ID_RSA` true. — **graft-ops/src/operation/choice.rs: `looks_like_secrets_table`**
- [x] Fixture: chosen `Whole` on a partially staged file commits the working file; `StagedPart` commits the staged content; an out file's staged entry is identical before/after (`git ls-files -s` line compared). — **graft-ops/tests/it/commit_chosen.rs: `whole_on_a_partly_staged_file_commits_the_working_file_and_leaves_the_rest_staged`, `staged_part_commits_the_staged_content_only`**
- [x] Deleted and untracked chosen paths commit as removal / addition; afterwards `git status` shows them clean. — **graft-ops/tests/it/commit_chosen.rs: `deleted_and_untracked_files_chosen_commit_as_removal_and_addition`**
- [x] `Chosen` equal to the staged set plans exactly today's single line (plan test). — **graft-ops/src/operation/tests/chosen.rs: `a_choice_of_exactly_the_staged_set_plans_todays_one_line`**
- [x] `u` after a `Chosen` commit: HEAD, `git ls-files -s` and working tree equal to before (today's undo steps, no new code). — **graft-ops/tests/it/commit_chosen.rs: `undo_after_a_chosen_commit_puts_head_index_and_files_back`**
- [x] Amend with `Include::Chosen([a])`: new HEAD has old HEAD's parents and old tree + `a`; `u` restores HEAD and index. Amend with `Include::Staged` and nothing staged: tree unchanged, message changed. — **graft-ops/tests/it/commit_chosen.rs: `amend_of_a_chosen_file_keeps_the_parents_and_adds_only_that_file`, `amend_of_what_is_staged_with_nothing_staged_rewords_only`**
- [x] `NothingChosen` refusal with its words; a failing `pre-commit` hook leaves HEAD and `git ls-files -s` untouched and no scratch file behind. — **graft-ops/src/operation/tests/chosen.rs: `nothing_chosen_is_refused_for_commit_and_amend`; graft-ops/tests/it/commit_chosen.rs: `a_failing_pre_commit_hook_leaves_head_and_the_index_and_no_scratch`**
- [x] `command_line` of a `Scratch` line starts `GIT_INDEX_FILE=`; every existing plan test passes unchanged. Each tick names its test. — **graft-ops/src/operation/tests/chosen.rs: `the_logged_line_names_the_real_scratch_path`, `a_choice_runs_on_the_scratch_index_then_resets_the_chosen_paths`; existing plan tests unchanged, workspace nextest 6511/6511**

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~65k). Split point if it overruns: executor `IndexFile` + plan here; `first_choice` / `looks_like_secrets` (pure, no executor) as a follow-on.

## NOTES (builder, 2026-10-17)
No compaction. Scout used (the ticket's).
- **Executor** (`apply/run.rs`): `run_plan` copies the real index via the snapshot code's `TempIndex` (new `copied_as` fixed name `commit-index` in the names dir, keeps mtime — the racy-clean fix; new `remove` returns the error) before the first `Scratch` line; `GIT_INDEX_FILE` set per `Scratch` line; removed after the plan, success or failure. Failed removal → new `AfterState::ScratchIndexLeft { path, error }` in `cleanup` (outcome reason / `unread`).
- **Command Log**: `command_line()` has no path, so a `Scratch` line spells `GIT_INDEX_FILE=<scratch index> git …` (preview, plan tests); `apply.rs` and `fold.rs` log `command_line_in(<names dir>/commit-index)` — runnable (`the_command_log_names_the_scratch_index_it_ran_on`).
- `Invocation` literals: 4 (`clone.rs`, `operation/remote.rs`, `apply/remote/tests.rs`, `apply/tests/stop_hooks.rs`) + the 2 constructors `new`/`with_stdin` set `IndexFile::Real`.
- `CommitParams` and `Include` derive `Default` (`Include::Staged`): used once at `graft/src/update/commit_editor.rs` amend-at-edit-stop, to keep that file under 400 lines. Other literals gained `include: Include::Staged`.
- File-size ratchet rows raised (`over_400.txt`): `apply/error.rs` 570, `operation/refusal.rs` 425, `operation/tests.rs` 2010, and five test files +1..+4.
- Unmerged entries are left out of `first_choice` (no commit runs while one is open). An untracked secrets file when something *is* staged is `NotStaged`, per spec — the screen loses the secrets hint there.
- A copy failure before the first `Scratch` line surfaces as `RunError::Spawn("the index could not be copied for the commit: …")`, which reads "could not run git: …". Symptom-level wording; a typed error would need a new `ApplyError` path.
- **Verified extra**: hooks see the scratch index (`hooks_see_the_scratch_index`); a leftover file is overwritten (`a_leftover_scratch_index_is_overwritten`); linked worktree commits from its own index (`a_linked_worktree_commits_from_its_own_index`); step 4 failing after the commit landed closes the entry `Aborted` with git's words and `u` restores HEAD, index and files exactly (`a_failed_reset_after_the_commit_is_undone_by_u`).
- **Unverified**: a *Pending* entry left by a process that died between steps 3 and 4 — today `recover_interrupted` closes it `interrupted` with the state read then (HEAD moved, real index not reset); no fixture can kill the process there. Also unverified: the `ScratchIndexLeft` path (no fixture fails the removal without also failing the after-state's own temp index in the same dir).

# git-reads-and-patches: porcelain status, staged/unstaged diffs, the hunk patch builder, stash list, in-progress state, and every spawn through `run`

Status: done — all 8 pieces committed (see NOTES below for the two-session history)
Blocked by: none
Build with: Sonnet — parsers with oracle tests against real git, plus small `graft-git` additions; mechanical by CLAUDE.md's table.
Needs: none (`ops-model` and `ops-execution` call these; `changes-pane` calls `status::read`, `DiffSpec::{Staged, Unstaged}`, `hunk_patch`).

## Read first

- `docs/design-notes/m3-working-tree-undo-and-stash.md` — Interfaces (`graft-git` additions paragraph); Proposed changes 1, 8
- `PROMPT.md` Technology "Writing to the repo" (lines 51–54: `--porcelain=v2 -z`, `core.quotepath=off`, typed args)
- `docs/reviews/m2-details-and-diffs.md` "R2 Major 1" (lines 621–627) and "Fixes, round 1e" opening paragraph — why a read must not write the index, and that only `git status` honours `GIT_OPTIONAL_LOCKS=0`
- `KNOWN_GAPS.md` "m2-details-and-diffs/01 — git spawns outside the runner" (line 332) — the two spawns to move
- `crates/graft-git/MAP.md` §run, §diff (`DiffSpec`, `list_files`, `file_diff`, `parse_unified`, `Hunk`, `DiffLine`), §status, §repo (`RepoHandle`, `git_dir`, `common_dir`), §version, §commit_graph
- `crates/graft-fixtures/MAP.md` — `Shape`, `copy_into_temp`, `git` (the hermetic runner)

## Scope

In — in this order, one commit each:

1. `GitInvocation::env(self, key, value) -> Self` in `run.rs` (adds to the child's env after the fixed ones; used for `GIT_INDEX_FILE`, `GIT_AUTHOR_*`). Move `version::system_git` and `commit_graph::write` onto `run` (`commit_graph::write` uses `--git-dir` and a `()` stop, as the KNOWN_GAPS row says). Closes that row.
2. `status.rs`: `WorkingTreeStatus`, `StatusEntry`, `Change`; `parse_porcelain_v2` (pure; one `cargo-fuzz` target beside the M2 ones); `read` = one spawn `git -c core.quotepath=off status --porcelain=v2 -z --untracked-files=all --renames` through `run` (`GIT_OPTIONAL_LOCKS=0` is already set by the runner). Keep `counts`; reimplement it over `read` if that is simpler, but its signature stays.
3. `diff/read.rs`: `DiffSpec::Staged` (index vs `HEAD`; on an unborn head, vs the empty tree `4b825dc642cb6eb9a060e54bf8d69288fbee4904`, via `diff-index --cached`) and `DiffSpec::Unstaged` (worktree vs index, via `diff-files`), for both `list_files` and `file_diff`; an untracked path under `Unstaged` still returns the hunk-less `FileDiff` M2 defined. The index-untouched test from round 1e is extended to both specs.
4. `diff/patch.rs`: `Selection`, `hunk_patch` — builds a complete one-file unified patch (`--- a/<path>` / `+++ b/<path>`, or `/dev/null` for added/deleted files, mode lines when `old_mode != new_mode`) holding one hunk. `Lines(idx)`: unselected `Added` lines are dropped, unselected `Removed` lines become `Context`, counts in the `@@` header recomputed; `WholeHunk` reproduces git's own hunk byte-for-byte. Paths are bytes; `\ No newline at end of file` markers preserved.
5. `stash.rs`: `StashEntry { index, commit, message, at, paths }`, `list` — reads `refs/stash`'s reflog newest-first through gix (read-only; no `git stash list` spawn) and each stash commit's touched paths via one `diff-tree --name-only -r -z <stash>^ <stash>` (+ `<stash>^3` for the untracked commit when present) spawn per entry, capped at 50 entries.
6. `state.rs`: `InProgress`, `in_progress` — presence of `MERGE_HEAD`, `rebase-merge/`, `rebase-apply/`, `CHERRY_PICK_HEAD`, `REVERT_HEAD`, `BISECT_LOG` under `RepoHandle::git_dir()`; a plain fs check, no spawn.
7. `repo.rs`: `RepoHandle::git_path(&self, relative: &Path) -> Result<PathBuf, RunError>` (`git rev-parse --git-path <relative>`, absolute result) and `RepoHandle::worktree_name(&self) -> String` (`"main"` when `git_dir == common_dir`, else the last component of `git_dir`).
8. `crates/graft-git/MAP.md` lines for every item above.

Out: any write to the repository, any `ops` code, any UI, `add -N` intent-to-add (M7 gap, note change 8).

**Verify in a throwaway repo before coding** (the M2 lesson: test the claim, don't trust the doc): (a) `git status --porcelain=v2 -z` leaves `.git/index` bytes unchanged on a stat-dirty tree; (b) `diff-files` / `diff-index --cached` likewise; (c) the v2 record shapes for a rename (`2 R. …`), an unmerged path (`u …`), an untracked path (`? …`), a submodule (`1 .M S…`); (d) `git apply --cached` accepts the built line-selected patch (a test in this group applies one and checks `diff-index --cached` shows exactly the selected lines).

## Signatures

```rust
// crates/graft-git/src/run.rs
impl GitInvocation { pub fn env(self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self; }

// crates/graft-git/src/status.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Change { Added, Modified, Deleted, Renamed, Copied, TypeChanged }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct StatusEntry { pub path: PathBuf, pub original_path: Option<PathBuf>, pub staged: Option<Change>, pub unstaged: Option<Change>, pub untracked: bool, pub unmerged: bool, pub submodule: bool }
impl StatusEntry { pub fn is_staged(&self) -> bool; pub fn is_unstaged(&self) -> bool; }
#[derive(Clone, Debug, PartialEq, Eq, Default)] pub struct WorkingTreeStatus { pub entries: Vec<StatusEntry> }
impl WorkingTreeStatus { pub fn staged(&self) -> impl Iterator<Item = &StatusEntry>; pub fn unstaged(&self) -> impl Iterator<Item = &StatusEntry>; pub fn untracked(&self) -> impl Iterator<Item = &StatusEntry>; pub fn is_clean(&self) -> bool; pub fn counts(&self) -> WorkingTreeCounts; pub fn entry(&self, path: &Path) -> Option<&StatusEntry>; }
pub fn parse_porcelain_v2(bytes: &[u8]) -> Result<WorkingTreeStatus, StatusError>;   // pure, fuzz target
pub fn read(repo: &RepoHandle, stop: &dyn Stop) -> Result<WorkingTreeStatus, StatusError>;

// crates/graft-git/src/diff/read.rs   (DiffSpec gains two variants; existing ones unchanged)
pub enum DiffSpec { Commit { commit: Oid, against: Option<Oid> }, Range { base: Oid, head: Oid }, WorkingTreeVsHead, Staged, Unstaged }

// crates/graft-git/src/diff/patch.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub enum Selection { WholeHunk, Lines(Vec<usize>) }   // indices into `Hunk::lines`, Added/Removed only
#[derive(Debug, PartialEq, Eq)] pub enum PatchError { NoSuchHunk { hunk: usize }, NoSuchLine { line: usize }, NotAChangedLine { line: usize }, Binary, NothingSelected }
pub fn hunk_patch(diff: &FileDiff, hunk: usize, selection: &Selection) -> Result<Vec<u8>, PatchError>;

// crates/graft-git/src/stash.rs
#[derive(Clone, Debug, PartialEq, Eq)] pub struct StashEntry { pub index: usize, pub commit: Oid, pub message: String, pub at: Timestamp, pub paths: Vec<PathBuf> }
pub fn list(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<StashEntry>, StashError>;
pub enum StashError { Reflog(String), Run(RunError), Stopped }

// crates/graft-git/src/state.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum InProgress { Merge, Rebase, CherryPick, Revert, Bisect }
impl InProgress { pub fn name(self) -> &'static str; pub fn continue_command(self) -> &'static str; pub fn abort_command(self) -> &'static str; }
pub fn in_progress(repo: &RepoHandle) -> Result<Option<InProgress>, std::io::Error>;

// crates/graft-git/src/repo.rs
impl RepoHandle { pub fn git_path(&self, relative: &Path) -> Result<PathBuf, RunError>; pub fn worktree_name(&self) -> String; }
```

## Acceptance

- [ ] Oracle test: for every `Shape` copied into a temp dir with a scripted dirty state (one staged edit, one unstaged edit, one staged+unstaged, one untracked, one staged rename, one deleted), `read().counts()` equals `git status --porcelain=v2` counted by the test itself, and `entry(path)` states match the record letters.
- [ ] `.git/index` bytes are identical before and after `status::read`, `list_files(Staged)`, `list_files(Unstaged)`, `file_diff(Staged, …)`, `file_diff(Unstaged, …)` on a stat-dirty tree (touch every file first). One test, five spawns, asserted by bytes as round 1e's test does.
- [ ] `hunk_patch(WholeHunk)` for every hunk of every corpus shape's first commit reproduces the corresponding bytes of `git diff-tree -p` (the same oracle the M2 parser test uses); `Lines` selecting one added line of a two-line hunk yields a header with recomputed counts, and `git apply --cached` of it on a temp copy stages exactly that line (`diff-index --cached -p` contains it and not the other).
- [ ] `stash::list` on a temp copy with two `git stash push -u` entries returns 2 entries newest first, messages equal to git's, `paths` equal to the files stashed (untracked one included); on a repo without stashes → `Ok(vec![])`.
- [ ] `in_progress` returns `Some(Merge)` after a scripted conflicting merge on a temp copy and `None` after `git merge --abort`.
- [ ] `git_path("graft/undo-journal.jsonl")` on a linked worktree (temp copy + `git worktree add`) resolves under `.git/worktrees/<name>/`; `worktree_name` is that `<name>` there and `"main"` in the main worktree.
- [ ] The fuzz target builds (`fuzz/` workspace) — run at review only.
- [ ] No `Command::new("git")` remains in `crates/graft-git/src/` outside `run/`.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

~600 lines code (status 220, patch 150, stash 90, state 40, repo/run 60) + ~450 test. Seams: after piece 3, after piece 5. Delegate the "how does M2's `list_files` resolve sides" question to a scout; don't read `diff/read.rs` whole.

## NOTES (handoff at seam after piece 3, 2026-09-19)

Pieces 1–3 done and committed (`081f811e` piece 1+2 status, `113fb0a` piece 3
Staged/Unstaged). Gates green on both `graft-git` and the full workspace
(`cargo fmt --check`, `cargo test --quiet`, `cargo clippy --all-targets -D
warnings`, `cargo build --quiet`). Stopping at the spec's own "seam after
piece 3" rather than continuing into piece 4, per Size note above — pieces
4–7 are a distinct, substantial remaining scope (hunk_patch + fuzz target,
gix-based stash reflog reading, fs-check state, repo.rs git_path/worktree_name)
each needing its own throwaway-repo verification.

Remaining, in order:
4. `diff/patch.rs`: `Selection`, `PatchError`, `hunk_patch`. Verify
   `git apply --cached` behavior on a built `Lines` patch in a throwaway repo
   before coding (spec item (d) under "Verify in a throwaway repo"). Add a
   `parse_porcelain_v2` fuzz target too (scoped in piece 2 but not yet added —
   see gap below) while in `fuzz/`.
5. `stash.rs`: `StashEntry`, `list`. Verified during earlier exploration:
   `git diff-tree --no-commit-id --root --name-only -r -z <stash>^3` is
   required for the untracked-files commit (plain `<stash>^3` prints nothing;
   git suppresses root-commit diffs by default). Read `refs/stash` reflog via
   gix, no `git stash list` spawn.
6. `state.rs`: `InProgress`, `in_progress` — plain fs check, no spawn.
7. `repo.rs`: `git_path`/`worktree_name`. Verified: `GitArg::Path` cannot be
   used for `--git-path <value>` — its auto-`--`-insertion breaks the
   invocation (`git rev-parse --git-path -- x` → "fatal: bad revision").
   Use two `GitArg::Word` tokens instead. Also verified: `git rev-parse
   --git-path` prints a path relative to `-C` in the main worktree but
   absolute in a linked worktree — join with `workdir().unwrap_or_else(git_dir)`
   only when the result isn't already absolute.
8. Final `MAP.md` sweep for pieces 4–7's public items.

Known gap not yet closed: the `parse_porcelain_v2` fuzz target (scoped in
piece 2) was not added — fold into piece 4's fuzz-target work, or split out
if that's cleaner.

Per CLAUDE.md, builders don't edit KNOWN_GAPS.md — flagging for the
conductor: piece 1 (`GitInvocation::env`, moving `system_git`/
`commit_graph::write` onto `run`) is done and closes KNOWN_GAPS row
`m2-details-and-diffs/01`'s first bullet.

## NOTES (continuation, 2026-09-19)

Pieces 4-8 done, committed: dfa30d6 (piece 4: `diff/patch.rs`
`Selection`/`PatchError`/`hunk_patch`, plus the `parse_porcelain_v2` fuzz
target closing the gap above), 4cd6542 (pieces 6-7: `state.rs`
`InProgress`/`in_progress`; `repo.rs` `git_path`/`worktree_name`), 24f1eab
(piece 5: `stash.rs` `StashEntry`/`StashError`/`list`; piece 8, the final
MAP.md sweep, done incrementally in each piece's own commit rather than
separately).

Verified in throwaway repos before coding, one check each: `git apply
--cached` accepts a minimal patch with no `diff --git`/index line (just
`--- `/`+++ `/`@@`); git omits `,1` from a hunk header when a range's count
is 1 (`@@ -0,0 +1 @@`, not `+1,1`); a scripted conflicting merge sets
`MERGE_HEAD` and `git merge --abort` clears it; `git stash push -m`
prefixes the message with `On <branch>: `.

Gates green throughout: `cargo fmt --check`, `cargo test --quiet`
(workspace), `cargo clippy --all-targets --message-format=short -- -D
warnings`, `cargo build --quiet`; fuzz workspace (`fuzz/`) builds. No
compaction. Two builder sessions total for this group (piece count and
per-piece throwaway-repo verification made it too big for one context, as
the spec's own "Size" note anticipated).

## NOTES (review fix round, 2026-09-19)

Fixed every Critical and Major from `docs/reviews/m3-01-git-reads-and-patches.md`.
Minors left to the conductor.

- **C1** (vacuous porcelain oracle) — cause. `script_six_dirty_states` →
  `script_dirty_states`, which scripts as many of the five tracked states as
  the shape's `git ls-files` count allows (`Option` per state) and always the
  untracked one; the `< 6` skip is gone. The loop now asserts it checked all
  but at most one shape (20 of 21 here; only the non-UTF-8-ref shape may drop)
  and that some shape carried all five states. New
  `all_six_dirty_states_on_a_repository_built_for_them` builds a five-file
  `Fixture` so rename and delete letters are proven whatever the corpus holds.
- **M1** (`Lines` had no reverse form) — cause. New public
  `diff::PatchDirection { Forward, Reverse }`; `hunk_patch` takes it as a
  fourth argument. **Spec deviation:** the spec's signature was three
  arguments; there is no correct reverse patch without a direction, and the
  review recommended this. `Reverse` mirrors the rule (unselected `Added` →
  context, unselected `Removed` → dropped). Proven with real git in temp
  repos, byte-compared: stage one line (`apply --cached`), discard one line
  (`apply -R`), unstage one line (`apply --cached -R`), restore one removed
  line, and that the forward form is still refused by `apply -R`.
- **M2** (no-newline fusion) — cause, in the builder. New
  `PatchError::NoNewlineBoundary { line }`: a `\ No newline at end of file`
  line that the selection leaves mid-side is refused instead of emitted.
  Cost to the user: a single line cannot be staged or discarded out of a hunk
  that ends a file with no trailing newline when the marker would be
  stranded — the whole hunk still works, and is tested to produce git's own
  bytes both ways.
- **M3** (one one-line hunk per shape) — cause. The oracle now also runs on
  each shape's newest non-merge commit with a parent, and on a new
  `awkward_shapes_fixture`: a two-hunk file with a `@@ … @@ fn tail() {`
  heading, a no-trailing-newline file, a rename with an edit, a mode change,
  a deletion and an addition. A counter asserts hunks were actually compared.
  `Lines` header text (`@@ -2,7 +2,8 @@`) is asserted.
- **M4** (`status::read` ignored `truncated`) — cause. `read` now sets
  `output_cap(OUTPUT_CAP)` and goes through `status_from`, which returns
  `StatusError::TooLarge { bytes }`. Split out so the capped case is testable
  without a 64 MiB tree.
- **M5** (`has_untracked_commit` swallowed errors) — cause. Returns
  `Result<bool, StashError>`; new `StashError::Object(String)`, `?`-ed in
  `stash_paths`.

`crates/graft-git/MAP.md` updated for `PatchDirection`, `hunk_patch`'s new
argument, `PatchError::NoNewlineBoundary`, `StatusError::TooLarge`,
`StashError::Object`.

Gates: `cargo fmt --all --check`, `cargo clippy --all-targets
--message-format=short -- -D warnings`, `cargo build --quiet`, `cargo test
--quiet` — all green, 0 failures, 9 ignored. No compaction.

# diff-reads: file list and one file's diff from git, with truncation and cancellation

Status: draft
Build with: Sonnet — composes `git-runner` and `diff-parser`; the decisions (which git command, what truncation means) are made here in the spec.
Needs: `git-runner` (`GitInvocation`, `GitArg`, `Stop`, `RunError`), `diff-parser` (`parse_unified`, `parse_file_list`, `FileDiff`, `FileList`, `Truncated`)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Interfaces (`graft-git::diff`); Hard-to-reverse choice 1; New UI "States" (what truncation and binary must carry)
- `PROMPT.md` §4 Comparing (lines 271–370: ignore-whitespace, context size, rename/copy detection) and §6.3 "huge files truncated with Load more"
- `crates/graft-git/MAP.md` — `run`, `contained`, `diff` sections (after `git-runner` and `diff-parser` land)

## Scope

In:
- `crates/graft-git/src/diff/read.rs`: `DiffSpec`, `DiffOptions`, `DiffError`, `list_files`, `file_diff`.
- Commands (typed args only):
  - files: `diff-tree --no-color --no-ext-diff -M -C -r -z --raw --numstat [--root] [-w] <old> <new>` — one invocation (`--raw --numstat` together), split on the parser side; for `Commit { against: None }` use the first parent, `--root` when there is none; `Range` = `base head`; `WorkingTreeVsHead` = `diff-index --no-color --no-ext-diff -M -C -z --raw --numstat HEAD` plus untracked from `ls-files -z --others --exclude-standard` as `FileStatus::Untracked` entries (no counts).
  - one file: same command family with `-p -U<context> [-w] -- <path> [<old_path>]`, `--no-renames` off; `file_diff` for an `Untracked` entry returns `FileDiff` with no hunks and `truncated: None` (the pane says "untracked · no diff yet").
- Truncation: when a file's hunk lines exceed `max_lines`, keep whole hunks up to the limit, then `truncated = Some(Truncated { shown_lines, total_lines })`; `max_lines == usize::MAX` means all (the `L` key).
- Every command runs with `output_cap` 64 MiB; `truncated` from the runner becomes `DiffError::TooLarge { bytes }`.
- `DiffOptions::default()` = `context: 3, ignore_whitespace: false`.
- Tests on fixtures. MAP.md lines.

Out: word spans, sanitising, caching (app), `--remerge-diff` (radar probe), submodules (row in KNOWN_GAPS if a fixture has one; none does).

## Signatures

```rust
// crates/graft-git/src/diff/read.rs
pub enum DiffSpec { Commit { commit: Oid, against: Option<Oid> }, Range { base: Oid, head: Oid }, WorkingTreeVsHead }
pub struct DiffOptions { pub context: u8, pub ignore_whitespace: bool }    // Default: 3, false
pub enum DiffError { Run(RunError), Parse(ParseError), TooLarge { bytes: usize }, NoHead }
impl std::fmt::Display for DiffError {}                                   // one line for the pane
pub fn list_files(repo: &RepoHandle, spec: &DiffSpec, opts: &DiffOptions, stop: &dyn Stop) -> Result<FileList, DiffError>;
pub fn file_diff(repo: &RepoHandle, spec: &DiffSpec, entry: &FileEntry, opts: &DiffOptions, max_lines: usize, stop: &dyn Stop) -> Result<FileDiff, DiffError>;
```

## Acceptance

- [ ] `list_files(Commit { HEAD of Shape::FeatureBranches, None })` equals the hermetic `git show --stat`'s file set and totals.
- [ ] A merge commit (`Shape::OctopusMerge` tip) lists the diff against its first parent; `Commit { against: Some(second parent) }` lists a different set.
- [ ] `Range { base, head }` on `Shape::Linear` (root → tip) lists every file the tip has.
- [ ] `WorkingTreeVsHead` on `copy_into_temp(Shape::Linear)` after the test writes one modified, one staged-new, one untracked file: three entries with statuses `Modified`, `Added`, `Untracked`.
- [ ] `file_diff` with `max_lines: 10` on a fixture file whose diff has > 10 lines: `hunks` end on a hunk boundary, `truncated == Some(_)`, `total_lines` equals the untruncated count; `usize::MAX` returns them all with `truncated == None`.
- [ ] `context: 0` and `context: 8` change hunk line counts as `git diff -U0`/`-U8` do; `ignore_whitespace` on a whitespace-only change gives no hunks.
- [ ] Renamed file in a fixture (`Shape::FeatureBranches` if it has one, else the test makes one in a temp copy): `entry.old_path` set, `file_diff` runs with both paths and parses.
- [ ] `Stop` pre-set → `DiffError::Run(RunError::Stopped)`, no parse attempted.
- [ ] Non-UTF-8 path fixture: `file_diff` for that entry parses (path bytes round-trip through `GitArg::Path`).
- [ ] MAP.md: five lines added under `diff`.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet`.

## Size

One crate, one file (~220 lines) + tests (~200). Fits: ≤ 18 tool calls. Read MAP.md for the runner and parser, not their source.

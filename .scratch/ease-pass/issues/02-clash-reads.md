# clash-reads — what the merge preview, the pick undo and the look need from git

Status: done
Blocked by: none
Build with: Opus — brownfield reads in one scouted crate; every behaviour is pinned by fixture and property tests, so not Fable (scout recommends `builder-opus`).
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours M2, M3, M12, M17; Interfaces and data shapes.
- `.scratch/ease-pass/scouts/02-clash-reads.md` (whole, ≤1 page) — its file:line facts stand.
- `crates/graft-git/MAP.md`: `history_reads` (`forecast_merge`/`Forecast`, `merge_tree_step`, `compare_span`/`CompareSpan`, `merge_base`, `changed_paths`, `CommitSummary`), `conflict` section (`Region`, `ConflictFile`, `NoSuchBlock`, `parse`, `open`), `FileDiff`, `GitVersion`, `GitInvocation::run_raw`.

## Scope
In (graft-git only):
- `merge_forecast`: the files the merge changes and, per clashing file, how many places clash (`None` when not counted), or not checkable with a reason. New function beside `forecast_merge` in `history_reads/forecast.rs`.
- `ConflictFile::reopen`: a taken block back to open, reusing the existing `NoSuchBlock`.
- `line_origins`: pure, for the read-only look's "from yours / from theirs" sample.
Out: preview drawing (Mp), the pick stack (P), the look (Fr). `forecast_merge`, `Forecast`, `merge_tree_step` and their callers (`graft-ops/src/tour.rs:308`, `graft-ops/src/preview/landing.rs:137`) stay as they are.
**Dropped from the earlier draft (already true in code):** `CommitSummary` keeps `{ id, parents, subject: String, author: Identity }`; the author date is `author.when: Timestamp`, filled by `parse_summary_line` today (`history_reads.rs:206-246`). No struct change, no literal site touched. Mp reads `summary.author.when` and `summary.author.name`.

## How `merge_forecast` reads (decided; scout found `--name-only` gives conflicting names only)
1. Version gate as `forecast_merge` (`forecast.rs:71-81`, no-base form: < 2.38 → not checkable with today's reason).
2. `compare_span(repo, ours, theirs, stop)`: unrelated → `MergeCheck::NotCheckable(UNRELATED.into())` without running merge-tree. **Unrelated histories answer: not checkable**, because `git merge` refuses them by default too, so the preview never promises a merge `y` cannot make.
3. `git merge-tree --write-tree --no-messages -z <ours> <theirs>` **without `--name-only`**, via `run_raw`. Exit 0 → tree id, no clashes. Exit 1 → tree id (first record, kept this time) + conflicted-file records (`<mode> <oid> <stage>\t<path>`, deduplicated by path, order kept). Any other exit, or exit 1 with no tree id → `NotCheckable(first stderr line)` (today's rule, `forecast.rs:124-127`).
4. `changed` = `changed_paths(<merge-base>, theirs)` — the files the merge brings in. Merge-base from step 2's `CompareSpan` if it carries the split point, else `merge_base` (no stop; say which in NOTES). Clashing files appear in it too; Mp subtracts.
5. `places` per clashing path: read the blob `<tree>:<path>` from the written tree (one `git cat-file --batch` for all paths, or one `cat-file blob` each — builder's call, no new module), `parse` it, `Some(file.open())` when parse succeeds and `open() > 0`; parse error, binary, or no markers (modify/delete, rename clash) → `None`.

## Signatures
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClashForecast { pub path: Vec<u8> /* same raw bytes as Forecast::Conflicts.paths */, pub places: Option<usize> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeForecast { pub changed: Vec<Vec<u8>>, pub changed_capped: bool, pub clashes: Vec<ClashForecast> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeCheck { Checked(MergeForecast), NotCheckable(String) }
pub const UNRELATED: &str = "the two branches share no history";

pub fn merge_forecast(repo: &RepoHandle, git: &GitVersion, ours: Oid, theirs: Oid, stop: &dyn Stop)
    -> Result<MergeCheck, RunError>;

impl ConflictFile { pub fn reopen(&mut self, block: usize) -> Result<(), NoSuchBlock>; }   // existing NoSuchBlock (conflict.rs:329)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin { Yours, Theirs, Both }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineOrigin { pub line: u32, pub origin: Origin }
/// Diffs are base→result, base→yours, base→theirs; `line` is 1-based in the result.
pub fn line_origins(result: &FileDiff, yours: &FileDiff, theirs: &FileDiff) -> Vec<LineOrigin>;
```
`changed` uses `changed_paths`' own element type if it is not `Vec<u8>` — one path type for both fields, say which in NOTES; `changed_capped` is `changed_paths`' cap flag. `git` is the version callers already hold (`graft-ops/src/preview/landing.rs:137` passes one to `forecast_merge`). `stop` is the crate's `&dyn Stop` (`run.rs:118`). `block` indexes as `take` does. `reopen` turns `Region::Resolved { block, .. }` back into `Region::Conflict(block)` — the block is kept whole (`conflict.rs:220-232`); a block that is not resolved, or an index past the end → `NoSuchBlock(block)`. Update `NoSuchBlock`'s MAP line ("the index `take` or `reopen` was given").

## Touches
`history_reads/forecast.rs` (new fn beside the old; old untouched), `history_reads` exports; `conflict.rs`/`conflict/document.rs` (`reopen` beside `take`); `Region`/`ConflictFile` users in graft-ops (`apply_take`, `graft-ops/src/conflict.rs:125,160`) and graft-ui (`RegionView`) — `reopen` adds, changes nothing existing; the diff module (`line_origins`); MAP.md. No `CommitSummary` literal changes.

## Acceptance
- [x] `merge_forecast` fixtures: clean merge → `Checked`, no clashes, `changed` = the files theirs changed since the merge-base; one file clashing in 2 places → `places: Some(2)`; a modify/delete clash → that path with `places: None`; unrelated histories → `NotCheckable(UNRELATED)`; git below 2.38 (version stub) → `NotCheckable(_)` with `forecast_merge`'s reason text. — **Test:** merge_forecast::{a_clean_merge_lists_what_theirs_changed_and_no_clashes, a_file_clashing_in_two_places_counts_two, a_modify_delete_clash_names_the_path_with_no_count, unrelated_histories_are_not_checkable, an_old_git_is_not_checkable_with_forecast_merges_reason}
- [x] `merge_forecast` with a stopped `Stop` → `Err(RunError::Stopped)`. — **Test:** merge_forecast::a_raised_stop_is_stopped
- [x] Property: `parse → take(i, t) → reopen(i) → render` == original bytes, all `t`, every block. — **Test:** reopen_tests::take_then_reopen_renders_the_original (proptest)
- [x] `reopen` on an open block and on an out-of-range index → `Err(NoSuchBlock(i))`. — **Test:** reopen_tests::reopen_refuses_an_open_block_and_an_index_past_the_end
- [x] `line_origins` table test: line only in yours, only in theirs, in both, in neither (not listed). — **Test:** origins::tests::each_added_line_names_its_side_and_unclaimed_lines_are_not_listed
- [x] Existing `forecast_merge` tests (`tests/it/history_reads.rs:238,286,410`, `src/history_reads/tests.rs:60`) unchanged and passing. Each tick names its test. — **Test:** existing tests/it/history_reads.rs + src/history_reads/tests.rs forecast tests, unchanged bodies, green in 6455/6455

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~45k).

# clash-reads — what the merge preview, the incoming list, the pick undo and the look need from git

Status: draft
Build with: (blank — brownfield; conductor sets after the scout)
Needs: none

## Read first
- `docs/design-notes/ease-pass.md`: Mock behaviours M2, M3, M12, M17; Interfaces and data shapes.
- `crates/graft-git/MAP.md`: `forecast_merge`/`Forecast`, `commits_between`/`CommitSummary`/`parse_rev_list`, `conflict` section (`Region`, `ConflictFile`, `Take`, `parse`), `FileDiff`.
- The scout's file.

## Scout questions
- What `forecast_merge` runs and returns; whether `merge-tree --write-tree` output already gives changed paths and conflict hunks.
- Every constructor of `CommitSummary` (struct literal sites break when `when` is added).
- `Region::Resolved { block, take }`: does it keep the whole original block (needed for `reopen`)?

## Scope
In (graft-git only):
- `merge_forecast`: one read giving the files the merge changes and, per clashing file, how many places clash (`None` when not counted). `Ok(None)` = not checkable (today's `NotCheckable`).
- `CommitSummary.when` (author date) for the incoming list's age column.
- `ConflictFile::reopen`: a taken block back to open.
- `line_origins`: pure, for the read-only look's "from yours / from theirs" sample.
Out: preview drawing (Mp), the pick stack (P), the look (Fr). `forecast_merge` stays; its callers are not moved here.

## Signatures
```rust
pub struct ClashForecast { pub path: PathBuf, pub places: Option<u32> }
pub struct MergeForecast { pub changed: Vec<PathBuf>, pub clashes: Vec<ClashForecast> }
pub fn merge_forecast(repo: &RepoHandle, ours: Oid, theirs: Oid, stop: &dyn Stop)
    -> Result<Option<MergeForecast>, RunError>;

pub struct CommitSummary { pub id: Oid, pub parents: Vec<Oid>, pub subject: Vec<u8>, pub author: Vec<u8>, pub when: Timestamp }
                                     // existing fields keep their types; `when` is added

pub struct NoSuchBlock(pub usize);
impl ConflictFile { pub fn reopen(&mut self, block: usize) -> Result<(), NoSuchBlock>; }

pub enum Origin { Yours, Theirs, Both }
pub struct LineOrigin { pub line: u32, pub origin: Origin }
/// Diffs are base→result, base→yours, base→theirs; `line` is 1-based in the result.
pub fn line_origins(result: &FileDiff, yours: &FileDiff, theirs: &FileDiff) -> Vec<LineOrigin>;
```
`stop` takes the crate's existing stop form. `when` uses `graft_core::Timestamp`.

## Touches
Every `CommitSummary` literal (scout's list) and `parse_rev_list`'s format string; `Region`/`ConflictFile` users in graft-ops (`apply_take`) and graft-ui (`RegionView`) — `reopen` adds, changes nothing existing; MAP.md.

## Acceptance
- [ ] `merge_forecast` fixture: clean merge → no clashes, `changed` = theirs' files; one file clashing in 2 places → `places: Some(2)`; unrelated histories → `Ok(None)` or today's not-checkable meaning (state which).
- [ ] `parse_rev_list` reads `when`; existing rev-list tests still pass.
- [ ] Property: `parse → take(i, t) → reopen(i) → render` == original bytes, all `t`, every block.
- [ ] `reopen` on an open or out-of-range block → `NoSuchBlock`.
- [ ] `line_origins` table test: line only in yours, only in theirs, in both, in neither (not listed). Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Medium (~45k).

# resolver-picks — take, advance in the file, undo a pick, stage once at the review's `y`

Status: ready
Blocked by: 02-clash-reads, 03-ease-vocabulary
Build with: (blank — brownfield; conductor sets after the scout). **Planner recommends Fable**: passes all three tests — approach decided here, a wrong staging model is structural, index states can't be judged from the diff.
Needs: `ConflictFile::reopen`, `NoSuchBlock` (clash-reads); `resolver.review`, `resolver.undo_pick`, `resolver.pause`, `finish.undo_pick` action ids (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (merge-*, undo-a-pick, pause-and-resume) and landing points, Mock behaviours M5, M8–M10, M12, M14–M16, Hard-to-reverse choice 3, Questions 7 and 12.
- `docs/design/02-keymap.md` §2.6, §2.6a, §2.6b.
- `crates/graft-ops/MAP.md`: `Resume`, `resume`, `finish_summary`, `SummaryError`, `FinishMergeParams`, `apply_take`, `TakeError`, `Opened`. `crates/graft/MAP.md`: `update::resolver`, `update::finish`, `update::banner`, `update::relaunch`, `update::clash`.
- The scout's file.

## Scout questions
- Where a stop lands today and what P11 auto-open does; what `⏎` stages in the resolver and how "file done" is tracked.
- How `finish_summary` reads (index vs disk) and how relaunch finds a live merge's unmerged files.
- Every `Resume` / `FinishMergeParams` construction site.
- How a stash clash (`update::clash`) enters and leaves the resolver.

## Scope
In (graft-ops; graft app `update::resolver`, `update::finish`, `update::banner`, `update::relaunch` and their tests):
- Stop lands on the resolver, cursor on the first open block of the first file (0 keys). P11 auto-open of the summary dropped for merges.
- `o`/`t`/`b`: `apply_take`, push a `Pick`, cursor to `next_open_in_file`; file solved → stays, cursor stays; **nothing staged in a merge**. On a taken block: notice `already taken · u undoes it`, no write.
- `n`: `next_open` across files (list order); none open → notice `no open clashes left · y reviews the merge` (merge) / today's finish words (others).
- `u`: pop the last pick, `reopen` + `write_back` against the fingerprint, cursor on that block (switching file if needed). File changed outside → notice in words, stack kept. Empty stack → notice `nothing to undo here`.
- `y` (alias `c`) when every file is solved → the review (Fr draws it); `y` with open clashes → notice `N clashes still open · n goes to the next`.
- Review: `y` commits via `Resume::Continue{solved}` / `ContinueWith{message, solved}` or `FinishMerge` with `solved`, staging the solved files **once**; `u` → back to the resolver with the last pick reopened; `esc` → resolver unchanged.
- `⏎` in a merge: on a solved file → next file; on an open file → refused as today.
- Relaunch mid-merge: an unmerged file with no markers counts as solved.
- **Q7 a — pause:** `esc` on the clash screen → the graph with the operation banner drawn on top and **focused** (today's §2.6a banner); the bar leads `⏎ back to the clashes`. Banner `⏎` → the resolver, cursor on the **first open clash** (`next_open`), picks kept. Journey `pause-and-resume` = `esc ⏎`, 2 keys.
- **Q12 a — rebase, cherry-pick, revert, stash put-back:** the same advance-in-file, `n` to the next open clash, and `u` pick undo as a merge. Finishing stays as today: `⏎` marks the file done **and stages it**; `c` goes on (rebase/cherry-pick/revert); a stash clash's last `⏎` closes to the changes pane as today. `u` on a file already marked done refuses `<file> is marked done · d starts it over`. Picks in a file not yet marked done undo as in a merge.
Out: drawing (L, Fr), the look screen content (Fr), keymap entries (V).

## Signatures
graft-ops:
```rust
pub enum Resume { Continue { solved: Vec<PathBuf> }, ContinueWith { message: Vec<u8>, solved: Vec<PathBuf> }, Skip, Abort }
pub struct FinishMergeParams { pub merge_head: Oid, pub message: Option<Vec<u8>>, pub solved: Vec<PathBuf> }
/// The review's facts as if `solved` were staged: reads a scratch index, never the real one.
pub fn finish_preview(repo: &RepoHandle, solved: &[PathBuf], stop: &dyn Stop) -> Result<FinishSummary, SummaryError>;
```
graft app (`update::resolver`):
```rust
pub struct Pick { pub path: PathBuf, pub block: usize, pub take: Take }
#[derive(Default)]
pub struct Picks { /* stack */ }
impl Picks {
    pub fn push(&mut self, pick: Pick);
    pub fn pop(&mut self) -> Option<Pick>;
    pub fn last(&self) -> Option<&Pick>;
    pub fn is_empty(&self) -> bool;
    pub fn in_file(&self, path: &Path) -> usize;
    /// Drops the picks in `path` (a non-merge file marked done with `⏎`).
    pub fn seal(&mut self, path: &Path);
}
pub struct BlockAt { pub index: usize, pub open: bool }
/// First open block, top-down.
pub fn next_open_in_file(blocks: &[BlockAt]) -> Option<usize>;
/// (file index, block index) of the first open block in list order.
pub fn next_open(files: &[(PathBuf, Vec<BlockAt>)]) -> Option<(usize, usize)>;
```
`Picks` lives for one operation; cleared on finish or abort; kept across pause (`esc`) and relaunch-free resume; not journaled (a pick is a disk write inside the live operation, which the operation's backup already covers — scout confirms).

## Touches
Every `Resume` and `FinishMergeParams` site; `resume` for shell-begun merges; journal entries for finish/abort; `u` routing (§2.6a refusal must stay for every other screen mid-operation — test it from the graph); banner focus and its bar; finish summary for non-merge operations (unchanged); stash clash close path (`update::clash`); relaunch path; resolver and finish captures (recaptured by L, Fr).

## Acceptance
- [ ] Fixture merge, one file, 2 clashes: `o` → cursor on clash 2, file still listed `?`; `o` → file `✓`, cursor stays; index still unmerged for that path (`git ls-files -u` non-empty).
- [ ] `u` → clash 2 open again on disk (bytes equal pre-pick), cursor on it; `u` again → clash 1.
- [ ] Changed-outside: edit file between pick and `u` → notice, disk untouched, stack kept.
- [ ] `y` all solved → review state; review `y` → merge commit with both parents, index clean; review `u` → resolver, last pick open.
- [ ] Journey `m y o y y` lands on the graph at the merge commit (state test; the pty walk is A's).
- [ ] Q7: `esc` on the resolver → graph, banner focused; `⏎` → resolver, cursor on the first open clash, earlier picks still taken on disk and `u` still undoes them.
- [ ] Shell-begun merge (`git merge` outside, then relaunch): same keys, finishes through `FinishMerge` with `solved`.
- [ ] Relaunch after solving on disk but before `y`: files read as solved.
- [ ] Q12: rebase stop, one file with 2 clashes: `o` advances within the file; `u` undoes it; `⏎` stages the file (`git ls-files -u` empty for it); `u` then refuses with `d starts it over`; `c` continues the rebase.
- [ ] Q12: stash put-back clash: `o` then `u` restores the block; last `⏎` closes to the changes pane as today.
- [ ] From the graph mid-merge, `u` still refuses with today's §2.6a words. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~75k). If the scout finds the shell-begun path separate from `Pending`, split: picks + advance + `u` + Q12 here, review `y` staging and Q7 pause as a follow-on.

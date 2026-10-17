# resolver-picks — take, advance in the file, undo a pick, stage once at the review's `y`

Status: draft — **blocked on Q7 and Q12** (design note, Questions for the user).
Build with: (blank — brownfield; conductor sets after the scout). **Planner recommends Fable**: passes all three tests — approach decided here, a wrong staging model is structural, index states can't be judged from the diff.
Needs: `ConflictFile::reopen`, `NoSuchBlock` (clash-reads); `resolver.review`, `resolver.undo_pick`, `finish.undo_pick` action ids (ease-vocabulary).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (merge-*, undo-a-pick, pause-and-resume) and landing points, Mock behaviours M5, M8–M10, M12, M14–M16, Hard-to-reverse choice 3.
- `docs/design/02-keymap.md` §2.6, §2.6a, §2.6b.
- `crates/graft-ops/MAP.md`: `Resume`, `resume`, `finish_summary`, `SummaryError`, `FinishMergeParams`, `apply_take`, `TakeError`, `Opened`. `crates/graft/MAP.md`: `update::resolver`, `update::finish`, the operation-stopped landing.
- The scout's file.

## Scout questions
- Where a stop lands today and what P11 auto-open does; what `⏎` stages in the resolver and how "file done" is tracked.
- How `finish_summary` reads (index vs disk) and how relaunch finds a live merge's unmerged files.
- Every `Resume` / `FinishMergeParams` construction site.

## Scope
In (graft-ops; graft app `update::resolver`, `update::finish` and their tests):
- Stop lands on the resolver, cursor on the first open block of the first file (0 keys). P11 auto-open of the summary dropped for merges.
- `o`/`t`/`b`: `apply_take`, push a `Pick`, cursor to `next_open_in_file`; file solved → stays, cursor stays; **nothing staged**. On a taken block: notice `already taken · u undoes it`, no write.
- `n`: `next_open` across files (list order).
- `u`: pop the last pick, `reopen` + `write_back` against the fingerprint, cursor on that block (switching file if needed). File changed outside → notice in words, stack kept. Empty stack → notice `nothing to undo here`.
- `y` (alias `c`) when every file is solved → the review (Fr draws it); `y` with open clashes → notice `N clashes still open · n goes to the next`.
- Review: `y` commits via `Resume::Continue{solved}` / `ContinueWith{message, solved}` or `FinishMerge` with `solved`, staging the solved files **once**; `u` → back to the resolver with the last pick reopened; `esc` → resolver unchanged.
- `⏎` in a merge: on a solved file → next file; on an open file → refused as today.
- Relaunch mid-merge: an unmerged file with no markers counts as solved.
- `esc` pause per Q7's answer.
- Non-merge operations (rebase, cherry-pick, revert, stash put-back): keep `⏎` staging; `u` on a marked-done file refuses `d starts it over`; the rest per Q12.
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
}
pub struct BlockAt { pub index: usize, pub open: bool }
/// First open block, top-down.
pub fn next_open_in_file(blocks: &[BlockAt]) -> Option<usize>;
/// (file index, block index) of the first open block in list order.
pub fn next_open(files: &[(PathBuf, Vec<BlockAt>)]) -> Option<(usize, usize)>;
```
`Picks` lives for one operation; cleared on finish or abort; not journaled (a pick is a disk write inside the live operation, which the operation's backup already covers — scout confirms).

## Touches
Every `Resume` and `FinishMergeParams` site; `resume` for shell-begun merges; journal entries for finish/abort; `u` routing (§2.6a refusal must stay for every other screen mid-operation); finish summary for non-merge operations (unchanged); operation banner; resolver and finish captures (recaptured by L, Fr); relaunch path.

## Acceptance
- [ ] Fixture merge, one file, 2 clashes: `o` → cursor on clash 2, file still listed `?`; `o` → file `✓`, cursor stays; index still unmerged for that path (`git ls-files -u` non-empty).
- [ ] `u` → clash 2 open again on disk (bytes equal pre-pick), cursor on it; `u` again → clash 1.
- [ ] Changed-outside: edit file between pick and `u` → notice, disk untouched.
- [ ] `y` all solved → review state; review `y` → merge commit with both parents, index clean; review `u` → resolver, last pick open.
- [ ] Journey `m y o y y` lands on the graph at the merge commit (state test; the pty walk is A's).
- [ ] Shell-begun merge (`git merge` outside, then relaunch): same keys, finishes through `FinishMerge` with `solved`.
- [ ] Relaunch after solving on disk but before `y`: files read as solved.
- [ ] Rebase stop: `⏎` still stages; `u` on a done file refuses with `d starts it over`. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
Large (~75k). If the scout finds the shell-begun path separate from `Pending`, split: picks + advance + `u` here, review `y` staging as a follow-on.

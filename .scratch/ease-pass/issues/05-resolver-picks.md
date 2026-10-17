# resolver-picks — take, advance in the file, undo a pick, `n` to the next open clash

Status: ready (rewritten 2026-10-17 after this group's scout, `.scratch/ease-pass/scouts/05-resolver-picks.md`; split: the merge's stage-at-`y` model, the `y` row, the review's keys and Q7 pause moved to **05b-merge-review-at-y**)
Blocked by: 02-clash-reads, 03-ease-vocabulary, 03b-ease-vocabulary-rest
Build with: Opus (`builder-opus-xhigh`) — the pick stack's correctness is bytes on disk, checkable by fixture (bytes equal pre-pick); the structural, index-state work is 05b's (Fable).
Needs: existing, unchanged: `ConflictFile::{take, take_all, reopen, blocks, open}`, `BlockState::{Open, Took(Take)}`, `NoSuchBlock` (graft-git; `reopen` already exists, `graft-git/src/conflict/document.rs:89`; `NoSuchBlock` `graft-git/src/conflict.rs:329`); `apply_take`, `apply_take_all`, `TakeError::{Changed, NoSuchBlock, Write}`, `Opened` (graft-ops `conflict`, MAP.md :73-79); ids `resolver.next_block` (`n`, `keymap/table.rs:323`), `resolver.prev_block` (`N`, :324), `app.undo` (`u`, :78); from 03b: `HintContext::Clash(ClashBar { at, op })` (its `u`/`n` words are already drawn — this group makes them true).

## Read first
- `docs/design-notes/ease-pass.md`: Journeys (merge-three-clashes, undo-a-pick), Mock behaviours M8–M10, M12, Hard-to-reverse choice 3, Questions 12 and 16.
- `docs/design/02-keymap.md` §2.6 (both tables), §2.6a paragraph "`u` and `U` are refused…" (the shell-merge exception).
- `crates/graft-ops/MAP.md` `conflict` entries (:73-79). `crates/graft-git/MAP.md` conflict entries (:74-94). `crates/graft/MAP.md`: `update::resolver`, `update::clash`, `jobs::conflict`.
- The scout's file (all of it; its file:line facts stand).

## Decided here (scout contradictions resolved)
- **Why picks need no backup, in any merge** (scout contradiction 4): a pick is a file write inside the live operation, not a journal entry, in a merge begun in Graft or in the shell (§2.6a: "a block take … is a file write inside the merge, not an entry"). Undoing it needs only the stack (in memory) and the file's fingerprint. What makes a pick safe is **the index, not a backup**: until the file is staged (`⏎` in this ticket; the merge's `y` after 05b) git's conflict stages 1–3 for that path stay in the index, so the conflict can always be rebuilt (`d`, where §2.6 offers it). Abort is today's (Graft: the operation's backup; shell: `git merge --abort`, journaled). Nothing new is journaled.
- **Re-taking a taken block replaces its take, as today** (`document.rs:63-66`), and pushes a pick that remembers the earlier take, so `u` restores *that* take, not an open block. The mock ignores `o/t/b` on a taken block (its bar there offers only `n`, `u`); that departure is **Question 16** — build (a) replace; (b) is one branch in `take_block` (notice `already taken · u undoes it`, no write).
- **`O`/`T`, `⌃O`/`⌃T`** (scout contradiction 8): today's meanings kept (§2.6 rows :347-348). Each pushes **one** pick holding every block it changed, so one `u` undoes the whole keypress. `O`/`T` mark the file done (they stage, `take.rs:266-267`) → that file's picks are sealed, as `⏎`. 05b changes what `O`/`T` do in a merge.
- **A stale pick never blocks the stack:** `u` on a pick whose file changed (fingerprint, `TakeError::Changed`) or is out of step (`NoSuchBlock`) writes nothing, says so, and **drops that file's picks**, so the next `u` reaches earlier picks in other files. (The earlier "stack kept" left every later `u` stuck on the same notice.) `e` returning with the file's bytes changed drops that file's picks too.
- **`u` on a file marked done** (cursor's file staged): refused with today's words `<file> is done · d starts it over` (`done.rs:57 done_refusal`), nothing popped.

## Scope
In (graft-ops `conflict` (one fn); graft app `update::resolver` (`take.rs`, `walk.rs`, `resolved.rs` seal call only, `resolver.rs` `act`/`undo_answered`), `update::clash` (stack lifetime only), their tests). Every operation with clashes: merge (either origin), rebase, cherry-pick, revert, stash put-back (Q12 a).
- **`o`/`t`/`b`** (`take_block`, `take.rs:52`): as today through `Command::TakeSide` (:163, queued :158); push a `Pick`; the cursor moves at keypress (`walk::after_take`, :202) to `next_open_in_file` — the **first** open block top-down (the mock: `C.findIndex(c=>!c.pick&&c.file===f)`); none open → the cursor stays (file solved, stays on screen). Nothing new staged.
- **`n`** (`resolver.next_block`, `resolver.rs:373`, `walk.rs:29`): `next_open` across files in list order, from the top (the mock's `nextOpen`). None open → today's finish words for the operation (05b changes the merge's to `y`). `N` keeps today's meaning (`walk.rs:29`, `forward=false`), taken blocks included — flagged for the usability walk, not changed.
- **`u`** in the resolver (call `resolver.rs:379` via `asks_undo` :394 → `undo_answered` :409): pop the last pick; `apply_untake` restores every block it changed (reopen, then the earlier take where there was one), one write against the fingerprint; cursor on its first block, switching file if needed. Empty stack: stash clash → today's `clash::undo_clause` words; otherwise notice `nothing to undo here`. Failures as Decided above, each in words, never swallowed.
- **Stack lifetime:** held with the live operation's state, **not** inside the resolver's, so closing the resolver keeps it. Cleared when the operation finishes, aborts, or goes on to its next stop (rebase/cherry-pick/revert `c`, `s`), and when a stash clash closes after its last `⏎` (`close_after_last`, `resolved.rs:161`). Not kept across a relaunch (memory only; a relaunch's `u` says `nothing to undo here`).
- **Seal:** `⏎` (`mark_resolved`, `resolved.rs:25`) and `O`/`T` seal the file's picks after the stage succeeds; a failed stage seals nothing.
Out: what a merge stages and when, the `y` row, the review, Q7 pause, stop landing (05b); drawing (L); hint words (03b).

## Signatures
graft-ops `conflict`:
```rust
/// One block put back as it was before a pick: `None` = open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Restore { pub block: usize, pub before: Option<Take> }
/// Reopens each block, re-takes `before` where `Some`, one write against `file.fingerprint`.
/// `repo`, `path`, `file` and the return type exactly as `apply_take`'s; errors are `TakeError`'s
/// (`Changed` leaves the bytes untouched, as `apply_take_all`).
pub fn apply_untake(/* apply_take's repo, path, file */, restore: &[Restore]) -> /* apply_take's */;
```
graft app (`update::resolver`):
```rust
/// One keypress's worth of takes: `o/t/b` one block; `O/T/⌃O/⌃T` every block it changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pick { pub path: PathBuf, pub blocks: Vec<Restore> }
#[derive(Clone, Debug, Default)]
pub struct Picks { /* stack */ }
impl Picks {
    pub fn push(&mut self, pick: Pick);
    pub fn pop(&mut self) -> Option<Pick>;
    pub fn last(&self) -> Option<&Pick>;
    pub fn is_empty(&self) -> bool;
    pub fn in_file(&self, path: &Path) -> usize;
    /// Drops `path`'s picks: the file was staged, edited by hand, or changed outside.
    pub fn seal(&mut self, path: &Path);
    pub fn clear(&mut self);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockAt { pub index: usize, pub open: bool }
/// First open block, top-down.
pub fn next_open_in_file(blocks: &[BlockAt]) -> Option<usize>;
/// (file index, block index) of the first open block in list order.
pub fn next_open(files: &[(PathBuf, Vec<BlockAt>)]) -> Option<(usize, usize)>;
```
`Restore.block` indexes as `ConflictFile::take`/`reopen` do; `before` is read from `ConflictFile::blocks` (`BlockState::Took`) before the take is sent.

## Touches
`take_block` and `take_all_key`/`settle_key` (`take.rs:52,163,202,266-267`); `walk.rs` (`n`; `N` untouched — test it); `mark_resolved` seal (`resolved.rs:25,55`); `undo_answered` and `asks_undo` — `u` from the graph mid-operation must keep §2.6a's refusal (test from the graph and the banner); the shell-merge `u`/`U` exception on journaled whole-file choices and start-overs (§2.6a) — unchanged, test; whole-file cards (`Opened.file == None`): not picks, unchanged; `e` re-read; the stash clash close path; `jobs::conflict` (the untake job beside the take job, same queue); resolver captures (recaptured by L); existing `NoSuchBlock` handling `graft-ops/src/conflict.rs:125,160` (unchanged); `write_back` fingerprint path (:161,185).

## Acceptance
- [ ] Fixture merge, one file, 2 clashes: `o` → cursor on clash 2; `o` → cursor stays, file has 0 open (`ConflictFile::open`).
- [ ] `u` → clash 2 open again on disk (bytes equal pre-pick), cursor on it; `u` → clash 1 open, bytes equal the file git wrote.
- [ ] Re-take: `o` on clash 1, `k`/`N` back to it, `t` → took theirs; `u` → kept-yours again (not open); `u` → open.
- [ ] `⌃O` on a file with 3 open → all taken, one pick; one `u` → all 3 open, bytes equal.
- [ ] `n`: from file 2 with file 1 still open → cursor on file 1's first open clash; taken blocks skipped; none open → today's finish words. `N` unchanged (existing walk tests pass unedited).
- [ ] Changed-outside: edit the file between pick and `u` → notice, disk untouched, that file's picks gone, next `u` undoes the previous file's pick.
- [ ] `u` with an empty stack → `nothing to undo here`; in a stash clash → today's clause.
- [ ] Shell-begun merge (`git merge` outside, relaunch): `o` then `u` restores the bytes; `git ls-files -u` lists the path throughout; no journal entry added.
- [ ] Q12 rebase stop, one file with 2 clashes: `o` advances within the file; `u` undoes it; `⏎` stages (`git ls-files -u` empty for it); `u` on that file → `<file> is done · d starts it over`; `c` continues; the stack is empty at the next stop.
- [ ] Q12 stash put-back clash: `o` then `u` restores the block; last `⏎` closes to the changes pane as today.
- [ ] From the graph mid-merge, `u` still refuses with §2.6a's words. Each tick names its test.

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k.

# resolver-picks — take, advance in the file, undo a pick, `n` to the next open clash

Status: done (ease-pass-05, branch `worktree-agent-a251081ca0824c3a6`; see NOTES). Was: ready (rewritten 2026-10-17 after this group's scout, `.scratch/ease-pass/scouts/05-resolver-picks.md`; split: the merge's stage-at-`y` model, the `y` row, the review's keys and Q7 pause moved to **05b-merge-review-at-y**)
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
- [x] Fixture merge, one file, 2 clashes: `o` → cursor on clash 2; `o` → cursor stays, file has 0 open (`ConflictFile::open`). — `it::resolver::picks::a_take_moves_to_the_first_open_clash_and_stays_once_the_file_is_solved`; top-down (back up to an open clash above): `…::a_take_goes_back_up_to_an_open_clash_above_the_cursor`.
- [x] `u` → clash 2 open again on disk (bytes equal pre-pick), cursor on it; `u` → clash 1 open, bytes equal the file git wrote. — `it::resolver::picks::u_puts_each_pick_back_byte_for_byte_newest_first`; op level: `graft-ops conflict::untake_tests::{untake_reopens_a_block_to_the_bytes_git_wrote, untake_of_a_second_pick_leaves_the_first}`.
- [x] Re-take: `o` on clash 1, `k`/`N` back to it, `t` → took theirs; `u` → kept-yours again (not open); `u` → open. — `it::resolver::picks::u_after_a_retake_restores_the_earlier_take` (uses `N`); op level `untake_tests::untake_of_a_retake_restores_the_earlier_take`.
- [x] `⌃O` on a file with 3 open → all taken, one pick; one `u` → all 3 open, bytes equal. — `it::resolver::picks::one_u_undoes_a_whole_ctrl_o`; op level `untake_tests::untake_of_a_take_all_reopens_every_block_in_one_write`.
- [x] `n`: from file 2 with file 1 still open → cursor on file 1's first open clash; taken blocks skipped; none open → today's words (`no more conflicts this way`, `walk.rs` `no_more_conflicts`; the finish row once every file is done, unchanged). — `it::resolver::picks::n_goes_to_the_first_open_clash_from_the_top`, `…::n_crosses_up_to_a_card_still_open_above`; unit `update::resolver::tests::first_open::{n_on_the_first_open_block_says_so, n_on_a_card_that_is_the_first_open_file_says_so, n_goes_up_to_the_first_open_block_rather_than_cross}`. **`N`: its code (`walk_blocks`, `forward=false`) is untouched and every existing `N` assertion is kept, but "pass unedited" does not hold literally** — tests that mixed `n` steps into a walk were edited for `n`'s new meaning (listed under NOTES), and three former forward-walk tests now walk with `N`.
- [x] Changed-outside: edit the file between pick and `u` → notice, disk untouched, that file's picks gone, next `u` undoes the previous file's pick. — `it::resolver::picks::a_pick_whose_file_changed_is_dropped_and_u_reaches_the_one_before` (the words are a Sticky status, `b.txt changed on disk since that pick · nothing undone`); op level `untake_tests::{untake_never_clobbers_a_changed_file, untake_out_of_step_writes_nothing}`.
- [x] `u` with an empty stack → `nothing to undo here`; in a stash clash → today's clause. — `it::resolver::picks::u_with_no_pick_says_so`; stash: `it::resolver::picks::lifetime::a_stash_clash_takes_and_undoes_then_closes_as_today` (`esc, u puts it all back`); unit `update::resolver::tests::u_in_the_resolver_says_where_undo_is`.
- [x] Shell-begun merge (`git merge` outside, relaunch): `o` then `u` restores the bytes; `git ls-files -u` lists the path throughout; no journal entry added. — `it::resolver::picks::lifetime::a_pick_in_a_merge_begun_in_the_shell_is_a_file_write_and_no_entry`; relaunch forgets: `…::lifetime::picks_outlive_a_closed_resolver_but_not_a_relaunch`.
- [x] Q12 rebase stop, one file with 2 clashes: `o` advances within the file; `u` undoes it; `⏎` stages (`git ls-files -u` empty for it); `u` on that file → `<file> is done · d starts it over`; `c` continues; the stack is empty at the next stop. — `it::resolver::picks::lifetime::a_rebase_stop_takes_and_undoes_and_its_next_stop_starts_clean` (also `s` and `a` clear the stack). The done words are asserted by prefix (`a.txt is done`).
- [x] Q12 stash put-back clash: `o` then `u` restores the block; last `⏎` closes to the changes pane as today. — `it::resolver::picks::lifetime::a_stash_clash_takes_and_undoes_then_closes_as_today`.
- [x] From the graph mid-merge, `u` still refuses with §2.6a's words. Each tick names its test. — `it::resolver::picks::lifetime::mid_merge_u_off_the_resolver_still_refuses_and_keeps_the_pick` (graph and banner; `finish or abort the merge first`, no `Command::Undo`, the pick kept).

## Gates
fmt, clippy `-D warnings`, nextest, `cargo test --doc` — workspace-wide.

## Size
~50k.

## NOTES (builder, 2026-10-17)
**Compacted three times mid-ticket.** Everything in Scope is built; nothing of 05's scope remains. Left for others: captions/recaptures (L), the bar's `n` words (03b/L), the usability question below.

Scout used: `.scratch/ease-pass/scouts/05-resolver-picks.md`.

**Deviations from Signatures (said, not silent):**
- `Pick` has a third field `after: Arc<ConflictFile>` — the document the pick's write left; the untake is applied to it and its fingerprint is what disk must still hold. Without it the untake has no document to reopen against once the resolver moved on.
- `Picks` also holds the untake in flight (`undoing`, off the stack while it runs; back on top only on an I/O failure) and a `landing` (an untake on another file puts the cursor on its block once that file's read lands). Plus `reconcile(path, fingerprint)` (a re-read with other bytes seals that file) and `take_landing`.
- `Command::UntakeSide(JobId, Pick)` carries the pick, no request struct; the job is `jobs::conflict::untake` (`jobs/conflict/write.rs`, beside `take`, moved there unchanged).
- `AppState.picks` is `Box<Picks>`. AppState sat exactly at clippy's `large_stack_arrays` bound for a 2-element array of `(&str, AppState)`; even boxed, +8 bytes tripped three test arrays. Fixed **in the tests**, by the precedent at `resolver/tests/take_all.rs:126` (makers in the array, not whole states): `update/banner/focus_keys.rs` `banner_states`, `update/remote/tests.rs` `pull_refusals_are_words_not_commands`, `view/tests.rs` `the_lane_images_never_cover_the_confirm_box_or_the_credential_prompt`. `vec!` was tried first: `useless_vec` refuses it.
- `n` with the cursor already on the first open clash says `already on the first open clash` (`walk.rs` `ON_FIRST_OPEN`). The mock does nothing there; the bar-keys sweep forbids a drawn key that does nothing, so it speaks instead.
- `U` in the resolver keeps today's words (`unpick.rs` `undo_answered`, moved verbatim from `resolver.rs`); `u`/`⌃Z` are the pick's.
- Q16 (re-take replaces) lives in one place: `take.rs` `take_block`'s comment marks the one branch a refuse answer changes. Q17 (stop landing) untouched.

**Moved, unchanged:** `take`/`TakeFailure` → `jobs/conflict/write.rs`; `asks_undo`/`undo_answered` → `resolver/unpick.rs`; `conflict_arrived`'s whole-file branch → `answers.rs` `card_arrived` (clippy `too_many_lines`).

**Tests edited for `n`/`u`'s new meaning** (no assertion about `N` changed):
- unit `resolver/tests.rs`: `n_on_a_card_crosses…` → `N_on_a_card_crosses_to_the_next_file`; `a_card_says_no_more_this_way…` presses `N`; `n_on_the_last_card_in_a_rebase…` → `N_…`; `n_walks_blocks_even_with_a_search_live` → `n_moves_the_resolver_cursor_even_with_a_search_live`; `u_in_the_resolver_says_where_undo_is` now checks `U` (old words) and `u` (`nothing to undo here` / the stash clause) apart.
- unit `resolver/tests/cross.rs`: `n_and_N_reach_a_taken_block` → `N_reaches_a_taken_block_and_n_skips_it`; `n_crosses_before_it_wraps` → `N_…`; `n_wraps_when_no_other_file_is_open` → `N_…` (+ `n` lands on the one open block).
- `keymap_words/resolver.rs`: the resolver's undo-words site presses `U`.
- it `resolver/takes.rs`: `n_and_N_walk_every_block` → `N_reaches_a_taken_block_and_n_goes_up_the_list`; `n_past_the_last_block_opens_the_next_conflicted_file` → `N_before_the_first_block_opens_the_previous_conflicted_file`.
- it `resolver/handover.rs`, `resolver/modal.rs`: setup `n` presses replaced by setting the cursor (they test the hand-over and the modal, not the walk).
- it `resolver/file_list.rs`: its card `n` test moved to `resolver/picks.rs` `n_crosses_up_to_a_card_still_open_above` (now crosses up).
- `m5_walk`: journeys `two_files_out_of_order` and `walk_back_a_done_file` step with `j` where they pressed `n` (same step counts); `n_past_the_last_block_says_no_more` starts with `N`; captures `m6-01` keys `t B` (was `n t B`) and `m6-12-after-t` keys `j t` (was `n t`). **The committed `docs/shots` frames/captions for m6-01 and m6-12 are stale until L recaptures.**

**Usability question (for the walk, not decided here):** no key walks *forward* past an open block any more — `n` goes to the first open clash from the top, `N` walks backward (taken blocks included), `j`/`k` step files. Forward stepping is only by mouse. Flagged per Scope ("`N` … flagged for the usability walk").

**Gaps — broken:** none known.
**Gaps — unverified:**
- Memory: each pick holds an `Arc<ConflictFile>` of its file; picks × document size is unmeasured (no perf run in a builder).
- A rebase continued in the shell while the app runs: picks of the old stop stay until a read reconciles them. Safe — an untake checks the fingerprint and writes nothing on a mismatch (tested at op level) — but not walked end-to-end.
- The bar still draws `n next` on an open clash; its words belong to 03b/L.
- No capture taken: nothing drawn changed except notice words; the m6-01/m6-12 frames above are stale.

**Touches (checked how):** `jobs::conflict` (new `untake`; runner `dispatch.rs`/`reads.rs`; `event_loop/label.rs`) — it tests; `update::clash`, `tree_answers.rs`, `banner/stop.rs`, `undo/answers.rs`, `resolved.rs` (clear/seal points) — `lifetime.rs` tests; graph/banner `u` mid-merge (§2.6a) — `mid_merge_u_off_the_resolver_still_refuses_and_keeps_the_pick`; shell-merge `u`/`U` exception on journaled whole-file choices — graph path untouched (`git diff` shows no change in `update/undo.rs`; `undo/answers.rs` only clears picks with the choices), existing `undo/tests.rs` pass; whole-file cards — not picks, `card_arrived` moved unchanged; MAP.md (graft, graft-ops) — `map_coverage`/`map_lines_are_short`; `over_400.txt` — `file_size`; three foreign test files for `large_stack_arrays` (above); keymap — no new key, ids `resolver.next_block`/`app.undo`/`app.redo` as shipped.

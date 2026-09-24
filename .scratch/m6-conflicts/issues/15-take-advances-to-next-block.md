# fix-take-advances-to-next-block: `o`/`t`/`b` move the cursor to the next open block; `n`/`N` reach taken blocks, so a take can be fixed by keyboard, and cross to the next file at the end

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
**Revised 2026-09-24 in place, from `.scratch/m6-conflicts/scout-15.md` (main `c56b64e`), items 1–7:**
- 1: paths are `update/resolver.rs` (struct at `:97`) and its submodules `update/resolver/{walk,take,answers,files,tests}.rs`. There is no `resolver/mod.rs`.
- 2: `AppState::choices` is dropped from this ticket. Ticket 19 records it in `take_landed`, which every answer passes through, queued ones included. No dependency on 19.
- 3: `FileState::Done` → `RowState::Done` (`files.rs:23,34`).
- 4: end-of-file order follows the keymap (`02-keymap.md:267`): **cross first**, then wrap in this file, else `no more conflicts`. Why the keymap is right: under wrap-first, `n` keeps circling a file that still has an open block and never leaves it. That is a trap (UX H3, user control). Cross-first is one rule that depends on position only.
- 5: the wording is **done**. That is Amendment 2's word (`done ✓`, `every file done ✓`, `⏎ marks done`, note :471, :687). `walk.rs:63`'s `resolved` changes. CONTEXT.md has no term for it; see "For the conductor".
- 6: crossing walks `listed_files` order (the order `resolved_advance` uses, `answers.rs:257-270`), not `next_conflicted_file`'s unmerged order (`walk.rs:88`). This ticket owns crossing in list order. Ticket 17 keeps only crossing from a done file.
- 7: `tests.rs:381` `a_take_or_a_base_in_flight_keeps_its_own_words` changes: its take half now queues, and its base half is unchanged.
- The scout's "could not confirm": `ConflictFile::take(block, take) -> Result<(), NoSuchBlock>` is in-memory and public. Block indexes count `Conflict` and `Resolved` regions, so they are stable across takes (`crates/graft-git/MAP.md` :53).

Blocked by: 13-resolver-file-list.md (merged), 10-resolver-line-numbers.md, 25-stable-file-order-and-done-file-open.md (building: `done_refusal`, the stable list order).
Build with: **Opus** (scouted: `scout-15.md`). The work is new state over an async write: a queue, an optimistic cursor and a failure path. Every rule is fixed here and tests can judge it. It is not a Fable ticket: the shape is not structural and nothing depends on it beyond 17.
Needs: `listed_files`, `select_file`, `RowState::Done` (13). `done_refusal` (25). `SideNames` (10), for the header words. `graft_git::conflict::ConflictFile::{take, blocks, open}`.
Crates: graft.

## Read first
- `.scratch/m6-conflicts/scout-15.md` (whole).
- `.scratch/m6-conflicts/issues/25-stable-file-order-and-done-file-open.md`: Decisions "D1 order" and "D2 keys on a done file".
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P2 (:355–:358, as corrected; see "For the conductor"), and Interfaces, the P2 block (:320–:323).
  - Amendment 2: Journeys J5 and J9 (:450, :454), and "Where each event lands him" (:465–:473).
- `crates/graft/MAP.md` §`update::resolver`. `crates/graft-git/MAP.md` §`conflict`, `ConflictFile` (:53).
- `docs/design/02-keymap.md` §2.6 (:258–:273).
- TESTING.md "Conflicts walk", scenario 1 (:2132) and scenario 7 (:2166), and the capture row `m6-08-no-more-conflicts` (:2252).

## Scope
In:
1. **Advance, at keypress.**
   - When `o`/`t`/`b` is accepted, the cursor moves at once to `after_take(&pending, block)`. `pending` is `pending_file(resolver)` with this take applied. It then scrolls into view.
   - `None` → the cursor stays, and the notice reads `all blocks taken · ⏎ marks <file> done`.
   - This applies in both arrangements (the card and the three panes).
   - A whole-file card is unchanged: its `o`/`t` mark the file done.
   - `take_landed` (`answers.rs:157`) no longer re-scrolls to the block that landed. It keeps the cursor wherever the keys put it.
   - `take_landed`'s notice: `all blocks taken · ⏎ marks <file> done` when `file.open() == 0` and nothing is queued, else `None`.
2. **Queue, not refuse.**
   - While a *take* is in flight (`in_flight.is_some()`), `o`/`t`/`b` append `(cursor, take)` to `queued` and advance as in item 1. They push no command.
   - At 64 queued, the next one is refused with today's `still writing the last take`.
   - Any other job in flight (an open or re-read, a stage, a base load, a restart) keeps today's refusal and words (`take.rs:58-61`, `:68-71`).
   - `take_landed` pops the front of `queued`. It pushes its `TakeSide` against the file that just landed, in the same update, so `queued` is never non-empty while `in_flight` is `None`.
   - A failed take (`answers.rs:136-137`) or a `Changed` re-read (`:141-150`) clears `in_flight` and `queued`. The notice is today's failure words, plus ` · <n> queued takes dropped` when `n > 0`. The cursor stays.
   - `pending_file` returning `Err(NoSuchBlock)` refuses the key with notice `no block <i> · re-read the file`, and pushes nothing. This is never swallowed (rule 3).
   - Every refusal keyed on the job (switching files, `⏎`, `d`, `e`, `M`, `B`) covers the queue as well, because of that invariant. There is no new refusal site.
3. **`n`/`N`** (keymap :267). `next_block` walks every block, taken ones included. Past the last block (for `N`, before the first):
   1. The list holds another file that is not `RowState::Done` → `select_file` it. The search runs in `listed_files` order from this file's row, wrapping. `n` lands on its block 0, `N` on its last block. The notice reads `▸ <file>`.
   2. Otherwise this file has an open block → the cursor goes to the first open block (for `N`, the last). The notice reads `wrapped · <k> still open`.
   3. Otherwise → `no more conflicts · ⏎ marks <file> done`. The cursor is unmoved.
   - On a whole-file card (one block, `file` is `None`), `n`/`N` go straight to rule 1. If no file qualifies, the card's words stay as today (`tests.rs:72`).
   - A cross while `in_flight` is `Some` is refused with `still writing the last take`.
4. **Done file.** On a done file, `o`/`t`/`b`/`n`/`N` are answered by 25's `done_refusal` before any of this. This ticket adds no check of its own; the test below pins the order.
5. **Words.** `walk.rs:63`: `⏎ marks <path> resolved` → `⏎ marks <path> done`. Update every test that asserts the old words.
6. **TESTING.md.**
   - Scenario 1: `n t n b` → `t b`, since `o` on block 1 already moved the cursor to block 2. Fix the step text to match.
   - Scenario 7: set up with `zdiff3.txt` as the only file not done.
     - `n` with its block open → `wrapped · 1 still open`.
     - After `t`, `n` and `N` → `no more conflicts · ⏎ marks zdiff3.txt done`.
     - Keep "The user decides that wording."
     - Row `m6-08-no-more-conflicts` gets the same words.
7. **MAP.md** `crates/graft/MAP.md` §`update::resolver`: `in_flight`, `queued`, `pending_file`, `next_block`, `after_take`, the cross rule, and "`next_open_block`/`next_conflicted_file` removed or unused". If a caller outside `n`/`N` keeps `next_conflicted_file`, say which.

Out: bulk takes (16); `O`/`T` (P6); `j`/`k` (17); crossing *from* a done file (17); recording `choices` (19).

## Signatures
```rust
// graft/src/update/resolver.rs  (ResolverState at :97)
pub struct ResolverState {
    /* existing */
    pub in_flight: Option<(usize, Take)>,        // the take whose TakeSide is out; Some exactly when `job` is a take
    pub queued: VecDeque<(usize, Take)>,         // pressed while `in_flight` is Some, in order, cap 64; empty whenever in_flight is None
}
pub const TAKE_QUEUE_CAP: usize = 64;

// graft/src/update/resolver/walk.rs
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>;   // any state; None past the end
pub fn after_take(file: &ConflictFile, taken: usize) -> Option<usize>;                          // next open after `taken`, wrapping; None = none open
pub(super) fn cross_target(state: &AppState, forward: bool) -> Option<usize>;                   // listed_files index: next not-Done row after the open path, wrapping, never itself

// graft/src/update/resolver/take.rs
pub(super) fn pending_file(resolver: &ResolverState) -> Option<Result<ConflictFile, NoSuchBlock>>; // `file` cloned, in_flight then queued applied; None when no file is open
```

## Acceptance (name the verifying test when you tick)
- [ ] `update::resolver::tests::a_take_moves_to_the_next_open_block`: three open blocks. `t` → cursor 1; `t` → 2; `t` → stays on 2, notice `all blocks taken · ⏎ marks three.txt done`.
- [ ] `…::takes_typed_faster_than_the_disk_are_queued_in_order`: `t t t` with no answer between them.
  - One `TakeSide` is out, and `queued.len() == 2`. The cursor is already on 2.
  - As each answer lands, exactly one `TakeSide` is pushed, for blocks 1 and then 2.
  - After the third answer, `in_flight` is `None` and `queued` is empty.
- [ ] `…::a_queued_take_writes_what_git_would` (real git, `three.txt`): `t t t` before any answer lands. On disk the bytes equal `git checkout --theirs three.txt`'s.
- [ ] `…::a_failed_take_drops_the_queue_and_says_so`: `t t t`, then the first answer fails. `in_flight` is `None`, `queued` is empty, the notice ends `· 2 queued takes dropped`, and no further `TakeSide` is pushed.
- [ ] `…::a_take_while_another_job_runs_keeps_its_own_words` (replaces `a_take_or_a_base_in_flight_keeps_its_own_words`, `tests.rs:381`): a base load or a stage in flight → today's words, nothing queued. A take in flight → queued, no refusal.
- [ ] `…::the_65th_queued_take_is_refused`.
- [ ] `…::n_and_N_reach_a_taken_block`: after `t` on block 0, `N` → cursor 0. `o` re-takes it, and the cursor advances to block 1.
- [ ] `…::n_at_the_last_block_crosses_in_list_order`: files `a`, `b`, `c` in the list, with `b` `Done`. On `c`'s last block, `n` → `select_file` of `a` (wrapped past the end, `b` skipped), cursor 0, notice `▸ a`. On `a`'s block 0, `N` → `c`, its last block.
- [ ] `…::n_crosses_before_it_wraps`: `a` has an open block 0 and the cursor is on its last block, and `b` is open. `n` → `b`, not `wrapped`.
- [ ] `…::n_wraps_when_no_other_file_is_open`: one file with block 0 open and the cursor on the last block. `n` → cursor 0, notice `wrapped · 1 still open`.
- [ ] `…::n_past_the_end_with_nothing_open_says_no_more`: notice `no more conflicts · ⏎ marks <file> done`, cursor unmoved.
- [ ] `…::n_on_a_card_crosses_to_the_next_file` (existing, `tests.rs:58`): still passes, now in list order. On the two-file fixture, `n` on `gone.txt`'s card → `notes.txt`, block 0 (J9's first step).
- [ ] `…::a_cross_waits_for_the_take_in_flight`: `t` on the last open block, then `n` before the answer → `still writing the last take`, nothing selected.
- [ ] `…::a_take_on_a_done_file_is_refused_by_done_refusal`: with `done == Some(_)`, `t` → status `<path> is done · d starts it over`. No `TakeSide` is pushed, and `in_flight`/`queued` are untouched.
- [ ] Walk scenario 1 passes with `t b`, and scenario 7 with its new setup and words.
- [ ] Capture `docs/shots/m6/fix-advance-120x40.txt`: `three.txt` after one `t`. **The look is the user's call (rule 6); colour is unverified (rule 5).** What right looks like:
  - Block 1's header reads `conflict 1 of 3 · took theirs · feature`.
  - The `▏` rule is on block 2's rows.
  - The identity row reads `▸ three.txt · 3 conflicts · 2 open`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: two pure functions, one queue with its invariant, one cross rule, ~15 tests, one capture, two TESTING.md scenarios. One context.

## For the conductor (lines outside this ticket)
- Design note :356. Replace "Past the last block: first open block if any (`wrapped · 2 still open`), else today's `no more conflicts` notice." with: "Past the last block: the next file not done, in list order; else this file's first open block (`wrapped · 2 still open`); else `no more conflicts · ⏎ marks <file> done`."
- Design note :612–:613: reorder the owed row to match `02-keymap.md:267`, and mark it written.
- Design note :320–:323: the struct is in `graft/src/update/resolver.rs`. Add `in_flight`. `queued` excludes the take in flight.
- Design note :466: "A take → the cursor moves to the next open block **at the keypress** (P2); takes typed faster than the disk queue, and a failed one drops the queue with a count."
- Design note :169: `⏎ marks token.rs resolved` → `done`.
- `02-keymap.md:270` (`Enter`): "Mark this file resolved" → "done". This is a keymap wording edit.
- CONTEXT.md: add **Done (file)**, "a conflicted file he has finished: staged, no longer unmerged; shown `done ✓`." Avoid: resolved.
- Ticket 25 :71 and :135, and ticket 17: crossing in list order is now 15's. Ticket 17 keeps only "`n` crossing *from* a done file".
- Ticket 19 item 3: "record per-block choices in `take_landed`, the one site every take answer passes, queued ones included".

# fix-take-advances-to-next-block: `o`/`t`/`b` move the cursor to the next open block; `n`/`N` reach taken blocks, so a take can be fixed by keyboard, and cross to the next file at the end

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 13-resolver-file-list.md, 10-resolver-line-numbers.md
Was: draft · **P2 approved** (user, 2026-09-23: "keep all 5"). The §2.6 keymap rows are to be written into `02-keymap.md` by the conductor (the note's "Keymap rows owed"). The file-crossing clause (item 3) is Amendment 2's reading of P2, which J9 counts on.
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Opus**: the take queue is new state over an async write, judged by tests.
Needs: `fix-resolver-file-list` (`select_file`, list order, `FileState::Done`) for item 3's crossing. `fix-resolver-line-numbers` (`SideNames`) for the notice words.
Crates: graft.

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P2; Interfaces (the P2 block).
  - Amendment 2: Journeys J5, J9, "Where each event lands him"; Keymap rows owed (§2.6); New UI, the card mockup (a taken header).
- `crates/graft/MAP.md` §`update::resolver` (`ResolverState`, `taken`, `next_open_block`, `scroll_block_into_view`, the `still writing the last take` refusal).
- `docs/design/02-keymap.md` §2.6.
- TESTING.md "Conflicts walk" scenario 1 (`n t n b`) and scenario 7 (`n` past the last block).
- **Scout questions:**
  1. Where `o`/`t`/`b` set `job` and refuse while a take is in flight.
  2. Where `cursor_block` is set on a `Taken` answer.

## Scope
In:
1. **Advance.** When `o`/`t`/`b` is pressed, the cursor moves at once to `after_take(file_with_this_take_applied_in_memory, taken)`, computed on a clone with `ConflictFile::take` (pure), and scrolls into view. `None` → the cursor stays, and the notice reads `all blocks taken · ⏎ marks <file> done`. This applies in both arrangements (card and three panes). The whole-file card is not affected: it has one block and its `o`/`t` mark the file done.
2. **Queue, not refuse.**
   - `o`/`t`/`b` pressed while a take is in flight are appended to `queued` (cap 64; the 65th is refused with today's words) and sent in order as each answer lands.
   - A failed take or a `Changed` re-read drops the queue, with the notice `take failed · re-read · <n> queued takes dropped`.
   - Every answer records `AppState::choices` exactly as a single take does (`fix-finish-summary-screen` item 3).
3. **`n`/`N`.** `next_block` walks every block, taken ones included. At the last block (for `N`, the first):
   - Another block in this file is open → go to the first one (for `N`, the last), with the notice `wrapped · <k> still open`.
   - Otherwise, the list holds another file not `Done` → `select_file` it, in list order, wrapping. `n` lands on its block 0, `N` on its last block. The notice reads `▸ <file>`.
   - Otherwise → today's `no more conflicts · ⏎ marks <file> done`.

   A cross while a take is in flight or queued is refused with `still writing the last take`.
4. **TESTING.md.** Update scenario 1's keys (`n t n b` → `t b`) and the walk's steps. Scenario 7 keeps its words when the fixture has one file.

Out: bulk takes (P3); keys other than `n`/`N`'s reach; P7's `j`/`k`.

## Signatures
```rust
// graft/src/update/resolver/walk.rs
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>;   // any state; None past the end
pub fn after_take(file: &ConflictFile, taken: usize) -> Option<usize>;                          // next open after `taken`, wrapping; None = none open
// graft/src/update/resolver/mod.rs
pub struct ResolverState { /* existing */ pub queued: VecDeque<(usize, Take)> }
```

## Acceptance (name the verifying test when you tick)
- [ ] `update::resolver::tests::a_take_moves_to_the_next_open_block`: three open blocks. `t` → cursor 1; `t` → 2; `t` → stays on 2 with the notice `all blocks taken`.
- [ ] `…::takes_typed_faster_than_the_disk_are_queued_in_order`: `t t t` with no answer between them → one `TakeSide` out and two queued. When the answers land there are three takes on disk, in block order 0, 1, 2, and the bytes equal git's `--theirs`.
- [ ] `…::a_failed_take_drops_the_queue_and_says_so`.
- [ ] `…::n_and_N_reach_a_taken_block`: after `t` on block 0, `N` → cursor 0; `o` re-takes it; the cursor advances to block 1.
- [ ] `…::n_at_the_last_block_crosses_to_the_next_file`: two-file fixture, on `gone.txt` (one open whole-file card), `n` → `notes.txt`, block 0.
- [ ] Walk scenario 1 passes with `t b`.
- [ ] Capture `docs/shots/m6/fix-advance-120x40.txt`: `three.txt` after one `t`. **What right looks like:**
  - Block 1's header reads `conflict 1 of 3 · took theirs · feature`.
  - The `▏` rule is on block 2's rows.
  - The identity row reads `▸ three.txt · 3 conflicts · 2 open`.
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium: two pure fns, one queue, one cross rule, five tests, one capture, one TESTING.md edit. One context.

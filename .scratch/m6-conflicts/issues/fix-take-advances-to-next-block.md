# fix-take-advances-to-next-block: `o`/`t`/`b` move the cursor to the next open block; `n`/`N` reach taken blocks so a take stays fixable by keyboard

Status: draft · **gated on Proposed change P2** (02-keymap §2.6 `n`/`N` row). Do not file until approved.
Build with: set by the conductor after a scout (brownfield); expected **Opus** — the take queue is new state over an async write, judged by tests.
Needs: none (runs after `fix-resolver-line-numbers` if both touch `view::resolver`; shares `update/resolver/` with `fix-take-every-open-block` — same group or serial).
Crates: graft.

## Read first
- `docs/design-notes/m6-conflicts.md` — Amendment: Prior art row "Advance after a take", Journey J5, Interfaces (P2 block), Where I'd differ P2, New UI ("after t").
- `crates/graft/MAP.md` §`update::resolver` (`ResolverState`, `taken`, `next_open_block`, `scroll_block_into_view`, the `still writing the last take` refusal).
- `docs/design/02-keymap.md` §2.6 (as amended by P2).
- TESTING.md "Conflicts walk" scenario 1 (`n t n b`), scenario 7 (`n` past the last block).
- Scout questions: where `o`/`t`/`b` set `job` and refuse while a take is in flight; where `cursor_block` is set on a `Taken` answer.

## Scope
In:
1. **Advance.** When `o`/`t`/`b` is pressed, the cursor moves at once to `after_take(file_with_this_take_applied_in_memory, taken)` (compute on a clone with `ConflictFile::take`; pure) and scrolls it into view. `None` → cursor stays; notice `all blocks taken · ⏎ marks <file> resolved`.
2. **Queue, not refuse.** `o`/`t`/`b` pressed while a take is in flight are appended to `queued` (cap 64; the 65th is refused with today's words) and sent in order as each answer lands. A failed take or a `Changed` re-read drops the queue; notice `take failed · re-read · <n> queued takes dropped`.
3. **`n`/`N` (P2).** Walk every block, taken included, via `next_block`. Past the last (or before the first): the first open block if any, notice `wrapped · <n> still open`; none open → today's `no more conflicts · ⏎ marks <file> resolved`. Crossing to the next file stays `⏎`'s job.
4. Update TESTING.md scenario 1's keys (`n t n b` → `t b`) and the walk's steps; scenario 7 keeps its words.

Out: bulk takes (P3); any key other than `n`/`N`'s reach; the whole-file card (it has one "block").

## Signatures
```rust
// graft/src/update/resolver/walk.rs
pub fn next_block(file: &ConflictFile, from: Option<usize>, forward: bool) -> Option<usize>;
pub fn after_take(file: &ConflictFile, taken: usize) -> Option<usize>;
// graft/src/update/resolver/mod.rs
pub struct ResolverState { /* existing */ pub queued: VecDeque<(usize, Take)> }
```

## Acceptance (name the verifying test at tick time)
- [ ] `update::resolver::tests::a_take_moves_to_the_next_open_block` — three open blocks: `t` → cursor 1; `t` → 2; `t` → stays 2, notice `all blocks taken`.
- [ ] `…::takes_typed_faster_than_the_disk_are_queued_in_order` — `t t t` with no answer between: one `TakeSide` out, two queued; answers land → three takes on disk, block order 0,1,2, bytes equal git's `--theirs`.
- [ ] `…::a_failed_take_drops_the_queue_and_says_so`.
- [ ] `…::n_and_N_reach_a_taken_block` — after `t` on block 0, `N` → cursor 0; `o` re-takes it; cursor advances to block 1.
- [ ] Walk scenario 1 passes with `t b`; J5 in the journey walk at `t t t ⏎` (the acceptance ticket asserts the count).
- [ ] Capture `docs/shots/m6/fix-advance-120x40.txt`: `three.txt` after one `t`. **What right looks like:** block 1's header reads `took theirs`, the `▏` rule is on block 2's rows, the title reads `2 of 3 conflicts open`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc` — workspace-wide.

## Size
Small–medium: two pure fns, one queue, four tests, one capture, one TESTING.md edit. One context.

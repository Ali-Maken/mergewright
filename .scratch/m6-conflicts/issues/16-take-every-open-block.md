# fix-take-every-open-block: `Ctrl+O` / `Ctrl+T` take yours / theirs in every open block of this file, in one write

Status: done 2026-09-24 (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
**Revised 2026-09-24 in place (planner, spec-only), from `.scratch/m6-conflicts/scout-16.md` (main `f585af1`), items 1–5:**
- 1: `apply_take_all` takes the **document**, like `apply_take` (`graft-ops/src/conflict.rs:151`; MAP §conflict `apply_take`). `expected: Oid` is dropped: a re-read renders taken blocks as plain lines, so they would vanish and the rest re-index.
- 2: the queue element becomes `(TakeAt, Take)`. `TakeAt::Every` is one entry, one write, one keypress (D2).
- 3: `Ctrl+O`/`Ctrl+T` go through 28's `done::refuses`, and are dispatched in `act` ahead of the non-plain swallow (D3).
- 4: `AppState::choices` is removed from scope. It is ticket 19's.
- 5: written against **ticket 28's post-split layout**. 16 builds after 28 merges.
Was: draft · **P3 approved** (user, 2026-09-23: "keep all 5"). §2.6 row written (`02-keymap.md:263`). Still open: whether Zed's terminal claims `Ctrl+T`. The acceptance capture from his terminal checks it (**unverified** until then).
Blocked by: `28-done-file-start-over-only-where-it-can` (splits `update/resolver.rs`, adds `handover.rs`, `done::refuses`, the `Esc`/`e`/`M` queue refusals). 15 and 10 are merged.
Build with: **Sonnet** (scouted: `scout-16.md`). Every shape is fixed below: `take_all` is `take` in a loop, `apply_take_all` is `apply_take` with one call swapped, and the queue change is one enum threaded through named sites. Not Fable: nothing later hangs on it, and the tests judge it.
Needs:
- graft-git: `ConflictFile::take`, `open`, `render`, `write_back` (`conflict/document.rs:66`, `conflict/disk.rs:76`).
- graft-ops: `apply_take`, `TakeError` (`conflict.rs:151`, `:114`).
- graft (after 28): `done::refuses`, `done_refusal`; 15's `in_flight`/`queued`/`TAKE_QUEUE_CAP`/`pending_file`/`take_side`/`take_refusal`/`take_landed`; 10's `side_names`/the took-line; `side_keys`.
Crates: graft-git, graft-ops, graft-ui (`hints` Resolver table), graft.

## Read first
- This ticket. `.scratch/m6-conflicts/scout-16.md` (whole).
- `.scratch/m6-conflicts/issues/28-done-file-start-over-only-where-it-can.md`: R5, R6, R7, and its Signatures (the layout you build on).
- `docs/design-notes/m6-conflicts.md`: Amendment 1 "Approved changes" P2, P3; Amendment 2 Journeys J5, J8, J9.
- `docs/research/conflict-resolution.md` §1.5 (the lazygit maintainer's caveat: "every block" is not `checkout --theirs`).
- `crates/graft-git/MAP.md` §conflict (`take`, `open`, `write_back`); `crates/graft-ops/MAP.md` §conflict (`apply_take`, `TakeError`); `crates/graft/MAP.md` §`update::resolver` (as 28 left it: `in_flight`/`queued`, the take advance, `done_refusal`, `side_keys`); `crates/graft-ui/MAP.md` §`graph_screen::hints`.
- `docs/design/02-keymap.md` §2.1 ("Bulk variants use a modifier", :142), §2.6 :263.

## Decisions
- **D1: the document, not an `Oid`.** `apply_take_all(repo, path, file, take)`:
  - clone `file`, then `take_all`, then `write_back(.., file.fingerprint)` once, then set the new fingerprint;
  - `Changed { now }` leaves the bytes untouched, the same contract as `apply_take`.
  - Blocks already `Resolved` keep their take and their index, and `blocks().len()` is unchanged.
- **D2: the queue carries where the take lands.** New `TakeAt { Block(usize), Every }`.
  - It replaces `usize` in `in_flight`, `queued` (`update/resolver.rs:174`) and `TakeRequest.block` (`update/events.rs:640`, renamed `at`).
  - `pending_file` replays `Block(i)` as `take(i, t)` and `Every` as `take_all(t)`.
  - The job (`jobs/conflict.rs:247`) sends `Block` to `apply_take` and `Every` to `apply_take_all`.
  - **Cap:** an `Every` entry is **one** entry against `TAKE_QUEUE_CAP` (one keypress, one write). When full, it is refused like any take: `still writing the last take`.
  - **Dropped count:** a failed take, a `Changed` re-read or a hand-back drops the queue. An `Every` entry counts **1** in `<n> queued takes dropped`, and the words are unchanged. The count is keypresses, which is what he typed.
  - **Count for the notice:** read at landing, not carried. It is the request's `file.open()`, the document the write was applied to. `take_landed` sends each request against the document that landed (MAP §`update::resolver`), so this is exact. No count field travels in `TakeAt`.
- **D3: order of checks on `Ctrl+O`/`Ctrl+T`** (control set, alt not set; `Ctrl+Alt+…` is still swallowed):
  1. 28's `done::refuses(key)` returns `true` for them, so a done file answers with `done_refusal`'s words (28 R5).
  2. `act` dispatches them to `take::take_all_key` **before** the non-plain swallow (pre-split `resolver.rs:329`).
  3. A whole-file or binary card (`whole_file.is_some()`) → status `this card is one choice · <side_keys>`, for example `this card is one choice · o yours  t theirs`. `side_keys` is used, not a literal `o or t` (MAP `side_keys`: every card status offering `o`/`t` uses it).
  4. `take_refusal`: another job (read, base load, git write), or the cap. Words as today.
  5. `pending_file` has `open() == 0` → notice `nothing open in <file>`. No entry and no command.
  6. Otherwise `take_side(.., TakeAt::Every, take, ..)`: send it, or queue it behind `in_flight`.
- **D4: cursor and notice.**
  - At the keypress, the cursor stays and `top` is unchanged. No `all blocks taken` notice is set at the press.
  - At landing, **when the landed entry is `Every`**, the notice is the took-line from `side_names` + ` in <n> blocks` (` in 1 block` when n = 1).
    - If the landed document has `open() == 0` and nothing is queued, it continues with ` · ⏎ marks <file> done`: `took theirs · feature in 3 blocks · ⏎ marks three.txt done`. This replaces `all_taken` (`answers.rs:243`) for that landing.
    - If takes are queued behind it, the notice is the took-line only. Each later landing sets its own.
- **D5: choices are out.** Ticket 19 owns `AppState::choices`. Every take lands through `take_landed`, so 19 records per block there. The conductor adds that line to 19 (below).

## Scope
In:
1. graft-git: `ConflictFile::take_all` (D1). It sits beside `take` in `conflict/document.rs`. MAP §conflict gets one line.
2. graft-ops: `apply_take_all` (D1), beside `apply_take`. MAP §conflict gets one line.
3. graft, in **28's post-split layout**. Paths are as 28 leaves them; where 28's M3 split moved `take_landed` out of `answers.rs`, follow MAP §`update::resolver`.
   - `update/events.rs`: `TakeAt`, and `TakeRequest.at`.
   - `update/resolver.rs`: `in_flight`/`queued` retyped; the `act` arm for `Ctrl+O`/`Ctrl+T` (D3 step 2).
   - `update/resolver/done.rs`: `refuses` gains the two keys (D3 step 1).
   - `update/resolver/take.rs`: `take_all_key`; `take_side` takes `TakeAt`; `pending_file` replays `Every`.
   - `update/resolver/answers.rs` (or its 28 split): `take_landed` reads `TakeAt` for the notice (D4) and sends the queue front's `TakeAt`.
   - `jobs/conflict.rs`: dispatch per D2.
   - `update/keys.rs` `BOUND_ACTIONS` (`:755`, resolver rows `:806–:820`): two rows.
   - `handover_refusal` and the `Esc` refusal (28) read only `is_some`/`is_empty`, so they need no change. Confirm it in NOTES.
4. graft-ui `hints/lists.rs` `resolver_list` (`:255`):
   - After `b both`, add `^O all <ours role>  ^T all <theirs role>`, with the same role words as that table's `o`/`t`.
   - Short rung `^O/^T all`.
   - Not in `resolver_card_list` or `resolver_done_list`.
5. MAP.md: graft §`update::resolver` (`TakeAt`, the bulk arm, D4 words); graft-ui §hints.

Out:
- `Ctrl+B` (tmux's prefix).
- Staging (still `⏎`; P6's `O`/`T` are ticket 17).
- Any other file.
- `AppState::choices` (ticket 19).

## Signatures
```rust
// graft-git/src/conflict/document.rs
impl ConflictFile {
    /// Every `Conflict` region becomes `Resolved { take }`; `Resolved` ones are untouched. How many changed.
    pub fn take_all(&mut self, take: Take) -> usize;
}

// graft-ops/src/conflict.rs
/// `apply_take` for every open block: one `write_back` against `file.fingerprint`. Takes the document, not an Oid (D1).
pub fn apply_take_all(repo: &RepoHandle, path: &Path, file: &ConflictFile, take: Take) -> Result<ConflictFile, TakeError>;

// graft/src/update/events.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TakeAt { Block(usize), Every }
pub struct TakeRequest { /* path, file, */ pub at: TakeAt, pub take: Take }   // was `block: usize`

// graft/src/update/resolver.rs
pub struct ResolverState { …, pub in_flight: Option<(TakeAt, Take)>, pub queued: VecDeque<(TakeAt, Take)> }

// graft/src/update/resolver/done.rs (28's; extended)
pub(super) fn refuses(key: &KeyPress) -> bool;   // + Ctrl+O, Ctrl+T (no Alt)

// graft/src/update/resolver/take.rs
pub(super) fn take_all_key(state: &mut AppState, commands: &mut Vec<Command>, take: Take, next_id: /* take_key's type */) -> bool;
// take_side: `block: usize` → `at: TakeAt`; pending_file: replays TakeAt.
```

## Acceptance (name the verifying test when you tick)
- [x] graft-git `conflict::tests::take_all_equals_taking_each_block` (verified: `conflict::tests::take_all_equals_taking_each_block`, all 7 marker files × 3 takes): for every fixture file and each `Take`, `take_all` renders byte-identical to `take(i)` over every open `i`, and returns that count.
- [x] graft-git `conflict::tests::take_all_keeps_blocks_already_taken` (verified: same name): `three.txt`, `take(0, Ours)`, then `take_all(Theirs)` → returns 2, block 0 is still `Ours`, and `blocks().len() == 3`.
- [x] graft-ops `conflict::tests::take_all_never_clobbers_a_changed_file` (verified: same name): the bytes change on disk → `Changed { now }`, and the file's bytes are unchanged.
- [x] graft-ops `conflict::tests::apply_take_all_after_a_take_keeps_it` (verified: same name; also re-reads disk): `apply_take(.., 0, Ours)`, then `apply_take_all(&landed, Theirs)` → the render equals per-block (ours, theirs, theirs), and `blocks().len() == 3` (D1: the case the `Oid` form lost).
- [x] (verified: `it::resolver::take_all::ctrl_t_takes_theirs_everywhere_then_enter_resolves` — real git lives in the `it` harness, not `update::resolver::tests`; expected bytes are the parser's per-block theirs; for `three.txt` these equal `checkout --theirs`, so the "differ" case is not exercised) graft `update::resolver::tests::ctrl_t_takes_theirs_everywhere_then_enter_resolves`, real git: on `three.txt`, `Ctrl+T ⏎` → the bytes equal per-block theirs (not `checkout --theirs` where the two differ), and `ls-files -u` no longer lists the file.
- [x] (verified: `update::resolver::tests::take_all::ctrl_t_waits_behind_a_queued_take`) graft `…::ctrl_t_waits_behind_a_queued_take`: `o` (in flight, block 0), then `Ctrl+T`:
  - `queued == [(Every, Theirs)]`, one `TakeSide` out.
  - The first landing sends `at: Every` against the landed document.
  - The final render is ours, theirs, theirs.
- [x] (verified: `update::resolver::tests::take_all::a_bulk_take_is_one_queued_entry`) graft `…::a_bulk_take_is_one_queued_entry`:
  - With `in_flight` set and 63 queued, `Ctrl+T` queues (64). The next `t` or `Ctrl+O` → `still writing the last take`.
  - A `Changed` answer with `[Block, Every]` queued → `… · 2 queued takes dropped`, and the count survives the re-read (28's review-15 M2).
- [x] (verified: `update::resolver::tests::take_all::ctrl_t_with_nothing_open_writes_nothing`) graft `…::ctrl_t_with_nothing_open_writes_nothing`: all blocks taken (pending included) → notice `nothing open in three.txt`, no command, `queued` unchanged.
- [x] (verified: `update::resolver::tests::take_all::ctrl_t_notice_counts_the_blocks`; queued case `…::ctrl_t_notice_waits_while_takes_are_queued`) graft `…::ctrl_t_notice_counts_the_blocks`:
  - Merge: `Ctrl+T` lands → `took theirs · feature in 3 blocks · ⏎ marks three.txt done`, and the cursor block is unchanged from before the press.
  - After `t` on block 0 has landed, `Ctrl+O` → `… in 2 blocks …`.
  - One open block → `in 1 block`.
- [x] (verified: `update::resolver::tests::take_all::ctrl_t_on_a_card_is_one_choice`) graft `…::ctrl_t_on_a_card_is_one_choice`: a whole-file card and a binary card → `this card is one choice · o yours  t theirs`, no command.
- [x] (verified: `update::resolver::tests::take_all::ctrl_t_on_a_done_file_is_refused`) graft `…::ctrl_t_on_a_done_file_is_refused`: `Ctrl+O` and `Ctrl+T` on a done file → `done_refusal`'s words (28 R5, the can and cannot forms), no command.
- [x] (verified: `update::resolver::tests::take_all::ctrl_alt_t_does_nothing`; asserts no TakeSide/Operate, resolver and status unchanged) graft `…::ctrl_alt_t_does_nothing`: no take, no words (the swallow is unchanged for Alt).
- [x] (corrected by ticket 32, review m6-amend-16 M2: as first ticked, the built rung was `^O all  ^T all`, not the combined form. Now verified: `graph_screen::hints::tests::resolver_offers_take_all` — one hint `⌃O/⌃T all` after `b both`, merge and rebase, absent from card and done lists) graft-ui `graph_screen::hints::tests::resolver_offers_take_all`: the table has `^O all …  ^T all …` after `b both`, the short rung is `^O/^T all`, and it is absent from the card and done lists.
- [x] Journey `three-conflicts-all-theirs` at 2 keys (acceptance ticket 21). Walked at the note's current count, **1** (P6 moved it: `T` finishes the file, no `⏎`): `every_scenario::journey_three_conflicts_all_theirs`, route `T`; bytes equal `git merge-file -p --theirs` block by block (ticket 21).
- [x] (written and pinned: `it::resolver::take_all::take_all_capture`; look NOT judged — user's eye, rule 6) Capture `docs/shots/m6/fix-take-all-120x40.txt`, pinned by `it::resolver::take_all_capture`. **What right looks like:**
  - All three block headers read `took theirs · feature`.
  - The notice reads `took theirs · feature in 3 blocks · ⏎ marks three.txt done`.
  - The bar shows `^T all theirs` or `^O/^T all`.
  - The user judges by eye (rule 6). Colour is unverified (rule 5). `Ctrl+T` in Zed's terminal stays **unverified** until his capture.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small–medium across four crates. One context: two leaf functions, one enum threaded through six named sites, one hint row, ~13 tests and one capture.

## For the conductor (outside this ticket)
- `docs/design-notes/m6-conflicts.md:330` → `pub fn apply_take_all(repo: &RepoHandle, path: &Path, file: &ConflictFile, take: Take) -> Result<ConflictFile, TakeError>;   // the document, as apply_take (ticket 16 D1)`
- `docs/design-notes/m6-conflicts.md:323` → replace both `(usize, Take)` with `(TakeAt, Take)`, and append `; pub enum TakeAt { Block(usize), Every } in update/events.rs (ticket 16); a bulk take is one queue entry`.
- `docs/design/02-keymap.md:263`: append `On a whole-file card or a done file, refused in words.`
- Ticket 19: add "a take lands through `take_landed` as `TakeAt::Block` or `TakeAt::Every`; `Every` records a choice for every block the landed document took that the request's document had open."

## NOTES (builder, 2026-09-24)
Scout: used `scout-16.md` (pre-28); re-found sites by name. No compaction.
- Ticks 14/15. Journey `three-conflicts-all-theirs` left to acceptance ticket 21.
- `handover_refusal`/`Esc` refusal: read only `in_flight.is_some()` / `queued.is_empty()` (`handover.rs:21`) — unchanged, confirmed.
- **Bar cost (by eye, user's call):** two new full hints push the resolver bar past its room. At 120 cols it now reads `… ^O all yours  ^T all theirs  B base  e edit  n next  N prev  ⏎ resolved  M mergetool  d  esc` (`d start over`/`esc close` lose verbs); at 80 `… ^T all  B  e  n  N  ⏎  M  d  esc`. Ladder raises in list order, so the takes keep full rungs ahead of the tail. 7 committed captures regenerated (`fix-card-names-numbers-*`, `fix-file-list-many-120x40`, `fix-file-list-rebase-step2-120x40`, `fix-stop-lands-in-resolver-*`); `it::resolver::enter_on_conflicts_row_opens_the_resolver` now looks for ` esc`, not `esc close`.
- **Short rung deviation:** the ladder has per-hint rungs; `^O/^T all` as one entry is not expressible. Built `^O all  ^T all` (short form `all` for the six `all <role>` verbs, added to `short_form` and to the user's-table test with a comment citing P3). Flag for conductor.
- **Glyph:** spec says `^O`/`^T`; the rest of the app writes `⌃A`, `⌃R`. Followed the spec; likely wording drift — conductor's call.
- Status line: an `Every` landing sets only the notice (no `took … · k of m open` status), as D4 names only the notice.
- `Ctrl+T` in Zed's terminal: unverified. Colour unverified.

# fix-done-file-start-over-only-where-it-can: `d` is offered on a done file only where ops can start it over; the box never promises what ops refuses

Status: draft (filed 2026-09-24 by the planner from `docs/reviews/m6-amend-25.md` Major 1, 2, 3; main `0f7fb66`)
Blocked by: 15-take-advances-to-next-block.md (building in `graft` now, same crate; ADR-0036). 25 and 27 are merged.
Build with: **Opus** (recommended). Brownfield (ADR-0035): scout first, and the conductor confirms after it. One ops read-only gate plus the app wiring. Every rule is fixed here and tests can judge it. Not Fable: fails test 1, because nothing later hangs on this shape.
Needs: graft-ops `apply::stage_source`, `operation::shape_of_stages`, `check_restart`, `prepare_restart` (all existing; MAP §operation `RestartParams`, §apply). graft: 25's `DoneShape`, `done_refusal`, `write_not_landed`, `jobs::conflict::open`, `Opening`, `OpenFailure`; `restart_box` (`update/resolver/restart.rs`). graft-ui: `HintContext::Resolver { done, restart, .. }`.
Crates: graft-ops, graft, graft-ui.

## Why
- **M1 (review 25).** Every done file shows `d starts it over` and `d start over`, and `d` always opens the box ("the conflict comes back with all its markers"). ops then refuses **after** the confirm:
  - a whole-file shape in an app-begun merge (`NotMarkerShape`, `graft-ops/src/apply/restart.rs:194`);
  - any done file of a shell-begun merge (`NoStagesKept`, `:178`, `:201`; gap row M6/25).
- The app's pre-box check (`update/resolver/restart.rs:32`) reads only `whole_file`, and `settle` (`update/resolver/done.rs:97`) clears it on every done file. m6-05 Major 1 was this exact defect on the card route. It came back because the app **mirrors** ops' rule instead of asking ops.
- Breaks `02-keymap.md:111` (a key shown must work) and UX H5 (block the impossible; don't warn after). As built, learning "no" costs `d → ⏎` = 3 keys and a false promise.
- **M3.** `update/resolver.rs` is 414 lines, before ticket 15 adds more (PROMPT.md 6.2).

## Decisions
- **R1: one gate, in ops.** New `graft_ops::apply::restart_gate` is the only thing that decides whether `d` can start a path over. `prepare_restart` calls it first and keeps nothing of its own, so the offer and the act cannot drift.
  - Unmerged path: `check_restart`'s rule (`how.has_markers()`), answering `Index`.
  - Not unmerged: `stage_source`, then `shape_of_stages` and `has_markers`.
  - It only reads: no journal line, no ref moved, no file written.
- **R2: asked when the file opens, off the UI thread.** `jobs::conflict::open` calls the gate on the **done route**, before it reads the file, so a failed read still carries the answer.
  - It reads `RepoFacts` (`read_facts`, `with_remotes: false`) for the gate. The route itself still comes from the job's own status read, as today.
  - Staleness: the answer can go stale if the repo changes behind the app. ops' own refusal after the confirm stays as the backstop. That path is the race only, and it releases `restarting` (25's `write_not_landed`).
- **R3: one app predicate.** `start_over_refusal(state)` gives `None` exactly when `d` opens the box. It is read by three places, and none keeps a copy:
  - `restart_box`, before the box. The transient refusals (a write running, still reading) stay ahead of it, unchanged.
  - `view::rows::resolver_context` → `HintContext::Resolver.restart`.
  - The done identity row and the done card's keys.
- **R4: hide, and word it on the press.**
  - **Can:** unchanged from 25.
    - Identity row: `▸ <path> · done ✓ · d starts it over` (+ ` · edited since done`).
    - Bar: `d start over  e edit  esc close`.
    - Other keys: `<path> is done · d starts it over`.
  - **Cannot:**
    - Identity row: `▸ <path> · done ✓` (+ ` · edited since done`).
    - Bar: exactly `e edit  esc close`.
    - The card's keys (removed, binary, unreadable) are empty.
    - `d`: status line `can't start over · <ops' explain()>`, for example `can't start over · gone.txt: deleted by them — no markers to take from`. No box, no command.
    - Other refused keys: `<path> is done`.
    - The gate could not be read (a non-`Refused` error): `Cannot` with why = `could not check: <explain()>`. **Never** `Can` on an error (rule 3).
  - Unmerged whole-file card: today's words and today's refusal, now through `start_over_refusal`. Its bar must not show `d` (asserted below; fix it if it does).
- **R5: split `update/resolver.rs`** by pure moves (review 25 M3):
  - `act`'s done gate → `update/resolver/done.rs` as `refuses(key)`.
  - `edit_here` (and the `M` hand-over if it sits beside it) → a new child, `update/resolver/handover.rs`.
  - Target: under 400 lines **after** 15's additions. If no seam gets there, report the count and say so. Use `codebase-design`.

## Read first
- This ticket. `docs/reviews/m6-amend-25.md` Major 1, Major 3.
- `.scratch/m6-conflicts/issues/25-stable-file-order-and-done-file-open.md`: Decisions D2, and NOTES "Spec contradiction".
- `crates/graft-ops/MAP.md`: §operation `RestartParams` and `shape_of_stages` (:10–:11); §apply `StageSource`/`stage_source`, `prepare_restart` (:155–:156); §operation `Refusal` m6 additions (:47).
- `crates/graft/MAP.md`: §`update::resolver` (:307–:328); §`jobs` `conflict` (:695); §`update::confirm` `RestartConflict`.
- `crates/graft-ui/MAP.md`: `HintContext::Resolver`, `every_context`.
- `docs/design/02-keymap.md` :111–:114 and the §2.6 `d` row (:266).
- `ARCHITECTURE.md` before the split.

## Scout questions (brownfield; ≤1 page, `.scratch/m6-conflicts/scout-28.md`)
1. `prepare_restart`'s exact signature and error type. Does `stage_source` need more of `RepoFacts` than `status` and `in_progress`?
2. What `jobs::conflict::open` holds: can it call `read_facts`, as `conflict_set` does?
3. Every reader of `ResolverState::done` and `OpenFailure::done` (the type changes).
4. `update/resolver.rs` line count after 15, and where `act`'s done gate and `edit_here` sit.
5. On an unmerged whole-file card, does `resolver_context` draw `d` in the bar today?
6. Which `it/resolver/file_list.rs` tests pin "box, then refusal" (`:196`). They change.

## Scope
In:
1. graft-ops: `restart_gate` (Signatures). `prepare_restart` calls it and drops its own copy of the two checks. MAP §apply.
2. graft job: on the done route, `restart_gate` → `StartOver`. It goes into `Opening::Done` on success and into `OpenFailure.done` on failure (was `bool`).
3. graft update:
   - `ResolverState.done: Option<DoneOpen>`.
   - `start_over_refusal`, and `restart_box` uses it.
   - `done_refusal` words per R4.
   - `conflict_opened`/`open_failed` fill `start_over`.
4. graft view: the identity row, the done card's `keys`, and `resolver_context.restart`, from `start_over_refusal`.
5. graft-ui: `HintContext::Resolver { done: true, restart: false }` gives exactly `e edit`, `esc close`. Add it to `every_context` and to `coverage_slot`.
6. R5's split. MAP.md for graft-ops §apply and graft §`update::resolver`/§`jobs`.

- **Conductor ruling 2026-09-24 (from ticket 15 NOTES):** `e`/`M` while a take is in flight or queued are **refused in words** (`a take is still landing` or similar), not handed over. Today `edit_here` has no refusal and `handed_back` drops the queue with `<n> queued takes dropped` — the editor would open a file a take is still writing, and the user's typed takes are lost. Fix at the cause: `edit_here`/mergetool check `in_flight`/`queued`. Failing test first.
Out:
- Making `d` **work** on whole-file shapes, or in shell-begun merges. See the note amendment's P10 and gap row M6/25.
- Review 25 Minors 2–5. `e` on a removed card.

## Signatures
```rust
// graft-ops/src/apply/restart.rs (re-exported from `apply`)
/// The one answer to "can `d` start this path over?". Read-only. `prepare_restart` calls it first.
/// Ok = where the stages will come from. Err(Refused(NotMarkerShape | NoStagesKept)) = ops' refusal.
/// Any other Err = could not tell.
pub fn restart_gate(repo: &RepoHandle, facts: &RepoFacts, path: &Path) -> Result<StageSource, ApplyError>;
// If prepare_restart's error type today is not ApplyError, use that type, and name the deviation in NOTES. Never a second type.

// graft/src/jobs/conflict.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartOver { Can, Cannot { why: String } }   // why = ops' explain(), raw; sanitised where drawn
pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>, StartOver) }
pub struct OpenFailure { /* words, not_unmerged */ pub done: Option<StartOver> }   // was `done: bool`; Some = the done route

// graft/src/update/resolver/done.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoneOpen { pub shape: DoneShape, pub start_over: StartOver }
// ResolverState: `pub done: Option<DoneOpen>` (was Option<DoneShape>)
pub fn done_refusal(state: &AppState) -> Option<String>;   // R4 words
pub(super) fn refuses(key: &KeyPress) -> bool;             // act's done gate, moved (R5)

// graft/src/update/resolver/restart.rs
/// None ⇔ `d` opens the box. Structural only: the transient refusals stay in restart_box, ahead of it.
pub(crate) fn start_over_refusal(state: &AppState) -> Option<String>;
```

## Acceptance (name the verifying test when you tick)
- [ ] `e`/`M` refused while a take is in flight or queued; nothing dropped — test:
- [ ] graft-ops `it::restart_gate::*`, real git, one test per row. Each asserts the value, and that the journal's line count and every `refs/graft/backup/*` are unchanged:
  - `an_unmerged_marker_file_is_index` → `Ok(Index)`.
  - `an_unmerged_whole_file_is_not_marker_shape` → `Refused(NotMarkerShape)`.
  - `a_marker_file_done_in_an_app_merge_is_backup` → `Ok(Backup { .. })`.
  - `a_whole_file_done_in_an_app_merge_is_not_marker_shape`: `gone.txt` kept with `o`.
  - `a_done_file_in_a_shell_merge_is_no_stages_kept`.
- [ ] graft-ops `it::restart_gate::the_gate_and_the_restart_agree`: for each path above, `restart_gate` is `Err(Refused(r))` ⇔ `apply(RestartConflict)` answers `Refused(r)` (the same `r`). An `Ok` path restarts to `UU`/`AA`.
- [ ] graft `jobs::conflict::tests::a_done_open_carries_the_gate`, real git, app-begun merge: a done marker file → `Done(_, Can)`, and `gone.txt` → `Done(_, Cannot { why })` with `why` containing `no markers`.
- [ ] graft `update::resolver::tests::file_list::a_done_file_that_cannot_start_over_hides_d`:
  - Identity row `▸ gone.txt · done ✓` with no `d`.
  - Hint table exactly `e edit`, `esc close`.
  - `d` → status `can't start over · …`, `confirm` is `None`, no command pushed.
  - `t` → `gone.txt is done`.
- [ ] graft `…::file_list::a_done_file_that_can_start_over_offers_d`: 25's words and bar, unchanged.
- [ ] graft `…::file_list::an_unreadable_done_file_keeps_its_gate`: `OpenFailure { done: Some(Can) }` → the card's keys `d starts it over`. With `Cannot`, the keys are empty.
- [ ] graft `…::file_list::a_gate_that_could_not_be_read_hides_d`: `Cannot { why: "could not check: …" }` → no `d`, and the words on `d` carry the why.
- [ ] graft `…::d_is_in_the_bar_exactly_when_the_box_opens`. Over these fixtures: document, unmerged whole-file card, done text Can/Cannot, removed Can/Cannot, binary, unreadable. For each: the hint table contains `d` ⇔ `d` sets `confirm` to the restart box. **The guard against a third repeat.**
- [ ] graft-ui `graph_screen::hints::tests::a_done_file_that_cannot_start_over_offers_edit_and_close`, and `every_context_reaches_every_shape` with the new slot.
- [ ] graft `it::resolver::file_list::a_whole_file_done_file_offers_no_start_over`, real git (`app_merge_with_a_whole_file_conflict`, `o` on `gone.txt`, click its row):
  - No `d` on the identity row or the bar.
  - `d` → the words, no box.
  - `git status` and the journal are unchanged.
- [ ] graft `it::resolver::file_list::a_shell_merge_done_file_offers_no_start_over`: replaces the box-then-refusal pin (`:196`). No box, the words on `d`.
- [ ] graft `it::resolver::file_list::a_done_marker_file_starts_over_in_place` asserts the journey count. The inputs from the landing after `⏎` are click row · `d` · `→` · `⏎` = **4** = the note's `walk-back-a-done-file`. After them: `UU`, the same cell, and the cursor on block 0.
- [ ] R5: `update/resolver.rs` line count in NOTES, before and after (≤ 400, or the reason). Gates green, and the existing `update::resolver::tests::*` pass unchanged.
- [ ] Captures, pinned by `it::resolver::file_list::done_start_over_capture`: `docs/shots/m6/fix-done-start-over-{can,cannot}-120x40.txt` and `…-cannot-200x50.txt`. **The look is the user's call (rule 6); colour is unverified (rule 5).** What right looks like:
  - can, 120x40: `▸ <marker file> · done ✓ · d starts it over`, and bar `d start over  e edit  esc close`.
  - cannot, 120x40: `▸ gone.txt · done ✓` with nothing after it, bar `e edit  esc close`, and after `d` the status row `can't start over · gone.txt: deleted by them — no markers to take from`.
  - cannot, 200x50: the same identity row beside the list column, one card (25's ruling 4).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one ops read-only function and a refactor of its caller, one job payload, one app predicate across three readers, one hint form, one split, ~14 tests, 3 captures. One context. If it will not fit, split at the crate seam: graft-ops first (the first two boxes).

## For the conductor (outside this ticket)
- **Note amendment, below: apply it to `docs/design-notes/m6-conflicts.md`** (the planner had Write only, no Edit). It fixes review 25 Major 2.
- Ticket 21: walk the two new `journey-counts` rows. `open-a-done-file` asserts 1 input. `walk-back-a-done-file` asserts 4 inputs, and uses `k` in place of the click once 17 (P7) has landed.
- Gap row M6/25: once 28 lands, change "the refusal comes after the box" to "no box: `d` is not offered and answers in words". The row stays **broken** (the capability is still missing) until P10, or 20 gives shell-begun merges a backup.
- The m6-05 Major 1 row, if one is open: the root cause was the app mirroring ops' rule. R1/R3 remove the mirror.

---

## Note amendment — exact text

### A. Append to `docs/design-notes/m6-conflicts.md`, at the end

````markdown
---

## Amendment 3 — 2026-09-24 — a done file: read it, start it over (review m6-amend-25 M1, M2)

Read: `docs/reviews/m6-amend-25.md`; `docs/research/conflict-resolution.md` (**CR**) 1.10, 1.11, §3 :287; `docs/research/ux-principles.md` (**UX**) 1.3, 2, 3.1, 3.5, §5; 02-keymap :111, §2.6 `d`; ticket 25 (spec + NOTES); graft-ops, graft MAPs. No source. Ticket: `28-done-file-start-over-only-where-it-can`.

### Need
- **Who / task:** he pressed `⏎` on a file and then saw he took the wrong side. Or he wants to look again at what he chose before `c`.
- **What hurts:** 25 built it, but `d` is offered on every done file. ops refuses after the confirm on a whole-file shape, and on **any** done file of a merge begun in the shell (J3's start). No field note yet: this comes from review 25, not from his use.

### Prior art (CR · T take · R reject · I invent)
| Aspect | Best answer | Ours before | Rec · why |
|---|---|---|---|
| See a done file | Fork: an open request to show how each conflict was resolved in the file list (CR 1.10, #1304, **(?)** title only); JetBrains: a resolved file leaves the list (CR 1.1) | a done row keeps its place and opens read-only (25) | **T** the Fork ask, as built. H1: what he chose stays one click away |
| Walk back a `⏎` | Tower: "reset a merge conflict to its unmerged state, even after it has been resolved" (CR 1.11 :207; CR §3 :287 recommends **T**) | `d` on every done file; ops refuses two shapes after the confirm | **T** Tower's reset, **only where ops can do it**. **I**: ops' own gate decides the offer when the file opens (UX H5: block the impossible, don't warn after; keymap :111) |
| Why it can't | not in CR | a false promise, then a refusal | **I**: `d` is not drawn, and pressing it answers ops' words with no box (H9) |
| Confirm the walk-back | Tower: **(?)**, not in CR | the box, cursor on Cancel (ADR-0038 §1) | **Keep.** It is not undoable while the operation is open, so the confirm is required (UX 3.5) |

### Journeys (UX 1.3; `[M]` = a decision)
- **J11 walk back a mistaken `⏎`.** Start: the landing after `⏎` (the next file open, the done row above it). Steps: click the done row (or `k` once 17/P7 lands) · `d` [M: read the box] · `→` · `⏎`. **4**. Tower: the count is not in CR **(?)**.
  - Lands: the same file, unmerged again, open blocks, cursor on block 0, `▸` on its row in the same cell.
- **J12 open a done file to read it.** Click its row (or `k`/`j`, P7). **1**. Lands: the file's working-tree lines, read-only; bar `d start over  e edit  esc close`, or `e edit  esc close` where it cannot start over. The Fork ask; no count in CR.
- **As built, where it can't:** `d → ⏎` = 3 keys, the box's false promise, then the refusal. **Now:** no `d` in the bar, and `d` answers in words. **0** keys wasted.

**Where each event lands him (additions).**
- A done row opened → the done file, as J12.
- `d` where ops can → the box; `→ ⏎` → J11's landing.
- `d` where ops can't → no box. Status line `can't start over · <ops' words>`. Stays on the done file.
- ops refuses after the confirm (the repo changed behind the app) → stays on the done file, ops' words, the write released.

### Interfaces (amendment 3)
Ticket 28's Signatures: `graft_ops::apply::restart_gate`, `jobs::conflict::StartOver`, `Opening::Done(Arc<DoneFile>, StartOver)`, `OpenFailure.done: Option<StartOver>`, `DoneOpen`, `start_over_refusal`.

### Hard-to-reverse choices (amendment 3)
1. **Who decides "can `d` start this over"?** Options: the app mirrors ops' rules, or ops' own gate, called by both the offer and the act. **Ops' gate.** The mirror has drifted twice (m6-05 M1, m6-amend-25 M1).
2. **When is it asked?** Options: when the file opens (in the job), or when `d` is pressed (a check job before the box). **When it opens.** The bar is right before he looks (H6). Cost: it can go stale, and ops' refusal after the confirm is the backstop.

### Where I'd differ (Proposed, not decided)
- **P10: `d` starts a whole-file shape over**, in an app-begun merge. `restore::restore_stages` already puts a path's recorded stages back; the working tree goes back to the side git left. The box would read `the choice comes back: keep or delete`. Why: J11 fails today on every modify/delete, which is J5b's shape. Trade-off: a second restart path in ops, and new box words. A merge begun in the shell still cannot, because nothing kept the stages (gap M6/25; REUC rejected, graft-ops MAP §apply `stage_source`).

### New UI (amendment 3)
The done file per Amendment 2 "A done file clicked opens read-only". The identity row and the bar now follow the gate:
```
can, 120x40      ▸ both.txt · done ✓ · d starts it over
                 d start over  e edit  esc close
cannot, 120x40   ▸ gone.txt · done ✓
                 e edit  esc close
   after d       can't start over · gone.txt: deleted by them — no markers to take from      (status row, no box)
```
- The eye goes first to the identity row. Whether `d starts it over` is there is the answer.
- The card (removed / binary / unreadable) carries `d starts it over` as its keys only when the gate says yes.
- Other refused keys: `<path> is done · d starts it over` when it can, `<path> is done` when it can't.

### Genuinely risky tickets (amendment 3)
None. Ticket 28 goes on Opus: every rule is fixed here, and `d_is_in_the_bar_exactly_when_the_box_opens` judges it.
````

### B. Add two rows to the `journey-counts` block, before `a-whole-one-file`

```
open-a-done-file              1                     # Fork #1304 asks for it (CR 1.10); no count in CR
walk-back-a-done-file         4                     # Tower reset-to-unmerged (CR 1.11): count (?)
```

### C. Fix three lines in place (review 25 Minor 1; this ticket's shapes)
- `:571` → `#[derive(Clone, Debug, PartialEq, Eq)] pub enum DoneShape { Text { unstaged: bool }, Binary { unstaged: bool }, Removed, Unreadable { why: String } }   // wrapped in DoneOpen { shape, start_over } (ticket 28)`
- `:575` → `pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>, StartOver) }   // graft jobs::conflict (built: 25; StartOver: 28)`
- `:701–:703`: in the done-file bullet, replace `Bar: \`d start over  n next  N prev  e edit  esc close\`.` with ``Bar: `d start over  e edit  esc close`, or `e edit  esc close` where ops cannot start it over (Amendment 3). A done file is one card at every width (ticket 25, ruling 4).``

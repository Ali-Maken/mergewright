# fix-done-file-start-over-only-where-it-can: `d` is offered on a done file only where ops can start it over; the box never promises what ops refuses

Status: done (built 2026-09-24 on main `f585af1`; see NOTES) — filed 2026-09-24 by the planner from `docs/reviews/m6-amend-25.md` Major 1, 2, 3; main `0f7fb66`)
**Revised 2026-09-24 in place, from `.scratch/m6-conflicts/scout-28.md` (main `4a6703d`), items 1–4:**
- 1: `stage_source` reads `ctx.journal`. The gate reads the journal itself, read-only, through `journal::parse`, the way `operation_conflicts` does. It never calls `Journal::open`, which writes.
- 2: "One gate" is restated against the real chain: `check_restart` (inside `Operation::check`), then `prepare_restart`.
- 3: 442 lines. The split names `mergetool_here` and `edit_here`.
- 4: the pins are listed by line.
Blocked by: none. 15 is merged; 25 and 27 are merged.
Build with: **Opus** (scouted: `scout-28.md`). One ops read-only gate plus the app wiring. Every rule is fixed here and tests can judge it. Not Fable: fails test 1, because nothing later hangs on this shape.
Needs:
- graft-ops: `stage_source`, `prepare_restart` (`apply/restart.rs:67`, `:167`); `check_restart` (`operation/conflict.rs:238`); `shape_of_stages` (`:267`); `journal::parse`, `journal_path`.
- graft: 25's `DoneShape`, `done_refusal`, `write_not_landed`; `jobs::conflict::open`, `Opening`, `OpenFailure`; `restart_box`; 15's `in_flight`/`queued`/`drop_takes`.
- graft-ui: `HintContext::Resolver { done, restart, .. }`, `resolver_done_list`.
Crates: graft-ops, graft, graft-ui.

## Why
- **M1 (review 25).** Every done file shows `d starts it over` and `d start over`, and `d` always opens the box ("the conflict comes back with all its markers"). ops then refuses **after** the confirm:
  - a whole-file shape in an app-begun merge (`NotMarkerShape`, `graft-ops/src/apply/restart.rs:194`);
  - any done file of a shell-begun merge (`NoStagesKept`, `:178`, `:201`; gap row M6/25).
- The app's pre-box check (`update/resolver/restart.rs:32`) reads only `whole_file`, and `settle` (`update/resolver/done.rs:97`) clears it on every done file. m6-05 Major 1 was this exact defect on the card route. It came back because the app **mirrors** ops' rule instead of asking ops.
- It breaks `02-keymap.md:111` (a key shown must work) and UX H5 (block the impossible; don't warn after). As built, learning "no" costs `d → ⏎` = 3 keys and a false promise.
- **M3.** `update/resolver.rs` is **442** lines at `4a6703d` (PROMPT.md 6.2).

## Decisions
- **R1: one set of rules, asked by the offer and the act alike.** Today the act runs two halves:
  - `Operation::check` → `check_restart` (`operation.rs:908`): an **unmerged** path must have a marker shape.
  - `apply` → `prepare_restart` (`apply.rs:200`): a path **not unmerged** goes through `stage_source`; `None` → `NoStagesKept`; `Backup` → `shape_of_stages` → `NotMarkerShape`/`NoStagesKept`; `Index` → `RestartPrep::Nothing`.

  **Change:**
  - (a) Move `prepare_restart`'s refusal half, `stage_source` plus the shape check, into one crate-private core, `restart_source`, over a journal-entry slice.
  - (b) `stage_source(ctx, …)` becomes a wrapper passing `ctx.journal.entries()`. `prepare_restart` calls `restart_source` and keeps only what it does with the answer (`Nothing` for `Index`, the prep for `Backup`).
  - (c) The new public `restart_gate` first calls `check_restart` (made `pub(crate)`) for an unmerged path, else `restart_source`.
  - No rule is written twice. The act reaches the same two functions through `check` and `prepare_restart`; the gate reaches them directly.
- **R2: the gate reads the journal without a lock.** `restart_gate` takes `(repo, facts, path)`, like `operation_conflicts`:
  - It reads `journal_path` and its bytes, then `journal::parse`. A torn last line is not an error (`Parsed.torn_tail`).
  - A missing file means no entries.
  - **Never `Journal::open`** (it cuts a torn tail, a write).
  - The app never touches the journal file (graft-ops seam rule). For the `Stop` the chain walk needs, do what `operation_conflicts` does.
- **R3: asked when the file opens, off the UI thread.** `jobs::conflict::open` calls the gate on the **done route**, before it reads the file, so a failed read still carries the answer.
  - Facts: `system_git()` + `read_facts` with `FactsScope::default()`, as `conflict_set` does (`jobs/conflict.rs:287`). The route still comes from the job's own status read (`:126`).
  - Staleness: the answer can go stale if the repo changes behind the app. ops' refusal after the confirm stays as the backstop. That path is the race only, and it releases `restarting` (25's `write_not_landed`).
- **R4: one app predicate.** `start_over_refusal(state)` gives `None` exactly when `d` opens the box. It is read by three places, and none keeps a copy:
  - `restart_box`, before the box, after `write_refusal`, which is unchanged.
  - `view::rows::resolver_context` → `HintContext::Resolver.restart`. It is `!card` today (`rows.rs:485`).
  - The done identity row and the done card's keys.
- **R5: hide, and word it on the press.**
  - **Can:** unchanged from 25.
    - Identity row: `▸ <path> · done ✓ · d starts it over` (+ ` · edited since done`).
    - Bar: `d start over  e edit  esc close`.
    - Other refused keys: `<path> is done · d starts it over`.
  - **Cannot:**
    - Identity row: `▸ <path> · done ✓` (+ ` · edited since done`).
    - Bar: exactly `e edit  esc close`.
    - The card's keys (removed, binary, unreadable) are empty.
    - `d`: status line `can't start over · <Refusal::explain()>`, for example `can't start over · gone.txt: deleted by them — no markers to take from`. No box, no command.
    - Other refused keys: `<path> is done`.
    - The gate could not be read (`RestartReadError`): `Cannot` with why = `could not check: <explain()>`. **Never** `Can` on an error (rule 3).
  - An unmerged whole-file card already has no `d` (`graft-ui/…/hints/lists.rs:287`). Its words at `restart.rs:32` stay; they now come through `start_over_refusal`.
- **R6: `e`/`M` wait for the takes** (the conductor's ruling, from 15's NOTES). While `in_flight` is `Some` or `queued` is non-empty:
  - `e` and `M` are refused on the status line with `a take is still landing · try again in a moment`.
  - There is no hand-over, and `drop_takes` is not called.
  - Cause: `edit_here` (`resolver.rs:393`) and `mergetool_here` (`:335`) refuse nothing, and `write_refusal` (`:251`) checks only `job`/`operating`. The check goes into the hand-over module (R7) as `handover_refusal`, which calls `write_refusal` first.
- **R7: split `update/resolver.rs`** by pure moves, apart from R6's check (review 25 M3). Use `codebase-design`.
  - `mergetool_here` and `edit_here`, with `handover_refusal`, go to a new child, `update/resolver/handover.rs`.
  - `act`'s done gate (`:302–:315`) goes to `update/resolver/done.rs` as `refuses(key)`.
  - Target: under 400 lines. If no seam gets there, report the count and say so.

## Read first
- This ticket. `.scratch/m6-conflicts/scout-28.md` (whole). `docs/reviews/m6-amend-25.md` Major 1, Major 3.
- `.scratch/m6-conflicts/issues/25-stable-file-order-and-done-file-open.md`: Decisions D2, and NOTES "Spec contradiction".
- `crates/graft-ops/MAP.md`:
  - §operation `RestartParams` and `shape_of_stages` (:10–:11), `Refusal` m6 additions (:47);
  - §apply `StageSource`/`stage_source`, `prepare_restart` (:155–:156);
  - §conflict `operation_conflicts` (:68), the read-only journal pattern;
  - §journal `parse`, and §journal (file handle) `journal_path`.
- `crates/graft/MAP.md`: §`update::resolver`; §`jobs` `conflict`; §`update::confirm` `RestartConflict`.
- `crates/graft-ui/MAP.md`: `HintContext::Resolver`, `every_context`.
- `docs/design/02-keymap.md` :111–:114; §2.6 `d` (:266), `e` (:271), `M` (:272).
- `ARCHITECTURE.md` before the split.

## Scope
In:
1. graft-ops:
   - `restart_source` (crate-private core), `restart_gate` and `RestartReadError`, per R1/R2.
   - `stage_source` wraps the core. `prepare_restart` calls the core.
   - `check_restart` becomes `pub(crate)`.
   - MAP §apply.
2. graft job:
   - On the done route: facts, then `restart_gate`, then `StartOver`.
   - `StartOver` goes into `Opening::Done` on success, and into `OpenFailure.done` on failure (was `bool`, `:89`).
   - `failed()` (`:225`) sets `done: None`.
3. graft update:
   - `ResolverState.done: Option<DoneOpen>`. Readers are in scout §"graft update / view".
   - `start_over_refusal`, and `restart_box` uses it.
   - `done_refusal`'s words per R5. Fix its doc at `done.rs:38`.
   - `conflict_opened`/`open_failed` fill `start_over`.
4. graft view: the identity row, the done card's `keys`, and `resolver_context.restart`, from `start_over_refusal`.
5. graft-ui:
   - `resolver_done_list` (`lists.rs:300`) honours `restart`: `restart: false` gives exactly `e edit`, `esc close`.
   - A second done form goes in `catalogue.rs:141`, with its own slot in `coverage_slot` (`tests.rs:1968`).
6. R6: `e`/`M` refused while a take is in flight or queued. Write the failing test first.
7. R7's split. MAP.md for graft-ops §apply and graft §`update::resolver`/§`jobs`.

- **Conductor, from `docs/reviews/m6-amend-15.md` (2026-09-24):**
  - **M1:** `Esc` during a take queue drops queued takes silently (`update/resolver.rs:317`, `:412-426`; `t t t Esc` sends 1, drops 2, status `Ready`, no words). Same rule as `e`/`M`: refused in words while a take is in flight or queued. Failing test first.
  - **M2:** on a `Changed` re-read, the dropped-takes count vanishes when the re-read lands (`answers.rs:174-190`, `:102`, `:117-119`: problem notices filtered on landing). The count must survive the re-read. Failing test first (`reread: true`).
  - **M3:** sizes: `update/resolver/answers.rs` 412, `update/resolver.rs` 442 (already in scope), `update/resolver/tests/advance.rs` 430 — split where a seam exists.
Out:
- Making `d` **work** on whole-file shapes, or in shell-begun merges. See the note amendment's P10 and gap row M6/25.
- Review 25 Minors 2–5, and `e` on a removed card.

## Signatures
```rust
// graft-ops/src/apply/restart.rs (restart_gate and RestartReadError re-exported from `apply`)
/// Can `d` start this path over? Read-only: no journal line, no ref moved, no file written, no Journal::open.
/// Outer Err = could not tell. Inner Err = ops' refusal, the same value the act would return.
pub fn restart_gate(repo: &RepoHandle, facts: &RepoFacts, path: &Path)
    -> Result<Result<StageSource, Refusal>, RestartReadError>;
pub enum RestartReadError { Facts(FactsError), Journal(JournalError), Io(io::Error) }
impl RestartReadError { pub fn explain(&self) -> String; }

/// The not-unmerged half: stage_source, then the shape check. Shared by prepare_restart and restart_gate.
pub(crate) fn restart_source(repo: &RepoHandle, entries: &[JournalEntry], facts: &RepoFacts, path: &Path, stop: /* stage_source's Stop type */)
    -> Result<Result<StageSource, Refusal>, FactsError>;
// stage_source(ctx, facts, path) keeps its signature and delegates to the lookup inside restart_source over ctx.journal.entries().
// check_restart (operation/conflict.rs:238): pub(super) → pub(crate), unchanged otherwise.

// graft/src/jobs/conflict.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartOver { Can, Cannot { why: String } }   // why raw; sanitised where drawn
pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>, StartOver) }
pub struct OpenFailure { /* words, not_unmerged */ pub done: Option<StartOver> }   // was `done: bool`; Some = the done route

// graft/src/update/resolver/done.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoneOpen { pub shape: DoneShape, pub start_over: StartOver }
// ResolverState: `pub done: Option<DoneOpen>` (was Option<DoneShape>)
pub fn done_refusal(state: &AppState) -> Option<String>;   // R5 words
pub(super) fn refuses(key: &KeyPress) -> bool;             // act's done gate, moved (R7)

// graft/src/update/resolver/restart.rs
/// None ⇔ `d` opens the box. Structural only: write_refusal stays ahead of it in restart_box.
pub(crate) fn start_over_refusal(state: &AppState) -> Option<String>;

// graft/src/update/resolver/handover.rs (new; mergetool_here, edit_here moved here)
pub(super) fn handover_refusal(state: &AppState) -> Option<String>;   // write_refusal, then in_flight/queued (R6)
```

## Acceptance (name the verifying test when you tick)
- [x] Review-15 M1: `Esc` refused while a take is in flight or queued; nothing dropped — test: `update::resolver::tests::advance::esc_waits_for_the_takes_in_flight` (red first).
- [x] Review-15 M2: dropped-takes count survives a `Changed` re-read — test: `update::resolver::tests::advance::a_changed_reread_keeps_the_dropped_count` (red first).
- [x] Review-15 M3: answers.rs, advance.rs ≤ ~400 or no-seam noted — answers.rs 412 → 333, advance.rs 430 → 394 (counts in NOTES); verified by `wc -l`, no test.
- [x] `e`/`M` refused while a take is in flight or queued, and nothing dropped. Tests `update::resolver::tests::advance::e_waits_for_the_takes_in_flight` and `…::m_waits_for_the_takes_in_flight`: `t t` with no answer, then `e` / `M` → status `a take is still landing · …`, no `OpenConflictEditor`/`OpenMergetool` command, `in_flight` and `queued` unchanged. After the takes land, `e` hands over. Ticked by those two tests (red first); each asserts **nothing** is sent (stronger than no editor/mergetool command).
- [x] graft-ops `it::restart_gate::*`, real git, one test per row (the five names below, as written; `gate()` asserts the journal bytes, `refs/graft/backup/*` and `git status` unchanged). Each asserts the value, and that the journal file's bytes and every `refs/graft/backup/*` are unchanged:
  - `an_unmerged_marker_file_is_index` → `Ok(Ok(Index))`.
  - `an_unmerged_whole_file_is_not_marker_shape` → `Ok(Err(NotMarkerShape))`.
  - `a_marker_file_done_in_an_app_merge_is_backup` → `Ok(Ok(Backup { .. }))`.
  - `a_whole_file_done_in_an_app_merge_is_not_marker_shape`: `gone.txt` kept with `o`.
  - `a_done_file_in_a_shell_merge_is_no_stages_kept`.
- [x] graft-ops `it::restart_gate::a_torn_journal_tail_is_read_not_cut`: a torn last line gives an answer, and the file's bytes are unchanged afterwards (no `Journal::open`).
- [x] graft-ops `it::restart_gate::the_gate_and_the_restart_agree`: for each path above, `restart_gate` gives `Ok(Err(r))` ⇔ `apply(RestartConflict)` answers `Refused(r)`, with the same `r`. An `Ok(Ok(_))` path restarts to `UU`/`AA` (asserted as ≥ 2 stage lines in `ls-files -u` plus `<<<<<<<` back in the file).
- [x] *(conductor, 2026-09-24: this job test was removed by ticket 30 with its fixture copy; the claims are now checked end to end by `it::resolver::file_list::a_done_marker_file_starts_over_in_place` (Can) and `it::resolver::file_list::start_over::a_whole_file_done_file_offers_no_start_over` (Cannot, `d` answers in words). The `why` containing `no markers` is pinned by the `fix-done-start-over-cannot` capture's status row via `done_start_over_capture`.)* Was: graft `jobs::conflict::tests::a_done_open_carries_the_gate`, real git, app-begun merge: a done marker file → `Done(_, Can)`, and `gone.txt` → `Done(_, Cannot { why })` with `why` containing `no markers`.
- [x] graft `update::resolver::tests::file_list::start_over::a_done_file_that_cannot_start_over_hides_d` (child module `start_over`, not `file_list` itself — file_list.rs was already long):
  - Identity row `▸ gone.txt · done ✓` with no `d`.
  - Hint table exactly `e edit`, `esc close`.
  - `d` → status `can't start over · …`, `confirm` is `None`, no command pushed.
  - `t` → `gone.txt is done`.
- [x] graft `…::file_list::start_over::a_done_file_that_can_start_over_offers_d`: 25's words and bar, unchanged.
- [x] graft `…::file_list::start_over::an_unreadable_done_file_keeps_its_gate`: `OpenFailure { done: Some(Can) }` → the card's keys `d starts it over`. With `Cannot`, the keys are empty.
- [x] graft `…::file_list::start_over::a_gate_that_could_not_be_read_hides_d`: `Cannot { why: "could not check: …" }` → no `d`, and the words on `d` carry the why.
- [x] graft `…::file_list::start_over::d_is_in_the_bar_exactly_when_the_box_opens` (10 fixtures; also asserts 5 offer `d`). Over these fixtures: document, unmerged whole-file card, done text Can/Cannot, removed Can/Cannot, binary, unreadable. For each: the hint table contains `d` ⇔ `d` sets `confirm` to the restart box. **The guard against a third repeat.**
- [x] graft-ui `graph_screen::hints::tests::a_done_file_that_cannot_start_over_offers_edit_and_close`, and `every_context_reaches_every_shape` with the new slot (`COVERAGE_SLOTS` 32).
- [x] graft `it::resolver::file_list`, the existing pins changed (scout item 4) — tests `a_shell_merge_done_file_answers_d_in_words` (renamed from `a_done_file_opens_read_only_and_d_starts_it_over`), `a_done_marker_file_starts_over_in_place` (the can path, kept, helper inlined and counted), `done_open_capture` (3 shots re-taken). Also changed: `it::resolver::last_file`'s pin on shell-merge `added-by-them.txt` → `▸ added-by-them.txt · done ✓` (same cause, not listed by the scout):
  - The `start_over` helper (`:194`) at its shell-merge use (`:204`, `gone.txt`) → no box, the words on `d`, `restarting` never set. Rename the test to say so.
  - Its other use (`:245`) is kept if it is the can-start-over path; otherwise the same change.
  - `done_open_capture` (`:314–:328`): shell-merge `gone.txt` now reads `▸ gone.txt · done ✓` with bar `e edit  esc close`. Re-take `docs/shots/m6/fix-file-list-done-open-{120x40,200x50,187x50}.txt`.
- [x] graft `it::resolver::file_list::start_over::a_whole_file_done_file_offers_no_start_over`, real git (`app_merge_with_a_whole_file_conflict`, `o` on `gone.txt`, click its row):
  - No `d` on the identity row or the bar.
  - `d` → the words, no box.
  - `git status` and the journal are unchanged.
- [x] graft `it::resolver::file_list::a_done_marker_file_starts_over_in_place` asserts the journey count. The inputs from the landing after `⏎` are click row · `d` · `→` · `⏎` = **4**, the note's `walk-back-a-done-file`. After them: `UU`, the same cell, and the cursor on block 0.
- [x] R7: `update/resolver.rs` line count in NOTES (442 → 385, `wc -l`), 442 → ≤ 400 (or the reason). Gates green, and the existing `update::resolver::tests::*` pass unchanged apart from the renamed pins — gates green (nextest 2869/2869, 1 skipped). **Not strictly unchanged**: fixtures moved to `DoneOpen`/`StartOver::Can` and `dropped: 0`; the walk tests moved verbatim from `advance.rs` to `tests/cross.rs`; one literal in `update/tests.rs` became `ResolverState::opening` (clippy `too_many_lines`). No assertion changed.
- [x] Captures, pinned by `it::resolver::file_list::start_over::done_start_over_capture` (files written and matching; **the look is not ticked** — the user's): `docs/shots/m6/fix-done-start-over-{can,cannot}-120x40.txt` and `…-cannot-200x50.txt`. **The look is the user's call (rule 6); colour is unverified (rule 5).** What right looks like:
  - can, 120x40: `▸ <marker file> · done ✓ · d starts it over`, and bar `d start over  e edit  esc close`.
  - cannot, 120x40: `▸ gone.txt · done ✓` with nothing after it, bar `e edit  esc close`, and after `d` the status row `can't start over · gone.txt: deleted by them — no markers to take from`.
  - cannot, 200x50: the same identity row beside the list column, one card (25's ruling 4).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium to large. One context, as follows:
- one ops core extracted, plus a read-only public gate;
- one job payload;
- one app predicate across three readers, and one hand-over refusal;
- one hint form and one split;
- ~18 tests and 6 captures.

If it will not fit, split at the crate seam: graft-ops first (the gate, the core and their four boxes).

## For the conductor (outside this ticket)
- **Note amendment, below: apply it to `docs/design-notes/m6-conflicts.md`** (the planner had Write only, no Edit). It fixes review 25 Major 2. If it is already applied, replace its "Interfaces (amendment 3)" line and "Hard-to-reverse" choice 1 with the text below.
- Ticket 21: walk the two new `journey-counts` rows. `open-a-done-file` asserts 1 input. `walk-back-a-done-file` asserts 4 inputs, and uses `k` in place of the click once 17 (P7) has landed. **Done by ticket 21:** `every_scenario::journey_open_a_done_file` (route `k`, 1) and `journey_walk_back_a_done_file` (route `j d → ⏎`, 4 — `j`, since `notes.txt` is below the file it lands on).
- Gap row M6/25: once 28 lands, change "the refusal comes after the box" to "no box: `d` is not offered and answers in words". The row stays **broken** (the capability is still missing) until P10, or until 20 gives shell-begun merges a backup.
- The m6-05 Major 1 row, if one is open: the root cause was the app mirroring ops' rule. R1/R4 remove the mirror.

---

## Note amendment — exact text

### A. Append to `docs/design-notes/m6-conflicts.md`, at the end

````markdown
---

## Amendment 3 — 2026-09-24 — a done file: read it, start it over (review m6-amend-25 M1, M2)

Read: `docs/reviews/m6-amend-25.md`; `docs/research/conflict-resolution.md` (**CR**) 1.10, 1.11, §3 :287; `docs/research/ux-principles.md` (**UX**) 1.3, 2, 3.1, 3.5, §5; 02-keymap :111, §2.6 `d`; ticket 25 (spec + NOTES); `scout-28.md`; graft-ops, graft MAPs. No source. Ticket: `28-done-file-start-over-only-where-it-can`.

### Need
- **Who / task:** he pressed `⏎` on a file and then saw he took the wrong side. Or he wants to look again at what he chose before `c`.
- **What hurts:** 25 built it, but `d` is offered on every done file. ops refuses after the confirm on a whole-file shape, and on **any** done file of a merge begun in the shell (J3's start). No field note yet: this comes from review 25, not from his use.

### Prior art (CR · T take · R reject · I invent)
| Aspect | Best answer | Ours before | Rec · why |
|---|---|---|---|
| See a done file | Fork: an open request to show how each conflict was resolved in the file list (CR 1.10, #1304, **(?)** title only); JetBrains: a resolved file leaves the list (CR 1.1) | a done row keeps its place and opens read-only (25) | **T** the Fork ask, as built. H1: what he chose stays one click away |
| Walk back a `⏎` | Tower: "reset a merge conflict to its unmerged state, even after it has been resolved" (CR 1.11 :207; CR §3 :287 recommends **T**) | `d` on every done file; ops refuses two shapes after the confirm | **T** Tower's reset, **only where ops can do it**. **I**: ops' own rules decide the offer when the file opens (UX H5: block the impossible, don't warn after; keymap :111) |
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
- `e`/`M` while a take is still landing → refused in words, the takes kept (ticket 15 ruling).

### Interfaces (amendment 3)
Ticket 28's Signatures:
- `graft_ops::apply::restart_gate(repo, facts, path) -> Result<Result<StageSource, Refusal>, RestartReadError>`. It reads the journal through `journal::parse`, never `Journal::open`.
- The crate-private `restart_source`, shared with `prepare_restart`.
- `jobs::conflict::StartOver`, `Opening::Done(Arc<DoneFile>, StartOver)`, `OpenFailure.done: Option<StartOver>`, `DoneOpen`, `start_over_refusal`, `handover_refusal`.

### Hard-to-reverse choices (amendment 3)
1. **Who decides "can `d` start this over"?** Options: the app mirrors ops' rules, or ops' own rules, asked by both the offer and the act. **Ops' rules.** The act reaches them through `check_restart` (in `Operation::check`) and `restart_source` (in `prepare_restart`); the offer reaches the same two through `restart_gate`. The mirror has drifted twice (m6-05 M1, m6-amend-25 M1).
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
None. Ticket 28 goes on Opus: every rule is fixed here, and `d_is_in_the_bar_exactly_when_the_box_opens` plus `the_gate_and_the_restart_agree` judge it.
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

## NOTES (builder, 2026-09-24)

- **Scout used:** `.scratch/m6-conflicts/scout-28.md`; cited lines re-checked against `f585af1`.
- **Compacted once mid-ticket.** Nothing remains; everything after the compaction (split, clippy, gates, MAP, ticks) was done in the second context.
- **Cause, not symptom.** The app no longer mirrors ops' rule: `graft_ops::apply::restart_gate` runs `check_restart` then `restart_source`, and `prepare_restart` matches on the same `restart_source`. The job asks it before the read; `start_over_refusal` is the one app predicate behind the box, the bar, the identity row, the card keys and `done_refusal`. Cost to the user: one extra git read (facts + journal parse) per done-file open.
- **Deviations (rule on these):**
  1. `RestartReadError` has a fourth variant, `Run(RunError)`: `journal_path` returns `RunError`, so the ticket's three variants could not carry it without swallowing.
  2. `handover_refusal` asks the takes **before** `write_refusal`, not after as the signature comment says: a take in flight *is* `ResolverState::job`, so `write_refusal` would answer `still writing the last take` and the `a take is still landing` words could never show. Returns `Option<&'static str>`, not `String`.
  3. The unit tests live in `update::resolver::tests::file_list::start_over`, and the new it tests in `it::resolver::file_list::start_over` (child modules), not in `file_list` itself.
  4. `it::resolver::last_file` had a pin on a shell-merge done file offering `d`; changed the same way.
  5. graft-ui's existing done-bar test now builds `restart: true` explicitly.
- **Splits (`wc -l`):** `update/resolver.rs` 442 → 385 (`e`/`M` to new `handover.rs`; `⏎`'s `mark_resolved` + `resolved_advance` + `every_file_done` to new `resolved.rs`, 149). `answers.rs` 412 → 333. `tests/advance.rs` 430 → 394 (walk tests to new `tests/cross.rs`, 164).
- **Verified:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run` (2869 passed, 1 skipped), `cargo test --doc`, all workspace-wide. Red-first seen for the 4 advance tests and 4 of 5 `file_list::start_over` unit tests (the fifth pins behaviour 25 already had).
- **Gaps — unverified:** colour of every changed row (rule 5); the look of the 6 captures is the user's call (rule 6).
- **Gaps — broken (unchanged by this ticket, now refused honestly):** `d` still cannot start over a whole-file done file or any done file of a shell-begun merge (ops refuses: `NotMarkerShape` / `NoStagesKept`); the user now sees that before the box instead of after it.
- **Not done here (conductor's, outside the crate set):** the design-note and keymap edits under "For the conductor" and "Note amendment".

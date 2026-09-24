# Review — m6-conflicts/31: finish from the resolver

Fixed hash `78465a03`; diff `ba7d76fb..78465a03`, 29 files, +1186 / −292. Read-only.
Reviewer did **not** run gates, the heavy tier or the release perf run: the conductor runs them for this amend
(brief). Nothing below claims a test result I did not see. Every behavioural finding is from reading, with its line;
none was observed at runtime.

**Counts: Critical 0 · Major 2 · Minor 7.**

## Tick spot-check

All 14 ticked boxes were checked against the test bodies at `78465a03`. **14 held.** Two caveats. Box 2's "status line
not written" is asserted weakly (Minor 2), but `file_list::after_the_last_file_the_notice_says_c_finishes` asserts it
exactly (`Status::Ready`). Box 8's "failed first" can't be checked from the diff; I took it on the builder's word.
The journey box (`a-finish` = 2) is honestly left unticked (ticket 21's walk does not exist).

## Conductor's questions

- **S5 premises recorded against real git:** yes. `it::resolver::last_file::{stage_landed, done_read_landed}`
  (`tests/it/resolver/last_file.rs`) asserts all three. (1) Between the stage and the read, the screen is blank below
  row 0 (`frame[1..]` empty; see Minor 5). (2) The read answers `Opening::Done`. (3) The notice survives and settles.
  The test waits for both reads and sees only the end state. It does not control which read lands first. Both orders
  reach the same state by reading (`landing_notice` keeps a non-problem notice; `settle` rewrites a `CHECKING` one).
- **`settle`/`abandon` stranding `checking the files`:** they themselves are right (`status_read.rs:62-69`, `:82-87`).
  **But two read-ended paths call neither** → Major 1.
- **`c` vs keymap §2.6 :269–:275:** matches. `c` finishes a merge through the summary, else continues. It is refused
  while a file is open (`continue_refusal`), and the resolver stays after the last file. `Enter` stays with
  `every file done ✓ · c finishes …`, and a stash pop closes (`resolved.rs:117-132`). `Esc` is untouched. One extra
  refusal is not in :269's row: take/write landing (Minor 4).
- **`resume_answered` fix (`finish/answers.rs:143-146`):** right for `Merged`, and tested by
  `finish_here::a_merge_committed_from_the_resolver_lands_on_the_graph` (resolver `None`, finish `None`, status
  `merged feature into main`). It is incomplete: the `Closed` answer has the same hole (Minor 1).
- **Deviation (`checking the files` first):** it is not a deviation from Scope. Scope 3.4 prescribes exactly
  `(CHECKING, false)` while a read is owed, and the stage always asks for one. The acceptance line's "notice
  `every file done ✓ …`" contradicted Scope 3.4. The builder followed Scope, recorded it, and tests both moments
  (`finish_here::the_resolver_stays_after_the_last_file`). Accept.
- **Is `c` discoverable while the bottom row is ` stage b.txt ✓`?** Yes, in words. In the same frame the notice row
  under the title reads `every file done ✓ · c finishes the merge` (capture `docs/shots/m6/fix-finish-from-the-resolver-120x40.txt`,
  frame 1, line 5). Only the bar's `c finish` waits for the next key. How it looks is the user's call.
- **Swallowed errors:** none new. `let _opened: bool` (`banner.rs:433`) is pre-existing, but it now has a second
  caller and still has no comment (Minor 7). There is no `unwrap`/`expect` outside tests in the diff.
- **File sizes:** see Major 2 and Minor 6.

## Critical

None.

## Major

### M1. A failed status read leaves the resolver's notice saying `checking the files` indefinitely

- **Where:** `crates/graft/src/update/mod.rs:671` (`Product::WorkingTree(Err(_))`) and `mod.rs:940` (`job_failed`).
  Both call `status_read::answered` and discard the answer. Only `job_cancelled` (`mod.rs:903-904`) calls `abandon`.
  The `CHECKING` notice is set at `update/resolver/resolved.rs:150` and `update/banner.rs:416`.
- **What:** before 31, `CHECKING` only ever lived on the status line, and both paths overwrite that line with
  `Status::Failed`. So the comments "the failure below replaces the line either way" (`mod.rs:669-670`, `:938-939`)
  were true. Now `CHECKING` also lives on the resolver's notice row, which neither path touches. After a failed or
  panicked `ReadWorkingTree`, the status line shows the failure while the notice row still says
  `checking the files`, and nothing is still checking. The next `c` then counts the stale tree, which still lists the
  staged file as unmerged. It answers `1 file still conflicted · ⏎ marks done` on a done file, where `⏎` is refused.
- **Why it matters:** this is the stranding the brief asked about. A status field claims work in progress that has
  ended, next to a failure that contradicts it. No test covers it: `words_and_c.rs` exercises only `JobCancelled`
  (`an_abandoned_read_clears_the_notice`).
- **Fix (cause, not symptom):** in both paths, run `status_read::abandon` (or a notice-only half of it) when
  `answered` is `true`, before the `Status::Failed` write. Keep the failure on the line. Correct the two comments. Add
  tests for `WorkingTree(Err)` and `JobFailed`: after the last stage, the notice is `None` and the status is the
  failure. The ticket's Scope 5 named only `settle`/`abandon`, so this is a spec gap and not the builder's fault.

### M2. `resolved_advance` is 91 lines, and `update/resolver.rs` is 435 with no size row (PROMPT.md 6.2)

- **Where:** `crates/graft/src/update/resolver/resolved.rs:61-151` (was 74 lines at `ba7d76fb`, `:59-132`).
  `crates/graft/src/update/resolver.rs` went 412 → 435.
- **What:** the function now holds three landings: the next file, the stash-pop close, and the stay-after-last. Each
  has its own comment block separating it, which is 6.2's own trigger for a split. `resolver.rs` was already over
  ~400, and no `KNOWN_GAPS.md` row names it (rows 642/647 cover `rows.rs`/`banner.rs` only).
- **Why it matters:** readability is a hard rule. This function is where the next landing change will go.
- **Fix:** extract `close_after_last(state)` (stash pop) and `stay_after_last(state, commands, done, from, next_id)`
  (P8). That leaves `resolved_advance` as guard, pick next, dispatch. Move `finish_pressed` into `resolved.rs` or a
  `finish_here.rs` child to bring `resolver.rs` back under 400, or file a size row.

## Minor

1. **A `Closed` answer from a summary opened in the resolver leaves the resolver open under it.**
   `finish/answers.rs:149-150` (`close_keeping_draft`; `state.resolver` untouched). There are two routes:
   - `Operated::Incomplete` (`banner.rs:643-650`): the merge commit **ran**, and the resolver stays open on a done file
     of an operation that has ended.
   - `Operated::Conflicted` with paths (`banner.rs:655-667`): `land_in_resolver` → `open_resolver`, which refuses while
     one is open (MAP §`update::resolver`), so the new stop gets the stale resolver. The builder flagged this half as
     an unverified candidate.

   The `FinishAnswer::Closed` doc (`finish.rs:126-127`, "the banner takes it from here") is true only if the resolver
   is gone. Fix at the cause: every non-`Kept` answer closes the resolver (move the `state.resolver = None` above the
   `Done` check in `resume_answered`), with one test per route. Found by reading; not observed. Merge-only this
   milestone, and both routes are rare.
2. **Weak assertion.** `finish_here::the_resolver_stays_after_the_last_file` (`finish_here.rs:79-84`) proves "the
   status line is not written" only as `status != Busy(CHECKING)`. A write of `every file done` would pass it.
   Fix: `assert_eq!(landed.status, <status before the landing>)`.
3. **Impossible branch instead of a type.** `resolver_context_list` (`graft-ui/…/hints/lists.rs:83-95`) re-matches
   `HintContext::Resolver` and returns `Vec::new()` for a case its caller rules out. Fix: destructure in
   `context_list`'s arm and pass the fields (same shape as KNOWN_GAPS row 574). `offering_c` (`lists.rs:116`) inserts
   at `len - 1` on the unasserted premise that "every resolver table ends with `esc close`".
   `a_finishable_done_file_offers_c` pins it for the three tables, which is enough; name the premise in the test.
4. **Words for `c`'s refusals: incomplete in the docs, and split across two rows.**
   - Between the stage and the done read, `c` answers `still reading the file` (`resolver.rs:267`, via
     `handover_refusal`). The design note's new line (:475) names only `a take is still landing …` /
     `still writing the last take`, and keymap :269 names neither.
   - `finish_pressed` puts the handover refusals on the **status line** (`resolver.rs:386`), while `continue_pressed`
     puts its own on the **notice row** (`banner.rs:416`, `:424`). One key now answers on two rows depending on the
     reason.

   Spec-legal ("in their own words", no row named). Question for the conductor: should all of `c`'s answers in the
   resolver go on the notice row?
5. **The frame is blank while the last file's reopen read is out.** Premise 1: `resolver_view` answers `None` while
   an opening read is out (`view/resolver.rs:40-51`), and `stage_landed` asserts that everything below row 0 is empty,
   bar included. This mechanism existed before for next-file reopens. Since 31, every finish passes through it too.
   It's brief, and how it looks is the user's call. If it shows, keep drawing the previous document until the read
   lands.
6. **Stale size rows.** `update/banner.rs` is 739 (row 647 says 640), and `view/rows.rs` is 706 (row 642 says 683).
   Both grew here (+25, +4). Update the rows.
7. **`let _opened: bool`** (`banner.rs:433`) ignores `open_finish`'s `false` ("one already open") with no comment
   saying why that's safe. It is safe, because the summary is modal and `c` can't reach here while it is open. Rule 3
   asks for that comment. `open_finish`'s doc still says "(or, later, the resolver)" (`finish.rs:131`): later is now.

## Checked and fine

- `every_file_done_words`'s `c_live` widening (`resolved.rs:157-158`). The stash-pop path still gets no `c`, and
  `after_the_last_stash_file_the_status_says_every_file_done` is unchanged.
- The order in `continue_pressed` is unchanged. From the resolver, the rebase path closes the resolver and focuses the
  banner **before** `resume` (`banner.rs:436-443`).
- `finishable` gating (`view/rows.rs:507-510`), `every_context` forms (`catalogue.rs:145-180`, stash pop excluded),
  and the `BOUND_ACTIONS` row (`keys/bound.rs:112`).
- MAP.md: both crates updated for every item in Scope 10.
- Feature pipeline: design note Amendment 2 has Prior art (:427) and Journeys (:439; J6 = 2, `a-finish` 2 at :493).
  The walk that counts it is ticket 21 and does not exist yet, and the ticket says so.
- Architecture: no new crate edges. The graft-ui change stays in `graph_screen::hints`.

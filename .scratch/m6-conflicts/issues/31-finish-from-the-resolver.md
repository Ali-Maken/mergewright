# fix-finish-from-the-resolver: `c` in the resolver finishes; the resolver stays open after the last file, with the words on its notice row

Status: done 2026-09-24 (journey box left to ticket 21; see NOTES). Was: ready — respecified 2026-09-24 (spec-only pass) against `scout-31.md` (main `293f2b0d`), which found 7 contradictions with the first spec; each is resolved below and marked **(S-n)** for scout item n. Was split out of `19-finish-summary-screen.md` Part B, ruling S8. **P8 is approved and already in the keymap** (`02-keymap.md:269` `c`, `:270` `Enter` — scout-31 "Keymap"), so this isn't gated and there is no key to propose.
Blocked by: 34 + 35 (`open_finish`, `ReturnTo`, merged), 13 (R4/R5 and their landing tests), 17, **36** (same crate `graft`, ADR-0036: not concurrent; 36 may split `update/finish.rs` and `update/resolver/answers.rs` — every place below is named by **function**, re-find it by name).
Build with: **Opus** — brownfield, scouted; the rules are fixed below, but three premises the scout could not confirm (S-5, the resolver closing on resume/commit, the notice surviving the done read) must be observed and a wrong one stopped on, not patched round.
Needs: 34/35 (`open_finish(.., ReturnTo, ..)`, `ReturnTo::Resolver`, `finish_key`'s `escape`); 13 (`resolved_advance`, `every_file_done`); banner `continue_pressed`, `continue_refusal`; `status_read::{owed, settle, abandon, CHECKING}`; resolver `reopen`, `done_refusal`, `takes_refusal`, `write_refusal`, `landing_notice`.
Crates: graft-ui (`graph_screen::hints` only: `context.rs`, `lists.rs`, `catalogue.rs`), graft.

## Read first
- `.scratch/m6-conflicts/scout-31.md` — all of it (1 page).
- `docs/design-notes/m6-conflicts.md` Amendment 2: Journeys J4 and J6 (:447-460), "Where each event lands him" (:467-476), P8 (:636-638).
- `docs/design/02-keymap.md` §2.6 :269-:275; §2.6a :284-:289 (where the resolver's `Esc` lands); §2.6b :323.
- `crates/graft/MAP.md` §`update::resolver` (`resolved_advance`, `every_file_done`, `ResolverState`, `listed_files` … `done_refusal`, `handover`), §`update::banner` (`continue_refusal`, `continue_refused`), §`update::finish` (`ReturnTo`, `open_finish`, `resume_answered`), §`update::status_read`.
- `crates/graft-ui/MAP.md` §`graph_screen::hints` (the `Resolver { .. }` context and its three tables).

## Scope
**In.**

1. **One `c` for both places (S-1).** `banner::continue_pressed` becomes `pub(crate)`, gains `from: ReturnTo`, and **keeps returning `()`**. The banner's `banner_key` passes `ReturnTo::Banner`, behaviour unchanged. The checks and their order are unchanged:
   1. `status_read::owed` → `CHECKING`.
   2. `continue_refusal`.
   3. A merge → `open_finish(.., from, ..)`.
   4. Otherwise → `resume(Continue)`.

   With `from == Resolver`:
   - step 1 writes the resolver's notice `(CHECKING, false)`, not the status line;
   - step 2 writes the notice `finish_refused_notice(reason)` = `<reason> · ⏎ marks done`, not a problem. Example: `1 file still conflicted · ⏎ marks done` (design note :474). **(S-4)** The banner keeps `continue_refused`'s `… space marks resolved`: `Space` is how you mark resolved outside the resolver (§2.6a :302), `⏎` is how you do it inside (§2.6 :270). They're two places with two keys, so the words differ on purpose.
   - step 4: the resolver closes and `banner_focus` is set **before** the `Resume::Continue` is pushed. From then on the state is exactly what the banner's `c` leaves, so the resume's answers land as they do today. Why: `open_resolver` refuses while the resolver is open (MAP §`update::resolver`), so a next stop could not open it fresh (P5, :475-476) if it stayed.
2. **Resolver key `c`.** In `resolver::act`, ahead of the `_ if plain => scroll_key` fallthrough:
   1. `takes_refusal`, then `write_refusal` (the `handover` order), in their own words;
   2. then `continue_pressed(.., ReturnTo::Resolver, ..)`.

   `c` is **not** added to `done::refuses`: it has to act on a done file, because after the last file that's where he is. Below `fits`, only `Esc` acts, as today.
3. **The resolver stays after the last file (S-5).** `resolved_advance`'s none-left branch:
   - **`banner_view(state).is_none()`** (stash pop; no banner): keep 13's close unchanged. The graph gets status `every file done ✓`.
   - **Otherwise:**
     1. Don't set `resolver = None` or `banner_focus`.
     2. **Reopen the path just staged** through the same private `reopen` the next-file branch uses. Its read then answers `Opening::Done` and the done card settles read-only (ticket 25), so `done_refusal` gates `o t b ⏎ B M n N O T ^O ^T`.
     3. Re-read the set as the next-file branch does.
     4. **Then** set the notice: `(CHECKING, false)` if `status_read::owed`, else `(every_file_done_words(state), false)`.
     5. Don't write the status line.
   - Remove the comment "Ticket 19 Part B keeps the resolver open instead" from `resolved_advance`; this is that change.
   - **Unconfirmed premises: observe and report, don't reason forward.** The scout couldn't confirm what this branch draws. The builder records all three in the ticket NOTES:
     - what the document shows between the stage landing and the done read landing;
     - whether the done read answers `Opening::Done` or `<path> resolved outside`;
     - whether the notice survives that landing (`landing_notice`).

     If the read answers `resolved outside` or closes the resolver, or if the notice is dropped: **stop, report `blocked: <what>` with the function and line**, and don't add a special case.
4. **`every_file_done` (S-2).** Split out the words:
   - `every_file_done_words(&AppState) -> String` holds the words. The `c …` clause shows whenever `c` is live: `banner_view(state).is_some() && (state.banner_focus || state.resolver.is_some())`. That's the old `banner_focus` rule, widened to the second place `c` now works.
   - `every_file_done` keeps its signature and returns today's `Status` variant around those words, so existing callers are unchanged.
5. **`settle` and `abandon` see the notice row (S-3).** Both keep today's status-line path. Each adds one more case, for when the resolver is open and its notice is `(CHECKING, false)`:
   - `settle` writes the verdict **to the notice**: `continue_refusal(state)` is `Some(r)` → `finish_refused_notice(r)`; `None` → `(every_file_done_words(state), false)`.
   - `abandon` sets the notice to `None`.

   A notice cleared meanwhile (scroll, take, …) isn't resurrected; the next `c` asks again. Same as the status line today.
6. **`Esc` from a summary opened here.** Back to the resolver on the same path, draft kept. The scout reads `escape` as already doing this (it doesn't touch `state.resolver`; the key order is finish → resolver). Pin it with a test, and don't change it unless the test fails.
7. **Commit from a summary opened here.** Wherever `FinishAnswer::Merged` is handled, `state.resolver` must be `None` afterwards, landing as J6 (:459-460): the graph, cursor on the merge commit. The scout didn't cover this. Write the test first. If it fails, close the resolver in that handler (name the function in NOTES). A `Kept` answer leaves the summary and the resolver under it as they are.
8. **Hints (S-7).**
   - `HintContext::Resolver` gains `finishable: bool`. Set it in `view::rows::resolver_context` as `banner_view(state).is_some() && continue_refusal(state).is_none()`.
   - When it's true, **every** Resolver table (open file, whole-file card, done file) gets `c <verb>` just before `esc close`. The verb is `finish` when `roles == SideRoles::Merge`, else `continue`.
   - No new `short_form` row: `finish` has none and keeps its whole word; `continue` already has `cont`.
   - In practice `finishable` is only true on the done table. It's gated rather than drawn always because the bar never names a key that would refuse (the `Banner { conflicted: false }` precedent).
   - Extend `every_context`, `coverage_slot` and `marked_form`/`pane_focused_form` for the new field.
   - Add the resolver `c` row to `update::keys::bound::BOUND_ACTIONS`.
9. **Tests to rewrite (S-6).** Find each by function name.
   - `resolver/tests/file_list.rs`: rewrite `after_the_last_file_the_banner_says_c_finishes` and `after_the_last_file_of_a_rebase_says_c_continues` (13's name, whatever its exact tail) to the stay-open landing. Rename them to say "notice".
   - `resolver/tests.rs`: rewrite `all_resolved_from_the_pane_offers_c_where_c_continues`. Opened from the Pane, after the last file the resolver is open, its notice carries `c continues …`, and `banner_focus` is unchanged.
   - `after_the_last_stash_file_the_status_says_every_file_done` stays green, **unchanged**.
   - Any existing done-table test asserting the exact `d start over  e edit  esc close` keeps passing with `finishable: false`.
10. MAP.md: graft §`update::resolver`, §`update::banner`, §`update::status_read`, §`update::finish` (if item 7 changes it); graft-ui §hints.

**Out.**
- The summary itself (34/35).
- `c` in the banner, apart from the `from` parameter.
- Any other resolver key.
- Rebase/cherry-pick/revert summaries (M7).

## Signatures
The `next_id` parameter keeps whatever type `continue_pressed` takes today; the scout didn't record it. Don't change it.

```rust
// graft/src/update/banner.rs — was private, returns () as today
pub(crate) fn continue_pressed(state: &mut AppState, commands: &mut Vec<Command>, from: ReturnTo, next_id: /* as today */);

// graft/src/update/resolver/resolved.rs
pub(crate) fn every_file_done_words(state: &AppState) -> String;
pub(crate) fn every_file_done(state: &AppState) -> Status;            // unchanged signature; wraps the words
pub(crate) fn finish_refused_notice(reason: String) -> (String, bool); // ("<reason> · ⏎ marks done", false)
// resolved_advance: signature unchanged; none-left branch per Scope 3

// graft/src/update/status_read.rs — signatures unchanged; each gains the notice-row case (Scope 5)
// settle, abandon

// graft-ui/src/graph_screen/hints/context.rs
HintContext::Resolver { base: bool, restart: bool, whole_file: bool, roles: SideRoles, done: bool, finishable: bool }
```

## Acceptance (name the verifying test when you tick; truth from git)
- [x] `update::resolver::tests::finish_here::c_in_the_resolver_opens_the_summary`: merge, every file done, `c` → `finish` is `Some` with `return_to == Resolver`, `resolver` still `Some`.
- [x] `update::resolver::tests::finish_here::the_resolver_stays_after_the_last_file`: merge, stage of the last file lands → `resolver` is `Some` on the same path, notice `(every file done ✓ · c finishes the merge, false)`, the status line is not written, and `banner_focus` is false. **Deviation, recorded:** the stage always asks for a status read, so at the landing the notice is `(checking the files, false)` (the box below, m6-conflicts/26 order); the test asserts that, then the `every file done ✓ · c finishes the merge` notice once the read lands.
- [x] `update::resolver::tests::finish_here::the_last_file_reopens_read_only`: then the done read answers → `resolver.done` is `Some`, the notice is unchanged, `o` answers `done_refusal`'s words.
- [x] `update::resolver::tests::finish_here::c_with_files_open_is_refused_in_words`: notice `(1 file still conflicted · ⏎ marks done, false)`, no command pushed (asserts no `Resume`/`ReadFinishSummary`/`Operate`; the fixture's ambient `EvilProbe` rides along with any key).
- [x] `update::resolver::tests::finish_here::c_while_a_take_is_landing_is_refused`: `takes_refusal`'s words, no command.
- [x] `update::resolver::tests::finish_here::c_in_a_rebase_resolver_continues`: `Resume::Continue` pushed, `finish` `None`, `resolver` `None`, `banner_focus` true.
- [x] `update::resolver::tests::finish_here::esc_from_the_summary_returns_to_the_resolver`: `resolver` `Some` on the same path, `finish` `None`, `finish_draft` holds the typed subject, the next key reaches `resolver_key` (`o` answers `done_refusal`).
- [x] `update::resolver::tests::finish_here::a_merge_committed_from_the_resolver_lands_on_the_graph`: `Merged` answer → `resolver` `None`, `finish` `None`. Failed first; fixed in `update/finish/answers.rs::resume_answered`.
- [x] `update::resolver::tests::words_and_c::a_status_read_out_shows_checking_on_the_notice_row`: notice `(checking the files, false)`, status line untouched; after `settle` the notice holds the verdict. `update::resolver::tests::words_and_c::an_abandoned_read_clears_the_notice`.
- [x] `update::banner` existing `c` tests green, words unchanged: `update::banner::tests::continue_refuses_in_words_while_anything_is_unmerged`, `…::the_refusal_counts_one_file_in_the_singular`, `…::continue_with_nothing_unmerged_queues_an_unstoppable_resume`; `RESOLVE_KEYS` (`space marks resolved`, `banner.rs:212`) untouched.
- [x] Scope 9's three rewrites: `update::resolver::tests::file_list::after_the_last_file_the_notice_says_c_finishes`, `…::file_list::after_the_last_file_of_a_rebase_the_notice_says_c_continues`, `update::resolver::tests::all_resolved_from_the_pane_offers_c_where_c_continues`; the stash test (`it::resolver::last_file::last_file_closes_to_where_it_opened`, its `last_file_without_one` half) green and unchanged.
- [x] `hints::tests::every_context_reaches_every_shape`, `every_bound_action_is_hinted_live_or_is_named_here`, `every_key_the_bar_draws_is_answered` green (full nextest run, 2963/2963); new `graph_screen::hints::tests::a_finishable_done_file_offers_c` asserts `d start over  e edit  c finish  esc close` (merge) and `c continue` (rebase).
- [x] **Observation, not pass/fail:** Scope 3's three premises recorded in NOTES, each shown by `it::resolver::last_file::last_file_closes_to_where_it_opened` (real git) and its capture.
- [x] Journey `a-finish` = 2 from `every file done` in the resolver (`c ⏎`), counted by ticket 21's walk. Ticked by ticket 21: `every_scenario::journey_a_finish` asserts the `every file done ✓ · c finishes the merge` notice and an open resolver, then counts `c ⏎`. Was **not ticked:** ticket 21's walk does not exist yet (no `a-finish` in any crate). The two steps exist: `c` → summary (`finish_here::c_in_the_resolver_opens_the_summary`), `⏎` commits (ticket 19's summary).
- [x] Capture produced by `it::resolver::last_file::last_file_closes_to_where_it_opened` (look is the user's call; see NOTES for the bar-row question): `docs/shots/m6/fix-finish-from-the-resolver-120x40.txt`: a two-file merge with both files done. **What right looks like:**
  - the resolver is still drawn, the list shows both rows `✓`;
  - the title is the second file;
  - the notice row under the title reads `every file done ✓ · c finishes the merge`;
  - the bar reads `… e edit  c finish  esc close`.

  The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one branch rewritten, one key, one parameter, two status-read cases, one hint field, ~14 tests, one capture. One context.

## For the conductor (exact text)
- `02-keymap.md` :269/:270 already carry P8 (scout-31). **No keymap edit.**
- `docs/design-notes/m6-conflicts.md` :475-476, replace with:
  `- \`c\` on a rebase, cherry-pick or revert → continues with no summary (merge only this milestone; gap row → M7). From the resolver it closes to the banner as it sends, so the next stop opens the resolver fresh (P5).`
- The same note, after :474, add:
  `- \`c\` while a take or write is landing → refused in the resolver's own words (\`a take is still landing …\` / \`still writing the last take\`), nothing sent.`

## NOTES (builder, 2026-09-24)
- **Compacted mid-ticket** (once). Nothing remains but the journey box, which is ticket 21's walk.
- Scout used: `scout-31.md`.
- **S5 / Scope 3 premises, observed on real git** (`it::resolver::last_file::last_file_closes_to_where_it_opened`):
  1. Between the stage landing and the done read landing: resolver on `b.txt`, `job` Some, `file`/`done` None, notice `(checking the files, false)`; **nothing drawn under the header** — `view/resolver.rs:40-50` (`resolver_view` answers `None` while any opening read is out, the reopen included). By design of the existing read path, not new.
  2. The read answers `Opening::Done` (not "resolved outside"): resolver stays, `done` Some.
  3. The notice survives the read (`landing_notice`) and settles to `every file done ✓ · c finishes the merge`.
- **Question for the conductor/user (capture):** at the moment the read lands the bottom row is ` stage b.txt ✓`, not the bar — a finished write's `Status::Done` lasts until the next key (`update/keys.rs:106`, `view/frame/bars.rs:274-276`). The bar `… e edit  c finish  esc close` shows after any key. The capture holds both frames split by `--- after the next key ---`. The "bar reads" line holds only in the second.
- Also: between the stage and the done read, `c` answers the sticky `still reading the file` (write refusal), per spec order.
- Fix beyond the resolver: `update/finish/answers.rs::resume_answered` `Merged` branch now also closes the resolver (J6 landing).
- Candidate, not done: a merge `Closed` answer from the summary leaves the resolver under it; if `--continue` stops again the resolver may show a stale file. Unverified.
- Test rewrites outside Scope 9 (behaviour moved from status line to notice row): `update/resolver/tests/words_and_c.rs` (rewritten), `update/tests.rs` (`classified_form`, `screen_for`), `tests/it/resolver.rs` (`take_all_and_mark` pump), `tests/it/resolver/file_list.rs` (`a_rebase_lists_only_this_steps_files`, `at_step_two`), `tests/it/finish/choices.rs` + `captures.rs` (`c_on_the_banner` → `c_for_the_summary`; Esc before abort/outside), `tests/it/m5_walk/{resolving,resolved_operations,vocabulary}.rs`.
- Colour unverified (captures carry none).

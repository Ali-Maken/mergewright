# Review — m6-conflicts ticket 28 (done file: `d` only where ops can; folds review-15 M1–M3)

Reviewed read-only at `4e720360`, diff `f585af1..4e720360` (42 files, +2028 −606).
Gates: none rerun. The conductor reported 2869 passed and 1 skipped. Nothing below needed a run to confirm: every finding rests on the lines it cites.

**Counts: Critical 0 · Major 1 · Minor 7.**

## What was verified, and how

- **One rule, no mirror (ops side): holds.**
  - The act runs `apply.rs:196` `op.check` → `check_restart` (`operation.rs:908`), then `apply.rs:200` `prepare_restart` → `restart_source` (`apply/restart.rs:296`).
  - The gate runs `check_restart` (`apply/restart.rs:195`), then the same `restart_source` (`:200`).
  - `find_source` is the one lookup behind both `stage_source` and `restart_source`. The shape check is written once (`:137–:148`).
  - The only difference is that the gate skips `Operation::check`'s prelude. It is equivalent today; see Minor 1.
  - Facts scope: the job uses `FactsScope::default()`, which is what `RestartConflict::facts_scope()` returns (`operation.rs:777–:782`).
  - Journal: the gate uses `parse` and the act uses `Journal::open`. Both see the same entries, because `parse` leaves out the torn tail that `open` cuts.
- **No swallowed errors in new non-test code.**
  - `RestartReadError` carries every failure, including the added `Run` variant.
  - `start_over` (`jobs/conflict.rs:254`) turns each failure into `Cannot { "could not check: …" }` and never into `Can`.
  - The two `.ok()`/`.unwrap_or(0)` in the diff are pure moves (`handover.rs` `marker_line`, `resolved.rs`).
- **Sanitising:** `why` is raw in state. It is drawn through the status line (`view/status.rs:44`, `sanitize_line`). The identity row never carries it.
- **UI thread:** the gate runs in the `open` job, off the UI thread (R3).
- **Tick spot-check: 17 ticks checked, 17 held.** Each ticked test name was opened and matched against the tick's words.
  - Advance group, 4/4: `esc_waits_…`, `a_changed_reread_keeps_the_dropped_count`, `e_waits_…`, `m_waits_…`. They assert nothing sent, the words, and `in_flight`/`queued` unchanged. `e` hands over after the takes land.
  - ops `it::restart_gate`, 7/7: five value tests, the torn tail, and `the_gate_and_the_restart_agree`. `gate()` asserts the journal bytes, the backup refs and the porcelain status unchanged.
  - App unit `file_list::start_over`, 5/5, including the 10-fixture `d_is_in_the_bar_exactly_when_the_box_opens` with `offered == 5`.
  - Job `a_done_open_carries_the_gate`, 1/1. graft-ui `a_done_file_that_cannot_start_over_offers_edit_and_close` plus `COVERAGE_SLOTS = 32` (`graph_screen/tests.rs:1932`), 1/1.
  - it tests, 5/5: `a_whole_file_done_file_offers_no_start_over`, `a_done_marker_file_starts_over_in_place` (4 inputs, `UU`, same cell, block 0), `a_shell_merge_done_file_answers_d_in_words`, `done_open_capture`, `done_start_over_capture`. The `last_file` pin changed as stated.
  - Line counts, `wc -l` at `4e720360`: `update/resolver.rs` 385, `answers.rs` 333, `tests/advance.rs` 394. They match NOTES.
  - One tick is weaker than its words: `an_unreadable_done_file_keeps_its_gate` asserts the frame lacks `d starts it over`, not keys `== ""`. It holds in substance, and it is how Minor 3 slipped through.
- **Captures (6):** the words match the ticket. The look and colour are the user's to judge (rules 5, 6).
  - `fix-done-start-over-can-120x40`: `▸ shared.rs · done ✓ · d starts it over`, bar `d start over  e edit  esc close`. ✓
  - `…-cannot-120x40`: `▸ gone.txt · done ✓` with nothing after it, and bar `e edit  esc close`. After `d`, the status row adds a suffix; see Minor 5.
  - `…-cannot-200x50`: the identity row beside the list column, one document, bar `e edit  esc close`. ✓
  - `fix-file-list-done-open-{120x40,187x50,200x50}`: `▸ gone.txt · done ✓`, no `d`, bar `e edit  esc close`. ✓
  - All six are under 2 KB.
- **Pipeline:** Amendment 3, with Need, Prior art and Journeys, is in `docs/design-notes/m6-conflicts.md:789`. The journey rows are at `:497–:498`. J11 = 4 is asserted (`tests/it/resolver/file_list.rs:307`). J12 = 1 is not walked here; that is ticket 21's job.

## Critical

None.

## Major

### M1 — `jobs/conflict.rs` grew 429 → 602 lines, mostly a third copy of one fixture
- **Where:** `crates/graft/src/jobs/conflict.rs:455–:528`: `app_merge_with_a_whole_file_conflict`, `git`, `try_apply`, `apply_in`, used only by `a_done_open_carries_the_gate` (`:421`).
- **What:** PROMPT.md 6.2 sets files at about 400 lines. The file was already over, and this ticket added 173 lines (+40%). About 115 of them rebuild a merge fixture that exists twice already:
  - `crates/graft/tests/it/resolver/restart.rs:222`
  - `crates/graft-ops/tests/it/restart_gate.rs:25`

  The three copies already differ: file bodies, and `graft_fixtures::Shape::Trivial` against `TempRepo::init`.
- **Why it matters:** this ticket is itself a size split (review-15 M3, R7). Growing the job file past the limit in the same ticket undoes the point. Three hand-kept copies of one fixture will drift, and the test that says "Can" in one place and "Cannot" in another is only as good as its fixture.
- **Fix:** move the job test to a `jobs/conflict/tests.rs` child, or drop it for the it-level `a_whole_file_done_file_offers_no_start_over`, which covers the same answer end to end. Either way the production file ends near 363 lines. Longer term: one `graft_fixtures` shape for "Graft's merge stopped on a UU and a UD".

## Minor

### m1 — The gate reaches `check_restart`, not `Operation::check`
- **Where:** `crates/graft-ops/src/apply/restart.rs:195`.
- **What:** the act runs the whole of `Operation::check`: `bare`, then `check_conflict_state`, then `check_restart` (`operation.rs:874–:908`). The gate runs only the last. They agree today, because `check_conflict_state` always admits `RestartConflict` (`operation.rs:839`, `:854`) and no bare repository has a resolver.
- **Why:** a rule added later to `check`'s prelude for this kind would reach the act and not the offer. That is the drift this ticket exists to end. `the_gate_and_the_restart_agree` would catch it only on its five fixtures.
- **Fix:** in the gate, call `Operation::RestartConflict(RestartParams { path }).check(facts)`. Then "the same rule" is literal, and `check_restart` can go back to `pub(super)`.

### m2 — The app still mirrors ops for the unmerged whole-file card, and disagrees on binary `UU`/`AA`
- **Where:** `crates/graft/src/update/resolver/restart.rs:87–:93` builds `NotMarkerShape { how: whole.how }` from `whole_file`.
- **What:** `whole_file` is set for every open with no document, and that includes a binary `UU`/`AA` (`answers.rs:60–:74`). For `UU`/`AA`, ops' `check_restart` returns `Ok` (`operation/conflict.rs:248`, `has_markers`). So on a binary `UU` card the app answers `x.bin: both modified — no markers to take from` where ops would admit the restart.
- **Why:** the ticket kept this branch on purpose (R5, "its words stay"). It fails in the safe direction: it hides a key, and never offers a broken one. But it is the last place the app decides ops' rule, and it already gives a different answer.
- **Fix, either:**
  - carry an ops answer on the unmerged route too; or
  - make ops' `check_restart` refuse a binary `UU`/`AA` (`checkout --merge` cannot put markers into a binary), so both sides agree by ops' rule.

  Needs a ruling; not urgent.

### m3 — A done card that cannot start over draws a dangling separator
- **Where:** `crates/graft/src/view/resolver/whole_file.rs:79` passes `keys: ""`. `crates/graft-ui/src/conflict_resolver/draw.rs:164` draws `format!("{how} {} {keys}", self.dot())`.
- **What:** a removed, binary or unreadable done file with `Cannot` renders `removed · ` (the separator with nothing after it). No capture covers this state. The unit test asserts only that `d starts it over` is absent (`tests/file_list/start_over.rs:104–:107`).
- **Fix:** make `RegionView::WholeFile.keys` optional, or skip `· {keys}` when it is empty in `draw.rs`. Add a capture of a `Cannot` removed card for the user's eye.

### m4 — The dropped-takes count is still lost when the `Changed` re-read closes the resolver
- **Where:** `crates/graft/src/update/resolver/answers.rs:41` (`resolved_outside` → `closed_resolved_outside`, `:157–:161`) and `:148` (`open_failed` sets `state.resolver = None`).
- **What:** `open.dropped` is only said in `landing_notice`. If the re-read lands on the done route (the file was staged outside meanwhile), or fails, the resolver is dropped with the count and the status says only `resolved outside` or the failure.
- **Why:** this is review-15 M2 on a rarer path. The user's own takes vanish without a word.
- **Fix:** add ` · N queued takes dropped` to the status in both routes when `dropped > 0`.

### m5 — The `d` refusal row carries a suffix the ticket and note don't show
- **Where:** `docs/shots/m6/fix-done-start-over-cannot-120x40.txt`, the last row after `d`: `can't start over · gone.txt: deleted by them — no markers to take from · esc dismiss`.
- **What:** the text is a word-for-word mismatch. Ticket R5 and the note's New UI (`m6-conflicts.md`, Amendment 3, "after d") show the row without ` · esc dismiss`. The suffix is the standing sticky-line convention.
- **Fix:** add the suffix to the note's New UI block. It is a note edit, not a code change.

### m6 — The gate ignores the job's stop
- **Where:** `crates/graft-ops/src/apply/restart.rs:199–:200` (`&()`), called from `crates/graft/src/jobs/conflict.rs:254`, whose `stop` is in hand.
- **What:** `live_entry`, `current_step_chain` and `ls-tree` run to the end even after the user has left the file.
- **Why:** these reads are small; the cost is worker time on a large repository or a long backup chain.
- **Fix:** give `restart_gate` a `stop: &dyn Stop` parameter. This is a signature change to the ticket's spec, so it goes through the conductor.

### m7 — The journey count is counted by the test itself
- **Where:** `crates/graft/tests/it/resolver/file_list.rs:290–:307`.
- **What:** `inputs` counts only the steps routed through the `input` closure. An extra `app.key` written outside `input` would pass with `inputs == 4`. It is a guard only against the test's own shape.
- **Fix:** count at the harness. Have `App` count `key`/`click` calls since a mark, and assert the delta. Ticket 21 can adopt this for every `journey-counts` row.

## Notes (no finding)

- Builder deviation 2 (`handover_refusal` asks the takes before `write_refusal`) is right. A take in flight is `job`, so the other order would always say `still writing the last take`. The ticket's signature comment should be read as superseded.
- Builder deviation 1 (`RestartReadError::Run`) is needed: `journal_path` returns `RunError`.
- Test files that were already over 400 lines grew a little: `tests/it/resolver/file_list.rs` 623 → 655, `update/resolver/tests/file_list.rs` 933 → 938. New cases went into child modules, which is the right direction.

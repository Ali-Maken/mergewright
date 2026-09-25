# Review: M6 look-and-finish, wave 1 — tickets 54–58

Range `debf21b2..63effe89`, reviewed read-only against `63effe89`. The gates ran in a throwaway worktree at that hash,
with its own `CARGO_TARGET_DIR`, one command at a time. The worktree was removed afterwards.

Tickets: 54 conflict colour tokens · 55 the band and state colours · 56 the hint bar keeps the way out ·
57 undo and abort a merge begun in the shell · 58 start over a whole-file choice.

**Counts: Critical 0 · Major 2 · Minor 9.**

## Gates (measured at 63effe89)

| Check | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass (22 s) |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run` (fast tier) | **3141 passed, 1 skipped**, 138 s wall (7.1 budget: 3 min). Largest single process 272 MB |
| `cargo test --doc` | pass |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | **3176 passed, 0 failed, 1 ignored**, 392 s (budget: 10 min). Largest single process 945 MB |
| Release perf: `frames:: heavy::budgets_hold`, `--release`, one thread | **13 passed**, 74 s. Frame p99 5.2–9.0 ms, max 11.6 ms (budget 16 ms). First paint 15–104 ms; the slowest is realistic 100k with no commit-graph. Peak RSS 112 MB (50k refs) |
| `target/` | 6.4 GB |
| Fuzzing, benchmarks | none exist for this range |

The five slowest fast-tier tests: `m5_walk::every_scenario::merge_resolved_with_the_keys` 12.0 s ·
`diff_fixture::parse_unified_matches_numstat_for_every_shape` 8.8 s · `conflict_ops::whole_file_shapes_resolve_like_git`
7.8 s · `undo_cancel::cancel_at_every_step_boundary_is_clean` 7.0 s ·
`layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.7 s.

Green gates are not evidence of the claims below. What I verified is listed per section.

## Ticks spot-checked: 14 checked, 14 held

All 60 test names that the five Acceptance sections tick exist at `63effe89`. I grepped each `fn`; the three `journey_*`
tests come from the `walk!` macro in `every_scenario.rs:120-126`. All of them pass in the run above. I read these ticks
against their test bodies or captures:

- **54 (1):** `existing_tokens_reduce_as_before`. It compares every one of the first twelve tokens × three themes ×
  256/16 against `nearest_256`/`nearest_16` of the truecolor hue. Held.
- **55 (2):** `band_follows_the_block_state` (four block states, plain rows, dark theme);
  `file_list_text_is_unchanged_but_the_mark` (three goldens). Held.
- **56 (3):** `resolver_bar_keeps_the_way_out_at_120` (asserts `d start over`, a trailing `esc close`, and `j/k  M  d`);
  `only_the_resolver_marks_words`; the capture `m6-13-after-ctrl-t-120x40.txt`, whose last row ends
  `j/k  M  d start over  esc close`. Held.
- **57 (5):** `abort_of_a_shell_merge_is_journaled_and_undoable` (MERGE files byte-equal, the `ls-files -u` stages, the
  hand edit byte for byte, HEAD and branch unchanged); `undo_refuses_an_entry_from_before_a_live_merge` (both force
  values, `untouched` equal); `u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge` (the edit is back, the file
  is unmerged again, `MERGE_HEAD` is there, the Undo entry reverses the take); `operation_live_words_match_ops`; the
  capture `fix-shell-merge-abort-and-undo-120x40.txt`, whose three frames match "what right looks like", frame 3's
  amended `Uncommitted changes (2 unmerged)` included. Held.
- **58 (3):** `start_over_refuses_when_the_shape_changed_since_the_gate` (both directions; journal bytes,
  `refs/graft/`, `ls-files --stage`, status and file bytes all unchanged); `gate_and_apply_agree_on_every_shape` (eight
  rows); the capture `fix-start-over-whole-file-120x40.txt`, which has `d start over  e edit  esc close`, the box's two
  lines, `▸[ Cancel ]`, the card and `keep or delete`, and no `--merge` in frame 2. Held.

Not re-verifiable here: the "seen red" claims in 56 and 57. They are records of the builders' own mutation runs.

## Focus: can undo, abort or start over lose a choice he can't get back?

I found no path that loses one silently. What I read, with lines:

- **`RestartChanged` (58).** `prepare_restart` compares the recovered kind with `params.kind`
  (`graft-ops/src/apply/restart.rs:343`). `apply` calls it before `folded_step` and before `begin`
  (`apply.rs:202-205`), so a stale kind is refused before the chained backup, the Pending line and any git write. The
  test above pins "nothing written". The app sends the gate's own answer (`update/resolver/restart.rs:95-108`). The only
  default is `Markers`, for a file open with its blocks, where ops' `Index` arm is always `Markers`
  (`apply/restart.rs:249-253`). The `Cannot` arm is unreachable, because `restart_box` returns at `:36-44` first. So in
  the flows that exist today a spurious refusal is not expected. A whole-file start-over writes the stages back only
  after the step's chained backup (`apply/fold.rs:113-127`), as a marker start-over does. "edits since the choice are
  lost" is what the box says (`resolver/restart.rs:59`).
- **Abort of a shell merge (57).** It runs `git merge --abort` behind a `Full` backup, as its own entry, with every
  gate before `begin` (`apply/finish_merge.rs:102-131`): a live entry, no `MERGE_HEAD`, an octopus, a different head,
  an autostash. The autostash refusal matters: git applies the stash on abort, and `u` could not give it back.
  `merge_state.rs:100-106` records this, and the plain-git test `git_applies_merge_autostash_when_the_merge_is_aborted`
  checks it. What `u` does not put back is `MERGE_RR` and `AUTO_MERGE`. That is open as a question (KNOWN_GAPS M6/57).
- **The undo gate (57).** `refuse_while_live` (`graft-ops/src/undo/execute.rs:185-208`) runs before the drift box
  (`:160`). A pre-merge entry used to be reachable by forcing past the drift box, and is now refused outright. That is
  stricter than before, and nothing new is exposed. Errors from `in_progress` propagate (`:190-191`); none is swallowed.
- **The app's routing (57).** `shell_merge_known` (`update/undo.rs:130-136`) needs the journal read to have landed.
  For a merge Graft began in this session, `operate.rs:801` sets `live_entries` in the same update that sees the stop,
  so no window exists in which a Graft-begun merge is taken for the shell's. If ops is sent a `u` it should not have,
  ops refuses it.

## Critical

None.

## Major

### M1. At 80 columns the resolver's bar now hides `j/k` and `M`, and the finding never reached KNOWN_GAPS (56)

- **Where:** `crates/graft-ui/src/graph_screen/hints/fitting.rs:64-75`. Hints that keep their words are placed at
  `Full` before any other hint is placed even at its bare key.
- **What the capture shows** (a textual finding, so it stands on the capture): `docs/shots/m6/m6-13-after-ctrl-t-80x24.txt`.
  - The last row was `… ⌃O/⌃T all  O/T file  B  e  n  N  ⏎  j/k  M  d  esc` at `debf21b2`.
  - It is now `… ⌃O/⌃T  O/T  B  e  n  N  ⏎  d start over  esc close`.
  - So `j/k` and `M` are gone, and `⌃O/⌃T all` and `O/T file` lost their words.
- **Against the spec:** ADR-0044 §4 (`docs/adr/0044-…md:96-99`) says everyday keys "go bare first", and that the marked
  hints take their full rung "before any other hint is **raised from** its bare key". That wording has the others
  present at their bare keys. The ticket's Scope put the words ahead of the bare keys instead, and the code follows the
  ticket. At 80 the two readings differ, as the capture shows. At 120 I expect them to draw the same bar, but I did
  not run the other order.
- **Where the finding went:** the builder flagged it in 56's NOTES ("Finding for the user"), and the contact-sheet
  caption says it (`m6-contact-sheet-captions.tsv:87`). There is no KNOWN_GAPS row and no log line.
- **Why it matters:** he loses sight of two keys at 80 columns (visibility, `docs/research/ux-principles.md`). That is
  a spec question left undecided.
- **Fix:** conductor decision. Either (a) change the order to what the ADR literally says: all bare keys first, then
  the marked hints to `Full`, then the rest raised. Monotonicity still holds, because it is still one fixed sequence.
  Or (b) keep the current order and amend ADR-0044 §4 to say `j/k` and `M` may drop at 80. Open a KNOWN_GAPS row either
  way.

### M2. Two files pushed over the ~400-line limit (PROMPT.md §6.2) — 54, 55

- **Where:**
  - `crates/graft-ui/src/theme/mod.rs` went from 358 to 490 lines (the tests are out of line, at `:38`), all of it
    non-test code.
  - `crates/graft-ui/src/conflict_resolver/files.rs` went from 373 to 427 lines (tests out of line, at `:426`).
- **Why it matters:** readability is a hard rule, and both files crossed the limit in this range. Neither ticket's
  NOTES mentions it.
- **Fix:**
  - `theme/mod.rs`: move the conflict-token table and `pinned`/`tint` into a `theme/conflict.rs` child, next to
    `chrome.rs` and `depth.rs`.
  - `files.rs`: move the words split (`WordsTone`, the cell's two-style draw) into `files/words.rs`.

## Minor

### m1. `u` on the banner of a shell merge can open the undo-after-push box, and the refusal only comes after it (57)

- **Where:** `update/banner.rs:374-380`, and the comment at `:376-378`, which says "The refusal first". In a shell merge
  `in_progress_refusal` is `None`, so the code calls `undo_key`. That shows the push box first (`update/undo.rs:38-40`)
  when the next undo is a push.
- **What happens:** if the push came before the merge, "Undo locally" then reaches ops' gate and is refused with
  `OperationLive`. Nothing is written, and `Re-push` is refused by `check`'s in-progress wait. But the box is about an
  undo that cannot happen, and the comment is wrong in this case.
- **Fix:** extend KNOWN_GAPS M6/57 row 2 (the same question on the global path) to cover the banner path, and correct
  the comment.

### m2. The wording has two sources inside ops (57)

- **Where:** `graft-ops/src/preview.rs:715` and `undo/execute.rs:231` both build `finish or abort the {} first`.
  `operation_live_words_match_ops` pins only `execute`'s `explain()`.
- **Fix:** give `NotInvertible::OperationLive` one wording function that both call.

### m3. Stale doc comments on `restart_box` (58)

- **Where:** `update/resolver/restart.rs:17` still says "`checkout --merge -- <path>` behind it", and `:27` says "the
  box would promise markers". A whole-file start-over is `--ours`/`--theirs` and promises "keep or delete".
- **Fix:** word both lines by kind, as the module doc (`:1-4`) already does.

### m4. The Journey rows for 57 are missing from TESTING.md (57)

- **What:** 58 added `walk-back-a-whole-file-choice` to the journey table (`TESTING.md:2333`). 57 did not add
  `b-abort-shell-merge` (3) or `b-undo-a-take-in-the-shell` (1), and every other walked journey has a row.
- **Fix:** add both rows, with start, keys, count and "what right looks like".

### m5. Rows that 57 and 58 answered are still open and now stale (conductor)

- KNOWN_GAPS M6/08 :611 still says `a` "refuses" and asks whether `u`/`a` should work. Both are built.
- TESTING.md "Times I left", the row "Get back a hand edit after a whole-file take" (`TESTING.md:113`), still
  says "`u` and the banner's `a` both refuse". 57's Touches reserved its closing for the acceptance ticket, and asked
  the builder to name the proving test. The NOTES name none.
- KNOWN_GAPS M6/25 :673 says "Whole-file shapes likewise (P10, not decided)". 58 decided them for merges Graft began.
- **Fix:** close :611 with `shell_begun.rs::a_on_the_banner_aborts_a_shell_merge` and
  `::u_on_the_banner_restores_a_whole_file_take_in_a_shell_merge`, reword :673, and give acceptance 63 the named test
  for the "Times I left" row.

### m6. Question: does the gate's "same merge" let an entry through from an earlier, aborted instance of the same merge? (57)

- **Where:** `undo/execute.rs:196-200` compares only `merge_head` and `other_heads`.
- **The case:** shell merge of `feature` → a whole-file take (an entry, with `merge_after` = that merge) → `git merge
  --abort` in the shell → `git merge feature` again, which stops. `u` then reaches the take's inverse through the gate,
  and only `drift` stands between it and a force.
- **What I could not confirm:** that `drift` flags this case. I did not run it. Filed as a question.
- **To settle it:** a test for the case above. It should assert either the drift box or `OperationLive`.

### m7. Question: the start-over box for a binary `UU` chosen whole promises markers (58, decision 3)

- **What:** `gate_and_apply_agree_on_every_shape` records `Markers` for `pic.bin`, and NOTES say that what git leaves
  on disk is not asserted. The box will read "the conflict comes back with all its markers"
  (`update/resolver/restart.rs:54`).
- **Why it is only a question:** decision 3 kept today's behaviour on purpose, so this is not a defect of 58. The box's
  promise for a binary file is unverified.
- **Fix:** a KNOWN_GAPS row (unverified).

### m8. The spec marked `d start over` in the done list, and the code doesn't (56, decision 2)

- **Where:** `graph_screen/hints/lists.rs:346` pushes `hint("d", "start over")` without `.keeps_words()`.
- **Why it doesn't matter yet:** that list has at most three hints, all inside the floor, so the bar draws the same.
  It diverges as soon as the done list grows (ticket 60 adds `c finish` first).
- **Fix:** mark it, as decision 2 says.

### m9. The Touches check for the 57 app half says nothing about ops

- **What:** the app half's Touches line says "graft only in this half". That is true. The ops half's line says
  graft-core was edited outside the crate set (`OperationKind::AbortMerge`, `graft-core/src/journal.rs:94-98`). It was
  reported, it was necessary, and the journal round-trip test covers it.
- **Why it's filed:** so the knock-on is on record. Journal files written by this build carry `"kind":"abort_merge"`,
  which an older build cannot parse (`journal_round_trips_an_abort_merge_entry`).
- **Fix:** none needed now. Unreadable-line handling exists (`JournalState::unreadable`). Say so in the M6 log.

## Touches (knock-ons), per ticket

- **54:** the `Token::ALL` readers were grepped and the snapshot is additions only. Nothing draws the new tokens, as the
  ticket said. Held.
- **55:** captures regenerated, with the list in NOTES. `three_panes.rs` and `view/resolver/tests.rs` are unedited
  (the diff for `crates/graft/tests/it/resolver.rs` is 56's). The contact sheet was reassembled (part 1 178 204 B,
  part 2 153 885 B, both ≤ 180 000). The captions `fix-band-*` are present (`captions.tsv:125-126`). Held.
- **56:** bottom rows only, checked by `git diff -U0` per NOTES. `m6-13` was retaken and its captions (`:83`, `:87`)
  updated. The 80-column consequence was not routed (M1).
- **57:** journal (`kind_name`, the `J` row asserted), keymap §2.6a matches the code, banner `a abort` was already drawn
  (`m6-18-launch-shell-begun-120x40.txt:26`), captions row `:127`, contact sheet part 2 includes it. The TESTING
  journey rows are missing (m4), and the gap rows were not closed (m5).
- **58:** every `RestartParams`/`StartOver::Can` builder was adapted. The `fix-done-start-over-cannot-*` captures were
  retaken on a shell merge, with their captions reworded (`:58-59`), and the journey row and note fence were added.
  Held.

## Architecture, rule 3 and escapes

- **Crate rules:** no new dependency, and no Cargo change in the range. The pure core is unchanged: the new gate reads
  sit in ops' executor and in `apply`, and both run on jobs, not on the UI thread. `head_word` reads `state.refs`
  only (`banner.rs:537-552`).
- **`unwrap`/`expect`:** every hit added in the range is in test code (checked by `grep` on the diff; the hits are in
  `files/tests.rs`, `three_panes/band.rs`, `card_colours.rs`, `update/*/tests`).
- **Rule 3:** no new `let _ =`, `unwrap_or_default` or empty `Err(_)` in source. `start_over_kind`'s fallback to
  `Markers` is commented and unreachable for `Cannot` (see Focus).
- **Escapes:** the abort box's lines, the branch name included, go through `sanitize_line` (`banner.rs:525-527`). So do
  the restart box's title, lines and command (`update/resolver/restart.rs:73-81`).

## Pipeline

The design note's Amendment 2026-09-24 has **Need** (:869), **Prior art** (:890, citing
`docs/research/conflict-visuals-and-finish.md`, which exists) and **Journeys** (:902). J15 counts 3, J16 counts 1 and
J17 counts 4, and the `journey-counts` fence (:505, :511, :512) matches. The walks send `a → ⏎` (3), `u` (1) and
`k d → ⏎` (4). The prototype step is recorded as met (:866). Colour stays unverified (rule 5). No look is judged here.

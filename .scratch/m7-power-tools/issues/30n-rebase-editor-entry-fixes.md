# 30n: the rebase-editor entry, the open race, and four small fixes (review 08..rebase-keeps-drops Q1, Q2, m1, m3, m5, m6)

Status: done
Build with: Opus. Six local fixes across graft-ops and graft, each pinned by a named test. Q2 needs the stale-id path
traced (which answer is dropped, which modal holds the keys), not copied; nothing here is expensive to undo → not Fable.
Blocked by: **follow 24-a** (holds graft `update/operate.rs`, where `stage_all` lives — m6) **and 24-a2** (holds
graft `update/preview.rs` — m1, m3). **Not concurrent with 24-b** (both hold `tests/it/m7_walk/`, `docs/shots/m7/`,
the m7 captions TSV and contact sheet); either side of it. **Precede 30m** (it splits `update/rewrite_editor.rs`,
`update/operate.rs`, `update/preview.rs` and renames `by_name`; 30m's `Blocked by` gains 30n).
Scout: none. The review names every line (`docs/reviews/m7-08-to-rebase-keeps-drops.md` :162-172, :183-194,
:212-224, :268-297). Lines have moved since; re-grep each before editing.
Needs: rebase-preview-keeps-drops (merged): `keeps_drops`, `open_rewrite_from_rebase`, `MarkedBy`, `RewriteRequest::FromRebase`.

## Files (exact; outside this list → stop, `blocked: needs <file>`)
- graft-ops: the file holding `pub enum Refusal` and `Refusal::explain` (grep; MAP `operation` :56), `src/operation/history.rs`
  (`head_is`, ~:523) + the tests module beside each, `MAP.md`.
- graft: `src/update/preview.rs` (`keeps_drops_here`; the `o` unborn words ~:1026), `src/update/preview/tests.rs`,
  `src/update/rewrite_editor.rs` (`new`, `open_rewrite`, `open_rewrite_from_rebase`, `opened`),
  `src/update/rewrite_editor/after.rs` (only if the first `After` is derived there), `src/update/rewrite_editor/words.rs`
  and `src/view/rewrite_editor.rs` (only the site that turns `After::Invalid` into words — grep `AfterFacts::Invalid(`),
  `src/update/rewrite_editor/tests.rs`, `src/update/operate.rs` (`stage_all` only), `src/update/keys/edit_stop.rs`,
  the one other `stage_all(` caller (`Ctrl+A`; grep, name it in NOTES), their tests,
  `src/update/context_menu/tests.rs` (the one unborn-`o` test), `tests/it/m7_walk/rebase_line.rs`
  (`capture_keep_drops_editor` only), `MAP.md`.
- graft-ui: **none.** If the AFTER words turn out to be made in graft-ui → stop, blocked.
- Docs: `docs/shots/m7/rebase-keep-drops-editor-120x40.txt` (re-taken), `docs/shots/m7/m7-contact-sheet-captions.tsv`
  (row 4), `docs/shots/m7/m7-contact-sheet.txt` (re-assembled: `scripts/contact-sheet.sh --captions-order m7 docs/shots/m7`),
  `TESTING.md` :2849 (the editor capture's row) and any row the m3/m6 greps below find.

## Read first
- Review `docs/reviews/m7-08-to-rebase-keeps-drops.md` :162-172 (m1), :183-194 (m3), :212-224 (m5, m6), :268-297 (Q1, Q2).
- `.scratch/m7-power-tools/issues/rebase-preview-keeps-drops.md` Decided 3, 5, 7 and "Landing".
- `docs/design/02-keymap.md` §2.3b row `R` (:245), §2.4 `Ctrl+A` (:259).
- `docs/research/ux-principles.md` §2 H1, H9; §3.1.
- `crates/graft/MAP.md` `update::rewrite_editor` (:242-258), `update::operate` `stage_all` (:301), `space_stages_here`
  (:329), `AppState::modal_owns_bottom_row` (:169), `update::preview` `preview_key` (:492).
  `crates/graft-ops/MAP.md` `Refusal` (:56), rewrite `validate`/`TodoProblem` (:283).

## Decided (planner, 2026-09-29; not open)
1. **Q1 — arrival is not a refusal.** When the editor opened from a rebase has **every row `drop` and every one of
   them still `MarkedBy::AlreadyIn`** (none marked by him), AFTER reads `nothing kept yet · p keeps a commit`, not
   `every commit dropped · the branch would be its base again`. `y` stays refused there (no `y rewrite` in the bar).
   Once he marks any of those rows himself (`p` then `d` again), the ordinary words come back: it is then his choice.
   - **Why this option:** he pressed `R` to keep something; the first line he reads must say where he is and the next
     key, not refuse (ux-principles §3.1 "land where the next action is"; H9 "what, and the next key"; CLAUDE.md "a
     status must never read as an error when nothing is wrong"). The cursor is already on the newest drop (Decided 7),
     so `p` is one key.
   - **Rejected — exempt `FromRebase` from `EverythingDropped`:** an unedited `y` in the editor would then drop the
     whole branch, the one outcome he opened the editor to avoid; `r r` in the preview already does that; and
     graft-ops' `validate` would gain a rule that varies by how the editor was opened. Decided 3's anchor (the todo
     equals the rebase's) is about the todo and still holds; only the `y` gate differs.
   - **Rejected — accept as is:** the refusal on arrival is the defect.
   - The words are made in graft (it knows `MarkedBy`); graft-ops `validate` and `TodoProblem::explain` unchanged.
   - `y` in that state: whatever it does today on any `After::Invalid` stays. If it writes words, they are these words.
     Say in NOTES which it is (file:line).
   - m10's gap row is closed by this decision (conductor, not builder).
2. **Q2 — the newest open wins; an answer never opens the editor over another box.**
   - `open_rewrite` / `open_rewrite_from_rebase` with a read already out: **cancel the old read** (its id goes stale,
     the same path `Esc` uses while reading) **and start the new one**, status `reading <branch>…`. No silent return.
     Why: the last key is his current intent; refusing it makes him wait for a read he no longer wants (H3).
   - `opened` when the answer lands while the graph does **not** have the keyboard (any modal owns keys, or a graph
     overlay other than the editor is up — the same test `space_stages_here` makes; share it if it is a helper, copy
     nothing if it is inline, and stop blocked if sharing needs `update/state.rs`): **the editor is not set**, the
     answer is dropped, and the status is `Status::Sticky` reason `rewrite editor not opened · another box was open`,
     keys `R opens it` (request `Open`) or `r then R opens it` (request `FromRebase`). The status is hidden, not
     cleared, while the modal owns the bottom row (MAP :169), so he reads it when he closes the box (H1, H9).
   - `Esc` while reading: unchanged (cancels, as today).
3. **m1.** `keeps_drops_here` also requires `preview.acts()`. Key, row and footer then agree in the undrawable box
   (`preview cannot be drawn here: … · esc`): `R` swallowed there, as every unnamed key.
4. **m3.** New `Refusal::HeadUnborn`, words `HEAD has no commit yet · c makes the first one`. `head_is` returns it for
   `HeadState::Unborn` instead of `BranchChanged`; `check_pick` and `take_operation` both say it through it; graft's
   unborn-`o` words (`update/preview.rs` ~:1026) use `Refusal::HeadUnborn.explain()`. Key-free of `o` on purpose:
   grep `head_is(` — any caller other than those two must stay true under the words; list each caller in NOTES.
5. **m5.** `RewriteEditor::new` claims nothing it did not read: it derives `after` from its own todo — `validate`'s
   problem when it fails, else the same checking state `replan` sets before its plan answers. No hard-coded
   `TodoProblem::EverythingDropped` in `new`. `opened` still calls `replan` straight after.
6. **m6.** `stage_all` says why it staged nothing; both callers (`space` at an edit stop, `Ctrl+A`) set the status.
   - Nothing stageable → `nothing to stage`, set with the same `Status` kind as the stash's `nothing to stash`
     (grep that string; consistency H4).
   - A write running → `operate::InFlight::words()`, as `f` does (MAP :519).
   - Any other `false` path found in `stage_all` → its own variant with true words; list each in NOTES.

## Signatures
```rust
// graft-ops, operation (Refusal's file)
pub enum Refusal { /* … */ HeadUnborn, /* … */ }   // explain(): "HEAD has no commit yet · c makes the first one"

// graft::update::operate
pub(crate) enum NotStaged { NothingToStage, WriteRunning /* , any other path found, named */ }
impl NotStaged { pub(crate) fn status(&self) -> Status; }
pub(crate) fn stage_all(state: &mut AppState, commands: &mut Vec<Command>, next_id: /* as today */)
    -> Result<(), NotStaged>;                       // was -> bool

// graft::update::rewrite_editor — signatures unchanged; behaviour per Decided 2 and 5.
// open_rewrite / open_rewrite_from_rebase: a read out → cancel it, start the new one.
// opened: graph lacks the keyboard → editor not set, Sticky status per Decided 2.
```

## Acceptance (name the verifying test at tick time; **(red)** = write it, see it fail on today's code, then build)
- [x] **Q1:** (ticked: `an_unedited_rebase_editor_says_nothing_kept_yet` red then green; `a_drop_he_marks_again_reads_every_commit_dropped`, `an_open_editor_with_every_row_dropped_keeps_its_words` green guards; graft-ops `rewrite::plan::tests::from_rebase_unedited_matches_the_rebase_todo` green, unedited) `update::rewrite_editor::tests::an_unedited_rebase_editor_says_nothing_kept_yet` (red; AFTER words
  exact, no `every commit dropped`); `a_drop_he_marks_again_reads_every_commit_dropped` (p then d on every row →
  ordinary words); `an_open_editor_with_every_row_dropped_keeps_its_words` (`Open` entry, all `d` → ordinary words,
  unchanged). graft-ops `from_rebase_unedited_matches_the_rebase_todo` still green, unedited.
- [x] **Q1 capture** (ticked: `m7_walk::rebase_line::capture_keep_drops_editor` re-taken and asserting both; line 10 reads the new words, bar has no `y rewrite`; TSV row 4, TESTING.md :2855 (the row had moved from :2849), sheet re-assembled) re-taken by `capture_keep_drops_editor`, which now asserts `nothing kept yet · p keeps a commit`
  and the absence of `every commit dropped`: `docs/shots/m7/rebase-keep-drops-editor-120x40.txt`. **Right:** as before
  (3× `already in main · p keeps it`, `─ main` base row, `▸` on a3), and line 10 reads `nothing kept yet · p keeps a
  commit`; no `y rewrite` in the bar. TSV row 4 and TESTING.md :2849 say exactly that; contact sheet re-assembled.
  Colour unverified (rule 5).
- [x] **Q2:** (ticked: all four named tests red then green; `esc_while_reading_still_cancels` is the old `esc_while_reading_cancels_the_open` renamed, green) `update::rewrite_editor::tests::r_in_a_preview_while_a_read_is_out_opens_on_the_rebase` (red: the old
  answer dropped, the editor opens with `AlreadyIn` rows); `r_on_the_graph_while_a_read_is_out_restarts_it`;
  `an_answer_under_a_preview_does_not_open_the_editor` (red: preview still up, `rewrite.editor` `None`, status reason
  and keys `R opens it` exact); `an_answer_under_a_preview_from_a_rebase_names_r_then_r` (keys `r then R opens it`);
  `esc_while_reading_still_cancels` (green guard).
- [x] **m1:** (ticked: `update::preview::tests::r_is_unbound_when_the_preview_cannot_be_drawn` red then green; `r_is_unbound_in_other_previews`, `the_row_names_r_only_when_live` green) `update::preview::tests::r_is_unbound_when_the_preview_cannot_be_drawn` (red: undrawable box, `R` →
  no `OpenRewrite`, box stays); `r_is_unbound_in_other_previews` and `the_row_names_r_only_when_live` green.
- [x] **m3:** (ticked: graft-ops `operation::history::tests::head_is_refuses_an_unborn_head_as_head_unborn` red (did not compile: no variant), `operation::tests::head_unborn_says_there_is_no_commit_and_the_key_that_makes_one`; graft `o_on_an_unborn_head_says_check_picks_words` seen red on the old words, then green) graft-ops `head_is_refuses_an_unborn_head_as_head_unborn` (red) beside `head_is`; `explain` words test
  for `HeadUnborn`. graft `context_menu::tests::o_on_an_unborn_head_says_check_picks_words` expectation moved to the
  new words (red first). Any graft-ops test pinning `BranchChanged` for `Unborn`: updated, listed in NOTES.
- [x] **m5:** (ticked: `a_new_editor_claims_nothing_before_its_first_plan` red then green; `a_new_all_drop_editor_is_invalid_from_the_start` green before and after) `update::rewrite_editor::tests::a_new_editor_claims_nothing_before_its_first_plan` (red: `new` on a
  todo with a `pick` → AFTER is not `Invalid(EverythingDropped)`); `a_new_all_drop_editor_is_invalid_from_the_start`.
- [x] **m6:** (ticked: `update::keys::edit_stop_tests::space_with_nothing_to_stage_says_so`, `space_while_a_write_runs_says_so`, `update::detail::tests::ctrl_a_with_nothing_to_stage_says_so` all red then green; 07d `space` tests green) `update::keys::edit_stop` tests `space_with_nothing_to_stage_says_so` (red), `space_while_a_write_runs_says_so`
  (red); the `Ctrl+A` caller's `ctrl_a_with_nothing_to_stage_says_so` (red). 07d's `space` tests green; any test
  pinning `Ctrl+A`'s silence updated and named in NOTES.
- [x] MAP.md: (ticked: docs only; graft-ops :56, graft `open_rewrite`/`opened`/`stage_all`/`NotStaged`/`no_box_is_up`/`keeps_drops`) graft-ops `Refusal::HeadUnborn`; graft `stage_all`'s return and `NotStaged`, `open_rewrite`/`opened`'s
  race rules, `keeps_drops_here`'s `acts()`. Docs only.
- [x] Gates workspace-wide. (ticked: fmt, clippy -D warnings, nextest 4857/4857, test --doc all ok)

## Touches (check each; say how on the **Touches:** line)
- **`Refusal`:** every exhaustive `match` on it (graft-ops `explain`, graft `operate::refusal_status`, grep
  `Refusal::` in both crates); every `head_is(` caller; `docs/shots/` and TESTING.md for `HEAD moved to another branch`
  quoted about `o` (grep; none expected — say so).
- **AFTER words:** every test or snapshot quoting `every commit dropped` (grep graft, graft-ui, `docs/shots/`): only
  the re-taken capture and the sheet change; graft-ui `rewrite_editor` tests feed words and stay green.
- **The race:** every caller of `open_rewrite`/`open_rewrite_from_rebase` (`graph_key`'s `R`, `preview_key`'s `R`);
  `owns` and the stale-id set; `rewrite_key`'s `Esc` cancel; `opened`'s three outcomes (editor, refusal, failure) —
  the refusal and failure words are unchanged.
- **Walk:** `m7_walk::rebase_line::keep_a_suggested_drop` still ≤ 5 keys (`motion r R p y`), unedited.
- **`stage_all`:** both callers; the hint bar's `space stage` and `⌃A all` unchanged (captures listed by the grep above
  stay byte-identical); keymap §2.4 unchanged (it says nothing of silence).
- **Keymap:** no key added, removed or rebound. **Journal/undo:** no new operation; none touched.
- **24-a / 24-a2 edits** to `operate.rs` and `preview.rs`: rebase on their merges; do not touch their arms.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium (~50k). If it compacts, say so; split point: m3 (graft-ops + its one graft test) first, then the rest.

## NOTES (builder)

- No compaction. No scout (ticket: "Scout: none", review names every line; re-grepped each).
- **Q1 `y`:** `after::start`'s `After::Invalid` arm (`update/rewrite_editor/after.rs:95`) sets `editor.notice` — now `words::invalid`, so `y` there writes `nothing kept yet · p keeps a commit`. Words made in graft `words::invalid`; graft-ops `validate`/`explain` unchanged. Condition: `EverythingDropped` and every row `Drop` with `by_name == AlreadyIn`.
- **Q2:** gate at `update/rewrite_editor.rs:552`. Shared the predicate: extracted `keys::edit_stop::no_box_is_up` (modal owns keys, or any graph overlay) from `graph_has_the_keyboard`; focus deliberately not part of it (Tab to the pane during the read is not "a box"). Verified by test that the sticky survives `Esc` on the preview. Refusal/failure outcomes of `opened` unchanged. Replaced test `a_second_capital_r_while_reading_starts_nothing` (it pinned the old silent return) with `r_on_the_graph_while_a_read_is_out_restarts_it`.
- **m3 `head_is(` callers:** `check_merge`, `check_rebase`, `check_pick`, `check_revert`, `check_reset` (`Head` target), `operation/rewrite.rs:56` (rewrite). All true under `HEAD has no commit yet · c makes the first one`. `check_checkout`'s own `BranchChanged` (head_before compare) untouched. No graft-ops test pinned `BranchChanged` for `Unborn`. Note: `Refusal::UnbornHead` (amend/stash words) already exists; kept apart per Decided 4 — its words name amend and stash, false here. Conductor may want one variant later.
- **m6:** Ctrl+A caller is `update/detail/keys.rs` `changes_pane_write_key`; it now returns `true` on `Err` (was `false`, fall-through; Ctrl+A has no other meaning there, keymap §2.4). `nothing to stash` is **not** a `Status` anywhere (it is the stash prompt's inline refusal, `view/stash.rs:100`), so `nothing to stage` uses `Status::sticky` (the refusal kind, as `nothing to skip in a merge`). Deviation from signature: `WriteRunning(InFlight)` carries the running write for its words. Extra variants: `OnlyConflicted` (`stage all does not act on a conflicted file — resolve the conflict first`, `conflict_refusal`'s words), `Unread` (working tree not read; `Status::Waiting`, `still reading the changes — try again in a moment`). `stage_all` now `pub(crate)`. Operate's `stage_all_*` tests moved from bool to `Ok`/`Err(NothingToStage)`; no test pinned Ctrl+A's silence.
- **m5:** new `After::NotAsked` (drawn `Checking`); test fixture `open_an_editor_planned(false)` now sets `Invalid(EverythingDropped)` explicitly (it relied on the placeholder).
- Walk `keep_a_suggested_drop` green, unedited. Colour unverified (rule 5).

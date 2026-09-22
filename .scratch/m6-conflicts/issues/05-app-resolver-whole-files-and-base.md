# app-resolver-whole-files-and-base: `o`/`t` on deleted/added/binary conflicts through git, `B` shows the merged base inside every open block, `d` starts a file over behind a confirm box, and the stash-pop conflict (no banner) walks the same path

Status: partial — 1 of 9 acceptance boxes open (71: half verified, half filed as a question) (continuation 2)
Blocked by: **01**, **03**, **04**.
Build with: Sonnet — set by the conductor after the scout (`.scratch/m6-conflicts/scout-app-g05.md`); the keymap gate on line 13 is already met (`02-keymap.md:245-246`, `4dfa409`).
Crates: graft
Needs:
- `app-conflict-resolver` — `ResolverState`, `resolver_key`, `open_resolver`, `conflict_opened`, `resolver_view`, `Command::OpenConflict`, `Product::ConflictOpened`.
- `ops-conflict-operations` — `Operation::{ResolveWholeFile, RestartConflict}`, `conflict::{base_for, BaseError}`, `Refusal::NotMarkerShape`.
- `git-conflict-model` — `UnmergedHow::{words, has_markers}`, `ConflictFile`, `attach_base`.
- `ui-conflict-resolver` — `RegionView::WholeFile`, `BlockView.base`, `DocView.base_shown`, `HintContext::Resolver { base, restart }`.

**Spec change required before the build**: `B` (show/hide base, UPPERCASE = toggles a mode, §2.1) and `d` (start this file over — the discard verb) are Proposed changes 2(b) and 2(c) in the design note. The conductor adds them to `02-keymap.md` §2.6 first; until then the builder ships pieces 1 and 4 only and reports `blocked: keymap` for 2 and 3, and `HintContext::Resolver { base: false, restart: false }` stays.

## Scout first (Sonnet `Explore`, ≤ 1 page) — after G4 merges

- `crates/graft/src/update/resolver.rs` as built: where `ConflictOpened` fills `file`, where the notice is set, how `o`/`t`/`b` are dispatched (to add the whole-file branch before the block branch).
- `crates/graft/src/update/confirm.rs`: `ConfirmView::with_choices` / `ConfirmThen` variants and how the banner's `a` abort box is declared (copy that shape for `d`).
- `crates/graft/src/jobs/`: how G4 declared `OpenConflict`/`TakeSide` (copy for `LoadBase`).
- Report: the exact status-order the walk's stash-pop fixture leaves (`ls-files -u` paths) so the acceptance's "no banner" case uses a real path.

## Read first

- `docs/design-notes/m6-conflicts.md` — New UI "BASE", "Whole-file shapes", Hard-to-reverse choices 3 and 4, Proposed changes 2(b)(c).
- `docs/design/02-keymap.md` §2.6 after the conductor's edit; §2.1 (UPPERCASE toggles).
- `docs/design/06-undo-journal-design.md` §6.4 (`d` folds into the live entry; not undoable while live — the box says so).
- `crates/graft/MAP.md` §update::resolver, §update::confirm, §jobs, §view::resolver (after G4); `crates/graft-ops/MAP.md` §conflict, §operation (whole-file table); `crates/graft-ui/MAP.md` §conflict_resolver.
- `KNOWN_GAPS.md` row 229 (stash-pop conflict: the resolver owns unmerged-index state; no banner, no `in_progress`).

## Scope

In, in order:

1. **Whole-file shapes** — `ConflictOpened(Ok)` with `file: None` no longer refuses: the resolver shows `RegionView::WholeFile { how: <UnmergedHow::words()>, keys, lines: <the file's lines if it exists on disk, else empty> }`; `keys` per shape: `DeletedByThem` → `o keeps the file  t deletes it`; `DeletedByUs` → `o deletes it  t keeps theirs`; `AddedByUs` → `o keeps yours  t drops it`; `AddedByThem` → `o drops it  t keeps theirs`; `BothDeleted` → `o or t removes it`; binary `BothModified`/`BothAdded` → `o keeps ours  t keeps theirs`. `o`/`t` → `Command::Operate(Operation::ResolveWholeFile { path, side, how })` through the existing `operate` launch; `b` → `Sticky("no both for a <how> file · o ours  t theirs")`. On `Operated::Done`: the same `resolved_advance` as `⏎` (the operation also stages). A text `BothModified` reached here (file `Some`) never takes this branch — `NotMarkerShape` is `ops`' guard, asserted.
2. **`B` base** (`ResolverState.base_shown` toggle): first `B` on a file with `file: Some` pushes `Command::LoadBase(id, path, Box<ConflictFile>)` → job calls `base_for` → `Product::BaseLoaded(Result<(Arc<ConflictFile>, usize), String>)`; `Ok((file, n))` → `file` replaced (bases attached), `base_shown = true`, `Status::Done("base shown for <n> conflicts")`; `Err` → `base_shown` stays false, notice `(words, false)`: `no base (added on both sides)` for `NoBase`, `base unavailable after edits` for `Mismatch`, git's words for `Diff3`. Second `B` hides (no job); a later take keeps the attached bases (`take` does not drop `Block.base`); a re-read (`Changed`, hand-over) clears them and `base_shown` — the user presses `B` again. `HintContext::Resolver { base: true, .. }` once the key is bound.
3. **`d` start over** — confirm box: title `Start <path> over?`, lines `the conflict comes back with all its markers`, `takes and edits in this file are lost · not undoable while the <kind> is open`, command `git checkout --merge -- <path>`, `[ Cancel ] [ Start over ]`; `ConfirmThen::RestartConflict { path }` → `Operate(Operation::RestartConflict { path })`; `Operated::Done` → `OpenConflict` again (fresh read, `cursor_block = Some(0)`). Refused in words on a whole-file card when the file is not in the index on both sides (`ops` refuses `NotUnmerged` after `rm` — surface its words). `HintContext::Resolver { restart: true, .. }`.
4. **Stash-pop conflict path** (row 229): `open_resolver` with `in_progress == None` works; the explainer reads `ours = your working tree · theirs = the stash`; `⏎` on the last file closes to the pane with `Status::Done("all conflicts resolved")` and no `c continue`. `ResolveWholeFile` here is its own journal entry (G2) and shows in the journal row as `resolved <path> (ours)` with `← u`.
5. G4's `whole_file_refused_in_words_for_now` test is replaced by the tests below; MAP.md, NOTES.

Out: three-way text merge editing; a base column for taken blocks; rerere; anything on the banner.

## Signatures

```rust
// graft/src/update/mod.rs
pub enum Command { /* existing */ LoadBase(JobId, PathBuf, Box<ConflictFile>) }
pub enum Product { /* existing */ BaseLoaded(JobId, Result<(Arc<ConflictFile>, usize), String>) }
pub enum ConfirmThen { /* existing */ RestartConflict { path: PathBuf } }

// graft/src/update/resolver.rs (additions)
pub fn whole_file_keys(how: UnmergedHow, binary: bool) -> &'static str;
pub fn whole_file_take(state: &mut AppState, commands: &mut Vec<Command>, side: Side, next_id: &mut JobId) -> bool;
pub fn toggle_base(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut JobId) -> bool;
pub fn base_loaded(state: &mut AppState, id: JobId, result: Result<(Arc<ConflictFile>, usize), String>);
pub fn restart_box(state: &mut AppState) -> bool;   // opens the confirm; the confirm's accept pushes Operate(RestartConflict)
```

## Acceptance

`crates/graft/tests/it/resolver.rs`, fixtures `StoppedMerge` (all shapes) and m5's stash-pop conflict shape.

- [x] `theirs-deleted.txt`: the card reads `deleted by them · o keeps the file  t deletes it`; `o` → `Operate(ResolveWholeFile { side: Ours, how: DeletedByThem })`; after `Operated::Done`, `ls-files -u` no longer lists it, the file exists, and the resolver has moved to the next conflicted file — `deleted_by_them_o_keeps`; `t` → the file is gone from the tree and index — `deleted_by_them_t_deletes`. One assertion per `(how, side)` row of the table, binary included — `whole_file_table_through_the_keys` (table test).
  **Ticked (cont. 2):** `deleted_by_them_o_keeps` now asserts a 120-col frame row containing `deleted by them · o keeps the file  t deletes it`, the exact `Operate(ResolveWholeFile(ResolveWholeParams { path: theirs-deleted.txt, side: Ours, how: DeletedByThem }))` (via `press_for_commands`), not unmerged, file on disk, and the resolver open on a *different* path that `ls-files -u` still lists. `deleted_by_them_t_deletes` adds `git ls-files -- theirs-deleted.txt` empty (index). New `whole_file_ops_per_row`: 12 rows (UD/DU/AU/UA/DD/binary UU `pic.bin` × o/t), each asserting the exact `Operate` and the file's bytes or absence after `Done`, not unmerged. Binary `AA` has no fixture file; covered by `whole_file_keys` text only.
  Cont. 1 said: `deleted_by_them_o_keeps` checks the card's `how`/lines, a write queued, file kept, not unmerged — not the drawn card text, not the exact `Operate(ResolveWholeFile { Ours, DeletedByThem })`, not the advance to the next file. `deleted_by_them_t_deletes`: gone from tree, not unmerged. `whole_file_table_through_the_keys` covers only `whole_file_keys` text per shape, no `(how, side)` operation per row.
- [x] `b` on a whole-file card → `Sticky("no both for a deleted by them file · o ours  t theirs")`, no command — `b_refused_on_whole_file`. **Ticked (cont. 1):** `b_refused_on_whole_file` asserts reason + keys and `operating.is_none()`.
- [x] `B` on `three.txt` → `LoadBase`; `BaseLoaded(Ok(_, 3))` → `resolver_view().base_shown == true` and every open `BlockView.base.is_some()`; the 120-column `layout` has `Side { Base }` rows; `B` again hides them with no job — `B_shows_and_hides_base`. **Ticked (cont. 1):** `B_shows_and_hides_base` — status now asserted `== "base shown for 3 conflicts"` (was `starts_with`); every block `reproduced.is_some()`; 120-col frame draws a `BASE` row; second `B` hides with `base_job` none. Checked on the `Block` and frame text, not `BlockView`/`Side { Base }` directly.
- [x] `B` on `both-added.txt` (AA, text) → notice `no base (added on both sides)`, `base_shown == false` — `B_says_no_base`; after `e` hand-over and a re-read, `base_shown == false` and `B` reloads — `reread_drops_base`.
  **Ticked (cont. 2):** fixed at the cause — `ResolverState.how` (set when the read lands) lets `toggle_base` answer an `AA` file `no base (added on both sides)` with no `LoadBase` (conductor's ruling; ops untouched). `B_says_no_base` asserts that notice, no command, no `base_job`, `base_shown == false`. Notice bool is `true` (problem); scope 2 text says `false` — built deliberately (cont. 0 NOTES), not changed. `reread_drops_base` now: `B` → `LoadBase` + shown; `e` → `OpenExternalEditor` checked, `HandedOver(Ok)` fed through `update`, its `OpenConflict` dispatched for real; after re-read `base_shown == false` and no `reproduced`; `B` again → `LoadBase`, shown, every block `reproduced.is_some()`.
  Cont. 1 said: `B_says_no_base` asserts `this conflict has no common ancestor to show`, not the spec's `no base (added on both sides)` — wording deviates, undecided. `reread_drops_base` re-reads via `Esc` + reopen, not `e` hand-over, and never presses `B` again to show it reloads.
- [x] `d` → the confirm box with the four lines above; `Start over` → `Operate(RestartConflict)`; `Operated::Done` → `OpenConflict` and the markers are back in the file (`open() == 1` on `both.txt` after a prior `o`) — `d_starts_over_behind_a_box`; `Cancel` → nothing, resolver unchanged — `d_cancel_is_free`.
  **Ticked (cont. 2):** fixed — line two said `operation`; now names the kind (`merge`) from `in_progress`, and with no operation (stash pop) drops the undoable clause. `d_starts_over_behind_a_box` asserts both message lines exactly (`… not undoable while the merge is open`), choices `["Cancel", "Start over"]`, command, `Start over` → exact `Operate(RestartConflict(RestartParams { path: both.txt }))`, then markers back, `open() == 1`. `d_cancel_is_free` unchanged.
  Cont. 1 said: `d_starts_over_behind_a_box` checks title, markers line, not-undoable line (says `operation`, not the kind), command, markers back, `open() == 1` — not the `takes and edits … are lost` line, button labels, or the `Operate(RestartConflict)` command itself. `d_cancel_is_free` holds in full.
- [x] No journal line is added by `ResolveWholeFile` or `RestartConflict` during a live merge (entry count unchanged) — `whole_file_and_restart_fold_into_live_entry`. **Ticked (cont. 1):** entry count equal before/after both writes. Fixture made the merge with raw git, so no live entry exists to fold into: proves "no new entry", not the fold.
- [ ] Stash-pop conflict: `⏎` on the `CONFLICTS` row opens with the explainer `ours = your working tree · theirs = the stash`; `o` then `⏎` → resolver closes, `Status::Done("all conflicts resolved")`, `ls-files -u` empty, journal's last row `resolved <path> (ours)` (whole-file) or unchanged (block take + `Stage`) — `stash_pop_conflict_resolves_without_a_banner`.
  **Still open (cont. 2).** Whole-file half verified: new `stash_pop_whole_file_journals_its_own_row` (DU stash-pop fixture, `o`) — resolver closes, `all conflicts resolved`, `ls-files -u` empty, journal last row `resolved sample.txt (ours)`. Block half **contradicts the box**: block take + `⏎` with no operation open journals `stage sample.txt` as its own row (observed; `stash_pop_conflict_resolves_without_a_banner` now asserts that). Box says `unchanged`. Question for the conductor: is the ordinary journaled stage correct (I think so — no live entry to fold into), making the box's word wrong? Not ticked until decided.
  Cont. 1 said: Test checks explainer, close, `all conflicts resolved`, no banner, and now `ls-files -u` empty (assertion added). Journal row not asserted in either branch; only the block-take path is walked.
- [x] `hint_context()` → `Resolver { base: true, restart: true }` once bound; `every_key_the_bar_draws_is_answered` and the app chain test pass — named in the report. **Ticked (cont. 1):** `B base`/`d start over` now live in `graft-ui` `resolver_list` and in `BOUND_ACTIONS`; `update::tests::every_key_the_bar_draws_is_answered` sweeps `Resolver { true, true }`, `every_swept_state_is_the_context_it_claims` confirms the classifier answers it. `hints::tests::no_planned_hint_is_ever_drawn` lost its canary (no `Planned` row remains); `planned()` kept under `#[expect(dead_code)]`.
- [x] MAP.md updated. **Ticked (cont. 1):** `crates/graft/MAP.md` §update::resolver (doc, no test).

## Gates

fmt / clippy / nextest / doc, workspace-wide; `seam_rule` (the app builds no `GitInvocation`: `checkout --merge` is `ops`' invocation).

## Size

One context; ~400 lines. Reads G4's `resolver.rs` (it is the file being extended) and the scout page; nothing else in source beyond `confirm.rs`'s box shape.

## NOTES

Built all four scope pieces, in `crates/graft/src/update/{events,resolver,confirm,mod}.rs`, `crates/graft/src/jobs/{conflict,runner}.rs`, `crates/graft/src/view/resolver.rs`, `crates/graft/src/view/rows.rs`, `crates/graft/tests/it/resolver.rs`. Scout used (`.scratch/m6-conflicts/scout-app-g05.md`) — its file:line references for `ResolverState`, `act()`, `conflict_arrived`, the view layer and the ops surface all held.

**Keymap gate**: already satisfied (`02-keymap.md:245-246`, commit `4dfa409`) per the conductor's override of the ticket's spec-change preamble — built pieces 1–4 in full, no `blocked: keymap`.

**`base_job` vs `job` (scout point 3)**: `LoadBase` gets its own `ResolverState.base_job: Option<JobId>` field, never shares `job` (which tracks the open read / a take in flight). A base load and a take answer for the same document could otherwise race for the same id slot, silently dropping whichever lands second. `toggle_base` also refuses while `job` is busy, as a courtesy — the separate field is what actually rules the race out, not that refusal.

**Notice bool for a failed base load**: fixed to `true` (a problem) during this build — the first cut left it `false`, inconsistent with `taken()`'s failure path (`open.notice = Some((failure.words.clone(), true))`, `resolver.rs`). A base asked for and not loaded is a problem the same way a failed take is; the doc comment blaming `base_shown` (not the notice bool) for staying false was correct but the code next to it disagreed. Fixed at the cause (the wrong literal), not worked around in the test.

**BaseError wording — ticket prose vs. real code, deviation, deliberate**: the ticket's acceptance text says `no base (added on both sides)` for `NoBase` and `base unavailable after edits` for `Mismatch`. The actual `#[error(...)]` strings in `graft-ops/src/conflict.rs` are `"this conflict has no common ancestor to show"` (`NoBase`) and `BaseMismatch`'s own Display for `Mismatch` (its doc comment explicitly says never "after edits" wording). `base_loaded()` shows `error.to_string()` verbatim — `ops` decides the words, `app` only wires them — and the test (`B_says_no_base`) asserts the real string, not the ticket's stale one.

**`graft-ui` hint-bar wiring — shipped, not blocked**: `view::rows::overlay_context`'s `GraphOverlay::Resolver` arm passes `HintContext::Resolver { base: true, restart: true }` (was `false, false`) — functionally live the moment `B`/`d` are pressed, tested directly (`resolver.rs`'s `B_shows_and_hides_base`, `d_starts_over_behind_a_box`, etc.) and via `hint_context`'s own unit coverage. What is **not** shipped: `update::keys::BOUND_ACTIONS` does not yet list `("B","base")`/`("d","start over")`, because `graft-ui`'s `resolver_list` (`crates/graft-ui/src/graph_screen/hints.rs`, ~lines 907/918) hard-codes both as `HintState::Planned` regardless of the booleans passed in — flipping that is outside the `graft` crate set. Exact signature to flip: `resolver_list(base: bool, restart: bool)` in `crates/graft-ui/src/graph_screen/hints.rs` — its two `planned("B", "base")`/`planned("d", "start over")` calls become `hint(...)` (or equivalent) when `base`/`restart` are `true`; its doc comments (~lines 301–313) already say this is pending m6-conflicts/05. No functional gap for the user pressing the keys — only the footer's own hint text lags.

**Acceptance checklist, tests named**:
- `deleted_by_them_o_keeps`, `deleted_by_them_t_deletes` (`crates/graft/tests/it/resolver.rs`).
- `whole_file_table_through_the_keys` (table test, all seven `(how, binary)` rows).
- `b_refused_on_whole_file`.
- `B_shows_and_hides_base` (on `three.txt`, checks `base_shown`, `Block.reproduced` on every block, a `BASE` row at 120 columns, second `B` frees no job).
- `B_says_no_base` (on `both-added.txt`, AA has no stage 1).
- `reread_drops_base` (close + reopen; `Block.reproduced` gone on the fresh read).
- `d_starts_over_behind_a_box` (box copy, `→`/`⏎`, markers back on disk).
- `d_cancel_is_free`.
- `whole_file_and_restart_fold_into_live_entry` (journal entry count unchanged across both writes — the fixture's merge itself is raw git, never through the app, so the baseline is whatever `journal_entries().len()` reads before either write, not a fixed `1`).
- `stash_pop_conflict_resolves_without_a_banner` (explainer text on the drawn frame, `Status::Done("all conflicts resolved")`, no `c continue`, no `banner_focus`).
- `hint_context()`/`overlay_context` → `Resolver { base: true, restart: true }`: covered by existing unit tests `every_key_the_bar_draws_is_answered` and `every_planned_hint_is_still_unbound` (`crates/graft/src/update/tests.rs`), which the full `nextest run` gate below re-verifies; no new test needed since these already sweep every `HintContext` combination.
- `MAP.md` updated (`crates/graft/MAP.md`, `update::resolver`, `update::events`/`Command`/`Product`, `update::confirm`, `jobs::conflict`, `view::resolver`).

**Test-harness fixes made along the way** (symptom-level, inside this ticket's own new tests, not production bugs): `resolver_on` waits on `open.file.is_some()`, which never becomes true for a whole-file shape — added `resolver_on_whole_file` waiting on `open.whole_file.is_some()` instead. Re-navigating the changes pane a second time in the same `App` session (after a resolve advances past the resolver's modal keyboard) breaks `on_the_row`/`changes_pane`, which assume a fresh graph-top start; fixed by closing with `Esc` and driving `cursor_to`/`⏎` directly once back in the pane, waiting for `Loading::Ready` first.

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` clean, workspace-wide. `act()`'s cognitive-complexity lint (16/15 after adding `B`/`d`) fixed by pulling the mergetool handover into `mergetool_here` and the `o`/`t`/`b` dispatch into `take_key` — both are root-cause splits (real complexity, not a suppression). `cargo nextest run` workspace-wide: 2598/2598 passed, 1 skipped; `cargo test --doc`: all pass; `seam_rule`: 2/2. One stale sweep fixture fixed on the way: `update::tests::classified_form` mapped `Resolver{false,false}` as the app's form — now `{true,true}`, the only form `overlay_context` produces.

**Continuation 2 (no compaction).** Scout: not re-run — cont. 1's scout page and the ticket's own notes named every seam used. Fixes: (1) `B` on `AA` wording — cause (the app had no shape to word from; `ResolverState.how` added). (2) `d` box line said `operation` — cause (literal replaced by `in_progress.kind.name()`). Tests added: `whole_file_ops_per_row`, `stash_pop_whole_file_journals_its_own_row`; extended `deleted_by_them_o_keeps`, `deleted_by_them_t_deletes`, `B_says_no_base`, `reread_drops_base`, `d_starts_over_behind_a_box`, `stash_pop_conflict_resolves_without_a_banner`. `graft-ui` `hints::tests::no_planned_hint_is_ever_drawn` has an in-test positive control (`only_live` rejects a list with `planned("y", ..)`); `planned()`'s `expect(dead_code)` is now `cfg_attr(not(test), ..)`. The control proves the check rejects; it does not exercise `drawable`'s filter (no seam to inject a `Planned` row into `context_list`).

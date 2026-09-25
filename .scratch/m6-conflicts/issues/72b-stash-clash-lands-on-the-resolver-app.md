# stash-clash-lands-on-the-resolver-app: a clash opens the resolver, says which stash, and done drops a popped one

Status: done
Blocked by: 72
From: pick 15 (`.scratch/m6-conflicts/user-picks-2026-09-24.md` :24); review N3 (`docs/reviews/m6-usability-final.md`
:125-144). Split from 72 on scout 72's advice.
Build with: **Opus** — app state shape (a clash beside `Stop`), routing across operate, resolver, stop line, relaunch.
Scout: `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md` (done; covers this half, :8-13).
Needs: 72 — `PutBack`, `PutBackWhy`, `Clash`, `ApplyError::Clashed`, `StashClash`, `live_clash`,
`Operation::DropPutBack`; 72's NOTES (the interim arm's file:line, which this ticket replaces).
Crates: graft (`update/operate.rs`, `update/state.rs`, `update/resolver/{resolved,names}.rs`, `update/banner` stop
wordings, `update/relaunch.rs`, `update/stash.rs`, `update/journal.rs`, view rows, tests, walks).
Group: **E2**. Before 73.

## Read first
- `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md` :8-13, "Mismatches".
- `docs/design-notes/m6-stash-put-back.md` "Journeys" (stash rows), "Where each event lands him", "New UI".
- `docs/adr/0045-stash-put-back-is-recorded-and-one-entry.md` §2, §3, §6 ("Undo during a clash").
- `crates/graft/MAP.md` :76-89 (`RefBadges::stashes`, `is_stash`), :120 (`AppState::stopped`), :171 (`live_entries`),
  :218-229 (`operate`), :289-317 (`banner`, `stop_wordings`, `stop_line`), :321-335 (`resolver`, `OpenedFrom`,
  `open_resolver`), :429-449 (`journal`, `relaunch`, `stash`), :674-677 (`resolver_context`, `done_line`).
- 72's NOTES and MAP.md lines.

## Decided here (scout mismatches 2–4)
- **A clash is not a `Stop`.** `Stop::kind` is an `InProgress` (`update/state.rs:728`), and everything that reads
  `stopped` means "git has an operation part-way": banner, `c`/`a`, `live_operation`. A clash has no git state and no
  continue/abort. So it gets its **own field**, and no reader of `Stop` changes:
  `AppState::clash: Option<ClashState>`. Only `stop_line`/`stop_line_wordings` read both (a clash words the stop line when
  `stopped` is `None`).
- **Set and cleared** by one pure function, `refresh_clash`, run when a journal read, a working-tree read or a refs read
  lands: `live_clash(journal entries, in_progress, any unmerged, refs.stashes())`. The `Clashed` answer sets it straight
  away (`origin: ThisRun`) so the landing never waits for a read. A launch read that finds one sets `origin: LastRun`.
- **Landing** (mismatch 2): not `banner::land_in_resolver` (`operate.rs:842`, `OpenedFrom::Banner`: there is no banner).
  New `land_on_clash`: close the `S` overlay, open the changes pane, `open_resolver(first clashed path,
  OpenedFrom::Pane)`. `Esc` → the changes pane on `CONFLICTS`.
- **Names** (mismatch on `names.rs:39`): `side_names`' stash arm takes theirs from `state.clash`:
  `stashed · stash@{n}` when `index` is `Some`; `None` (today) for a shell pop.
- **Close** (`update/resolver/resolved.rs:141-155` `close_after_last`): with `state.clash` —
  `Pop`: launch `Operation::DropPutBack { stash }`, land on the changes pane; on its answer, status
  `every file done · stash@{0} dropped ✓ · u brings it back`. `Apply`: nothing runs; `every file done · stash@{0} kept ✓`.
  `SetAside`: as `Pop` (74 rewords it). No `state.clash` (shell pop): today's close, unchanged.
- **Notice** on landing: `stash@{0} clashed in 2 files · esc, u puts it all back` (`1 file`).
- **Stop line** with the resolver shut: `stash@{0} clashed · 1 file conflicted`; relaunch `stash@{0} clash from last run
  · 1 file conflicted`; the ladder drops the origin, then the count, then the name (`stop_wordings`' order).
- The interim arm from 72 is replaced by the `Clashed` arm: set `clash`, `land_on_clash`, notice.

## Scope
**In:** `ClashState`, `refresh_clash`, the `Clashed` arm, `land_on_clash`, names, close and drop, stop line, relaunch,
tests, walks, captures.
**Out:** ops (72); set-aside words and roles (74); unstaging after a resolved clash (KNOWN_GAPS).

## Signatures
```rust
// graft, update/state.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ClashState { pub clash: graft_ops::StashClash, pub origin: LiveOrigin }
// AppState gains: pub clash: Option<ClashState>,

// graft, update (module where the scout's readers meet; builder names the file in NOTES)
/// Recompute `AppState::clash` from what the app holds. Pure over state.
pub(crate) fn refresh_clash(state: &mut AppState);

// graft, update/resolver
/// Close `S`, open the changes pane, open the resolver on the first clashed path (`OpenedFrom::Pane`).
pub(crate) fn land_on_clash(state: &mut AppState, commands: &mut Vec<Command>, next_id: &mut dyn FnMut() -> JobId) -> bool;
// (the id-source parameter takes whatever `open_resolver` takes today)
```

## The failing tests first
- graft `update`: `a_clashing_pop_lands_on_the_resolver_on_the_first_clashed_file` (`S` closed, pane open,
  `OpenedFrom::Pane`); `esc_after_a_clash_lands_on_conflicts_in_the_changes_pane`;
  `the_stash_pane_is_named_by_its_index`; `the_last_file_done_after_a_pop_clash_drops_the_stash`;
  `after_an_apply_clash_the_stash_is_kept`; `a_shell_pop_clash_names_no_stash_and_drops_nothing` (today's close);
  `relaunch_with_a_clash_open_names_the_stash`; `the_stop_line_words_a_clash_at_80_and_120`;
  `a_clash_raises_no_banner_and_c_a_do_nothing_new`; `refresh_clash_follows_the_reads` (set on a journal read, cleared by
  a working-tree read with nothing unmerged).
- `every_key_the_bar_draws_is_answered` green with the clash landing.

## Touches (each entry, and how the builder checks it)
- **Readers of `Stop`** (`AppState::stopped`: `operate.rs:816`, `banner`, `relaunch`, `undo::operation_ended`,
  `live_operation`): **unchanged** by design. Check: `git diff` shows no `Stop` field change; their tests green.
- **Readers of `stop_line` / `stop_line_wordings`** (`view::rows::stopped_line`, `GraphView::stopped`): now see a
  clash. Check: the wordings test; `grep -rn "stop_line" crates/graft/src` listed in NOTES.
- **`live_operation` / `live_entries`**: a clash sets neither (no `c`/`a`). Check: the no-banner test.
- **Resolver close** (`resolved.rs:141-155`), `every_file_done_parts` (:295-305), `finish_owed` (`resolver.rs:221`;
  cleared `done.rs:115`, `answers.rs:224`, `status_read.rs:91`): Check: the done tests; a merge's close unchanged.
- **Stash overlay `S`**: closed on a clash; list re-read after the drop. Check: landing and drop tests.
- **Undo** (`esc u`): the pop's own entry; the drift box when a take was made since. Check: walk below.
- **Radar (66)** — *pin*: `is_stash` on a clash's unmerged tree (scout Q9, unchecked). Check: 66's walk
  `radar_bookkeeping::a_pull_rebase_marks_no_duplicates` green, and a selection test on the stash row during a clash
  (no evil probe).
- **Journal read cadence** — *pin*: graft MAP :430 says the journal is re-read on launch, after every write and on
  `Ctrl+R`; the scout saw it read only when `J` loads (`update/journal.rs:16,32,353`). The builder confirms with a test
  (`the_journal_is_read_after_a_write`); if false, **stop** and report — `refresh_clash` after an undo depends on it.
- **`Running::op`** — *pin*: 67 says it now holds every launched op; the `Clashed` arm reads the description from it.
  Check: a test that the notice names the popped index after a `Clashed` answer.
- **Hint bars**: `o yours  t stashed` (existing roles). Check: bar test at 80 and 120.
- **Tests on today's clash** (`graft/tests/it/resolver/stash_pop.rs:12,74`): updated to the new landing and close;
  NOTES lists each assertion that moved.
- **Captures** (scout :13): `docs/shots/m6/m6-07-all-resolved-*`, `fix-every-file-done-footer-120x40`, `fix-finish-*`,
  `m6-15-finish-row-after-esc-*`, `prototype-journeys.txt`, contact sheets — each `git diff --exit-code` or regenerated
  by its test, in NOTES. **Walks** `tests/it/m5_walk/{conflict_journeys,resolved_operations,resolving}.rs`,
  `resolver/file_list.rs`: green.
- **TESTING.md**: rows with `every file done` for a stash pop: listed in NOTES for 75.
- **Keymap doc** §2.6 `Enter`, §2.9 `p`/`⏎`: owed by the conductor.
- **MAP.md**: graft `ClashState`, `AppState::clash`, `refresh_clash`, `land_on_clash`, close, names, stop line.

## Acceptance (name the verifying test at tick time)
- [x] Every test above, red then green — `tests/it/stash_clash.rs` (all ten) and `update/clash/tests.rs`. Red observed
      by temporary mutation (restored, see NOTES) for: `refresh_clash_follows_the_reads`,
      `a_clash_found_by_the_launch_is_from_the_last_run`, `the_notice_names_the_popped_index_after_a_clashed_answer`,
      `a_clashing_pop_lands_on_the_resolver_on_the_first_clashed_file`,
      `esc_after_a_clash_lands_on_conflicts_in_the_changes_pane`, `the_stash_pane_is_named_by_its_index`,
      `the_last_file_done_after_a_pop_clash_drops_the_stash`, `after_an_apply_clash_the_stash_is_kept`,
      `relaunch_with_a_clash_open_names_the_stash`, `the_stop_line_words_a_clash_at_80_and_120`. **Not red by nature**
      (pins of what must not change): `a_shell_pop_clash_names_no_stash_and_drops_nothing`,
      `a_clash_raises_no_banner_and_c_a_do_nothing_new`, `selecting_the_stash_row_during_a_clash_probes_nothing`.
      `every_key_the_bar_draws_is_answered` green (full nextest).
- [x] The three pins: radar — `radar_bookkeeping::a_pull_rebase_marks_no_duplicates` green +
      `stash_clash::selecting_the_stash_row_during_a_clash_probes_nothing`; journal cadence — **true**,
      `update::clash::tests::the_journal_is_read_after_a_write` (both `Refresh::WorkingTree` and `Everything` emit
      `ReadJournal`); `Running::op` — `update::clash::tests::the_notice_names_the_popped_index_after_a_clashed_answer`
      (`StashPop { index: 2 }` → `stash@{2} clashed in 2 files …`).
- [x] Walk: `m6_stash_put_back::a_stash_pop_that_clashes_is_four_keys` — `S p o ⏎` = 4 keys; file `committed` (yours);
      stash list empty; done words; `u` → stash listed again, no box.
- [x] Walk: `m6_stash_put_back::a_stash_apply_that_clashes_is_four_keys` — `S ⏎ o ⏎` = 4, stash kept, `… kept` words.
- [x] Walk: `m6_stash_put_back::esc_u_puts_a_clashed_pop_back` — `esc u` (+`y` only if a box asks): tree `committed`,
      `status --porcelain` empty, nothing unmerged, stash list equal to before, `clash` None. ≤ 2 (+1).
- [x] Captures (rule 6; colour **unverified**, rule 5), written by the walks
      (`m6_stash_put_back::a_stash_pop_that_clashes_is_four_keys`, asserted by `check_shot`):
  - `docs/shots/m6/stash-clash-landing-120x40.txt`, `-80x24.txt` — **What right looks like:** no `S` overlay; the
    resolver on `notes.md`, panes `yours · main` and `stashed · stash@{0}`; notice
    `stash@{0} clashed in 1 file · esc, u puts it all back`; bar has `o yours`, `t stashed`; no git error text.
  - `docs/shots/m6/stash-clash-done-120x40.txt` — the changes pane, no `CONFLICTS` section; bottom row
    `every file done · stash@{0} dropped ✓ · u brings it back`.
- [x] MAP.md updated (doc, no test).

## NOTES (builder, 2026-09-25)
- **Compacted once** mid-ticket (context ran out after the unit tests and `stash_clash.rs`). Nothing left over; all
  deliverables below were done after it.
- **Scout used:** yes, the existing `.scratch/m6-conflicts/scouts/stash-clash-lands-on-the-resolver.md`; no new one.
- **`refresh_clash` lives in `crates/graft/src/update/clash.rs`** (new module) with `refresh_and_land`, `clashed`,
  and the words. Called from `journal::journal_event`, `update::working_tree_read`, `took_fact`'s `Refs(Ok)`.
- **Signature deviations:** `ClashState` is `pub`, not `pub(crate)` (a field of the public `AppState`; integration
  tests read it). `land_on_clash` takes `first: PathBuf` and `&mut impl FnMut() -> JobId` (as `open_resolver`).
  Extra state beside `clash`: `clash_facts_seen`, `clash_drop`, `clash_pane_owed` (all `pub(crate)`). The pane owe
  exists because the landing runs on the answer, before the dirty status read: `selection::reconcile` pulled the pane
  back to a commit. `clashed` takes `&Clash`. Visibility: `relaunch::land_on_the_work`, `stash::close` →
  `pub(in crate::update)`.
- **Tests are integration, not `update` unit tests**, for the nine named ones (`tests/it/stash_clash.rs`, real repo);
  `refresh_clash_follows_the_reads` is a unit test.
- **Red evidence:** by temporary mutation of `names.rs` (theirs `None`), `resolved.rs` (clash branch off),
  `open.rs` (pane owe off), `clash.rs` (landing off; relaunch landing off, isolated) — each restored from a
  scratchpad copy and the files grep-checked; final gates ran on the restored code.
- **Question (graft-ui, not changed): the clash stop line is not drawn at 80×24.** `graft-ui` `stopped_row` needs
  ≥12 cells; the graph hint bar leaves fewer, as for a merge stop with the banner unfocused. At 120 it draws
  `stash@{0} clashed · 1 file conflicted · …`. The 80 test asserts the ladder only. Unverified whether that is wanted.
- **Done capture:** at 120×40 the status row sits under the graph (stacked layout), not the frame's last row; and with
  yours taken the tree is clean, so the pane shows HEAD's commit (no changes list). Ticket said "bottom row"/"changes
  pane"; the capture's caption says what is drawn. Judge by eye.
- **Assertions that moved:**
  - `tests/it/m5_walk/resolved_operations.rs` `undo-a-resolution-in-a-stash-pop-conflict`: the interim status
    `pop stash@{0} left conflicts in 1 file` + `Esc Esc` + `Resolve` → lands in the resolver (S closed, notice);
    after `⏎` the stash is dropped (new mark `resolved`, list empty); a new first `u` brings the stash back (mark
    `drop undone`); the old two undos are now the second and third. TESTING.md row 13 rewritten to match.
  - `tests/it/stop_ladder_readers.rs`: `graft/src/update/clash.rs` added (it defines the clash ladder; not a reader).
  - `resolver/stash_pop.rs` tests: **none moved** — they are shell pops, green unchanged.
  - Three existing tests (`commit_editor/tests.rs:186`, `preview/tests.rs:984`, `remote/tests.rs:307`) built arrays of
    `AppState` that the new fields pushed over clippy's 16 KB `large_stack_arrays`; now `vec!` / an array of refs.
    Assertions unchanged.
  - `banner::resumed` and `operate::operated` each had one arm extracted (`resumed_incomplete`, `incomplete`) for
    `too_many_lines`; behaviour unchanged.
- **`stop_line` grep** (`grep -rn "stop_line" crates/graft/src`, non-test): `banner.rs:49` (re-export),
  `banner/stop.rs:320,334` (defs), `view/rows.rs:371` (`stopped_line`). `stop_wordings(`: `banner/stop.rs`,
  `update/clash.rs` (def).
- **Journal view (`J`)**, handed from 72: `view::journal::tests::a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop`
  — a `Committed` clashed pop reads `the index · the working tree · backup saved`, no `failed:`; the `DropPutBack`
  row `drop stash 0707070` is `← u`. A pin of today's view (no view code changed): green first run, not red.
- **TESTING.md rows with `every file done` for a stash pop (for 75):** row 12
  `stash-pop-conflict-resolves-without-a-banner` (shell pop, unchanged: `all conflicts resolved`/`every file done`);
  row 13 `undo-a-resolution-in-a-stash-pop-conflict` (rewritten here).
- **Touches checked:** captures in the spec (`m6-07-all-resolved-*`, `fix-every-file-done-footer-120x40`,
  `fix-finish-*`, `m6-15-finish-row-after-esc-*`, `prototype-journeys.txt`) — all asserted by `check_shot` in the
  full nextest run, green, `git status` shows no change to them. Walks `m5_walk::*`, `resolver/file_list.rs` green.
  Readers of `Stop` unchanged (no `Stop` field changed). Keymap doc §2.6/§2.9 owed by the conductor (not edited).

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, all
workspace-wide.

## Size
Medium–large (~60k): one state field, one pure refresh, one landing, one close branch, words, three walks.

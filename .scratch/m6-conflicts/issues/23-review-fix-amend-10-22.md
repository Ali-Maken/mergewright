# review-fix-amend-10-22: C1 false tick, M2 side names, M3 three over-long files, `--abort` blocked by theirs' name

Status: done
Blocked by: none (10, 11, 12, 22 merged)
Was: filed by the conductor 2026-09-24 from `docs/reviews/m6-amend-10-22.md` C1, M2, M3 and Minor 2 (KNOWN_GAPS row "Reading theirs' name can block `--abort`"). Review fixes, no new decisions.
Build with: **Opus** (medium). A git-read failure path plus three module splits.
Needs: none.
Crates: graft, graft-ui, graft-git.
**Blocks** `13-resolver-file-list.md` (13's scout reads the split files).

## Read first
- `docs/reviews/m6-amend-10-22.md`: C1, M2, M3, Minor 2 — each names its file and line.
- `crates/graft/MAP.md`, `crates/graft-ui/MAP.md`, `crates/graft-git/MAP.md`: the rows for the files below.
- `ARCHITECTURE.md` before moving modules; use the `codebase-design` skill for the splits.

## Scope
In:
1. **C1.** `crates/graft/src/view/resolver.rs:454-461`: draw the stash-pop case under `for columns in [80, 120]` too, and assert `stashed` on its own label row at 80. Then re-read ticket 10's Acceptance line 6 and make its words match the test.
2. **M2.** `update/resolver/restart.rs:39` and `update/resolver/walk.rs:63`: build the key hints from `side_names(state).roles`, as `whole_file.rs:109-112` does. Test: a rebase case per site asserting the status reads the rebase's side names, **and** a test that no resolver status line carries a bare `ours` / `theirs`.
   - Also fix the two stale doc comments at `graft-ui/src/graph_screen/hints.rs:958`, `:990` (KNOWN_GAPS row "Stale and misplaced doc comments", hints half only).
3. **M3.** Split, each to ≤ ~400 lines per file where a real seam exists (PROMPT.md 6.2), behaviour unchanged:
   - `graft-ui/src/conflict_resolver.rs` (1003). Review's suggested seam: `SideRoles`/`SideNames`/`BlockNumbers`/`number_width` → `conflict_resolver/names.rs`; per-row drawers → `conflict_resolver/draw.rs`.
   - `graft-git/src/conflict.rs` (1245).
   - `graft-ui/src/graph_screen/hints.rs` (1256).
   - Pure moves: no signature changes outside the crate, no test edits beyond `use` paths. Update each crate's MAP.md.
   - A file that has no seam to reach ~400 → say so in NOTES with its line count; don't force it.
4. **Minor 2 / gap row.** `incoming` (`graft-git/src/state.rs:197-225`) turns an unreadable `*_HEAD` commit into a failure of all of `in_progress_detail`, which fails the working-tree read (`graft/src/working_tree.rs:61`) and `live_entry` (`apply/resume.rs:90`); its `rev_list` passes `&()` (`:217`) so it can't be stopped.
   - **First, a failing test** that shows it: a fixture whose `MERGE_HEAD` (or `REBASE_HEAD`/`CHERRY_PICK_HEAD`) names an unreadable commit; assert the working-tree read / `in_progress_detail` still succeeds and `--abort` stays reachable. If the test passes on unchanged code, **stop and report** — the finding was read, not observed.
   - Fix at the cause: theirs' name is optional detail. Keep the operation's detail readable when theirs can't be named — e.g. `theirs` held as a `Result`/`Option` with the error carried to where the name is drawn (never swallowed, rule 3), or read in a separate view-only call. Your choice; say which and why in NOTES.
   - Make that `rev_list` stoppable if it stays.

Out: every other KNOWN_GAPS row; ticket 13's work.

## Acceptance (each tick names its test, at tick time)
- [x] C1: stash pop drawn at 80 and 120, `stashed` asserted on its own label row at 80 — test: graft `view::resolver::tests::one_name_per_side` (stash block now under `for columns in [80, 120]`; at 80 asserts a row that starts and ends with `stashed` and carries no `yours`; at 120 `│stashed`).
- [x] Ticket 10's Acceptance line 6 wording matches what the test does — textual: `.scratch/m6-conflicts/issues/10-resolver-line-numbers.md` line 96 (the `one_name_per_side` tick) reworded: every case at 80 and 120, cherry-pick's names spelled out, stash's 80/120 assertions named.
- [x] M2: `restart.rs` whole-file `d` status uses side names in a rebase — test: graft `update::resolver::tests::d_on_a_card_in_a_rebase_uses_the_side_names` (keys `o upstream  t yours`).
- [x] M2: `walk.rs` past-last-card status uses side names in a rebase — test: graft `update::resolver::tests::n_on_the_last_card_in_a_rebase_uses_the_side_names`.
- [x] M2: no resolver status carries a bare `ours`/`theirs` — test: graft `update::resolver::tests::no_resolver_status_carries_a_bare_ours_or_theirs` (source scan of every `update/resolver` file's non-test literals; mutation-checked: restoring the old `"o ours  t theirs"` at `restart.rs:39` fails it, naming `restart.rs:39`).
- [x] M3: the three files split, line counts after in NOTES; MAP.md updated — gates green (nextest 2761/2761); files: `graft-ui/src/conflict_resolver{.rs,/names.rs,/layout.rs,/draw.rs}`, `graft-git/src/conflict{.rs,/parser.rs,/document.rs,/disk.rs,/base.rs}`, `graft-ui/src/graph_screen/hints{.rs,/context.rs,/fitting.rs,/lists.rs,/catalogue.rs}`; MAP rows in `crates/graft-ui/MAP.md`, `crates/graft-git/MAP.md`, `crates/graft/MAP.md` (`side_keys`, `side_names`).
- [x] Minor 2: failing test first, red on unchanged code — test: graft-git `state::tests::an_unreadable_incoming_commit_still_reads_the_operation` (red before the fix: `Io(... fatal: bad object 1111…)`) and graft `working_tree::tests::an_unreadable_merge_head_still_reads_the_working_tree` (red before the fix at its `read(..).expect`).
- [x] Minor 2: unreadable theirs no longer fails `in_progress_detail`/working-tree read; `--abort` reachable; error not swallowed — test: the two above (green; both assert `theirs` is `Err` naming `MERGE_HEAD`), graft-git `state::tests::a_stopped_incoming_read_is_carried_on_theirs` (stop reaches the `rev-list`), graft `view::resolver::tests::an_unreadable_theirs_is_named_as_such` (drawn `theirs · could not read MERGE_HEAD`). "`--abort` reachable" is verified as far as: the working-tree read succeeds with `in_progress = Some(Merge)`, which is what the banner and its `a abort` draw from. **Not verified:** running `git merge --abort` itself against a bogus `MERGE_HEAD`.
- [x] Gates workspace-wide: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` (2761 passed, 1 skipped), `cargo test --doc --workspace` (all ok).

## NOTES
- **No scout**: the review named every file and line; read those directly.
- **C1** — cause (the test never drew stash at 80). The 80-column path already drew right (review saw the same); now pinned.
- **M2** — cause. One helper, `update::resolver::names::side_keys(state)`, builds `o <ours role>  t <theirs role>`; `whole_file.rs`, `restart.rs`, `walk.rs` all call it. `n_on_the_last_card_says_no_more` changed its expected keys from `o ours  t theirs` to `o yours  t stashed`: no operation + resolver open is stash pop's roles — the old assertion encoded the bug. Hint doc comments (`hints.rs` `resolver_list`, `resolver_card_list`) reworded to `o <ours>  t <theirs>`, read as `SideRoles`.
- **M3 line counts after** (before: 1003 / 1245 / 1360):
  - `conflict_resolver.rs` 296, `/names.rs` 101, `/layout.rs` 245, `/draw.rs` 399.
  - `graft-git conflict.rs` 357, `/parser.rs` 234, `/document.rs` 153, `/disk.rs` 311, `/base.rs` 224.
  - `hints.rs` 436, `/context.rs` 172, `/fitting.rs` 241, `/lists.rs` 290, `/catalogue.rs` 250. `hints.rs` stays ~436: the rest is `Hint`/`Rung`/`short_form` (its long doc is the user's table) plus `hints_for`/`whole_bar`/`drawable`, one seam; splitting further would cut the filter from the function it serves.
  - Pure moves: public paths unchanged (re-exports); visibility widened only to `pub(super)` where a sibling or the parent calls (`draw_row`, `set_text`, `fit_list`, `context_list`, `TAIL`). Test edits: `use` lines only (`Modifier` in conflict_resolver tests; `SideRoles`, `SanitizedText` in hints tests), which reached them through the parent's imports before.
- **Minor 2 choice** — `InProgressDetail::theirs` is now `Result<Option<CommitSummary>, IncomingError>`: the detail still reads, the error rides on the field and is drawn where the name would be (`theirs · could not read MERGE_HEAD: …`). Chosen over a separate view-only call because theirs already travels on the detail to `side_names`; a second call would need a second `AppState` field and a second job answer for one label. `IncomingError` holds text (`file`, `reason`) so the detail keeps `Clone`/`Eq`. Cause fix. Cost to the user: the label shows git's error text instead of a name; the banner and `--abort` stay.
- **Stoppable**: new `in_progress_detail_with_stop(repo, stop)`; `graft::working_tree::read` passes its job's stop. `in_progress_detail(repo)` keeps its signature (it is `graft-ops`' call, outside this crate set) and passes `&()` — **`graft-ops::apply::resume::live_entry` (`resume.rs:90`) still makes an unstoppable `rev-list`**; switching it to `_with_stop` needs `graft-ops`. It no longer fails on an unreadable theirs, since the error is no longer `StateError`. Conductor: gap row or a one-line follow-up in `graft-ops`.
- `InProgressDetail` literals in `graft` tests: `theirs: None` → `theirs: Ok(None)` (15 sites, mechanical).
- Not done here (builders don't touch `KNOWN_GAPS.md`): close the "Reading theirs' name can block `--abort`" row and the hints half of "Stale and misplaced doc comments"; add/close the size row for these three files.

# fix-stop-opens-the-resolver: a stop with anything unmerged lands in the resolver on the first conflicted file; `Esc` goes to the banner

Status: ready (filed 2026-09-23; P1–P8 approved by the user, P9 by the conductor — see docs/field-notes.md)
Blocked by: 12-landing-after-a-stop.md, 13-resolver-file-list.md
Was: draft · **P5 approved** (user, 2026-09-23: "keep all 5"). The conductor rewrites the §2.6a sentence in `02-keymap.md` (the note's "Keymap rows owed").
Build with: blank — brownfield (ADR-0035): **scout first**. Expected **Sonnet**: two call sites of an existing function, fully specified.
Needs: `fix-landing-after-a-stop` (`Status::Stopped`, `stop_reason`, the banner the resolver's `Esc` returns to); `fix-resolver-file-list` (the list shown on landing).
Crates: graft.

## Read first
- `docs/design-notes/m6-conflicts.md`:
  - Amendment 1: Approved P5.
  - Amendment 2: Journeys J1, J2, J7, "Where each event lands him"; New UI "Below 180 columns".
- `docs/research/ux-principles.md` §3.1.
- `crates/graft/MAP.md` §`update::resolver` (`open_resolver`, `OpenedFrom`), §`update::operate` (`operated`), §`update::banner` (`resumed`, `first_conflicted`, `unmerged_paths`).
- **Scout question:** whether the `Conflicted` answer's `paths` arrive in git's status order (`unmerged_paths`'s), or only after the re-read.

## Scope
In:
1. On a `Conflicted` answer (`operate::operated`) and on a `Conflicted` again (`banner::resumed`) with at least one unmerged path:
   - After `Status::Stopped` and banner focus are set, call `open_resolver(first, OpenedFrom::Banner)`.
   - `first` is the first of the answer's paths in git's order (answers beat re-reads). If the answer's order is not git's, wait for the working-tree read and use `first_conflicted`. Say which in NOTES.
   - The file list's `▸` is on `first`.
2. `Esc` in the resolver → the banner is focused, with the Stopped line and its keys on the bottom row (today's `OpenedFrom::Banner` path).
3. **Unchanged:**
   - a stop with nothing unmerged (banner only);
   - a launch into a live operation (banner, Decision 2);
   - a stash-pop conflict (no banner; resolver from the pane, as today).
4. **Existing tests.** Update `fix-landing-after-a-stop`'s `a_stop_keeps_the_banner_keys_on_the_bottom_row` to press `Esc` first; its assertions are unchanged. Say so in NOTES.

Out: any new key.

## Acceptance (name the verifying test when you tick)
- [ ] `update::operate::tests::a_conflicted_merge_lands_in_the_resolver`: `m m` → the resolver is open on `first_conflicted`, `opened_from == Banner`, the cursor is on block 0, and `file_cursor` is that file's index.
- [ ] `update::banner::tests::a_rebase_stopping_again_lands_in_the_resolver`.
- [ ] `update::resolver::tests::esc_from_a_landed_resolver_shows_the_banner_keys`.
- [ ] `…::an_edit_stop_with_nothing_unmerged_keeps_the_banner`.
- [ ] Journey `a-stop-to-first-conflict` at 0 keys (acceptance ticket).
- [ ] Capture `docs/shots/m6/fix-stop-lands-in-resolver-120x40.txt`, the two-file fixture. **What right looks like:**
  - The frame right after the stop is the resolver, with the header still `MERGING`.
  - The strip lists both files, with `▸` on the first.
  - The identity row reads `▸ <that file> · …`.
  - The bar starts with the resolver's keys (`o yours`).
  - The user judges by eye.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Small. One context.

## NOTES (builder, 2026-09-24) — PARTIAL, gates fail

Scout used (`scout-18.md`), lines re-checked at `4a6703d`. No compaction.

**Done (code + unit tests, green):**
- `banner/stop.rs` `land_in_resolver(state, commands, paths, next_id)`: opens on `paths[0]` with `OpenedFrom::Banner`; no-op when `paths` empty or a resolver is already open (else `open_resolver` would word "already open" over the stop). Called after `Stopped`+focus in `operate::operated` (Conflicted arm) and `banner::resumed` (Conflicted arm).
- `first` = **the answer's paths** (not the re-read): both come from `status.entries` filtered `unmerged`, same order (`graft_ops apply/resume.rs unmerged_paths` vs `banner.rs unmerged_paths`).
- `file_cursor` before the list loads: `listed_files` empty → `unwrap_or(0)` (`resolver/files.rs file_cursor`) = 0 = the first file's row once it lands.
- Block 0: `cursor_block` is `None` until the opening read lands; `answers.rs conflict_arrived` sets it (existing path, not re-tested here).
- Stash-pop: cannot reach either arm — `Operated::Conflicted.kind` is `InProgress` (Merge/Rebase/CherryPick/Revert/Bisect), no stash variant; `update/stash.rs` has no conflict handling.
- Ticket 12's test lives in `update/banner/tests.rs`; `a_stop_keeps_the_banner_keys_on_the_bottom_row` now presses `Escape` first, assertions unchanged. Same for `esc_on_the_banner_leaves_a_way_back_on_screen` (one more `Escape`).
- New tests passing: `operate::tests::a_conflicted_merge_lands_in_the_resolver`, `operate::tests::an_edit_stop_with_nothing_unmerged_keeps_the_banner`, `banner::tests::a_rebase_stopping_again_lands_in_the_resolver`, `banner::tests::esc_from_a_landed_resolver_shows_the_banner_keys` (in `banner::tests`, not `resolver::tests`: needs `a_merge_stops`).

**Why partial:** ~30 existing tests assume a stop lands on the banner. Failing at this commit:
- unit: `banner::tests::the_stop_line_outlives_a_dismissed_problem`.
- `tests/it/conflict_interim.rs`: ~21 tests, most via `stopped_on_the_conflict` (:151) — plus `landing_shots::capture_a_merge_that_stops`, `capture_after_esc_on_the_banner`, `capture_banner_for_docs_shots` (their captures change meaning).
- `tests/it/m5_walk/*`: 8 scenarios (step DSL over many files; M5 journey step counts may grow by one `Esc`).
- `tests/it/resolver.rs` `file_list::a_rebase_lists_only_this_steps_files` (helpers :52/:64 loop on `banner_focus`).

**Exact next step:** decide per test: (a) banner-subject tests → helper presses `Esc` after the resolver's opening read lands (fix `stopped_on_the_conflict` once); (b) captures of "the stop" → now show the resolver (this ticket's capture); (c) m5_walk scenarios → add an `Esc` step, and rule whether M5 step counts may grow. Then the capture `docs/shots/m6/fix-stop-lands-in-resolver-120x40.txt` (+187x50).

Acceptance: none ticked — gates red.

# review-fix-amend-10-22: C1 false tick, M2 side names, M3 three over-long files, `--abort` blocked by theirs' name

Status: ready
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
- [ ] C1: stash pop drawn at 80 and 120, `stashed` asserted on its own label row at 80 — test:
- [ ] Ticket 10's Acceptance line 6 wording matches what the test does — (textual, name the line)
- [ ] M2: `restart.rs` whole-file `d` status uses side names in a rebase — test:
- [ ] M2: `walk.rs` past-last-card status uses side names in a rebase — test:
- [ ] M2: no resolver status carries a bare `ours`/`theirs` — test:
- [ ] M3: the three files split, line counts after in NOTES; MAP.md updated — (gates green; list files)
- [ ] Minor 2: failing test first, red on unchanged code — test:
- [ ] Minor 2: unreadable theirs no longer fails `in_progress_detail`/working-tree read; `--abort` reachable; error not swallowed — test:
- [ ] Gates workspace-wide: fmt, clippy `-D warnings`, nextest, doctests

## NOTES

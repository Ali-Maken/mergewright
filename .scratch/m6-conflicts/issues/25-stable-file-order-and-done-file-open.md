# fix-stable-file-order-and-done-file-open: a file keeps its row when it is done, and a done row opens read-only

Status: done (2026-09-24, built on Opus from `scout-25.md`; three deviations and one spec contradiction under NOTES, for a ruling). Was: draft (spec-only pass 2026-09-24 on ticket 13's NOTES D1 and D2; conductor's rulings: D1 stable order, D2 spec stands)
**Revised 2026-09-24 in place, from `.scratch/m6-conflicts/scout-25.md` (main `c56b64e`), C1–C7:**
- C1, C2: the route moves into graft's job, which already reads its own status. `open_listed` and ops' `Opening` are dropped; graft-ops gains `open_done` only.
- C3: `n`/`N` do not cross from a done file here. They are refused in words. Crossing from a done file is ticket 17's (list order).
- C4: the refused keys are the bound ones only. They go through one helper, which the tickets binding `Ctrl+O`/`Ctrl+T` and `O`/`T` call.
- C5: no `j/k` clause. `j`/`k` still scroll here.
- C6: the `WholeFile` literal is filled in.
- C7: a failed done-open keeps the resolver open.
Blocked by: none. 13 and 14 are merged.
Build with: **Opus** (scouted: `scout-25.md`). A read-only resolver mode across the job, the answers, keys, view and hints. Every rule is fixed here, and tests can judge it. Not a Fable ticket.
Needs: 13's `listed_files`, `select_file`, `RowState`; the existing `open_conflict`, `jobs::conflict::open`, `Product::ConflictOpened`, `open_failed`, `reopen`, `restart_landed`, `handed_back`, `HintContext::Resolver`.
Crates: graft-ops, graft, graft-ui.

## Why
- D1. Rows jump when a file becomes done. `ConflictSet.paths` lists the unmerged paths first (`graft-ops/src/conflict/set.rs:28-31`). `listed_files` appends in that order with no sort (`update/resolver/files.rs:77-81`). This is the "confusing which file" field note (2026-09-23).
- D2. The spec (13 Scope 6) says a `Done` row opens read-only. `select_file` refuses it instead (`files.rs:168-178`). A done path reaching `open_conflict` becomes `not_unmerged`, and `open_failed` then closes the resolver (`answers.rs:104-106`).

## Read first
- `.scratch/m6-conflicts/scout-25.md` (whole).
- `.scratch/m6-conflicts/issues/13-resolver-file-list.md`: Scope 2, 6, 7 and NOTES D1, D2.
- `docs/design-notes/m6-conflicts.md` Amendment 2: New UI "Below 180 columns" (:650–:675).
- `crates/graft-ops/MAP.md` §conflict (`open_conflict`, `OpenError`), §operation `RestartParams`; `crates/graft/MAP.md` §`update::resolver`; `crates/graft-ui/MAP.md` `conflict_resolver` (`RegionView::WholeFile`), `graph_screen::hints` (`HintContext::Resolver`, `every_context`).

## Decisions
- **D1 order.** The list is sorted in **git's path order**: byte-wise over `as_os_str().as_encoded_bytes()`. This is the order of git's index and of `status`. It is **not** `Path`'s `Ord`, which compares by component and so puts `a/b` before `a.b`.
  - Enforced **once, in `listed_files`**, over the whole union: the set's paths, the extra unmerged paths and the open path. Only it sees the extras.
  - `ConflictSet.paths` is **not changed**, so no other reader's order changes. This ticket needs no trace of those readers.
- **D2 document.** A done file shows the **working-tree file**, parsed. That is what his takes produced, what `e` opens, and what a reopened file shows today.
  - If the file on disk differs from what is staged, the identity row says so. This is read from the status, never guessed.
  - Nothing on disk (resolved by removal) → a card. A binary → a card.
- **D2 route.** `jobs::conflict::open` already reads its own status (`jobs/conflict.rs:112`). It picks the route from that read: `open_conflict` if the status calls the path unmerged, else `graft_ops::conflict::open_done`. This is a choice made on git's state at open time, not a retry after a refusal. `not_unmerged` stays for the race where a path is staged between the read and the open.
- **D2 keys on a done file.** One helper, `done_refusal`, answers `Some("<path> is done · d starts it over")`. `resolver_key` asks it first for `o` `t` `b` `⏎` `B` `M` `n` `N`, and refuses on the status line with no command pushed.
  - `d`, `e`, `Esc` and the scroll keys (`j`/`k` included) act as today.
  - `d` goes to today's box, then `RestartConflict`: `check_restart` admits a path that is not unmerged, and `prepare_restart` reads its stages from the chain. `restart_box`'s `whole_file` refusal (`restart.rs:32`) does not apply, because `whole_file` is `None` on a done file.
  - **Not bound here:** `Ctrl+O`/`Ctrl+T` (ticket 16) and `O`/`T` (P6). Whichever ticket binds them calls `done_refusal` first (see the notes for the conductor at the end).
- **D2 failure.** A done-open error (`Read`, `TooLarge`, `Markers`) keeps the resolver open on that path, as a done card `done ✓ · could not read: <words>`. A failure on the unmerged route is unchanged: `open_failed` closes, as today.

## Scope
In:
1. graft-ops: `DoneFile`, `open_done` (Signatures).
   - It refuses `Refused(Unmerged { paths: vec![path] })` when the status calls the path unmerged. That is a defensive check; the job never routes such a path there.
   - No file on disk → `Removed`.
   - A NUL in the first 8 KiB → `Binary`. Git's rule, as in `graft_git::conflict::read`.
   - Otherwise it parses the file (`graft_git::conflict::read`) → `Text`. `unstaged` is true when the status shows a working-tree change for the path.
2. graft job: `jobs::conflict::open` routes as D2 says and returns graft's `Opening`. `Product::ConflictOpened` carries `Result<Opening, OpenFailure>`. `OpenFailure` gains `done: bool`, true for an error on the done route.
3. `conflict_opened` / `open_failed` (`answers.rs:17`, `:97`):
   - `Opening::Unmerged` → today's path, and `done` is cleared.
   - `Opening::Done` → `ResolverState::done = Some(shape)`. `Text` sets `file`. `Binary`/`Removed` leave `file` and `whole_file` `None`.
   - `Err` with `done: true` → `done = Some(Unreadable { why })`, resolver kept open, `file` `None`.
   - `whole_file`'s invariant becomes: `Some` exactly when `file` is `None`, the read has landed and `done` is `None`. Check every reader the scout lists (`files.rs:115`, `walk.rs:26`, `restart.rs:32`, `whole_file.rs:72,104`, `take.rs:23-33`, `view/resolver.rs:33`, `view/rows.rs:487-495`).
4. `select_file`: drop 13's `Done` refusal (`files.rs:168-178`). The in-flight refusals (`switch_refusal`) stay.
5. Keys: `done_refusal` and its use in `resolver_key`, as D2 says.
6. Re-reads: `handed_back` (`answers.rs:201-226`) and `reopen` (used by `restart_landed`) push `OpenConflict` as today, so the route alone decides between `Done` and `Unmerged`. `reopen` builds a fresh state, so `done` starts `None`.
7. View (`view::resolver`):
   - Identity row: `▸ <path> · done ✓ · d starts it over`, plus ` · edited since done` when `unstaged`.
   - `Binary`, `Removed` and `Unreadable` draw `RegionView::WholeFile { how, keys: "d starts it over", lines: vec![], note: None, ours_absent: false, theirs_absent: false }`. `how` is `done ✓ · binary`, `done ✓ · removed` or `done ✓ · could not read: <why>`, sanitised. At ≥180 columns they draw as ticket 14's card rows do.
   - The row keeps `done ✓` in the list, with `▸` on it.
8. graft-ui: `HintContext::Resolver` gains `done: bool`. When it is true, the table is exactly `d start over`, `e edit`, `esc close`.
   - `resolver_context` sets `done` from `ResolverState::done`.
   - Add the new form to `every_context` (`catalogue.rs:115-140`) and to `coverage_slot` (`graph_screen/tests.rs:1945`).
9. D1: `listed_files` sorts its union in git's path order. Row states, `file_cursor` and `resolved_advance` are unchanged; they read the sorted list.
10. MAP.md:
    - graft-ops §conflict.
    - graft §`update::resolver`: `:314` "so a done file moves after the open ones" and "refused … for a `Done` row" become false.
    - graft `jobs::conflict` / `Product::ConflictOpened`.
    - graft-ui `HintContext::Resolver`.

Out:
- `n` crossing from a done file, and crossing in list order at all (walk uses the unmerged order today, `walk.rs:90-95`) → ticket 17.
- Binding `Ctrl+O`/`Ctrl+T`/`O`/`T`.
- A done file whose text still has markers, staged from the shell: shown as parsed, read-only, no special words. Gap row.
- The unmerged route's `open_failed` closing, `ConflictSet.paths`' own order, a stash pop's done file dropping out at re-read (13's gap row), showing the index blob in place of the disk file.

## Signatures
```rust
// graft-ops/src/conflict/done.rs (new; re-exported from `conflict`)
pub enum DoneFile {
    Text { file: graft_git::conflict::ConflictFile, unstaged: bool },
    Binary { unstaged: bool },
    Removed,
}
pub fn open_done(repo: &RepoHandle, status: &WorkingTreeStatus, path: &Path) -> Result<DoneFile, OpenError>;
// open_conflict, OpenError: unchanged

// graft/src/jobs/conflict.rs
pub enum Opening { Unmerged(Arc<OpenedFile>), Done(Arc<DoneFile>) }   // DoneFile = graft_ops::conflict::DoneFile
pub struct OpenFailure { /* existing: words, not_unmerged */ pub done: bool }
// update/events.rs: Product::ConflictOpened(Result<Opening, OpenFailure>)   (was Result<Arc<OpenedFile>, OpenFailure>)

// graft/src/update/resolver
pub struct ResolverState { /* existing */ pub done: Option<DoneShape> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DoneShape { Text { unstaged: bool }, Binary { unstaged: bool }, Removed, Unreadable { why: String } }
pub fn done_refusal(state: &AppState) -> Option<String>;   // Some while the open file is done; the one site for these words

// graft-ui/src/graph_screen/hints/context.rs
pub enum HintContext { /* … */ Resolver { base: bool, restart: bool, whole_file: bool, done: bool, roles: SideRoles } }
```

## Acceptance (name the verifying test when you tick)
- [x] graft-ops `conflict::tests::open_done_reads_the_file_on_disk` — **on `both.txt`**, not `notes.txt`: the StoppedMerge fixture has no `notes.txt` (deviation 1). Throwaway merge, `notes.txt` resolved and `git add`ed → `Text`, `file.open() == 0`, `unstaged == false`. Edited after → `unstaged == true`.
- [x] graft-ops `conflict::tests::open_done_on_a_removed_path_is_removed` (`ours-deleted.txt`, `git rm`) and `conflict::tests::open_done_on_a_binary_is_binary` (`pic.bin`). (`DU` resolved by `git rm`) and `…::open_done_on_a_binary_is_binary`.
- [x] graft-ops `conflict::tests::open_done_refuses_an_unmerged_path` (`three.txt`) → `Refused(Unmerged { paths: [p] })`.
- [x] graft `jobs::conflict::tests::open_routes_on_the_status_it_reads` (`both.txt`), real git: the same path gives `Unmerged` before `git add` and `Done(Text)` after.
- [x] graft `update::resolver::tests::file_list::the_list_is_in_gits_byte_order`: `a/b`, `a.txt`, `B.txt`, `a.b` → `B.txt`, `a.b`, `a.txt`, `a/b`.
- [x] graft `update::resolver::tests::file_list::a_file_keeps_its_row_when_it_is_done`: three files. The middle one becomes `Done`, and a set re-read in ops' order (unmerged first) lands. The row order and `file_cursor` are unchanged.
- [x] graft `update::resolver::tests::file_list::extra_paths_sort_into_place`: a `Ready` set missing an unmerged path, and an open path missing from both. Each sits at its sorted index.
- [x] graft `update::resolver::tests::file_list::a_click_on_a_done_file_opens_it_read_only`: pushes `OpenConflict`. The `Done(Text)` answer → resolver open, `done == Some(Text { .. })`, identity row `▸ gone.txt · done ✓ · d starts it over`. With `unstaged`, it ends `· edited since done`.
- [x] graft `update::resolver::tests::file_list::a_done_card_for_removed_binary_and_unreadable`: `Removed` / `Binary` / `Err { done: true }` answers draw `done ✓ · removed` / `done ✓ · binary` / `done ✓ · could not read: …`, and the resolver stays open in each.
- [x] graft `update::resolver::tests::file_list::an_unmerged_open_failure_still_closes`: `Err { done: false }` → resolver closed, today's words. This pins the unchanged path.
- [x] graft `update::resolver::tests::file_list::a_done_file_refuses_in_words`: each of `o t b ⏎ B M n N` → status `gone.txt is done · d starts it over`, no command pushed. `j` still scrolls.
- [x] graft `update::resolver::tests::file_list::d_on_a_done_file_starts_it_over`: `d` → today's restart box. Accepting it pushes `RestartConflict`. `restart_landed` → `OpenConflict`. An `Unmerged` answer → `done == None`, open blocks shown.
- [x] graft `update::resolver::tests::file_list::a_done_file_handed_back_stays_open`: `e` → hand-back → a `Done` answer. The resolver is open, and `done` is `Some`.
- [x] graft-ui `graph_screen::hints::tests::a_done_file_offers_start_over_edit_and_close` and `graph_screen::tests::every_context_reaches_every_shape`: `Resolver { done: true, .. }` → exactly `d`, `e`, `esc`. `every_context_reaches_every_shape` passes.
- [x] graft `it::resolver::file_list::a_done_file_stays_listed`, **back to the spec's order**. Two-file merge, `o` on `gone.txt` → the list reads `gone.txt · done ✓    ▸ notes.txt …`, with `gone.txt` in the same cell as before the `o`.
- [x] graft `it::resolver::file_list::a_done_file_opens_read_only_and_d_starts_it_over` (first bullet as written; second bullet **adapted**, see NOTES "spec contradiction") and `it::resolver::file_list::a_done_marker_file_starts_over_in_place` (the second bullet's journey on a marker file), real git:
  - After `o` on `gone.txt`, click its row. The resolver stays open, the identity row reads `▸ gone.txt · done ✓ · d starts it over`, and the row order is unchanged.
  - `d`, then accept the box. `git status` calls `gone.txt` unmerged again. Once the status lands, its row reads `keep or delete`, in the same cell. **Not reachable:** ops refuses this restart. The test pins the refusal, and the journey is proven on `shared.rs` in an app-begun merge: `UU` again, `done == None`, same row order.
- [x] Captures, pinned by `it::resolver::file_list::file_list_capture` and `it::resolver::file_list::done_open_capture` (both also take 187x50): re-take `docs/shots/m6/fix-file-list-{120x40,160x48,80x24,200x50}.txt` and take new `fix-file-list-done-open-{120x40,200x50}.txt`. **The look is the user's call (rule 6); colour is unverified (rule 5).** What right looks like:
  - 120x40: `files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open`.
  - 80x24: `files   ✓ gone.txt   ▸ notes.txt 2/2 open`.
  - 200x50: in the column, `gone.txt   done ✓` sits above `▸ notes.txt  2 of 2 open`.
  - done-open 120x40: `files   ▸ gone.txt · done ✓    notes.txt · 2 of 2 open`, the rule, then `▸ gone.txt · done ✓ · d starts it over`, the file's plain lines, and a bar reading `d start over  e edit  esc close`.

## Gates
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo test --doc`, workspace-wide.

## Size
Medium: one graft-ops function, one job route and payload, one read-only resolver mode (keys, view, hints, failure), one sort, ~17 tests, 6 captures. One context. If it will not fit, split at the crate seam: graft-ops first (the first 3 boxes), then graft and graft-ui.

## NOTES (builder, 2026-09-24)
**Compacted once mid-ticket.** Nothing remains. All work after the compaction was gates, test fixes, the three deviations below, MAP.md and this note.
Scout used: yes, `scout-25.md`.

**Cause fixes** (each fixes the defect's cause, at no cost to the user):
- D1: `listed_files` sorts its union byte-wise (`update/resolver/files.rs`, `git_order`). `ConflictSet.paths` is unchanged.
- D2: the route is chosen in `jobs::conflict::open` from its own status read. `select_file`'s Done refusal is gone.
- Found while building `d` on a done file: a refused or failed resolver write never released `staging`/`restarting`. `update/mod.rs` `operate_answer` handled only `staged`. After a refused restart, every file switch was then refused ("still writing"). New `update::resolver::write_not_landed` clears them when the id matches. This is pinned by `a_done_file_opens_read_only_and_d_starts_it_over` (`restarting == None` after the refusal).

**Deviations (for a ruling):**
1. Ops tests use `both.txt`, not `notes.txt`: the StoppedMerge fixture has no `notes.txt`.
2. `ours_absent`/`theirs_absent` are `None`, not `false`: the fields are `Option<SanitizedText>` (`graft-ui` `RegionView::WholeFile`).
3. **Resolved outside still closes.** Scope 6, taken literally, lands the re-read after `$EDITOR`/`M` resolves and stages a conflicted file on the done card. That breaks TESTING.md:2194 (M5 mergetool check, "resolved outside, the resolver closes") and three existing tests: `it::resolver::handover::resolved_outside_closes`, `it::m5_walk::every_scenario::mergetool_hand_over_and_back`, `it::resolver::last_file::open_refused_…`. Kept the old rule at the new seam: a `Done` answer while an **unmerged document** is on screen (`open.file.is_some() && open.done.is_none()`, the condition `open_failed` used) closes with `<path> resolved outside` (`update/resolver/answers.rs` `conflict_opened`). A done row opened from the list, or a done file re-read (`a_done_file_handed_back_stays_open`), still shows the card. New test: `update::resolver::tests::file_list::an_unmerged_document_resolved_outside_still_closes`. The two existing handover/mergetool tests pass unchanged.
   - `it::resolver::last_file::open_refused_on_a_path_git_does_not_call_unmerged` is renamed `open_on_a_path_git_no_longer_calls_unmerged_shows_it_done`. A fresh open from a stale banner list is D2's route by design: it now opens the done card, where it used to close with `… is not conflicted`. `open_failed`'s `not_unmerged` branch now answers only the race between the job's status read and its open, and no test reaches it.
4. **A done file draws as one card at every width.** Scope 7 is silent on a done Text at ≥180 columns. The three panes drew the file on disk under `yours · main`, `result` **and** `theirs · feature` alike. For `gone.txt`, theirs deleted the file, so that label was false. New `view::resolver::arrangement(state, placed)` returns `Card` for a done file and is used by both the draw (`view/frame/conflicts.rs`) and `resolver_layout` (the mouse). The list keeps its column. Pinned by `done_open_capture`: no `theirs · feature` at any width.

**Spec contradiction (for a ruling):** the acceptance bullet "`d` on `gone.txt` → unmerged again, row reads `keep or delete`" is unreachable. `gone.txt` is a modify/delete in a shell-begun merge. `graft_ops` refuses its restart as `NoStagesKept` (`graft-ops/src/apply/restart.rs:178`, `:201`), and as `NotMarkerShape` for a whole-file shape (`:194`). A shell-begun merge has no backup chain holding stages, so no done file in it can start over, `notes.txt` included. The test pins the refusal (`gone.txt: the conflict's sides were not kept`) and that the write is released. `a_done_marker_file_starts_over_in_place` proves the journey on `shared.rs` in an app-begun merge. The user will meet this refusal on any shell-begun merge, which is worth a line in ticket 17's or the design note's journeys.

**Gaps:**
- Broken: none found.
- Unverified:
  - colour of the done row, card and identity row (rule 5);
  - the look of every capture (rule 6, the user's call);
  - a done file whose text still has markers, staged from the shell: shown parsed, read-only, no special words (Out, as specified; gap row for the conductor);
  - a `Refused(Unmerged)` race on the done route (staged → unmerged between the status read and `open_done`): the card shows ops' shell-milestone words. Not reachable by test.
- Contact sheet: `docs/shots/m6/m6-contact-sheet.txt` has never carried the `fix-file-list-*` captures (13's included), so I did not add them. The conductor decides whether the sheet should.

**Captures** (`docs/shots/m6/`, each under 2 KB). What right looks like:
- `fix-file-list-120x40.txt`: `files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open`.
- `fix-file-list-80x24.txt`: `files   ✓ gone.txt   ▸ notes.txt 2/2 open`.
- `fix-file-list-160x48.txt`: as 120x40, wider.
- `fix-file-list-200x50.txt` and `fix-file-list-187x50.txt`: in the left column, `gone.txt   done ✓` above `▸ notes.txt  2 of 2 open`.
- `fix-file-list-done-open-120x40.txt`: `files   ▸ gone.txt · done ✓    notes.txt · 2 of 2 open`, the rule, `▸ gone.txt · done ✓ · d starts it over`, `1 our plan`, bar `d start over  e edit  esc close`.
- `fix-file-list-done-open-200x50.txt` and `fix-file-list-done-open-187x50.txt`: the column with `▸ gone.txt   done ✓` above `notes.txt  2 of 2 open`; beside it one card, never three panes: `▸ gone.txt · done ✓ · d starts it over`, then `1 our plan`; same bar.

**Verified by:** unit and it tests named above, and by reading each capture file. Not verified: real-terminal use, colour.

## For the conductor (lines in other specs, not this ticket's to edit)
- Ticket 16 (`Ctrl+O`/`Ctrl+T`) and P6's `O`/`T` ticket: "on a done file, refuse through `update::resolver::done_refusal` first".
- Ticket 17 (P7): "moving between files, `n` crossing included, walks `listed_files` order and starts from a done file too". Today `next_conflicted_file` walks unmerged order and finds nothing from a done path (`walk.rs:90-95`).

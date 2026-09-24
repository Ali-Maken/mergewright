# Review: m6-conflicts ticket 25 (stable file order; a done file opens read-only)

Ticket `.scratch/m6-conflicts/issues/25-stable-file-order-and-done-file-open.md`: commits `92f62c5`..`70a3e00`, merged `798a176`. Diff `7d9a7ef..798a176`, code only. Read-only. Reviewer: Opus, 2026-09-24.

**Count: 0 Critical · 3 Major · 8 Minor.**

Rulings already made (not re-reported; `docs/log/m6.md:217`): `d` on a done file of a shell-begun merge is refused (gap row M6/25 → ticket 20); the "resolved outside" close is kept after `e`/`M` on a document; a stale-list open lands on the done card; a done file is always one card. Major 1 covers only the part of `d` those rulings do not.

## Gates

All ran one at a time on `main` at `c191a91`. Its `crates/` are identical to `798a176` (`git diff --stat 798a176 c191a91` touches only `KNOWN_GAPS.md` and `docs/log/m6.md`).

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2830 passed, 1 skipped. **104.5 s** by nextest's summary (118 s wall). A second, timed run: 115.9 s, peak RSS 274 MB (the largest process, `time -l`). |
| `cargo test --doc --workspace` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 2865 passed, 0 failed, 1 ignored. **325 s** wall. Max RSS 1.07 GB. |
| Release perf (`frames::` + `heavy::budgets_hold`, `--test-threads=1`) | **13 passed**, 73.1 s (92 s wall with the build). Frames p99 7.4–8.9 ms (max 12.5 ms, Pixel 50k refs). First paint 13.7–38.0 ms. |
| Fuzz, benchmarks | Not run. `fuzz/` targets graft-git's parser and the benches measure graph rows. This ticket touches neither: `open_done` calls the existing `graft_git::conflict::read`. |

`target/` is 18 GB.

The five slowest fast-suite tests (none from this ticket):
- `graft-ops::it conflict_ops::whole_file_shapes_resolve_like_git`: 12.7 s
- `graft-git::it diff_fixture::parse_unified_matches_numstat_for_every_shape`: 11.2 s
- `graft-ops::it undo_cancel::cancel_at_every_step_boundary_is_clean`: 10.4 s
- `graft-graph layout::stream::tests::scrolling_the_whole_history_keeps_the_resident_set_bounded`: 7.8 s
- `graft::it m5_walk::every_scenario::merge_resolved_with_the_keys`: 7.4 s

## Ticks

**17 checked, 17 held.** I read every named test against its tick. One is adapted by ruling: the `gone.txt` half of the `d` bullet is pinned as a refusal, and the journey is proven on `shared.rs` in an app-begun merge.

- Ops: `open_done_reads_the_file_on_disk` (on `both.txt`), `…_removed_path_is_removed`, `…_binary_is_binary`, `…_refuses_an_unmerged_path`. Held.
- Job: `open_routes_on_the_status_it_reads`. Held: real git, `Unmerged` before `git add`, `Done(Text { unstaged: false })` after.
- Unit, `update::resolver::tests::file_list`: `the_list_is_in_gits_byte_order`, `a_file_keeps_its_row_when_it_is_done`, `extra_paths_sort_into_place`, `a_click_on_a_done_file_opens_it_read_only`, `a_done_card_for_removed_binary_and_unreadable`, `an_unmerged_open_failure_still_closes`, `a_done_file_refuses_in_words` (all 8 keys, no command pushed, `j` scrolls), `d_on_a_done_file_starts_it_over`, `a_done_file_handed_back_stays_open`. Held.
- Hints: `a_done_file_offers_start_over_edit_and_close`, and `every_context_reaches_every_shape` with slot 30. Held.
- It: `a_done_file_stays_listed` (same cell, measured before and after `o`), `a_done_file_opens_read_only_and_d_starts_it_over`, `a_done_marker_file_starts_over_in_place` (`UU` again, `done == None`, same order). Held.
- Captures: `file_list_capture` and `done_open_capture`. The eight files match every "what right looks like" line (textual check only; colour and look are unverified, rules 5 and 6). All files are under 3 KB.

## Checks that held

- **The fix the builder was not asked for** (`update/mod.rs:467-472`, `answers.rs:333` `write_not_landed`). It fixes the cause. The fields are cleared only when the id matches, and only on the `!staged` branch, so a landed write still goes through `resolved_advance`/`restart_landed`. It costs the user nothing. Before it, a refused `⏎` stage or `d` restart left `staging`/`restarting` set, and `switch_refusal` (`files.rs:211`) then refused every later file switch with "a write is already running". I confirmed this by reading the code. The `restarting` half is pinned (`it/resolver/file_list.rs:229`). The `staging` half is not tested (Minor 4).
- **Rule 3.** `open_done` (`graft-ops/src/conflict/done.rs:48-68`) maps every `ReadError` arm. Only `NotFound` becomes `Removed`; every other io error is `Read`. A failed read on the done route becomes the card `done ✓ · could not read: <why>` (`graft/…/resolver/done.rs:74`), sanitised at `view/resolver.rs:127`. The non-test diff adds no `unwrap`, `expect`, `let _` or `unwrap_or_default`.
- **Sanitising.** The card's `how` and `keys` and the title go through `sanitize_line`. `done_refusal`'s path reaches the status line, which `view/status.rs:44` sanitises.
- **The UI thread.** The route is picked inside the job (`jobs/conflict.rs:83-92`). `update` does no git work.
- **D1.** The sort is done once, in `listed_files` (`files.rs:86`), with `git_order` over `as_encoded_bytes`. `ConflictSet.paths` is unchanged.
- **Keys.** `act` (`update/resolver.rs:277-287`) refuses exactly `o t b B M n N ⏎`, plain ones only. `d`, `e`, `Esc` and scrolling still act. The mouse path has no take action, so there is nothing to refuse there.
- **Crate rules.** graft-ops adds `DoneFile`/`open_done` only. graft-ui gains the `done` flag. No new dependency.

## Critical

None.

## Major

### M1. A done file offers `d starts it over` and its box promises markers where graft-ops always refuses: the m6-05 Major 1 defect again, on the done route

- **Where:** `update/resolver/restart.rs:32` (the pre-box refusal checks only `open.whole_file`), `update/resolver/done.rs:97` (`settle` sets `whole_file = None` on every done file), `restart.rs:62` (the box says "the conflict comes back with all its markers"), `graft-ops/src/apply/restart.rs:178`, `:194`, `:201` (the refusals after confirm).
- **What:** every done file draws `d starts it over` in its identity row and `d start over` in its bar. `d` always opens the box. graft-ops then refuses after the user confirms in two cases:
  - (a) Any shell-begun merge, `NoStagesKept`. This is ruled and has a gap row. The test shows the box opening first (`it/resolver/file_list.rs:196`) and the refusal after.
  - (b) **Not ruled:** a done file whose conflict was a whole-file shape, in an app-begun merge. For example `gone.txt` (deleted by them) kept with `o` in `app_merge_with_a_whole_file_conflict`. Here `stage_source` finds the backup, `shape_of_stages` answers `UD`, and `:194` refuses `NotMarkerShape`. I read this path; no test reaches it.
- **Why it matters:** `restart_box`'s own doc comment (`restart.rs:20-22`) records the rule this breaks: "Refused in words, before the box, on a whole-file card — `ops` restarts only a shape with markers to bring back, and the box would promise them (m6-05 review, Major 1)". The done route bypasses that check because `whole_file` is `None`. So the key is advertised, the box makes a false promise, and a refusal follows. For (a) that happens on every done file of every shell-begun merge, which is J3's everyday case.
- **Fix:** answer "can this file start over?" at open time, off the UI thread. For example, `open_done` could return `restart: Result<(), Refusal>` using graft-ops' own `stage_source` + `shape_of_stages` (read-only). When it is `Err`, drop `d` from the identity row and the bar, and have `d` refuse in words before the box, as `restart.rs:32` does for a card. Fixing (b) needs no new ruling. Whether (a) should stop advertising `d` is the conductor's call: it amends ruling 1's presentation, not its outcome.

### M2. Pipeline: the read-only done open and "start it over from the list" have no journey and no Prior-art row

- **Where:** `docs/design-notes/m6-conflicts.md`: the Prior art table at `:427-437` and the Journeys at `:439-466`. The only mention is the design text at `:700-703`.
- **What:** this ticket builds a new interaction: click a done row, read it, and optionally `d` → confirm → the conflict is back. No Journey counts its steps, and no Prior-art row decides it. Research exists: `docs/research/conflict-resolution.md:207` and `:287` cite Tower's "reset a merge conflict to its unmerged state, even after it has been resolved" and recommend taking it. It was never carried into the note's table as take/reject/invent, nor counted against Tower. The acceptance walk counts no steps for this journey, because the note has no count to compare with.
- **Why it matters:** CLAUDE.md "The feature pipeline" steps 3, 4 and 7. The reviewer checks built journeys' step counts against the note. With no count, a regression in steps cannot fail a test. The journey is also where M1's "box then refusal" would have been counted as wasted steps.
- **Fix:** add a Prior-art row ("Undo a `⏎` after the fact: Tower reset-to-unmerged. **T**") and a journey, for example *J11 walk back a mistaken `⏎`*: click the done row · `d` · `→` · `⏎` = **4**, landing on the file with its markers and the row in place, next to Tower's count. Then have `a_done_marker_file_starts_over_in_place` assert the count.

### M3. File size: `view/resolver.rs` grew 74 lines on a file already flagged, and `update/resolver.rs` crossed 400

- **Where:** `crates/graft/src/view/resolver.rs`: 727 → 801 lines (non-test 421 → 494: `done_card`, `arrangement`, `Identity::Done`). `crates/graft/src/update/resolver.rs`: non-test 391 → 412 (`act`'s done gate). `view/rows.rs` 683 → 696 is already covered by gap row M6/11+12.
- **What / why:** PROMPT.md 6.2 (~400 lines a file). Review 14's M1 already named `view/resolver.rs` at 727, and ticket 27 was filed to split it. This ticket adds to what 27 must move.
- **Fix:** fold these into ticket 27's scope. `done_card` + `Identity::Done` + `arrangement` belong together in a `view/resolver/done.rs`, mirroring `update/resolver/done.rs`. `act`'s done gate can move behind `done.rs` as `refuses(key) -> bool`, so `resolver.rs` drops back under 400.

## Minor

1. **The design note and ADR-0043 disagree with the build (textual).**
   - `m6-conflicts.md:703` gives the done bar as `d start over  n next  N prev  e edit  esc close`. The build and ticket C3 give `d start over  e edit  esc close`.
   - `:571` has `DoneShape` deriving `Copy` with no `Unreadable`. The built type has `Unreadable { why: String }` and is not `Copy`.
   - `:575` has `Opening::Unmerged(Opened)` / `Done(DoneFile)`. The built type is `Arc<OpenedFile>` / `Arc<DoneFile>`.
   - Neither ADR-0043 §2 ("three panes at 180 and wider") nor note `:700` says a done file is one card at every width (ruling 4).
   - Fix: edit the three note lines, and add one sentence to ADR-0043 §2 or the note.
2. **`Removed` drops `unstaged`** (`graft-ops/src/conflict/done.rs:62-63`; `view/resolver.rs:114` reads `unstaged` from `Binary` only). Take a file that was `git add`ed and then deleted on disk. The index still holds it, but the card reads `done ✓ · removed` with no `· edited since done`. That contradicts D2: "if the file on disk differs from what is staged, the identity row says so". The ticket's own signature has the same flaw. Fix: `Removed { unstaged }`, set from the status's worktree deletion.
3. **Question: a whole-file card resolved outside no longer closes.** `answers.rs:36` closes as "resolved outside" only when `open.file.is_some()`. A card has `file: None`, so `M` resolving a modify/delete now lands on the done card, while a document in the same situation closes. Before 25, a card closed with a sticky `… is not conflicted`. Ruling 2 covers documents. Is this split intended? Not tested either way.
4. **`write_not_landed`'s `staging` half is not tested.** Only `restarting` is pinned (`it/resolver/file_list.rs:229`). Add a unit test: set `staging = Some(id)`, answer `Operated::Failed` under that id, and assert `staging == None` and that a list click switches files.
5. **Question: `e` on a `done ✓ · removed` card.** `edit_here` (`update/resolver.rs:365-379`) opens `$EDITOR` on a path that does not exist, and saving recreates it unstaged. The bar still offers `e edit`. Should the removed card drop `e`, or word it (`e recreates it`)?
6. **Ticket text.** Acceptance tick 2 has a garbled duplicate clause: "(`DU` resolved by `git rm`) and `…::open_done_on_a_binary_is_binary`." Tick 1 still says "`notes.txt` resolved and `git add`ed" after the deviation note. Textual only.
7. **Contact sheet.** `docs/shots/m6/m6-contact-sheet.txt` carries none of the `fix-file-list-*` captures, 13's included (ADR-0037). The builder flagged this. It is the acceptance ticket's (21) to assemble.
8. **Gap rows the ticket asked for are missing** (rule 4). Ticket Out says "a done file whose text still has markers, staged from the shell … Gap row". `KNOWN_GAPS.md` has only the M6/25 `d` row. That case also draws each block's `o yours  t theirs  b both` header over keys that are refused, which is worth saying in the row. The builder's second unverified item also has no row: the `Refused(Unmerged)` race showing `could not read: <ops words>`.

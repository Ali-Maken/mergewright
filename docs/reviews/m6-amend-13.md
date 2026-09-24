# Review: m6-conflicts ticket 13 (resolver file list)

Ticket `.scratch/m6-conflicts/issues/13-resolver-file-list.md`: commit `00459ab`, merged `b0e08fd`. Range `db98aa2..b0e08fd`. Read-only. Reviewer: Opus, 2026-09-24.

**Count: 0 Critical · 2 Major · 13 Minor.**

Rulings already made (not re-reported): D1 and D2 go to ticket 25; D3–D7 are accepted. I confirmed the cause lines:
- **D1:** `graft-ops/src/conflict/set.rs:28-30` puts the files unmerged now first. `graft/src/update/resolver/files.rs:73-76` takes that as the list's order. So a file that is done moves to the end.
- **D2:** `graft/src/update/resolver/files.rs:168-178` refuses a `Done` row with `<path> is already done`.

## Gates

All ran one at a time on `main` at `75ee76c`. Its `crates/`, `Cargo.toml` and `Cargo.lock` are identical to `b0e08fd` (`git diff --quiet`). A builder and a scout were working other worktrees on the same machine (load average 3–4).

| Gate | Result |
|---|---|
| `cargo fmt --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo clippy --all-targets --features heavy-tests -- -D warnings` | pass |
| `cargo nextest run --workspace` | 2792 passed, 1 skipped. **104.4 s** wall (`/usr/bin/time`), nextest 104.0 s. |
| `cargo test --doc --workspace` | 3 passed |
| Heavy: `cargo test --features heavy-tests -- --test-threads=2` | 2827 passed, 0 failed, 1 ignored. **322.6 s** wall. Max RSS of one process 1.12 GB. |
| Release perf (`frames::` + `heavy::budgets_hold`) | **2 of 13 failed on the first run.** Both passed when rerun alone (below). |
| Fuzz, benchmarks | Not run: none is scoped to this ticket. |

The two failures on the first release run:
- `frames::fifty_thousand_refs_…_at_the_unicode_tier`: p99 55.3 ms, max 136 ms. 8 keys missed, keys 172–179. Every one was *queued* behind a frame that took under 1.3 ms; the keys' own frames were 0.24–1.27 ms. That is a stall of about 135 ms with no frame work in it.
- `frames::fifty_thousand_refs_…_scrolling_the_diff`: p99 10.6 ms, max 90.5 ms. 5 keys missed, keys 291–295, the same queued pattern. The keys' own frames were at most 3.1 ms.
- **Rerun alone, both passed.** Unicode p99 5.37 ms, max 5.50 ms. Diff scroll p99 5.11 ms, max 10.13 ms.
- This ticket touches only the resolver's draw and its mouse. The graph and diff scroll paths are not in the diff. The shape matches KNOWN_GAPS row 211 (release p99 is noisy) plus the concurrent builder. **Reported, not waved through.**

The other release numbers (first run):
- Frames:
  - Pixel 50k refs: p99 7.82 ms.
  - Pixel 100k: p99 5.92 ms.
  - Unicode 100k: p99 5.27 ms.
  - pane open: p99 5.31 ms.
  - Map: p99 4.88 ms.
- First paint:
  - 100k with 50k refs: 39.6 ms.
  - 100k with a commit-graph: 13.9 ms.
  - 100k without one: 15.9 ms.
  - Realistic, with a commit-graph: 14.0 ms.
  - Realistic, without one: 16.5 ms.
  - Peak RSS 31–103 MB.

Fast suite:
- The 5 slowest tests (nextest, `--status-level pass`):
  - `graft-ops conflict_ops::whole_file_shapes_resolve_like_git` 8.1 s
  - `graft-git diff_fixture::parse_unified_matches_numstat_for_every_shape` 7.8 s
  - `graft-ops undo_cancel::cancel_at_every_step_boundary_is_clean` 7.1 s
  - `graft m5_walk::every_scenario::merge_resolved_with_the_keys` 7.1 s
  - `graft-graph …scrolling_the_whole_history_keeps_the_resident_set_bounded` 6.6 s
- Max RSS of one process 272 MB (`time -l`).
- `target/` is 18 GB.

**Probes.** I ran two throwaway tests on an exported copy of `b0e08fd` (`git archive` to the scratchpad, its own target dir, deleted afterwards). Nothing was committed. Their output is quoted under Major 1 and Minor 1.

## Tick spot-check

I checked **all 15 ticks against their tests. All 15 held.** Each named test exists and asserts what its tick says, with the deviations the ticks themselves declare (D1 order, D5).
- Weak but held: `file_at_matches_what_is_drawn`'s rule-row and `│` half is vacuous (Minor 9). The rule row is covered for real by `a_click_on_the_document_still_picks_its_block`. Nothing covers a click on the `│`.
- Weak but held: `a_click_mid_take_is_refused_in_words` covers `job` and `base_job` only (Minor 7).
- The capture tick matches the NOTES' "as built" lines. It does not match the ticket's own lines. That difference is D1, already ruled.

## Pipeline, ADR, keymap, journeys

- **Design note** (line numbers at `b0e08fd`). Amendment 2 has Need (:409), Prior art (:422, citing `docs/research/conflict-layout-audit.md`, which exists) and Journeys (:434). This passes.
- **ADR-0043 §1.**
  - Strip below 180, column at 180 and up: built.
  - `THREE_PANE_MIN = 180`: built.
  - States `2 of 3 open` / `⏎ marks done` / `done ✓` / `keep or delete`: built.
  - "Files stay listed as `done ✓` until the operation ends": holds for merge and rebase. It does **not** hold for stash pop, which has no record; the builder reports this as a gap.
- **Keymap.** No key is bound. I grepped the non-test diff for `KeyCode` and found none. This matches R-keys and 02-keymap §2.6 (`j/k`, `O/T` and `c` belong to other tickets).
- **Journeys.**
  - J4, J8 and J9 land on "the next file not done, list order, wrapping" (`update/resolver/answers.rs:250-261`). For J9, from `notes` it wraps to `gone`. By the lines, J8 stays at 3 keys and J9 at 4.
  - No test in this ticket counts steps; that walk is ticket 21's. **Not verified by a walk here.**
- **Architecture.** `graft-ui`'s `files.rs` imports only `graft_term` and ratatui, so `ui` never imports `ops`. No `Cargo.toml` changed.
- **Sanitising.** List names go through `sanitize_line` (`view/resolver.rs` `file_list_view`). The notice and the status line are sanitised at draw time (`view/resolver.rs:51-54`, `view/status.rs:43-48`).

## The builder's question: `c` refused after a Stage, before the refresh

**Yes, it reads stale state.** Verified by the lines and by a probe.
- `continue_refusal` counts `unmerged_paths(state)`: `crates/graft/src/update/banner.rs:200-206`.
- `unmerged_paths` reads the cached `state.working_tree`: `banner.rs:77-86`.
- The Stage's answer does not update that cache. `operate::refresh_scope` only queues `Command::ReadWorkingTree` (`update/operate.rs:966-974`), in the same batch as the journal, stash and detail reads.
- In that same update, `resolved_advance` closes to the banner and says `c` works (`update/resolver/answers.rs:281-298`).
- Probe on `b0e08fd`, calling `resolved_advance` and then pressing `c` with the status not yet re-read:
  ```
  after stage: status=Done("every file done ✓ · c finishes the merge") banner_focus=true
  after c:     status=Sticky { reason: "1 file still conflicted", keys: Some("M mergetool  e edit  space marks resolved") }
  ```
  The screen invites `c`, then refuses it as an error when nothing is wrong. That breaks CLAUDE.md's status rule.

**Reachable at human speed? Not measured.**
- The window runs from the `Operated` answer to the `WorkingTree` answer: one full status read (gix) plus its place in the queue. The `c` must land inside it; a `c` typed while the Stage is still running goes to the resolver instead.
- On a small repo I expect the window to be well under a reaction time. On a large tree it may not be. I did not time it.
- The window is not new: the old words `all conflicts resolved · c continue` had the same race. This ticket kept it and made the invitation stronger.
- Filed as Minor 1, with its lines.

## Critical

None.

## Major

### 1. A click on the file list while `d`'s box is up closes the resolver, and the box is left behind

- **Where:**
  - `crates/graft/src/update/resolver/files.rs:190-202`: `switch_refusal` does not check `state.confirm`.
  - `crates/graft/src/update/resolver.rs:447-453`: `reopen` calls `state.resolver.take()` **before** `open_refusal` refuses on the box (`resolver.rs:268-270`), and then returns without putting the resolver back.
  - `update/mouse.rs:29-35`: the resolver claims every mouse event before anything checks for a box.
- **What:** `d` puts up "Start notes.txt over?". A click on another file's row reaches `select_file` → `reopen`. That closes the resolver, sets `finish the box first`, and leaves the box up over the graph.
  - Probe on `b0e08fd` (`listing(["conflicted.rs","later.rs"])`, `d`, click on row 1):
    ```
    resolver_open=false confirm_open=true status=Sticky { reason: "finish the box first" } commands=[]
    ```
  - Confirming the box afterwards still launches `checkout --merge` on that file (`update/confirm.rs:152-167`). The resolver is gone, along with its list state (`files`, `seen`).
- **Why it matters:** the user loses his place with one mis-aimed click. The rule in `open_refusal` ("a box that is waiting for an answer keeps it") is broken from the other side. No data is lost that the box did not already name, but the screen he was working in disappears under a question about it.
- **Fix (the cause):**
  - `reopen` should decide on the refusal *before* taking the resolver.
  - `switch_refusal` should also refuse while `state.confirm` is `Some`, with today's words.
  - Better still, `resolver_mouse` should not act on clicks while a box is open over it; the box owns the input.
  - Add a test: `d`, then a click on a list row, then assert the resolver is still open.

### 2. File size over the PROMPT.md 6.2 limit (about 400 lines)

- **Where:**
  - `crates/graft-ui/src/conflict_resolver/files.rs` is **612 lines** of non-test code. It is a new file.
  - `crates/graft/src/update/resolver.rs` grew from 421 to **476** lines (it was already over).
- **Why it matters:** the rule is hard (PROMPT.md :436). `files.rs` holds four separable things: the types and `placement`; the strip and the grid; the column; the widget.
- **Fix:**
  - Split into `files/strip.rs` (`strip_plan`, `grid_plan`, `window`, `one_row_cell`, `grid_cell_text`, `strip_label`) and `files/column.rs` (`column_plan`, `column_width`, `column_header`). Leave the types, `resolver_areas`, `file_at` and the widget in `files.rs`.
  - Move `open_resolver`, `start`, `open_refusal` and `reopen` out of `update/resolver.rs` into a child `open.rs`.

## Minor

1. **`c` refused on stale status after the last Stage.** See the question above: `banner.rs:200-206`, `banner.rs:77-86`, `answers.rs:281-298`, `operate.rs:966-974`.
   - **Fix the cause:** the app knows a status read is owed after the write. Give `ReadWorkingTree` an id (like `ReadRefs(JobId)` got in ticket 12) and hold `every file done · c …` until that read lands. Alternatively, have `continue_refusal` say `checking the files…` while that read is out, rather than count stale entries.
   - **To decide severity:** time the window on the user's repo.
2. **The split is computed twice, and the MAP and NOTES say once.**
   - `view/frame/conflicts.rs:44-46` builds its own `placement` and `resolver_areas` from the drawn `document` rect.
   - The mouse uses `view::resolver::resolver_split` (`view/resolver.rs:218`), which rebuilds the rect from `state.size`.
   - The graft MAP ("the one split both the draw and the mouse use") and the NOTES ("both get that area from `resolver_split`") are wrong.
   - The two agree today because `draw_resolver_banner` (`conflicts.rs:86-90`) and `resolver_split` do the same arithmetic.
   - **Fix:** have `draw_resolver` call `resolver_split`, or share one fn that takes the body rect.
3. **No gap on either side of the column's `│`** (textual).
   - `docs/shots/m6/fix-file-list-200x50.txt` row 4: `▏▸ notes.txt  2 of 2 open│  1 one`.
   - Row 2: `…│▸ notes.txt · 2 conflicts · 2 open`.
   - The design note's mockup (:631 at `b0e08fd`) has a space before and after `│`.
   - Cause: `column_width` is exactly the longest row (`files.rs:537`), and the doc starts at `list.right() + 1` (`files.rs:179-184`).
   - **Fix:** pad the column by one cell and start the doc one cell after the `│`.
4. **Unused parameter.** `whole_file_view(…, _columns: u16)` (`view/resolver.rs:64`). Remove it.
5. **`listed_files` runs about 4 times per frame, and each run is O(n²).**
   - The `paths.contains` / `unmerged.contains` scans over `Vec<PathBuf>` are at `update/resolver/files.rs:77-81` and `:89`.
   - The calls are `file_list_view`, then `file_cursor` (which calls `listed_files` again), both from `resolver_view` and `draw_resolver`. `body()` repeats them for keys and mouse events.
   - Not measured, and there is no test with a large conflict (hundreds of files).
   - **Fix:** use a `HashSet` for membership, and compute the cursor from the same `Vec`.
6. **Two full status reads per `⏎`.**
   - `jobs/conflict.rs:258-273` runs `read_facts`, which runs `graft_git::status::read` (`graft-ops/src/facts.rs:108`). That is beside the `ReadWorkingTree` the same Stage queues.
   - Not measured on a large tree.
   - **Fix:** a scope that skips the status and passes the unmerged paths in, or have `operation_conflicts` take the status the app already has.
7. **Untested branches.**
   - `switch_refusal`'s `staging` / `restarting` arm (`update/resolver/files.rs:198-200`, `a write is already running`).
   - The box case (Major 1).
8. **A resolver mouse test was narrowed.** `tests/it/resolver/modal.rs:106-109`, `mouse_uses_the_key_functions`, now clicks rows 5..20 instead of 0..14.
   - Rows 0–4 (the banner, the strip, the rule) are no longer checked for "no click reaches the graph".
   - **Fix:** keep rows 0..5 in the loop and assert only the graph selection there.
9. **Half of `file_at_matches_what_is_drawn` is vacuous.** `graft-ui/src/conflict_resolver/files/tests.rs:201-210` probes `dy == rows` and `dx == width`. Both lie outside the plan, so they can never hit anything.
   - **Fix:** an app-level click on the `│` column at 200 columns, asserting nothing changed.
10. **Stale docs quote the old title** (`file 12 of 13 · 2 of 3 conflicts open`), which the code no longer draws:
    - `TESTING.md:2245`;
    - `docs/shots/m6/m6-contact-sheet-captions.tsv:1,3,9` (and the sheet);
    - design note :152.
    - The `m6-01…m6-05` capture files predate this change.
    - **Fix:** regenerate them and correct their captions at the acceptance run (ticket 21).
11. **`(ours)` in the footer of every `fix-file-list-*` capture** (`resolved gone.txt (ours) ✓`).
    - This is a word in trailing parentheses, and git's side word rather than the side's one name. Both go against ADR-0043 §4 and 02-keymap §2.1.
    - Source: `graft-ops/src/operation/conflict.rs:129-135`. It predates this ticket; its captures show it.
12. **Question for the user's eye:** in the grid, an `Unread` row draws `…` after its name (`fix-file-list-many-120x40.txt` row 2: `src/handlers/handler_02.rs …`). Next to `… 7 more`, that can read as a cut name. The design chose `…` for Unread (:674-675 at `b0e08fd`). This is wording only.
13. **Question: the NOTES' "for that moment `gone.txt` reads `Unread`".**
    - By the lines it should read its last-seen state (`keep or delete`), not `Unread`. The card branch of `conflict_arrived` calls `remember` (`update/resolver/answers.rs:56`), and `listed_files` falls back to `seen` for a still-unmerged, not-open path (`files.rs:94`).
    - Not verified either way. The builder should say which run showed `Unread`.

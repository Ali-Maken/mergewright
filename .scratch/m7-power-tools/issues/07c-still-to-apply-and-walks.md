# 07c — part of 07 rewrite-stops (split by planner, filed by conductor 2026-09-27)

Status: ready. Shared context (settled scout findings, order, Decided, Signatures) is `07-rewrite-stops.md` lines 1-118; read it first. This file holds only part 07c plus Touches and Rules.

## 07c — still-to-apply block, the shell hand-over, walks, captures
Build with: **Sonnet**. The hand-over copies the `M` pattern (`outside.rs:173`, `running.rs:269`,
`update/mod.rs:384`); the block is one pure draw; walks and captures are fenced by counts here.
Needs: 07b (`Stop.rewrite`, `banner_view`); 03b's m7 walk harness.
Blocked by: 07b.
Files:
- graft-ui: `src/still_to_apply.rs` (**new**), `src/lib.rs` (mod + `pub use`), the graph_screen render file (the
  block's area between banner and rows; named after 09 lands).
- graft src: `update/banner.rs` (`pending_view`; `continue_pressed` :429 routes a shell-begun rebase/cherry-pick to
  `Command::ContinueInGit`); `update/commit_editor.rs` (`pane_c_acts` true for a shell-begun rebase or cherry-pick;
  the shell-begun words `:151-159` for keys still refused name `c`: `{kind} begun in the shell — c hands it to git`);
  `view/rows.rs` (`live_c` label); `update/events/command.rs`, `update/events.rs`, `event_loop/running.rs` (:214,
  :269), `event_loop/outside.rs` (beside `mergetool` :173, via `run_outside_terminal`), `jobs/runner.rs` (:1083),
  `update/mod.rs` (route beside `mergetool_returned` :384); forced arms `event_loop/label.rs`, `update/place/write.rs`.
- graft tests: `update/keys/live_c_tests.rs`, `tests/it/resolver/shell_begun.rs`, `update/banner/tests.rs`,
  `tests/it/m7_walk/rewrite_stops.rs` (**new**; conductor adds the `mod` line).
- Docs: MAP.md (graft, graft-ui), TESTING.md "Stops during a rewrite" + captions, contact sheet, `docs/shots/m7/`.

Acceptance:
- [ ] `still_to_apply_is_newest_first_with_followers`, `still_to_apply_caps_at_six_then_more`,
  `no_block_when_nothing_is_pending`, `no_ghost_rows_in_the_graph`;
- [ ] `c_in_a_shell_rebase_hands_over_to_git` and `…_cherry_pick_…`, with a fake hand-over as the `M` tests do
  (`banner/tests.rs:484,503,718`); `a_shell_begun_revert_is_still_refused`;
- [ ] the three return landings (finished / stopped again / non-zero) each a test;
- [ ] **Walks** (`m7_walk`, counted, fail over the fence):
  - `rewrite_stop_to_first_file`: after `y`, 0 keys, resolver on the first file; ≤ `noted_keys("rewrite-stop-to-first-file")`.
  - `edit_stop_lands`: 0 keys to the graph at the stopped commit, block drawn.
  - `edit_stop_amend_continue`: fs write, then `space a c` (3); commit contains the edit, rewrite finished. `space`
    is today's graph `space` (keymap §2.3); if one key from where the stop lands can't stage the edit, stop and
    report the count — don't rebind.
  - `abort_from_a_chain_stop`: `⇥ a y`; every branch back, HEAD on his branch.
- [ ] **Captures** (colour unverified):
  - `docs/shots/m7/rewrite-chain-stop-187x50.txt` — **Right:** `REBASING  step 3 of 5 · pick <sha> <subject> ·
    feature-b, branch 2 of 3`; the resolver below.
  - `docs/shots/m7/rewrite-edit-stop-187x50.txt` — **Right:** `still to apply · not commits yet` boxed; two `pick`
    rows; `feature-b (Sara): 2 commits follow, on top`; `▸` on `HEAD · stopped here`; bar `a amend it  c continue
    space stage`.
  - `docs/shots/m7/rewrite-edit-stop-80x24.txt` — **Right:** `REBASING  step 3 of 7` whole; no edit hint; block cut
    to `+N more`.
Size: ~55k.


## Touches (each part checks its own, reports how)
- **M6 banner and resolver** for merge, pull, stash clash: words and focus unchanged. Check: every m6 banner/resolver
  test and capture byte-identical (`check_shot`), or each change listed in NOTES. (07b, 07c)
- **Footer `stop_wordings`:** unchanged. Check: its tests. (07b)
- **`u`/`U` refusal while live:** unchanged; the amend at an edit stop is refused for `u` like any live step. Check:
  its test. (07b)
- **P14 for merge / cherry-pick / revert and conflict stops:** unchanged. Check: existing amend-refusal tests. (07a, 07b)
- **`resume_target` / `live_entries`:** an amend entry landing inside a live rewrite must not hide the rewrite's
  `Pending` entry. Check: `c_after_an_amend_at_an_edit_stop_continues_the_rewrite`. (07b)
- **`pane_c_acts`** (one place; `commit_key` and `view::rows::live_c` read it): now true for shell-begun rebase and
  cherry-pick. Check: `live_c_tests.rs`, `shell_begun.rs`. (07c)
- **Relaunch after kill -9:** reads the saved plan through `progress`. Check: `a_relaunch_mid_rewrite_shows_the_chain`. (07b)
- **Keymap:** §2.6a already amended; no doc edit here. Check: hint-bar tests.
- **Journal/undo:** the edit-stop amend is an ordinary `Amend` entry; the shell hand-over writes none (git's
  operation, begun outside, as M6). Nothing else new.
- **KNOWN_GAPS :545:** closed by the conductor (settled 1), not by a builder.

## Rules / Gates
As rewrite-engine. Workspace-wide gates. Each part commits at its seam.

## NOTES — builder 2026-09-28 (status: **blocked**, partial; not compacted; scout used: `scout07c` in session scratchpad)

**Blocked: needs `crates/graft/src/update/tests.rs`** (held by ticket 11). The hand-over's landing needs a new
`AppState` field, and `update/tests.rs:1274` builds `AppState` with an exhaustive literal. The one line it needs:
`continue_in_git_owed: None,` (field: `pub continue_in_git_owed: Option<(graft_git::state::InProgress,
std::process::ExitStatus)>`, `update/state.rs`). Lib builds and `clippy --workspace` (lib targets) is clean;
**the graft test target does not compile until that line lands**, so no graft test was run.
Why a field: finished / stopped again / non-zero-still-stopped can only be told apart by the working-tree read after
git returns (a rebase that stops on its next conflict exits 1, the same as a failure). No existing field or `Stop`
(also built literally in `update/tests.rs:4484`) could carry it without touching 11's files.

Done (uncommitted-tests, see above):
- graft-ui `still_to_apply.rs` (new): `PendingView`, `PendingRowView`, `draw_still_to_apply`,
  `still_to_apply_lines`, `still_to_apply_height`, `MAX_ROWS`, `TITLE`; `GraphView.pending`; `ScreenRows.pending`
  (≤ half the rows under the banner, ≥ 3 or none); drawn in `render.rs` `pending_block`. graft-ui tests pass
  (563): `still_to_apply_is_newest_first_with_followers`, `still_to_apply_caps_at_six_then_more`,
  `no_block_when_nothing_is_pending` (still_to_apply.rs), `no_ghost_rows_in_the_graph` (graph_screen/tests.rs).
- graft `banner::pending_view` (todo pending reversed → newest first, capped 6, `more` counted), fed by `view/rows.rs`.
- `Command::ContinueInGit { id, kind }`, `Product::ContinuedInGit(Result<ExitStatus, String>)`, `Handover::continue_in_git`
  + `outside::continue_command`, loop arms in `running.rs`, runner `not_a_job`, label, `place/write.rs` (write = true),
  `update/mod.rs` route → `banner::continued_in_git`; landing `banner::land_after_continue_in_git` run on the newest
  working-tree read in `working_tree_answered`.
- `continue_pressed`: shell-begun rebase/cherry-pick → `ContinueInGit`; shell revert → `resume` (refused, as today).
- `pane_c_acts` true for any rebase/cherry-pick; shell-begun words `{kind} begun in the shell — c hands it to git`.

**Gap — followers have no data.** `RewriteProgress` (graft-ops `rewrite.rs:278`) has no per-branch counts or tip
authors, so `feature-b (Sara): N commits follow, on top` cannot be drawn: `pending_view` passes `followers: vec![]`.
Needs a graft-ops read (outside this ticket's crate set). The widget draws followers when given them (tested).
**Gap — walk fence:** `docs/design-notes/m7-power-tools.md:830-832` has no `abort-from-a-chain-stop` row; `noted_keys`
would panic. Conductor/planner to add the count.

Next steps once unblocked: add the literal line; run gates; expect `update/tests.rs:4505` (pane_c_acts table) and the
shell-begun words tests (`commit_editor/tests.rs`, `live_c_tests.rs`, `resolver/shell_begun.rs`) to need updating to
the new behaviour; write the hand-over tests (fake return: feed `AppEvent::Produced(id,
Product::ContinuedInGit(Ok(ExitStatus::from_raw(code << 8))))`, then a working-tree answer) — three landings + two
hand-over tests + revert refusal; walks in `tests/it/m7_walk/rewrite_stops.rs`; captures; MAP.md; TESTING.md.
None of the acceptance boxes is ticked yet except as named above (graft-ui half of box 1).

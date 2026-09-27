# M7 parallel plan (ADR-0051, as amended 2026-09-27)

Planner, 2026-09-27. Read against ticket headers, Crates/Touches, graft/graft-ops MAP.md, and file globs/greps for
struct-literal sites. No source read.

## Rules this plan uses
- **Cap:** ≤ 3 builders on graft, ≤ 3 on any other crate, ≤ 5 in total.
- **Disjoint files**, counted strictly for code, the hint catalogue and captures. **MAP.md is ignored** (user,
  2026-09-27).
- **Two registries are counted as one file each**, because every ticket that adds a job or a hint edits them:
  - **JOBS** = `crates/graft/src/update/{events.rs, events/command.rs, mod.rs}` + `crates/graft/src/jobs/{mod.rs,
    kinds.rs, runner.rs}`.
  - **HINTS** = `crates/graft-ui/src/graph_screen/hints.rs` + `hints/{context.rs, catalogue.rs, lists.rs, lists/*,
    fitting.rs, tests.rs}` + `crates/graft/tests/it/hint_bar.rs`.
- **`crates/graft/src/view/rows.rs`** (row views and `hint_context`) is held by 8 tickets. It sets the floor:
  8 build waves.
- **New modules: a seam commit per wave.** Before each wave, the conductor adds each file marked `*` below as an empty
  file with a one-line `//!` doc, plus its `mod` line (graft-ops `lib.rs`, graft `update/mod.rs` / `jobs/mod.rs`,
  graft-ui `lib.rs`, `tests/it/main.rs`, `tests/it/m7_walk.rs`). Builders then never edit a `mod` list.
- **Needs your OK: `TESTING.md` and `docs/shots/m7/m7-contact-sheet-captions.tsv`.** This plan treats both like
  MAP.md: rows are appended, and the conductor resolves clashes at merge. Almost every ticket adds rows, so counting
  them would make the plan serial.
  - Fallback if you say no: each builder writes its rows verbatim under `## TESTING.md rows` in its ticket NOTES, and
    the conductor appends them at merge. The waves don't change.
- **Timing runs pause dispatch** (ADR-0051 §5): 16's release perf line, 12e's perf rev, and 09's "under 1 frame"
  measurement if it runs from a script.

## Waves
`G` = holds graft. `N` = no graft. Each wave starts once the previous wave's tickets it depends on have merged. A
ticket may start early when a slot frees, if its Blocked-by has merged and its files are disjoint from everything still
running.

| Wave | Builders (model) | Why together / notes |
|---|---|---|
| **1 (now, beside 30d)** | G **05** rewrite-preview (Opus) · G **14** compare-since-split (Sonnet) · N **10c** switch-list (Sonnet) · N **13a** submodule-log (Opus). **When 30d merges:** G **10b** switch-rule (Opus) | 30d, 05 and 14 are the 3 on graft. 10b waits for 30d because both hold `m6_stash_put_back/set_aside.rs`. graft-ops holders: 05, 10c, 13a (then 10b once 10c or 13a ends). Scouts first: 05, 10b (supplementary), 13 (one scout for 13a and 13b). |
| **2** | G **16** drawn-rows-are-his (Opus) · G **13b** submodule-rows (Opus) · N **01a** error-rows-ops (Opus). **When 16 merges:** G **09** amend-into-and-absorb-keys (Opus) | 16 needs 14. 13b needs 13a. 09 needs 05. 16 and 09 both hold rows.rs, `update/state.rs`, `update/tests.rs` and JOBS, so they run one after the other. Start 09 the moment 16 merges: it is on the critical path (09 → 24; 09 → 08). 16's perf line pauses dispatch. |
| **3** | G **27b** stop-after-exit (Opus) · G **20j** blame-shallow-capture (Sonnet) · G **16b** force-box-names-what-leaves (Sonnet) · N **11a** pick-refusals (Opus) | Starts as 16 and 13b merge; 09 may still be running. **Until 09 merges, 16b waits**: they share JOBS, and the graft cap is 3. 16b's `update/remote/tests.rs` is free once 05 merges. 27b holds graft-ops `apply/remote.rs` and `tests/it/set_aside.rs`, which 05 and 10b have released by now. 11a holds `operation.rs`/`operation/history.rs`, released by 05 and 10b. 20j: 32 and 33 are done. |
| **4** | G **24** journal-screen (Opus) · G **03b** pull-sets-aside (Opus) | 24 needs 09. 03b holds `apply.rs`, `operation.rs`, `put_back_index.rs`, `update/remote.rs` and `remote/in_flight.rs`, which clash with 01a, 10b, 11a, 05 and 16b, all merged by now. Nothing else is disjoint from 24: it holds rows.rs, JOBS, HINTS, `update/preview.rs` and `update/name_prompt.rs`. |
| **5** | G **10** branch-picker (Opus) · G **01b** error-rows-app (Sonnet) | 10 needs 10b and 10c. 01b needs 01a; it was kept out of wave 4 because 24 may touch `update/undo.rs`. |
| **6** | G **21** history-place (Opus) · G **11** cherry-pick-key (Opus) · N **30e** ops-doc-links (Sonnet) | 11 needs 10 and 11a. 30e waits for `apply.rs`/`apply/error.rs` to be free. |
| **7** | G **07** rewrite-stops (Opus) | Holds rows.rs, JOBS, HINTS, `view/frame.rs` and the banner. |
| **8** | G **08** rewrite-editor (Opus) · G **12e** final-perf-and-docs (Opus) | 08 needs 05, and must follow 09 (see "PlanRewrite" below). 12e writes its scenarios alongside 08; its perf runs wait until 08 merges, with dispatch paused. 12g must not be measuring. |
| **9** | G **15** daily-wording (Sonnet) | Holds JOBS (`Product::TopologicalOrder` gains the sort's `Duration`), `update/preview.rs`, `update/remote/tests.rs`, `m4_walk/push.rs` and `gallery.rs`, so it clashes with every earlier wave. |
| **10** | G **26** m7-acceptance (Fable) | Last. |

**10 waves.** The floor is 9: rows.rs forces 8 waves, plus 26.
- **Fold option (needs your OK):** allow append-only sharing of JOBS, where two tickets only add variants and arms and
  the conductor keeps both at merge. Then 15 joins wave 7 beside 07, and 16b need not wait for 09: no other file is
  shared. Result: 9 waves.

## Found while planning (conductor)
- **`Command::PlanRewrite` is declared by 08** (08 Signatures) **and called by 09** (09 Signatures). The two tickets
  can't run together.
  - In this plan 09 builds first and declares it, with `AmendInto` and `Absorb`. 08 adds `Open` and reuses it.
  - Add a line to 08's Needs: `Command::PlanRewrite`, `RewriteRequest` (09).
- **The new param fields reach many files.** 05's `RebaseParams::drops` and 10b's `OnEdits` are struct-literal fields,
  so every construction site has to change: 19 files for `RebaseParams`, 23 for `CheckoutParams`/`BranchCreateParams`,
  across graft-ops tests, graft `verbs.rs`, `drag.rs`, `preview.rs`, `remote/tests.rs`, `view/tests*`, and
  `m6_stash_put_back/set_aside.rs`. Their Files lines below carry those sites, which is what forced 10b after 30d and
  16b out of wave 1.
- **Unconfirmed: 32 is merged.** 32's status line reads "done (…; not merged)", but 32b is done. 20j needs 32 merged.
- **Unconfirmed: 20e is merged.** 20e's status line still reads "ready", but 20i, 20k and 20l built on it. 21 is
  blocked by 20.
- **Unconfirmed: 14a is merged.** 16b is blocked by 14a (`count_between`).
- **Still undecided:** 11a's set-aside half depends on whether the note's "Where I'd differ" 4 is approved.

## Leftover halves
- **20c, hints half:** covered. 20c's own "Hints" box is ticked "finished by 20e", with tests named. Only 20c's status
  line is stale.
- **27, item 1a:** open. It is carried whole by 27b (wave 3).
- **30c, K4 at 80 columns:** covered by 30d (building now).
- **30c, Minor 3 doc check:** open, and no later ticket covered it. Filed as **30e** (wave 6).

## Split out (ADR-0051 §1)
| New | From | Crates | Parent keeps |
|---|---|---|---|
| 01a | 01 rows :601 :613 :640 :496 :497 | graft-ops | — (01 is replaced by 01a + 01b) |
| 01b | 01 rows :494 :495 :572 | graft-ops, graft | — |
| 10c | 10's `switch_list` + filter | graft-ops | 10: overlay, keys, draw, walks, captures |
| 11a | 11's refusals + CherryPick set-aside | graft-ops | 11: `o`, slot 2, app tests, walks, captures |
| 13a | 13's `submodule_log` (+ fixture) | graft-ops, graft-fixtures | — (13 is replaced by 13a + 13b) |
| 13b | the rest of 13 | graft-git, graft-ui, graft | — |
| 30e | 30c Minor 3's open check | graft-ops (docs) | — |

Not split: 03b, 05, 10b, 15, 27b. In each, the non-graft half changes a type or behaviour that graft's build or tests
pin, so it doesn't stand alone. 05's and 10b's own "Split if needed" lines still apply if a builder compacts.

## Files (insert under each ticket's `Crates:` line)
I have no Edit tool, and rewriting 20 specs whole risks drifting their text, so each line is here for the conductor
to paste. The new tickets (01a, 01b, 10c, 11a, 13a, 13b, 30e) already carry theirs.
Paths are from the repo root. `*` = new file, pre-registered by the seam. JOBS and HINTS are defined above. For every
line: a file outside the list → the builder stops and reports `blocked: needs <file>` (ADR-0051 §3).

- **03b** — `Files:` crates/graft-ops/src/{apply.rs, facts.rs, operation.rs}; crates/graft-ops/tests/it/{put_back_index.rs, pull_set_aside.rs*}; crates/graft/src/update/{remote.rs, remote/in_flight.rs, clash.rs}; crates/graft/tests/it/{pull_over_edits.rs, m7_walk/pull.rs*}; docs/shots/m7/pull-over-edits-120x40.txt; MAP.md (graft, graft-ops); TESTING.md.
- **20j** — `Files:` crates/graft/src/{repository.rs, repos.rs, update/commit_graph.rs}; crates/graft/tests/it/{blame_place.rs, m7_walk/blame.rs}; docs/shots/m7/blame-shallow-187x50.txt; MAP.md (graft); TESTING.md.
- **14** — `Files:` crates/graft/src/update/detail/{state.rs, keys.rs, tests.rs, landing/tests.rs, cursor/tests.rs}; crates/graft/src/update/{selection.rs, tests.rs, place/tests/pinned_base.rs}; crates/graft/src/jobs/detail.rs; crates/graft/src/view/{detail.rs, rows.rs, entries.rs}; crates/graft/src/update/{geometry.rs, place/from.rs}; crates/graft-ui/src/{detail.rs, detail_pane/header.rs, detail_pane/tests.rs, graph_screen/badges.rs, graph_screen/render.rs, graph_screen/tests.rs}; crates/graft/tests/it/{gallery.rs, m7_walk/pr_contents.rs*}; docs/shots/m7/compare-since-split-{187x50,80x24}.txt; docs/shots/{m2-compare.txt, m2-05-feature-branches-x-j-j-x-160x40-unicode.txt} (only if a test writes them); MAP.md (graft, graft-ui); TESTING.md.
- **15** — `Files:` crates/graft-ops/src/{apply/remote.rs, apply/remote/tests.rs, mistake_radar/rules.rs, mistake_radar/mod.rs, mistake_radar/explain.rs}; crates/graft-ops/tests/it/mistake_radar.rs; crates/graft-core/src/mistake_radar.rs; JOBS (the sort answer's `Duration`) and the topological-sort job's file (builder names it); crates/graft/src/update/{commit_editor.rs, commit_editor/tests.rs, commit_graph.rs, preview.rs, preview/tests.rs, remote/tests.rs, remote/tests/lease.rs}; crates/graft/src/view/{commit_editor.rs, radar.rs, tests/bottom_row.rs}; crates/graft/tests/it/{remote_by_key.rs, m4_walk/push.rs, box_owns_the_bar.rs, m6_stash_put_back.rs, gallery.rs, status_under_a_box.rs, radar_bookkeeping.rs}; crates/graft-ui/src/{name_prompt.rs, name_prompt/tests.rs, preview_box.rs, preview_box/tests.rs}; docs/shots/m7/{commit-editor-fresh-120x40.txt, refused-box-single-close-120x40.txt}; each older capture a fast-tier test re-takes (listed in NOTES); MAP.md (graft-ops, graft, graft-ui).
- **16** — `Files:` crates/graft/src/{view/radar.rs, view/rows.rs, update/keys/view_change.rs, update/state.rs, update/tests.rs}; the history view's inclusion file under crates/graft/src/history/ (builder names it); JOBS; crates/graft/src/jobs/stash_only.rs*; crates/graft-ui/src/{graph_column/mod.rs, graph_column/tests.rs, graph_screen/badges.rs, graph_screen/render.rs, graph_screen/tests.rs}; crates/graft/tests/it/stash_in_history.rs*; docs/shots/m7/stash-in-history-{120x40,80x24}.txt; older `index on` captures a fast-tier test re-takes (listed); docs/perf/drawn-rows-are-his.txt; MAP.md (graft, graft-ui); TESTING.md.
- **16b** — `Files:` crates/graft/src/update/{remote.rs, remote/push_box.rs, remote/in_flight.rs, remote/tests.rs}; crates/graft/src/jobs/remote.rs; JOBS; crates/graft/tests/it/m4_walk/push.rs; docs/shots/m7/force-box-leaves-{120x40,80x24}.txt; MAP.md (graft); TESTING.md.
- **27b** — `Files:` crates/graft-git/src/run.rs; crates/graft-git/tests/it/run.rs; crates/graft-ops/src/apply/remote.rs; crates/graft-ops/tests/it/{undo_cancel.rs, set_aside.rs}; crates/graft-ops/src/{restore.rs, restore/*, apply/set_aside.rs, apply/set_aside/*} (only where the code relies on the wrong premise); crates/graft/src/update/credential.rs; crates/graft/src/jobs/{askpass.rs, operate.rs}; crates/graft/tests/it/{credential_by_key.rs, m4_walk/every_scenario.rs}; MAP.md (graft-git, graft-ops, graft).
- **12e** — `Files:` crates/graft/tests/it/{frames.rs, frames/*}; crates/graft/examples/budgets.rs; 12d's perf script under scripts/; docs/perf/m7-12-idle-advance.txt; MAP.md (graft); TESTING.md ("Performance").
- **09** — `Files:` the graph `a` route's file under crates/graft/src/update/keys/ (scout names it); crates/graft/src/update/{context_menu.rs, context_menu/slots.rs, context_menu/tests.rs, preview.rs, preview/tests.rs, state.rs, tests.rs}; crates/graft/src/view/rows.rs; JOBS (declares `Command::PlanRewrite`); crates/graft/src/jobs/rewrite.rs*; crates/graft/tests/it/m7_walk/amend.rs*; docs/shots/m7/{amend-into-preview-187x50, absorb-preview-120x40, amend-into-landed-120x40}.txt; crates/graft-ui/src/preview_box.rs (only if the scout finds the box can't draw the rows); MAP.md (graft); TESTING.md; captions TSV.
- **21** — `Files:` crates/graft/src/update/{place.rs, place/*, detail/keys.rs, state.rs, tests.rs}; crates/graft/src/view/rows.rs; the view file that draws `Place` (scout names it); JOBS; crates/graft/src/jobs/trace.rs*; HINTS; crates/graft-ui/src/{history.rs*, detail_pane/body.rs, detail_pane/line_render.rs, detail_pane/tests.rs}; crates/graft/tests/it/{blame_place.rs, history_place.rs*, m7_walk/history.rs*}; docs/shots/m7/history-*.txt; blame and detail-bar captures that change (re-shot, listed); MAP.md (graft, graft-ui).
- **24** — `Files:` crates/graft/src/update/{journal.rs, preview.rs, preview/tests.rs, name_prompt.rs, state.rs, tests.rs}; crates/graft/src/view/{journal.rs, rows.rs}; JOBS; crates/graft/src/jobs/journal_list.rs*; HINTS; crates/graft-ui/src/{journal_screen.rs, stash_screen.rs}; crates/graft/tests/it/m7_walk/journal.rs*; the m3/m5/m6 `J` tests and walks the scout lists (Q6); docs/shots/m7/{journal-*, put-back-*, journal-after-put-back-*, journal-expanded-*}.txt; docs/shots/{m3-journal-expanded.txt, m3-contact-sheet.txt}; every `J` capture re-shot (listed); MAP.md (graft, graft-ui). Not `update/undo.rs` or `jobs/undo.rs`: needing either → blocked.
- **10b** — `Files:` crates/graft-ops/src/{operation.rs, operation/history.rs, operation/history/tests.rs, operation/tests.rs, preview.rs, preview/kinds.rs, preview/tests.rs}; the graft-ops test files holding a `CheckoutParams`/`BranchCreateParams` literal: crates/graft-ops/tests/it/{history_ops.rs, set_aside.rs, set_aside_kill.rs, in_the_way.rs, in_the_way/perf.rs, in_the_way/one_shot.rs, octopus_merge.rs, put_back_index.rs, checkout_hook_fails.rs, undo_replaced.rs, recover.rs, undo_gate.rs, journal_list_oracle.rs, put_back_forecast.rs, switch_rule.rs*}; crates/graft/src/update/{switch.rs*, context_menu.rs, context_menu/verbs.rs, context_menu/verbs/tests.rs, name_prompt.rs, operate.rs, clash.rs, clash/tests.rs, history.rs, preview.rs, preview/tests.rs}; crates/graft/src/view/tests/bottom_row.rs; crates/graft/tests/it/{m6_stash_put_back/set_aside.rs, m5_walk/pick_revert_reset.rs, m5_walk/every_scenario.rs, m5_walk/refs_and_names.rs, name_and_drag.rs, m7_walk/switch.rs*}; docs/shots/m7/{switch-menu-clean-landed-120x40, new-branch-with-edits-preview-120x40}.txt; docs/shots/m5-ui-name-prompt*.txt; the m5/m6 captures re-taken (listed); MAP.md (graft-ops, graft); TESTING.md.
- **10** (graft-ops part is now 10c) — `Files:` crates/graft/src/update/{picker.rs*, state.rs, tests.rs}; the graph `C` route's file under crates/graft/src/update/keys/ and the modal predicate's file (scout 10 names them); crates/graft/src/view/{frame.rs, rows.rs, picker.rs*}; JOBS; crates/graft/src/jobs/picker.rs*; HINTS; crates/graft/src/update/preview.rs or crates/graft/src/update/journal.rs (only if the preview return is generalized — say which); crates/graft-ui/src/picker.rs*; crates/graft/tests/it/m7_walk/picker.rs*; docs/shots/m7/{picker-empty-query-187x50, picker-filtered-100x30, switch-landed-120x40, picker-remote-landed-120x40}.txt; MAP.md (graft, graft-ui); TESTING.md.
- **11** (graft-ops part is now 11a) — `Files:` the graph `o` route's file under crates/graft/src/update/keys/ (scout names it); crates/graft/src/update/{context_menu.rs, context_menu/slots.rs, context_menu/verbs.rs, context_menu/tests.rs, preview.rs}; crates/graft/tests/it/{context_menu.rs, m7_walk/cherry_pick.rs*}; docs/shots/m7/{take-preview-120x40, take-refused-merge-120x40, menu-slot-two-dim-120x40}.txt; the menu captures whose slot-2 label changes (re-taken, listed); MAP.md (graft); TESTING.md.
- **05** — `Files:` crates/graft-ops/src/{preview.rs, preview/kinds.rs, preview/tests.rs, operation/history.rs, operation/history/tests.rs, operation/tests.rs}; the graft-ops test files holding a `RebaseParams` literal: crates/graft-ops/tests/it/{in_the_way/replay.rs, put_back_index.rs, history_ops.rs, set_aside.rs, put_back_forecast.rs, rewrite_run.rs, conflict_fixtures.rs, rewrite_preview.rs*}; crates/graft-ui/src/{preview_box.rs, preview_box/tests.rs}; crates/graft/src/update/{preview.rs, preview/tests.rs, operate.rs, drag.rs, drag/tests.rs, context_menu/verbs.rs, context_menu/verbs/tests.rs, remote/tests.rs}; crates/graft/src/view/{preview.rs, tests.rs}; each `ConflictForecast` exhaustive match (scout Q2 names them); docs/shots/m7/{rebase-line-preview-187x50, rebase-squash-merged-preview-120x40, rebase-stop-forecast-80x24}.txt; captions TSV; MAP.md (graft-ops, graft-ui).
- **07** — `Files:` the graft-ops `check` for Amend (scout Q6 names the file; if it is `operation.rs`/`operation/history.rs`, nothing else may hold them); crates/graft-ops/tests/it/edit_stop_amend.rs*; crates/graft/src/update/{banner.rs, banner/stop.rs, banner/tests.rs, relaunch.rs, resolver.rs, state.rs, tests.rs}; the `a`/`c`/`Esc` route files under crates/graft/src/update/keys/ (scout names them); crates/graft/src/view/{frame.rs, frame/bars.rs, rows.rs}; JOBS; HINTS; crates/graft-ui/src/still_to_apply.rs* and the banner chain words' file (scout names it); crates/graft/tests/it/m7_walk/rewrite_stops.rs*; docs/shots/m7/{rewrite-chain-stop-187x50, rewrite-edit-stop-187x50, rewrite-edit-stop-80x24}.txt; MAP.md (graft, graft-ui, graft-ops); TESTING.md.
- **08** — `Files:` crates/graft/src/update/{rewrite_editor.rs*, state.rs, tests.rs}; the graph `R` route's file under crates/graft/src/update/keys/; crates/graft/src/view/{frame.rs, rows.rs, rewrite_editor.rs*}; JOBS (adds `RewriteRequest::Open` to 09's `PlanRewrite`); crates/graft/src/jobs/rewrite.rs; HINTS; crates/graft/tests/it/{m5_walk/journeys.rs, m7_walk/rewrite.rs*}; crates/graft-ui/src/rewrite_editor.rs*; docs/shots/m7/{rewrite-editor-187x50, rewrite-editor-140x40, rewrite-editor-100x30, rewrite-refused-fork-120x40}.txt; **every existing capture showing a commit row's bar** (re-taken, listed); MAP.md (graft, graft-ui); TESTING.md; captions TSV.
- **26** — `Files:` crates/graft/tests/it/{m7_walk.rs, m7_walk/*, m7_accept.rs*}; scripts/m7-accept.sh*; docs/shots/m7/{m7-contact-sheet*.txt, m7-contact-sheet-captions.tsv}; docs/perf/m7-accept.txt; TESTING.md ("M7 acceptance", "Times I left" :109).

## Seam commits (conductor, before each wave)
- **W1:** graft-ops `src/switch_list.rs`, `src/submodule.rs`; graft-ops tests `switch_list.rs`, `submodule_log.rs`,
  `switch_rule.rs`; graft-fixtures `src/shapes/submodule.rs` (only if 13's scout says no fixture exists); graft
  `src/update/switch.rs`; graft tests `m7_walk/pr_contents.rs`, `m7_walk/switch.rs`.
- **W2:** graft-ops tests `rewrite_preview.rs`, `error_rows.rs`; graft `src/jobs/stash_only.rs`, `src/jobs/rewrite.rs`;
  graft tests `stash_in_history.rs`, `submodule_truth.rs`, `m7_walk/amend.rs`.
- **W3:** graft-ops tests `cherry_pick.rs`.
- **W4:** graft-ops tests `pull_set_aside.rs`; graft `src/jobs/journal_list.rs`; graft tests `m7_walk/journal.rs`,
  `m7_walk/pull.rs`.
- **W5:** graft `src/update/picker.rs`, `src/view/picker.rs`, `src/jobs/picker.rs`; graft-ui `src/picker.rs`; graft
  tests `m7_walk/picker.rs`.
- **W6:** graft `src/jobs/trace.rs`; graft-ui `src/history.rs`; graft tests `history_place.rs`, `m7_walk/history.rs`,
  `m7_walk/cherry_pick.rs`.
- **W7:** graft-ops tests `edit_stop_amend.rs`; graft-ui `src/still_to_apply.rs`; graft tests
  `m7_walk/rewrite_stops.rs`.
- **W8:** graft `src/update/rewrite_editor.rs`, `src/view/rewrite_editor.rs`; graft-ui `src/rewrite_editor.rs`; graft
  tests `m7_walk/rewrite.rs`.
- **W10:** graft tests `m7_accept.rs`.

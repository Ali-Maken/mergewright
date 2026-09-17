# radar-wiring: the radar context job, its state, and the graph's marker truth

Status: ready
Blocked by: 13
Build with: Sonnet — same model and scope as `13-radar-context-and-markers`, split at its handoff seam. No new decisions; signatures are fixed in ticket 13's Signatures section.
Needs: `radar-context-and-markers` (`first_parent_chain`, `previous_tips`, `ReflogError`, `Radar`/`RadarSetting`/`Radar::policy`, `MarkerView`, `GraphEntry::marker`, `GraphView::marker_column`), `mistake-radar-detection` (`graft-ops` classify), `detail-model-and-jobs` (job pool, `Command`)

## Scope

Items 1–4 and 7 of ticket 13's "Remaining (next session)" handoff list, which is this ticket's specification — read it in full first:

1. `crates/graft/src/jobs/radar.rs`: `RadarContext`, `run_radar_context`, the `Task`/`Product`/`Command::RadarContext` arms.
2. `crates/graft/src/update/radar.rs`: `RadarState`, `AppState::radar`, dispatch at both `Command::ReadRefs` sites (startup + refresh — locate with a scout, don't guess).
3. `crates/graft/src/view/radar.rs`: `branch_facts`, `head_facts`, `repo_facts`, `findings_for`, `marker_for`, `radar_lines_for`; replace `view/rows.rs` and `view/entries.rs`'s placeholder `marker: None` / `marker_column: false` with real calls.
4. `marker_column`'s real truth: `Ready` context plus any finding, or a sticky bit if `History::rows` evicts off-screen rows — check that first.
7. Tests for 1–4 (fixture-based, plus the hand-built `AppState` slice in ticket 13's Tests section).

Out: `detail_pane.rs`'s radar line rendering and the capture — ticket 13c.

Gates include the purity guard: `update/radar.rs` no I/O, `view/radar.rs` no I/O.

## Acceptance

Ticket 13's acceptance rows for the job, state, facts and marker column. A fixture with a known finding shows its marker in the graph; a clean fixture reserves no marker column beyond ticket 11's compare accent.

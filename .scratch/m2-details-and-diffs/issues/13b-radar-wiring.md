# radar-wiring: the radar context job, its state, and the graph's marker truth

Status: done
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

## Done (session 2)

All of items 1, 2, 3, 4, 7 landed:

1. `jobs/radar.rs`: fixed the test (was using a nonexistent fixture API); real one uses `graft_fixtures::borrow` + `RepoStore`. `jobs/mod.rs`/`jobs/runner.rs` wire `Command::RadarContext` through `dispatch_detail`.
2. `update/radar.rs`: `RadarState`, `AppState::radar`, `request_context`/`context_landed`/`reset_for_refresh`. Dispatch is not literally at `Command::ReadRefs`'s two call sites (see Deviations) — `request_context` runs at the end of every `update`, gated on the default branch's tip being known, and `reset_for_refresh` is called from `keys::refresh_key`.
3. `view/radar.rs`: `branch_facts`, `head_facts`, `repo_facts`, `findings_for`, `marker_for`, `radar_lines_for`, plus the kind→phrase table. `view/rows.rs`'s two `marker_column: false` placeholders and `view/entries.rs`'s `row_entry` `marker: None` placeholder are wired to real calls.
4. `marker_column` truth: `Ready` context AND `AppState::radar_column_seen` (a new `Cell<bool>` field), set from `view::entries::row_entry` the first time a row's marker is `Some`. Confirmed `History::rows`/`Window` evicts off-screen rows, so the sticky bit is required, not optional.
7. Tests: `jobs/radar.rs` (1 fixture test), `update/radar.rs` (4 tests: ask-once, no-repeat-while-pending, stale-drop, refresh-clears), `view/radar.rs` (4 tests: foxtrot finding + EvilUnchecked line, policy-off suppresses the line, off-chain commit finds nothing, pre-`Ready` finds nothing).

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (412 tests total in `graft`) all pass. Actually verified by running them, not assumed.

## Deviations (challenge the spec, not silent)

- **Dispatch site**: ticket 13 says dispatch `Command::RadarContext` "at both `Command::ReadRefs` sites". Not literally possible: the default branch's tip isn't known until the refs job's `Product::Refs` and the history walk's `WalkFacts` have both landed, which is after `ReadRefs` is issued, not at the point it's issued. `request_context` instead runs at the end of every `update()`, gated on `Loading::Idle`/`Failed` and on the tip being resolvable — functionally "at startup and at refresh" since neither can ask any earlier.
- **`head_facts` source**: uses `AppState.repository.head` (already-fetched, pure), not `RepoHandle::head()`/`HeadState` — the latter is I/O, which `view` may never do (purity guard).
- **`RefBadges::branches()` added to `refs/mod.rs`**, not in this ticket's file list: needed so `branch_facts` can resolve a branch name to its tip purely from `AppState.refs`.
- **`[radar]` config → `AppState.radar.policy` wiring not done.** Not in items 1,2,3,4,7. `Policy::default()` is in effect; a real functional gap — logged in `KNOWN_GAPS.md` under `m2-details-and-diffs/13b`.
- **`Task` lost `#[derive(Copy)]`** (`update/events.rs`): `Task::RadarContext` carries a `RefName`, which isn't `Copy`. `jobs/tests.rs`'s `next_started` helper changed to take `&Task` (8 call sites updated) to satisfy clippy's `needless_pass_by_value` once the type stopped being trivially copyable.

Session 2 compacted once (mid-build); the jobs/update scaffolding from session 1 needed re-verification by compiler, which caught two API mismatches (fixed, see commit).

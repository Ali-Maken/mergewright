# radar-probe-evil-merge: is the selected merge's resolution honest?

Status: draft
Build with: Opus — a git subprocess whose exit/stderr shape decides three outcomes (clean/evil/not-checkable); getting that classification wrong is a silent correctness bug, not a style one.
Needs: `git-runner` (`GitInvocation`, `GitArg`, `RunError`, `Stop`), `mistake-radar-detection` (`EvilVerdict`, `finding_from_evil`, `Policy`), `radar-context-and-markers` (`RadarLine::EvilUnchecked`/`EvilChecking`/`EvilClean`/`EvilNotCheckable` already rendered by `detail_pane.rs`; `findings_for`'s extension point; `DetailState`/`Bounded` pattern)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Hard-to-reverse choice 2 ("lazy probes: evil per selected merge"); New UI "Detail pane" states bullet is silent on this — the wording is `radar-context-and-markers`'s (cited there)
- `.scratch/m2-details-and-diffs/issues/mistake-radar-detection.md` — `EvilVerdict`, `finding_from_evil` signatures and their acceptance (`NotCheckable` never becomes a `Finding`)
- `.scratch/m2-details-and-diffs/issues/git-runner.md` — `GitInvocation`/`GitArg`/`RunError` signatures, `command_line()`
- `.scratch/m2-details-and-diffs/issues/radar-context-and-markers.md` — `findings_for`/`radar_lines_for`'s exact `if` to extend, `DetailState` field-addition pattern (same pattern `compare-two-commits` and `context-actions-menu` used)
- `crates/graft-git/MAP.md` line 56 (`read_commit`) — parent count without a second read path
- `crates/graft-fixtures/MAP.md` — the `evil` shape (a merge whose resolution differs from a clean recursive merge) and a plain foxtrot/backward merge (clean) for contrast

## Scope

In:
- `crates/graft-ops/src/mistake_radar/probes.rs` (extends the file `mistake-radar-detection` created; adds one function, no new types beyond what that ticket already declared):
  - `pub fn probe_evil_merge(repo: &RepoHandle, merge: Oid, stop: &dyn Stop) -> EvilVerdict` — reads the commit first (`graft_git::history::read_commit`); `parents.len() < 2` → `NotCheckable("not a merge")`; `> 2` (octopus) → `NotCheckable("octopus merge")`; else runs `GitInvocation::new(repo).arg(Word("show".into())).arg(Flag("--remerge-diff")).arg(Flag("--format=")).arg(Flag("--name-only")).arg(Oid(merge)).run(stop)`; empty stdout → `Clean`; non-empty → `Evil { files: <stdout split on '\n', empty lines dropped, each line's raw bytes> }`; any `RunError` (including a git too old to know `--remerge-diff`) → `NotCheckable(err.to_string())`; `Stopped` → also `NotCheckable("cancelled")` (the caller already dropped the stale product by job id; this is only reached if someone calls the probe directly without going through the job).
- `crates/graft/src/jobs/radar.rs` (extends `radar-context-and-markers`'s file): `Task::EvilProbe { merge: Oid }`, `Product::EvilProbe(Oid, EvilVerdict)`, `Command::EvilProbe(JobId, Oid)`; a thin `run_evil_probe` wrapper calling `probe_evil_merge`.
- `crates/graft/src/update/detail.rs` (extends `DetailState`): `pub evil: Loading<EvilVerdict>` (the currently open commit only) and `pub evil_cache: Bounded<Oid, EvilVerdict>` (cap 16, same `Bounded` type `detail-model-and-jobs` defined). Trigger: in `DetailState::open` (and the selection-move arm), when the new spec is `DetailSpec::Commit(oid)` and `state.radar.policy.severity(EvilMerge).is_some()` — cache hit → `Ready` at once, no command; miss → `Pending` + `Command::EvilProbe(id, oid)`, cancelling any prior pending probe (`Cancel(old)`, same rule `diff`'s jobs use). `Compare`/`WorkingTree` specs never probe. `Product::EvilProbe`: stale-dropped by id, `Ok` path caches + `Ready`, no `Err` variant (the verdict itself carries `NotCheckable`).
- `crates/graft/src/view/radar.rs` (extends `radar-context-and-markers`'s `findings_for`/`radar_lines_for`, cited seams):
  - `findings_for`: when `state.detail.evil` is `Ready(EvilVerdict::Evil { files })` for the commit being asked about, append `finding_from_evil(id, &EvilVerdict::Evil{files}, &state.radar.policy)` (`Some` → push).
  - `radar_lines_for`: before falling back to `EvilUnchecked`, check `state.detail.evil` for this exact commit: `Pending` → `EvilChecking`; `Ready(Clean)` → `EvilClean`; `Ready(NotCheckable(reason))` → `EvilNotCheckable(sanitize(reason))`; `Ready(Evil{..})` → no extra line (it's already a `Finding` line from `findings_for`); `Idle` → the existing `EvilUnchecked` fallback.
- Tests: `probe_evil_merge` against fixtures (`evil` shape → `Evil{files non-empty}`; a clean foxtrot merge → `Clean`; a non-merge commit → `NotCheckable("not a merge")`); `update` tests for cache hit/miss/cancel/stale-drop; `radar_lines_for`/`findings_for` tests for each `Loading` state. MAP.md lines (`graft-ops`, `graft`).
- Capture: the `evil` fixture's merge selected, pane header showing the `⚠ evil merge` finding line, to `docs/shots/m2-evil-merge.txt`, ≤ 200 KB.

Out: force-push and duplicate probes (their own tickets), the Explain panel content for an evil finding (`explain-panel` calls the same `explain()` `mistake-radar-detection` already wrote — nothing new here), git-version detection ahead of time (the probe just tries the command and reads the error).

## Signatures

```rust
// crates/graft-ops/src/mistake_radar/probes.rs
pub fn probe_evil_merge(repo: &RepoHandle, merge: Oid, stop: &dyn Stop) -> EvilVerdict;

// crates/graft/src/jobs/radar.rs
pub fn run_evil_probe(repo: &RepoHandle, merge: Oid, stop: &dyn Stop) -> EvilVerdict;
// Task::EvilProbe { merge: Oid }
// Product::EvilProbe(Oid, EvilVerdict)
// Command::EvilProbe(JobId, Oid)

// crates/graft/src/update/detail.rs (extends DetailState)
// DetailState: + pub evil: Loading<EvilVerdict>, + pub evil_cache: Bounded<Oid, EvilVerdict>
```

## Acceptance

- [ ] `probe_evil_merge` on the `evil` fixture's merge → `Evil { files }` non-empty; on a clean merge (any other merge fixture) → `Clean`; on a non-merge commit → `NotCheckable("not a merge")`.
- [ ] Opening the `evil` merge's detail pane: one `Command::EvilProbe`; revisiting it in the same session is a cache hit, no second command.
- [ ] Moving off the merge before the probe answers cancels it (`Command::Cancel`); its late `Product::EvilProbe` changes nothing (stale by id).
- [ ] `findings_for` on the evil merge, once `evil` is `Ready(Evil{..})`, includes one `EvilMerge` finding; `radar_lines_for` shows no separate `EvilUnchecked`/`EvilClean` line alongside it.
- [ ] `radar_lines_for` on a clean merge: `EvilChecking` while pending, `EvilClean` once answered.
- [ ] `[radar] evil = off`: no probe is ever issued, `radar_lines_for` has no evil-related line at all.
- [ ] MAP.md: `graft-ops` (`probes::probe_evil_merge`), `graft` (`jobs::radar`, `update::detail` additions).
- [ ] Capture at `docs/shots/m2-evil-merge.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard on `update/detail.rs`.

## Size

Two crates in substance (`graft-ops` ~60 lines, `graft` ~140: jobs ~30, update ~60, view ~50). Tests ~160. ≤ 22 tool calls. Named seam if tight: commit `probe_evil_merge` + its fixture tests first (self-contained, no app dependency); the `update`/`view` wiring is the remainder.

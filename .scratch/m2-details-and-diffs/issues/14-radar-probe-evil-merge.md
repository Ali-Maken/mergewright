# radar-probe-evil-merge: is the selected merge's resolution honest?

Status: partial — the probe seam is done and committed (`b8e0aa7`); the
`jobs`/`update`/`view` wiring, its tests and the capture remain. See
**Handoff (session 1)** at the bottom.
Blocked by: 01, 04, 13, 13b

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

## Handoff (session 1, partial)

Done, committed at `b8e0aa7` (gates green for `graft-ops`; workspace
untouched otherwise):

- `crates/graft-ops/src/mistake_radar/probes.rs`: `probe_evil_merge` exactly
  as specified, plus four fixture tests. **Verified by running them**, not
  by inspection: `Shape::EvilMerge`'s tip → `Evil { files: ["evil.txt"] }`;
  `Shape::FoxtrotMerge`'s tip → `Clean`; `Shape::OctopusMerge`'s tip →
  `NotCheckable("octopus merge")`; the evil merge's first parent →
  `NotCheckable("not a merge")`. Local git is 2.54, so the
  "git too old for `--remerge-diff`" arm is code-reviewed, not observed.
- `crates/graft-ops/MAP.md`: one line for `probe_evil_merge`.

### Remaining, in order

1. `crates/graft/src/update/events.rs`: `Task::EvilProbe { merge: Oid }`,
   `Product::EvilProbe(Oid, EvilVerdict)`, `Command::EvilProbe(JobId, Oid)`.
2. `crates/graft/src/jobs/radar.rs`: `run_evil_probe` (thin wrapper, mirror
   `run_radar_context`). `crates/graft/src/jobs/runner.rs`: a
   `Command::EvilProbe` arm in `dispatch_detail` (copy the
   `Command::ContainedIn` arm at ~line 191, via a `repos`-opening helper like
   `radar_context` at line 34), and add `Command::EvilProbe(..)` to the
   routing list at ~line 448.
3. `crates/graft/src/update/detail.rs`:
   - `DetailState` + `pub evil: Loading<EvilVerdict>`, `pub evil_cache:
     Bounded<Oid, EvilVerdict>` (`const EVIL_CACHE: usize = 16`), both in
     `Default`.
   - **`load`/`open`/`follow` cannot see the policy** (they are `DetailState`
     methods; the policy lives on `AppState::radar`). Don't thread a param
     through all 16 call sites. Add instead:
     `pub fn probe_evil(&mut self, policy: &Policy, next_id: &mut impl
     FnMut() -> JobId) -> Vec<Command>` — reads `self.spec`/`self.shown`;
     returns `Command::CancelJob(old)` + `Idle` when the pane is hidden, the
     spec is not `DetailSpec::Commit`, or `policy.severity(EvilMerge)` is
     `None`; `evil_cache` hit → `Ready` with no command; miss → cancel any
     prior pending, `Pending(id)` + `Command::EvilProbe(id, oid)`.
     `state.detail.probe_evil(&state.radar.policy, next_id)` borrow-checks
     (disjoint fields, two-phase borrow).
   - `pub fn evil_probed(&mut self, id, oid, verdict)` — `return` unless
     `self.evil == Loading::Pending(id)`; else cache + `Ready`.
   - `refresh` must **not** clear `evil_cache`: a merge's tree is immutable,
     so the verdict can never go stale.
   - Call `probe_evil` right after the three sites that can land on a commit:
     `detail.rs` ~line 714 (the `Enter` arm — add `radar` to the
     `let AppState { detail, local_jobs, .. } = next;` destructure),
     `selection.rs:121` (`follow_selection`'s tail; wrap the existing body so
     every return path also gets the cancel), `context_menu.rs:138`
     (`OpenDetails`). The two `Compare` paths never probe.
   - A `Product::EvilProbe` arm beside `Product::ContainedIn` at
     `detail.rs:643` (it needs the job id, which that arm currently drops).
4. `crates/graft/src/update/mod.rs`: route `Product::EvilProbe(..)` into
   `detail::detail_event` (~line 85), add `Task::EvilProbe { .. }` to the
   ignored-`Started` list (~line 100), and `Product::EvilProbe(..)` to
   `took_fact`'s not-a-fact arm (~line 190).
5. `crates/graft/src/view/radar.rs`: the two cited seams. Guard both on
   "this is the open commit": `matches!(state.detail.spec,
   Some(DetailSpec::Commit(o)) if o == id)` — there is no `evil_at` field, so
   the spec is what ties the verdict to a commit.
   - `findings_for`: on `Ready(Evil { .. })`, push
     `finding_from_evil(id, verdict, &state.radar.policy)`.
   - `radar_lines_for`: inside the existing `if parents.len() >= 2 && …
     EvilMerge on … && context_ready` block, match `state.detail.evil`
     first: `Pending` → `EvilChecking`, `Ready(Clean)` → `EvilClean`,
     `Ready(NotCheckable(r))` → `EvilNotCheckable(sanitize_line(r))`,
     `Ready(Evil{..})` → push nothing (`findings_for` already emitted the
     line), `Idle`/`Failed` → today's `EvilUnchecked`.
6. Tests: `update` (cache hit/miss/cancel/stale-drop, `[radar] evil = off`
   issues nothing), `view::radar` (one per `Loading` state). MAP.md for
   `graft` (`jobs::radar::run_evil_probe`, `update::detail`'s two fields and
   two methods, the three `events.rs` variants).
7. Capture `docs/shots/m2-evil-merge.txt` (≤200 KB): a pty script under
   `.scratch/m2-details-and-diffs/`, modelled on
   `.scratch/m1-the-graph/pty-gallery.py`, driving the real binary against
   `Shape::EvilMerge`'s fixture, merge row selected, detail pane open,
   `⚠ evil merge` on the header's radar line. The user judges it by eye.

### Gotchas

- The ticket text says `Command::Cancel(JobId)`; the real name is
  **`Command::CancelJob`**.
- `Policy` is `Clone`, not `Copy` — pass `&Policy`.
- `update/detail.rs` is scanned whole by the purity guard
  (`update_cannot_reach_the_world` in `update/tests.rs`): no `GitInvocation`,
  no `Stop`, no thread or clock in there — `Loading`/`Bounded`/`Command`
  plumbing only.

### Why this stopped here

The ticket's own named seam. Session hit the 40-tool-call hard stop with the
probe committed and green; the remainder is six files across two crates plus
a pty capture, which is its own context.

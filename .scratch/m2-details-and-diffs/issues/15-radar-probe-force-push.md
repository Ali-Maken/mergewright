# radar-probe-force-push: did a tracked remote rewrite history under us?

Status: ready
Blocked by: 01, 04, 13

Status: draft
Build with: Opus — an ancestry check (`merge-base --is-ancestor`) whose exit code is the whole answer; a wrong mapping of exit codes to `ForcePushVerdict` is a silent false positive/negative, not a style bug.
Needs: `git-runner` (`GitInvocation`, `GitArg`, `RunError`, `Stop`), `mistake-radar-detection` (`ForcePushVerdict`, `findings_from_force_push`, `Policy`), `radar-context-and-markers` (`previous_tips`, `RadarLine::ForcePushNotCheckable`, `findings_for`'s extension point, `RadarState`/`jobs::radar` pattern)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Hard-to-reverse choice 2 ("lazy probes: force-push per tracked remote")
- `.scratch/m2-details-and-diffs/issues/mistake-radar-detection.md` — `ForcePushVerdict`, `findings_from_force_push` (its `at` = current tip), `Evidence::ForcePushed`
- `.scratch/m2-details-and-diffs/issues/git-runner.md` — `GitInvocation`/`GitArg`/`RunError`
- `.scratch/m2-details-and-diffs/issues/radar-context-and-markers.md` — `previous_tips` signature (this ticket calls it on the remote-tracking ref, not the local branch), `findings_for`'s exact seam, `RadarState`, `jobs::radar`'s `Task`/`Product`/`Command` pattern, `branch_facts`
- `crates/graft-git/MAP.md` line 82 (`RepoHandle::upstream`) — the remote-tracking ref a branch pulls from

## Scope

In:
- `crates/graft-ops/src/mistake_radar/probes.rs` (extends the file): `pub fn probe_force_push(repo: &RepoHandle, branch: &str, remote_ref: &RefName, stop: &dyn Stop) -> ForcePushVerdict` —
  `previous_tips(repo, remote_ref, 1)` (from `radar-context-and-markers`'s `history.rs`): empty → `NotCheckable("no remote-tracking reflog")`; else `previous = tips[0]`, `current = RepoHandle`'s current read of `remote_ref`'s target (the caller passes it in as `current: Oid`, no second ref read — see signature) — `GitInvocation::new(repo).arg(Word("merge-base".into())).arg(Flag("--is-ancestor")).arg(Oid(previous)).arg(Oid(current)).run(stop)`: exit 0 → `Clean` (fast-forward, not a rewrite); exit 1 → `Rewritten { remote_ref: remote_ref.as_bytes() lossy, previous, current }`; any other `RunError` → `NotCheckable(err.to_string())`.
- `crates/graft/src/jobs/radar.rs` (extends the file): `Task::ForcePushProbe { branch: String, remote_ref: RefName, current: Oid }`, `Product::ForcePushProbe(String, ForcePushVerdict)`, `Command::ForcePushProbe(JobId, String, RefName, Oid)`; `run_force_push_probe` wraps `probe_force_push`.
- `crates/graft/src/update/radar.rs` (extends `RadarState`): `pub force_push: std::collections::HashMap<String, ForcePushVerdict>` keyed by branch name. Trigger: when `Product::RadarContext` lands `Ok`, for every branch `branch_facts` reports with `upstream != UpstreamFacts::None`, emit one `Command::ForcePushProbe` (bounded by the repo's own branch count — real repos rarely exceed a few dozen local branches, no further cap needed). `Product::ForcePushProbe(branch, verdict)`: `force_push.insert(branch, verdict)` unconditionally (no staleness to drop — these aren't tied to the current selection).
- `crates/graft/src/view/radar.rs` (extends `findings_for`): for the commit `id` being asked about, if it is some branch's current tip and `force_push.get(branch) == Some(Rewritten{..})`, append `findings_from_force_push(&verdict, &state.radar.policy)` (its `at` is that tip). A `NotCheckable` verdict never reaches a finding (per `mistake-radar-detection`'s rule) — instead extend `radar_lines_for`: when `id` is a tracked branch's tip and its verdict is `NotCheckable(reason)`, add `RadarLine::ForcePushNotCheckable(sanitize(reason))`.
- Tests: `probe_force_push` against a fixture repo with a rewritten remote-tracking ref (construct one in the test: commit, push-simulate by moving the remote-tracking ref back then forward past a rewrite — use the fixture corpus's own hermetic git runner to fabricate this, cite `graft-fixtures/MAP.md`'s runner, don't hand-roll a second one) and a normal fast-forwarded one; `update`/`view` tests for the trigger, the map insert, and both finding/line outcomes. MAP.md lines (`graft-ops`, `graft`).
- Capture: a tip row with a force-pushed remote, pane header showing `⚠ force-pushed remote`, to `docs/shots/m2-force-push.txt`, ≤ 200 KB.

Out: evil-merge and duplicate probes (their own tickets), any UI for "which branches have upstreams" beyond what already exists (ref badges).

## Signatures

```rust
// crates/graft-ops/src/mistake_radar/probes.rs
pub fn probe_force_push(repo: &RepoHandle, branch: &str, remote_ref: &RefName, current: Oid, stop: &dyn Stop) -> ForcePushVerdict;

// crates/graft/src/jobs/radar.rs
pub fn run_force_push_probe(repo: &RepoHandle, branch: &str, remote_ref: &RefName, current: Oid, stop: &dyn Stop) -> ForcePushVerdict;
// Task::ForcePushProbe { branch: String, remote_ref: RefName, current: Oid }
// Product::ForcePushProbe(String, ForcePushVerdict)
// Command::ForcePushProbe(JobId, String, RefName, Oid)

// crates/graft/src/update/radar.rs (extends RadarState)
// RadarState: + pub force_push: std::collections::HashMap<String, ForcePushVerdict>
```

## Acceptance

- [ ] `probe_force_push` on a fabricated rewritten remote-tracking ref → `Rewritten{..}`; on a fast-forwarded one → `Clean`; on a branch whose remote-tracking ref has no reflog entry → `NotCheckable("no remote-tracking reflog")`.
- [ ] `RadarContext` landing triggers exactly one `ForcePushProbe` per upstream-tracked branch, none for branches without an upstream.
- [ ] A `Rewritten` verdict on a branch's tip yields one `ForcePushedRemote` finding at that commit; a `NotCheckable` verdict yields no finding but does yield a `RadarLine::ForcePushNotCheckable` line when that tip is selected.
- [ ] MAP.md: `graft-ops` (`probes::probe_force_push`), `graft` (`jobs::radar`, `update::radar` additions).
- [ ] Capture at `docs/shots/m2-force-push.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates in substance (`graft-ops` ~50 lines, `graft` ~130: jobs ~30, update ~50, view ~50). Tests ~150 (the fabricated-rewrite fixture setup is the fiddly part — budget it first). ≤ 22 tool calls. Named seam: `probe_force_push` + its two fixture cases fit alone; the trigger/map/view wiring is the remainder if the fixture fabrication runs long.

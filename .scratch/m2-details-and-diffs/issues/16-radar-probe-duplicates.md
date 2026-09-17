# radar-probe-duplicates: cherry-picked/duplicate commits in the newest 500

Status: ready
Blocked by: 01, 04, 13, 13b

Status: draft
Build with: Sonnet — one pipeline of two typed git calls into a grouping function; every step is independently testable against a fixture.
Needs: `git-runner` (`GitInvocation`, `GitArg::stdin`, `RunError`, `Stop`), `mistake-radar-detection` (`Duplicates`, `findings_from_duplicates`, `Policy`), `radar-context-and-markers` (`RadarState`/`jobs::radar` pattern, `findings_for`'s extension point)

## Read first

- `docs/design-notes/m2-details-and-diffs.md` — Hard-to-reverse choice 2 ("lazy probes: duplicates over a 500-commit window")
- `.scratch/m2-details-and-diffs/issues/mistake-radar-detection.md` — `Duplicates { groups, window, complete }`, `findings_from_duplicates` signature and its acceptance (one finding per copy, listing the others)
- `.scratch/m2-details-and-diffs/issues/git-runner.md` — `GitInvocation::stdin`, `GitArg::Count`/`Flag`/`Word`, `GitOutput`
- `.scratch/m2-details-and-diffs/issues/radar-context-and-markers.md` — `RadarState`, `jobs::radar`'s `Task`/`Product`/`Command` pattern, `findings_for`'s exact seam
- `crates/graft-fixtures/MAP.md` — the `cherry-pick-duplicates` shape (its exact commit count and which commits are copies of each other)

## Scope

In:
- `crates/graft-ops/src/mistake_radar/probes.rs` (extends the file): `pub fn probe_duplicates(repo: &RepoHandle, window: u32, stop: &dyn Stop) -> Result<Duplicates, RunError>`:
  1. `oids = GitInvocation::new(repo).arg(Word("log".into())).arg(Flag("--all")).arg(Count("-n", window)).arg(Flag("--format=%H")).run(stop)?` → lines → `Vec<Oid>`; `complete = oids.len() < window as usize`.
  2. `patches = GitInvocation::new(repo).arg(Word("log".into())).arg(Flag("--all")).arg(Count("-n", window)).arg(Flag("-p")).run(stop)?.stdout`.
  3. `ids = GitInvocation::new(repo).arg(Word("patch-id".into())).arg(Flag("--stable")).stdin(patches).run(stop)?.stdout` → each line `"<patch-id> <sha>"`; parse into `patch_id -> Vec<Oid>`, keeping only oids from step 1 (a merge's empty patch never appears here — that's expected, not an error).
  4. Group by patch-id; `groups` = every group with 2+ members, each ordered by its position in `oids` (step 1's order, for determinism); singletons dropped.
  5. `Ok(Duplicates { groups, window, complete })`.
  Any `RunError` from any of the three calls short-circuits with that error (the caller renders it as "duplicates: not checked — <line>", see below).
- `crates/graft/src/jobs/radar.rs` (extends the file): `Task::DuplicatesProbe { window: u32 }`, `Product::DuplicatesProbe(Result<Arc<Duplicates>, String>)`, `Command::DuplicatesProbe(JobId, u32)`; `run_duplicates_probe` wraps `probe_duplicates`, `window = 500`.
- `crates/graft/src/update/radar.rs` (extends `RadarState`): `pub duplicates: Loading<Arc<Duplicates>>`, `pub duplicate_findings: std::collections::HashMap<Oid, Finding>`. Trigger: `Product::RadarContext(Ok(_))` also emits one `Command::DuplicatesProbe(id, 500)` (fired once per repo open/refresh, not per selection). `Product::DuplicatesProbe(Ok(d))`: `duplicates = Ready(d.clone())`; `duplicate_findings` rebuilt once via `findings_from_duplicates(&d, &policy)` indexed by each `Finding::at`. `Err` → `Failed(msg)`.
- `crates/graft/src/view/radar.rs` (extends `findings_for`): if `state.radar.duplicate_findings.get(&id)` is `Some(f)`, push `f.clone()`.
- Tests: `probe_duplicates` on the `cherry-pick-duplicates` fixture (exact groups from its known layout — read the count from `01-fixture-corpus.md`'s "Shapes" table, don't discover it by running); on a fixture with no duplicates → `groups` empty, `complete: true` (small fixtures are always under 500); `update`/`view` wiring test (`Product::DuplicatesProbe` → `duplicate_findings` populated → `findings_for` returns it for a copy's id). MAP.md lines (`graft-ops`, `graft`).
- Capture: one of the duplicate commits selected, pane header showing `⚠ duplicate commit`, to `docs/shots/m2-duplicate-commit.txt`, ≤ 200 KB.

Out: evil-merge and force-push probes (their own tickets), any UI beyond the existing `Finding` rendering `radar-context-and-markers` already wired, windows other than 500 (no config key — PROMPT doesn't ask for one and the note doesn't add one).

## Signatures

```rust
// crates/graft-ops/src/mistake_radar/probes.rs
pub fn probe_duplicates(repo: &RepoHandle, window: u32, stop: &dyn Stop) -> Result<Duplicates, RunError>;

// crates/graft/src/jobs/radar.rs
pub fn run_duplicates_probe(repo: &RepoHandle, window: u32, stop: &dyn Stop) -> Result<Arc<Duplicates>, String>;
// Task::DuplicatesProbe { window: u32 }
// Product::DuplicatesProbe(Result<Arc<Duplicates>, String>)
// Command::DuplicatesProbe(JobId, u32)

// crates/graft/src/update/radar.rs (extends RadarState)
// RadarState: + pub duplicates: Loading<Arc<Duplicates>>, + pub duplicate_findings: std::collections::HashMap<Oid, Finding>
```

## Acceptance

- [ ] `probe_duplicates` on `cherry-pick-duplicates` returns the fixture's known group(s), `complete: true`.
- [ ] `probe_duplicates` on a fixture with no repeated content returns `groups: vec![]`.
- [ ] `Product::RadarContext(Ok)` triggers exactly one `DuplicatesProbe`; its answer populates `duplicate_findings` with one entry per copy.
- [ ] `findings_for` on a duplicate commit's id includes its `DuplicateCommit` finding, listing the other copies (via `Evidence::Duplicate`).
- [ ] `[radar] duplicate = off`: `findings_from_duplicates` (called from `mistake-radar-detection`, unchanged here) yields nothing, so `duplicate_findings` stays empty even though the probe still ran (the probe itself is policy-blind; only its findings are policy-gated — cheap enough not to skip the git calls, and simpler than threading policy into the probe).
- [ ] MAP.md: `graft-ops` (`probes::probe_duplicates`), `graft` (`jobs::radar`, `update::radar` additions).
- [ ] Capture at `docs/shots/m2-duplicate-commit.txt`, ≤ 200 KB; user judges by eye.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates in substance (`graft-ops` ~70 lines, `graft` ~90: jobs ~25, update ~40, view ~15). Tests ~120. ≤ 20 tool calls.

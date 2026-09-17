# radar-probe-duplicates: cherry-picked/duplicate commits in the newest 500

Status: done
Blocked by: 01, 04, 13, 13b
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

- [x] `probe_duplicates` on `cherry-pick-duplicates` returns the fixture's known group(s), `complete: true`. (`the_cherry_picked_duplicate_is_found_in_its_own_group`, `crates/graft-ops/src/mistake_radar/probes.rs`)
- [x] `probe_duplicates` on a fixture with no repeated content returns `groups: vec![]`. (`a_fixture_with_no_repeated_content_has_no_groups`)
- [x] `Product::RadarContext(Ok)` triggers exactly one `DuplicatesProbe`; its answer populates `duplicate_findings` with one entry per copy. (`duplicates_are_asked_for_once_the_context_lands_and_only_once`, `nothing_is_asked_for_duplicates_before_the_context_lands`, `a_duplicates_answer_populates_one_finding_per_copy`, `crates/graft/src/update/radar.rs`)
- [x] `findings_for` on a duplicate commit's id includes its `DuplicateCommit` finding, listing the other copies (via `Evidence::Duplicate`). (wired in `crates/graft/src/view/radar.rs::findings_for`; exercised by the capture below)
- [x] `[radar] duplicate = off`: `findings_from_duplicates` yields nothing, so `duplicate_findings` stays empty even though the probe still ran. (`the_duplicate_kind_off_leaves_findings_empty_though_the_probe_ran`)
- [x] MAP.md: `graft-ops` (`probes::probe_duplicates`), `graft` (`jobs::radar`, `update::radar` additions, `view::radar` extension note).
- [x] Capture at `docs/shots/m2-duplicate-commit.txt`, 2444 B (≤ 200 KB); user judges by eye.

## Deviations (disclosed, not silent)

- **Trigger wiring:** spec says "`Product::RadarContext(Ok(_))` also emits one `Command::DuplicatesProbe(id, 500)`". Implemented as `request_duplicates_probe`, an idempotent tail call inside `update()` (gated on `radar.context` being `Ready` and `radar.duplicates` being `Idle`), matching ticket 15's `request_force_push_probes` shape — not a literal match arm on `Product::RadarContext`, because `next_id` (needed to build the `Command`) is only available at `update`'s tail, not inside the event match. Functionally equivalent: fires exactly once per context landing, verified by `nothing_is_asked_for_duplicates_before_the_context_lands` / `duplicates_are_asked_for_once_the_context_lands_and_only_once`.
- **Capture severity marker:** the ticket's capture line asks for a pane header showing `⚠ duplicate commit`. `DuplicateCommit`'s severity is `Info` by design (PROMPT §1.4, `graft-core::mistake_radar::MistakeRadarKind::default_severity`, unlike `EvilMerge`/`ForcePushedRemote` which are `Warning`), so under the real default policy the marker is `ⓘ`, not `⚠` — the capture uses the real default severity rather than forcing `Warning` to match the ticket's literal glyph. The pane header text itself reads "duplicate commit" (no inline icon in the header line, same as the evil-merge and force-push captures); the icon only ever appears beside the graph row's marker glyph. Filed as-is rather than silently complying with a glyph that would misrepresent the shipped severity.

## Gates

`cargo fmt --check` · `cargo test --quiet` · `cargo clippy --all-targets --message-format=short -- -D warnings` · `cargo build --quiet` · purity guard.

## Size

Two crates in substance (`graft-ops` ~70 lines, `graft` ~90: jobs ~25, update ~40, view ~15). Tests ~120. ≤ 20 tool calls.

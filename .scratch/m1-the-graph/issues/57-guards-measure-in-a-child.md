# m1-the-graph/57 — Guards measure in a child process (peak RSS and footprint per fixture, not per test binary)

Status: ready
Build with: **Sonnet** (medium) — plumbing with an exact pass condition: one FFI read, one re-exec, one parse; the number either matches the script's or it doesn't.
Needs: none
Blocked by: none

## Why

`getrusage().ru_maxrss` and `ri_lifetime_max_phys_footprint` are **lifetime maxima of the process**. A ceiling test (`budgets_hold_* < 300 MB`) can only over-report under that, so it stays honest. A **slope** (peak on 100k minus peak on 10k) cannot: run after `budgets_hold_on_the_100k_fixture_without_a_commit_graph` (69 MiB) in the same test binary, both fixtures read 69 MiB and the slope is 0 — a false green. 56 reported the guard red at 237 B/commit from the script's per-process numbers; the full heavy run (`--test-threads=2`, alphabetical order puts the 100k tests first) would very likely show it green. That is finding 5's real bite: the 4× counters path is harness-only and `--gate` already sidesteps it, but a lifetime-max slope misleads a gate. Every slope guard measures in a fresh process from here on, and the 1M no-cg heap bar (ADR-0026, ≤ 100 B/commit of footprint) gets an in-process footprint reading it never had.

## Read first
- CLAUDE.md "Rules for every agent" (one heavy job at a time; laptop-safe tests)
- ADR-0026 "The gate at 1M" (the no-cg footprint bullet and the slope-guard bullet)
- `.scratch/m1-the-graph/issues/56-memory-curve-acceptance.md`: "Result so far" (the `--gate` path, `peak RSS now` lines), "For the M1 review" table header (what peak RSS and footprint mean there), the slope bullet, and "Spec-only pass 2" (finding 5's ruling)
- `crates/graft-fixtures/MAP.md` (`measure`, `peak_rss`, `Measured`, `MemoryError`)
- `crates/graft/MAP.md` `examples/budgets` line
- TESTING.md "The heavy tier" (the `MEASURING` convention, the commands at lines ~54–64)

## Scope
In:
- **`graft_fixtures::footprint()`** — this process's physical footprint, current and lifetime peak, via `proc_pid_rusage(getpid(), RUSAGE_INFO_V4)` (`ri_phys_footprint`, `ri_lifetime_max_phys_footprint`). macOS only; `Err(MemoryError::Unsupported)` elsewhere (add the variant if missing). Use the `libc` crate's `rusage_info_v4` if present; otherwise a local `#[repr(C)]` declaration with a comment naming the header (`<sys/resource.h>`, `struct rusage_info_v4`). One `unsafe` block, in the fixtures crate only. Unit test: a 64 MiB `Vec<u8>` touched page by page raises `peak` by ≥ 60 MiB (same shape as `measure`'s existing test).
- **`budgets --gate` and `--phases` print footprint** next to every `peak RSS now` line: `footprint now: <current MiB> / peak <peak MiB>`. `scripts/memory-curve.sh` keeps `/usr/bin/time -l` too; the two peaks must agree within 5 % on one 100k cell (report both).
- **`gated_in_child`** in `tests/it/budget_run.rs`: re-exec `std::env::current_exe()` with `--exact heavy::child_gated_run --nocapture` and `GRAFT_CHILD_REPO=<path>`; the child test runs exactly what `budgets --gate` runs on that path and prints one line `child_peak rss=<bytes> footprint_peak=<bytes> footprint_after_g=<bytes> first_paint_ms=<n>`; the parent parses that line. `child_gated_run` without the variable is a no-op pass. Any other exit or a missing line is `Err(ChildError)`, never a default. Hold `MEASURING` around the spawn.
- **Move `heavy::peak_rss_slope_per_commit_with_a_commit_graph` onto `gated_in_child`** (10k cg, then 100k cg, each a fresh process). Bar unchanged: ≤ 150 B/commit. Its failure message prints both peaks and the slope.
- **Prove isolation:** run the guard alone, then run it in the same invocation after `budgets_hold_on_the_100k_fixture_without_a_commit_graph` (`cargo test --release -p graft --features heavy-tests --test it -- heavy::budgets_hold_on_the_100k_fixture_without_a_commit_graph heavy::peak_rss_slope --test-threads=1 --nocapture`); the slope must be the same within 10 % both times and within 10 % of `scripts/memory-curve.sh`'s 10k-cg/100k-cg cells. Also record what the *old* in-process guard read in that second ordering (one run, before the move) — that number is the evidence for the KNOWN_GAPS row 56 writes.

Out: any change to what is measured (the gated path), any bar, the counters path (`order_bytes`, `cold_jump`, `far_jump`), Linux footprint.

## Signatures
```rust
// graft-fixtures (lib)
/// Physical footprint of this process: anonymous + compressed pages, in bytes — what
/// Activity Monitor's "Memory" and `/usr/bin/time -l`'s "peak memory footprint" report.
pub struct Footprint { pub current: u64, pub peak: u64 }
pub fn footprint() -> Result<Footprint, MemoryError>;   // Err(MemoryError::Unsupported) off macOS

// graft tests/it/budget_run.rs (shared by examples/budgets and tests/it/heavy.rs)
pub struct ChildPeak { pub peak_rss: u64, pub footprint_peak: u64, pub footprint_after_g: u64, pub first_paint: std::time::Duration }
pub enum ChildError { Spawn(std::io::Error), Exit { status: std::process::ExitStatus, stderr_tail: String }, NoReport }
/// Runs the gated path (open → first paint → G → g) on `repo` in a fresh copy of this test
/// binary, so the peaks are that run's own and not this process's lifetime maximum.
pub fn gated_in_child(repo: &std::path::Path) -> Result<ChildPeak, ChildError>;
```
`heavy::child_gated_run` (a `#[test]`, heavy feature) is the child side. Calls: `graft_fixtures::{peak_rss, footprint}`, the existing gated-run function `budgets --gate` uses.

## Acceptance
- [ ] `graft_fixtures::footprint()` exists, unit-tested; MAP.md line added; `MemoryError::Unsupported` documented.
- [ ] `budgets --gate` / `--phases` print `footprint now:` per stage; one 100k cell shows in-process peak within 5 % of `/usr/bin/time -l` (numbers in the ticket).
- [ ] `gated_in_child` + `heavy::child_gated_run`; a broken child (bad path) is `Err`, the test fails with the stderr tail, nothing is swallowed.
- [ ] Slope guard on `gated_in_child`; alone vs after-the-100k-test: same slope within 10 %; within 10 % of the script's cells. The old in-process reading in the second ordering recorded in the ticket.
- [ ] The guard is still **red** (≈ 237 B/commit) — this ticket fixes how it measures, not what it finds; say so in the report.
- [ ] TESTING.md "The heavy tier": one paragraph — slope guards run each fixture in a child; why.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`; the named heavy tests above, one invocation at a time, release.

## Size
One context, under ~80k: FFI + one spawn/parse + one test moved; the heavy runs are 10k/100k only (seconds each).

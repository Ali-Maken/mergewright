# m1-the-graph/27d — git/git memory: land C and D, get real margin, guard it

Status: done
Build with: **Opus** (high) — remainder of 27b, same scope and model
Blocked by: 27c
Leaves runnable: yes — git/git peak RSS under 300 MB with real margin, with and without commit-graph; heavy tier catches the regression class.

Why: 27b landed fix B (no cg RSS 305 → 290–294, footprint 204 → 173; cg 255 → 223–226). 6–10 MiB margin isn't enough. C hung on a race that 27c fixes.

## Read first
- Ticket 27b: "Handoff (what remains)" item 2, "Continuation 1" (measurements table, fixes, Next steps 2–5). Measurement script `target/27b/meas.sh` → `docs/perf/27b.txt` § `target/27b/meas.sh`.
- Ticket 27 "Results" table (to update)
- `crates/graft-git/MAP.md`, `crates/graft/MAP.md`

## Scope
**Update from 27c (conductor, 2026-09-13):** C's call site landed in 27c (commits f791d7b, 983184c): no cg peak RSS 250 MiB, footprint 162–164, 3 runs, no hang. Item 1 is now: measure C with a commit-graph (3×) and fill the before/after table from 27c's numbers.

In (27b "Next steps" 2–5):
1. Re-apply C's call site (`walk.next_ids` in `graft::history::topological_order`); measure 3× RSS and footprint, cg and no cg.
2. Wire D (`handle_with_object_cache` for the promotion handle); measure 3×, watch `G` time.
3. A′ only if still short of margin; judge on RSS.
4. Heavy-tier regression guard (27b scope item 3): a synthetic fixture option with realistic message/identity sizes, so the 100k heavy test sees what git/git does. If this is a ticket's worth alone, stop at that seam and say so.
5. Update 27's results table (git/git rows) and the KNOWN_GAPS rows 27/27b touched. Decide `prototypes/heap-profile-27b/Cargo.lock` (untracked): commit or ignore, per how other prototypes do it.

Out: changing the gate measure (RSS vs footprint is with the user; keep `getrusage` RSS). Behaviour changes to meet the budget: stop and ask.

## Acceptance
- [ ] Before/after table per fix, 3 runs, RSS + footprint + `G` time, cg and no cg.
- [ ] git/git no cg peak RSS under 300 MB with a margin you state and justify.
- [ ] Heavy-tier guard asserts, proven to fail by tightening; or a stated seam.
- [ ] Heavy tier once, alone, at the end; gates pass.

## Gates
`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo clippy --all-targets --features heavy-tests -- -D warnings` · `cargo test`

## Size
Under ~80k tokens. Seam: (a) C + D + A′ measured, (b) heavy-tier guard + tables.

## Result, part (a): C + D measured (2026-09-13, builder-opus)

Release, `examples/budgets` on git/git (85,649 commits), each run alone, 3 runs per cell. RSS = `getrusage` peak (the gate); footprint = `/usr/bin/time -l`. No cg = `target/27b/nocg.git` (commit-graph removed, see `docs/perf/27b.txt` § `target/27b/nocg.git` for regeneration). Script `target/27b/meas.sh`, raw `target/27b/meas/summary.txt` (labels `before`, `B`, `C27d`, `D`) → `docs/perf/27b.txt` §§ `target/27b/meas.sh`, `target/27b/meas/summary.txt`; 27c's own C runs in `target/27c/nocg-run*.txt`.

| Fix | no cg RSS | no cg footprint | no cg G | cg RSS | cg footprint | cg G |
|---|---|---|---|---|---|---|
| before (27b) | 303.7–306.7 | 202.8–205.7 | 1.26 s | 255.3–255.8 | 159.0–162.6 | 0.54 s |
| B (27b) | 290.3–294.4 | 173.1–174.2 | 1.25 s | 222.6–225.7 | 129.7–131.9 | 0.53–0.57 s |
| C (27c runs) | 250.2–250.7 | 162–164 | 0.88 s | — | — | — |
| C (27d re-run) | 248.8–250.4 | 154.2–163.8 | 0.86–0.87 s | 223.1–225.9 | 130.2–132.8 | 0.52–0.56 s |
| **D landed** | **219.1–221.5** | **141.5–142.3** | **0.85–0.86 s** | **223.6–225.3** | **130.5–132.6** | **0.53–0.54 s** |

- **C** (promotion walk reads ids only): −40 MiB no cg. No effect with a commit-graph, as expected: that order is not provisional, so no promotion runs.
- **D** (promotion handle `handle_with_object_cache(0)`, `graft::history::topological_order`): −29 MiB no cg, `G` slightly faster (the cache was never hit). No effect with cg, same reason.
- **A′ not tried:** not needed.
- **Margin:** worst of all 12 runs is 225.9 MiB (cg), so ≥ 74 MiB (25 %) under 300. Justification: run-to-run spread is ≤ 3 MiB; the next-largest step seen in this series was 40 MiB (C); git/git grows a few hundred commits a month (≈ a few KB of order per commit), so 74 MiB absorbs normal repo growth and allocator noise with room for one regression of C's size before the gate trips.

## Result, part (b): heavy-tier guard + tables (2026-09-13, builder-opus)

**Guard added.** `graft_fixtures::synthetic::Scale::with_realistic_commits()` / `CommitSize::{Small, Realistic}`: the same history shape with commit objects sized after git/git (measured here on the clone: mean 726 B, median 577, p90 1262; identities ~35 B). Messages are subject + 2–24 wrapped body lines + `Signed-off-by`; 1,000 identities; both picked by commit number, so the fixture is deterministic. Small keeps its old cache name. New heavy tests: `graft-fixtures heavy::the_realistic_variant_has_git_git_sized_commits_on_the_same_shape` (same commit/merge counts as small; mean commit 500–1000 B; > 500 authors) and `graft heavy::budgets_hold_on_the_100k_realistic_fixture_{without,with}_a_commit_graph`.

**Proven to fail by tightening:** `PEAK_RSS_LIMIT` at 125 MiB → realistic no-cg fails "peak RSS 130 MiB is over the 125 MiB budget" (cg passed, under 125). Reverted.

**What it does and does not catch** (debug, no cg, each alone, `target/27b/meas/*-nocg-*.log` → `docs/perf/27b.txt` § `target/27b/meas/*-nocg-*.log`):

| 100k fixture | C+D reverted | C+D landed | delta |
|---|---|---|---|
| small | 144.4 MiB | 121.3 MiB | −23 |
| realistic | 157.4 MiB | 130.6 MiB | −27 |
| git/git (release, part a; B only vs B+C+D) | 290.3–294.4 MiB | 219.1–221.5 MiB | −70 |

The realistic sizes move the fixture only slightly toward git/git. **Seam stated: the 300 MB guard cannot see a C+D-sized regression**; message bytes aren't the main multiplier. Suspect lane width (git/git ~28 lanes; `Row.lanes` top holder in 27b). Logged in KNOWN_GAPS, not built (a new fixture shape is a scope decision).

**Found while running the tier:** four budget children in parallel pushed first paint to 170–182 ms (68–72 ms alone), 3 tests failed on the 150 ms check. Cause is CPU contention between measuring processes, not Graft: `heavy.rs` now serialises children with a `MEASURING` mutex. Heavy tier therefore ran twice (fail, then 757/757 pass).

**Tables:** ticket 27 Results rows for git/git updated; KNOWN_GAPS row "git/git without a commit-graph breaks the 300 MB gate" closed, new row "heavy tier cannot see a git/git-sized memory regression"; TESTING.md fixture table rows added. `prototypes/heap-profile-27b/Cargo.lock` committed (graph-spike's is tracked too).

Gates: fmt, clippy, clippy `heavy-tests`, `cargo test` 747/747; heavy tier 757/757.

## NOTES
- No compaction. ~95k tokens used (over the 80k target: the heavy-tier contention cost an extra run).
